use rayon::ThreadPoolBuilder;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::assets;
use crate::error::{Error, Result};
use crate::files::{base_root, collect_files, normalized_root, relative, suffix};
use crate::manifest::{
  AssetKind, EntryStatus, LayoutState, ManifestEntry, RunManifest, RunStatus, Summary,
  destructive_write_warning, now_unix, summarize, write_manifest,
};
use crate::progress::{CancelToken, ProgressEvent, ProgressPhase, ProgressReporter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanOptions {
  pub root: PathBuf,
  pub jobs: usize,
  pub write_report: bool,
}

pub fn scan(options: ScanOptions) -> Result<RunManifest> {
  scan_with_progress(options, CancelToken::default(), ProgressReporter::default())
}

pub fn scan_with_progress(
  options: ScanOptions,
  cancel: CancelToken,
  reporter: ProgressReporter,
) -> Result<RunManifest> {
  let input = normalized_root(&options.root)?;
  let root = base_root(&input).to_path_buf();
  let started = now_unix();
  let mut files = Vec::new();
  collect_files(&input, &mut files)?;
  files.retain(|path| suffix(path).as_deref() != Some(".model"));

  let total = files.len();
  let processed = AtomicUsize::new(0);
  let mut detected: Vec<_> = run_in_pool(options.jobs, || {
    files
      .par_iter()
      .enumerate()
      .map(|(index, path)| {
        let result = if cancel.is_cancelled() {
          cancelled_entry(&root, path, index)
        } else {
          detect_file(&root, path).map(|mut entry| {
            entry.index = index;
            entry
          })
        };
        let done = processed.fetch_add(1, Ordering::SeqCst) + 1;
        reporter.report(ProgressEvent {
          phase: ProgressPhase::Scan,
          processed: done,
          total,
          current_path: relative(&root, path).ok(),
          message: None,
        });
        (index, result)
      })
      .collect()
  })?;
  detected.sort_by_key(|(index, _)| *index);

  let mut entries = detected
    .into_iter()
    .map(|(_, result)| result)
    .collect::<Result<Vec<_>>>()?;
  if cancel.is_cancelled() {
    for entry in &mut entries {
      if entry.status == EntryStatus::Planned {
        entry.status = EntryStatus::Cancelled;
      }
    }
  }

  let mut manifest = RunManifest {
    run_id: format!("scan-{started}"),
    root: input.display().to_string(),
    started_at_unix: started,
    finished_at_unix: Some(now_unix()),
    status: if entries
      .iter()
      .any(|entry| entry.status == EntryStatus::Cancelled)
    {
      RunStatus::Cancelled
    } else {
      RunStatus::Scanned
    },
    dry_run: false,
    non_restorable: true,
    destructive_write_warning: destructive_write_warning(),
    report_path: None,
    summary: Summary::default(),
    entries,
  };
  manifest.summary = summarize(&manifest.entries);
  if options.write_report {
    write_manifest(&root, &mut manifest)?;
  }
  Ok(manifest)
}

pub(crate) fn run_in_pool<T, F>(jobs: usize, work: F) -> Result<T>
where
  T: Send,
  F: FnOnce() -> T + Send,
{
  let pool = ThreadPoolBuilder::new()
    .num_threads(jobs.max(1))
    .build()
    .map_err(|error| Error::Invalid(error.to_string()))?;
  Ok(pool.install(work))
}

fn cancelled_entry(root: &Path, path: &Path, index: usize) -> Result<ManifestEntry> {
  Ok(ManifestEntry {
    index,
    relative_path: relative(root, path)?,
    asset_kind: AssetKind::UnsupportedUnknown,
    layout_state: LayoutState::UnsupportedUnknown,
    status: EntryStatus::Cancelled,
    warning: None,
    error: None,
    original_checksum: None,
    converted_checksum: None,
  })
}

fn detect_file(root: &Path, path: &Path) -> Result<ManifestEntry> {
  let suffix = suffix(path);
  let relative_path = relative(root, path)?;
  let (asset_kind, layout_state, status, warning) = match suffix.as_deref() {
    Some(".font") => match fs::read(path).map(|data| assets::classify_font(&data))? {
      Ok(LayoutState::SupportedX32) => (
        AssetKind::Font,
        LayoutState::SupportedX32,
        EntryStatus::Planned,
        None,
      ),
      Ok(LayoutState::AlreadyX64) => (
        AssetKind::Font,
        LayoutState::AlreadyX64,
        EntryStatus::AlreadyX64,
        None,
      ),
      Ok(_) => unreachable!("font classifier returned non-font state"),
      Err(error) => (
        AssetKind::Font,
        LayoutState::InvalidSupported,
        EntryStatus::Warning,
        Some(error),
      ),
    },
    Some(".animation") => {
      match fs::read(path).map(|data| assets::classify_animation(&data, &relative_path))? {
        Ok(LayoutState::SupportedX32) => (
          AssetKind::Animation,
          LayoutState::SupportedX32,
          EntryStatus::Planned,
          None,
        ),
        Ok(LayoutState::AlreadyX64) => (
          AssetKind::Animation,
          LayoutState::AlreadyX64,
          EntryStatus::AlreadyX64,
          None,
        ),
        Ok(_) => unreachable!("animation classifier returned non-animation state"),
        Err(error) => (
          AssetKind::Animation,
          LayoutState::InvalidSupported,
          EntryStatus::Warning,
          Some(error.to_string()),
        ),
      }
    }
    Some(".massunit") => {
      match fs::read(path).map(|data| assets::classify_massunit(&data, &relative_path))? {
        Ok(LayoutState::SupportedX32) => (
          AssetKind::MassUnit,
          LayoutState::SupportedX32,
          EntryStatus::Planned,
          None,
        ),
        Ok(LayoutState::AlreadyX64) => (
          AssetKind::MassUnit,
          LayoutState::AlreadyX64,
          EntryStatus::AlreadyX64,
          None,
        ),
        Ok(_) => unreachable!("massunit classifier returned non-massunit state"),
        Err(error) => (
          AssetKind::MassUnit,
          LayoutState::InvalidSupported,
          EntryStatus::Warning,
          Some(error.to_string()),
        ),
      }
    }
    Some(".stream") => {
      match fs::read(path).map(|data| assets::classify_stream(&data, &relative_path))? {
        Ok(LayoutState::SupportedX32) => (
          AssetKind::Stream,
          LayoutState::SupportedX32,
          EntryStatus::Planned,
          None,
        ),
        Ok(LayoutState::AlreadyX64) => (
          AssetKind::Stream,
          LayoutState::AlreadyX64,
          EntryStatus::AlreadyX64,
          None,
        ),
        Ok(_) => unreachable!("stream classifier returned non-stream state"),
        Err(error) => (
          AssetKind::Stream,
          LayoutState::InvalidSupported,
          EntryStatus::Warning,
          Some(error.to_string()),
        ),
      }
    }
    Some(".bnk") => {
      match fs::read(path).map(|data| assets::classify_soundbank(&data, &relative_path))? {
        Ok(LayoutState::SupportedX32) => (
          AssetKind::SoundBank,
          LayoutState::SupportedX32,
          EntryStatus::Planned,
          None,
        ),
        Ok(LayoutState::AlreadyX64) => (
          AssetKind::SoundBank,
          LayoutState::AlreadyX64,
          EntryStatus::AlreadyX64,
          None,
        ),
        Ok(_) => unreachable!("soundbank classifier returned non-soundbank state"),
        Err(error) => (
          AssetKind::SoundBank,
          LayoutState::InvalidSupported,
          EntryStatus::Warning,
          Some(error.to_string()),
        ),
      }
    }
    Some(ext) if assets::is_scriptdata_suffix(ext) => {
      match fs::read(path).map(|data| assets::classify_scriptdata(&data, &relative_path))? {
        Ok(LayoutState::SupportedX32) => (
          AssetKind::ScriptData,
          LayoutState::SupportedX32,
          EntryStatus::Planned,
          None,
        ),
        Ok(layout_state) => (
          AssetKind::ScriptData,
          layout_state,
          EntryStatus::AlreadyX64,
          None,
        ),
        Err(error) => (
          AssetKind::ScriptData,
          LayoutState::InvalidSupported,
          EntryStatus::Warning,
          Some(error.to_string()),
        ),
      }
    }
    Some(".texture") => (
      AssetKind::TextureDependency,
      LayoutState::UnsupportedKnown,
      EntryStatus::Unsupported,
      Some("texture atlases are dependencies and are not rewritten".to_string()),
    ),
    _ => (
      AssetKind::UnsupportedUnknown,
      LayoutState::UnsupportedUnknown,
      EntryStatus::Unsupported,
      None,
    ),
  };

  Ok(ManifestEntry {
    index: 0,
    relative_path,
    asset_kind,
    layout_state,
    status,
    warning,
    error: None,
    original_checksum: None,
    converted_checksum: None,
  })
}

#[cfg(test)]
#[path = "scan_tests.rs"]
mod tests;
