use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use tempfile::TempPath;

use crate::assets;
use crate::error::{Error, Result};
use crate::files::{base_root, checksum, commit_staged, normalized_root, write_staged};
use crate::manifest::{
  AssetKind, EntryStatus, LayoutState, RunManifest, RunStatus, dry_run_warning, now_unix,
  summarize, write_manifest,
};
use crate::progress::{CancelToken, ProgressEvent, ProgressPhase, ProgressReporter};
use crate::scan::{ScanOptions, run_in_pool, scan_with_progress};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertOptions {
  pub root: PathBuf,
  pub jobs: usize,
  pub write_report: bool,
  pub dry_run: bool,
}

pub fn convert(options: ConvertOptions, cancel: CancelToken) -> Result<RunManifest> {
  convert_with_progress(options, cancel, ProgressReporter::default())
}

pub fn convert_with_progress(
  options: ConvertOptions,
  cancel: CancelToken,
  reporter: ProgressReporter,
) -> Result<RunManifest> {
  convert_with_progress_excluding(options, &[], cancel, reporter)
}

pub fn convert_with_progress_excluding(
  options: ConvertOptions,
  excluded_asset_kinds: &[AssetKind],
  cancel: CancelToken,
  reporter: ProgressReporter,
) -> Result<RunManifest> {
  let input = normalized_root(&options.root)?;
  let jobs = options.jobs.max(1);
  let manifest = scan_with_progress(
    ScanOptions {
      root: input,
      jobs,
      write_report: false,
    },
    cancel.clone(),
    reporter.clone(),
  )?;
  convert_reviewed_with_progress_excluding(
    options,
    manifest,
    excluded_asset_kinds,
    cancel,
    reporter,
  )
}

pub fn convert_reviewed_with_progress_excluding(
  options: ConvertOptions,
  mut manifest: RunManifest,
  excluded_asset_kinds: &[AssetKind],
  cancel: CancelToken,
  reporter: ProgressReporter,
) -> Result<RunManifest> {
  let input = normalized_root(&options.root)?;
  validate_reviewed_manifest(&input, &manifest)?;
  let root = base_root(&input).to_path_buf();
  let started = now_unix();
  let jobs = options.jobs.max(1);
  manifest.run_id = format!("convert-{started}");
  manifest.started_at_unix = started;
  manifest.finished_at_unix = None;
  manifest.report_path = None;
  manifest.status = RunStatus::Completed;
  manifest.dry_run = options.dry_run;
  if options.dry_run {
    manifest.non_restorable = false;
    manifest.destructive_write_warning = dry_run_warning();
  }

  for entry in &mut manifest.entries {
    if excluded_asset_kinds.contains(&entry.asset_kind)
      && matches!(entry.status, EntryStatus::Planned | EntryStatus::Warning)
    {
      entry.status = EntryStatus::Excluded;
    }
  }

  if cancel.is_cancelled() {
    cancel_planned_entries(&mut manifest);
    return finish_convert(root, manifest, options.write_report);
  }

  let work_items = manifest
    .entries
    .iter()
    .filter(|entry| entry.status == EntryStatus::Planned)
    .map(|entry| WorkItem {
      manifest_index: entry.index,
      relative_path: entry.relative_path.clone(),
      asset_kind: entry.asset_kind,
    })
    .collect::<Vec<_>>();
  let stage_total = work_items.len();
  let stage_processed = AtomicUsize::new(0);
  let mut outcomes: Vec<_> = run_in_pool(jobs, || {
    work_items
      .par_iter()
      .map(|item| {
        reporter.report(ProgressEvent {
          phase: ProgressPhase::Process,
          processed: stage_processed.load(Ordering::SeqCst),
          total: stage_total,
          current_path: Some(item.relative_path.clone()),
          message: None,
        });
        let outcome = stage_one(&root, item, &cancel);
        let done = stage_processed.fetch_add(1, Ordering::SeqCst) + 1;
        reporter.report(ProgressEvent {
          phase: ProgressPhase::Stage,
          processed: done,
          total: stage_total,
          current_path: Some(item.relative_path.clone()),
          message: None,
        });
        outcome
      })
      .collect()
  })?;
  outcomes.sort_by_key(StageOutcome::manifest_index);

  let mut staged = Vec::new();
  for outcome in outcomes {
    match outcome {
      StageOutcome::Staged(conversion) => staged.push(conversion),
      StageOutcome::Failed {
        manifest_index,
        error,
      } => mark_failed(&mut manifest, manifest_index, error),
      StageOutcome::Cancelled { manifest_index } => {
        mark_cancelled(&mut manifest, manifest_index);
      }
    }
  }

  if cancel.is_cancelled() {
    discard_staged(&mut manifest, staged, &reporter);
    return finish_convert(root, manifest, options.write_report);
  }

  if options.dry_run {
    finish_dry_run(&mut manifest, staged, &reporter);
    return finish_convert(root, manifest, options.write_report);
  }

  let commit_total = staged.len();
  let mut commit_processed = 0;
  let mut pending = staged.into_iter();
  while let Some(conversion) = pending.next() {
    if cancel.is_cancelled() {
      let mut leftovers = vec![conversion];
      leftovers.extend(pending);
      discard_staged(&mut manifest, leftovers, &reporter);
      break;
    }

    let relative_path = conversion.relative_path.clone();
    let manifest_index = conversion.manifest_index;
    let original_checksum = conversion.original_checksum.clone();
    let converted_checksum = conversion.converted_checksum.clone();
    let target = root.join(&conversion.relative_path);
    reporter.report(ProgressEvent {
      phase: ProgressPhase::Process,
      processed: commit_processed,
      total: commit_total,
      current_path: Some(relative_path.clone()),
      message: None,
    });
    match commit_staged(&target, conversion.temp_path) {
      Ok(()) => {
        let entry = &mut manifest.entries[manifest_index];
        entry.status = EntryStatus::Converted;
        entry.layout_state = LayoutState::AlreadyX64;
        entry.original_checksum = Some(original_checksum);
        entry.converted_checksum = Some(converted_checksum);
      }
      Err(error) => mark_failed(&mut manifest, manifest_index, error.to_string()),
    }
    commit_processed += 1;
    reporter.report(ProgressEvent {
      phase: ProgressPhase::Commit,
      processed: commit_processed,
      total: commit_total,
      current_path: Some(relative_path),
      message: None,
    });
  }

  finish_convert(root, manifest, options.write_report)
}

fn validate_reviewed_manifest(input: &Path, manifest: &RunManifest) -> Result<()> {
  if manifest.root != input.display().to_string() {
    return Err(Error::Invalid(
      "reviewed manifest does not match the conversion input".to_string(),
    ));
  }
  for (index, entry) in manifest.entries.iter().enumerate() {
    let path = Path::new(&entry.relative_path);
    if entry.index != index
      || path.as_os_str().is_empty()
      || path
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
      return Err(Error::Invalid(format!(
        "invalid reviewed manifest entry: {}",
        entry.relative_path
      )));
    }
  }
  Ok(())
}

fn finish_convert(
  root: PathBuf,
  mut manifest: RunManifest,
  write_report: bool,
) -> Result<RunManifest> {
  manifest.finished_at_unix = Some(now_unix());
  manifest.summary = summarize(&manifest.entries);
  if write_report {
    write_manifest(&root, &mut manifest)?;
  }
  Ok(manifest)
}

struct WorkItem {
  manifest_index: usize,
  relative_path: String,
  asset_kind: AssetKind,
}

struct StagedConversion {
  manifest_index: usize,
  relative_path: String,
  temp_path: TempPath,
  original_checksum: String,
  converted_checksum: String,
}

enum StageOutcome {
  Staged(StagedConversion),
  Failed {
    manifest_index: usize,
    error: String,
  },
  Cancelled {
    manifest_index: usize,
  },
}

impl StageOutcome {
  fn manifest_index(&self) -> usize {
    match self {
      Self::Staged(conversion) => conversion.manifest_index,
      Self::Failed { manifest_index, .. } | Self::Cancelled { manifest_index } => *manifest_index,
    }
  }
}

fn cancel_planned_entries(manifest: &mut RunManifest) {
  for entry in &mut manifest.entries {
    if entry.status == EntryStatus::Planned {
      entry.status = EntryStatus::Cancelled;
    }
  }
  manifest.status = RunStatus::Cancelled;
}

fn mark_cancelled(manifest: &mut RunManifest, manifest_index: usize) {
  let entry = &mut manifest.entries[manifest_index];
  if entry.status == EntryStatus::Planned {
    entry.status = EntryStatus::Cancelled;
  }
  manifest.status = RunStatus::Cancelled;
}

fn mark_failed(manifest: &mut RunManifest, manifest_index: usize, error: String) {
  let entry = &mut manifest.entries[manifest_index];
  entry.status = EntryStatus::Failed;
  entry.error = Some(error);
  if manifest.status != RunStatus::Cancelled {
    manifest.status = RunStatus::CompletedWithFailures;
  }
}

fn discard_staged(
  manifest: &mut RunManifest,
  staged: Vec<StagedConversion>,
  reporter: &ProgressReporter,
) {
  let total = staged.len();
  for (index, conversion) in staged.into_iter().enumerate() {
    mark_cancelled(manifest, conversion.manifest_index);
    reporter.report(ProgressEvent {
      phase: ProgressPhase::Cleanup,
      processed: index + 1,
      total,
      current_path: Some(conversion.relative_path),
      message: Some("discarded staged conversion".to_string()),
    });
  }
}

fn finish_dry_run(
  manifest: &mut RunManifest,
  staged: Vec<StagedConversion>,
  reporter: &ProgressReporter,
) {
  let total = staged.len();
  for (index, conversion) in staged.into_iter().enumerate() {
    let entry = &mut manifest.entries[conversion.manifest_index];
    entry.status = EntryStatus::Converted;
    entry.layout_state = LayoutState::AlreadyX64;
    entry.original_checksum = Some(conversion.original_checksum);
    entry.converted_checksum = Some(conversion.converted_checksum);
    reporter.report(ProgressEvent {
      phase: ProgressPhase::Cleanup,
      processed: index + 1,
      total,
      current_path: Some(conversion.relative_path),
      message: Some("discarded dry-run output".to_string()),
    });
  }
}

fn stage_one(root: &Path, item: &WorkItem, cancel: &CancelToken) -> StageOutcome {
  if cancel.is_cancelled() {
    return StageOutcome::Cancelled {
      manifest_index: item.manifest_index,
    };
  }
  stage_one_result(root, item, cancel).unwrap_or_else(|error| StageOutcome::Failed {
    manifest_index: item.manifest_index,
    error: error.to_string(),
  })
}

fn stage_one_result(root: &Path, item: &WorkItem, cancel: &CancelToken) -> Result<StageOutcome> {
  let path = root.join(&item.relative_path);
  let data = fs::read(&path)?;
  if cancel.is_cancelled() {
    return Ok(StageOutcome::Cancelled {
      manifest_index: item.manifest_index,
    });
  }
  let original_checksum = checksum(&data);
  let converted = convert_bytes(&path, item.asset_kind, &data)?;
  if cancel.is_cancelled() {
    return Ok(StageOutcome::Cancelled {
      manifest_index: item.manifest_index,
    });
  }
  let converted_checksum = checksum(&converted);
  let temp_path = write_staged(&path, &converted)?;
  Ok(StageOutcome::Staged(StagedConversion {
    manifest_index: item.manifest_index,
    relative_path: item.relative_path.clone(),
    temp_path,
    original_checksum,
    converted_checksum,
  }))
}

fn convert_bytes(path: &Path, asset_kind: AssetKind, data: &[u8]) -> Result<Vec<u8>> {
  assets::convert(path, asset_kind, data)
}

#[cfg(test)]
#[path = "convert_tests.rs"]
mod tests;
