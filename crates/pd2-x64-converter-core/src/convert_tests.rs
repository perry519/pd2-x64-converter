use super::*;
use crate::assets::{legacy_font, looks_like_x64_font};
use std::sync::{Arc, Mutex};
use tempfile::tempdir;

#[test]
fn converts_a_single_file() {
  let dir = tempdir().unwrap();
  let path = dir.path().join("asset.font");
  fs::write(&path, legacy_font()).unwrap();

  let manifest = convert(
    ConvertOptions {
      root: path.clone(),
      jobs: 1,
      write_report: false,
      dry_run: false,
    },
    CancelToken::default(),
  )
  .unwrap();

  assert_eq!(manifest.root, path.display().to_string());
  assert_eq!(manifest.entries[0].relative_path, "asset.font");
  assert_eq!(manifest.entries[0].status, EntryStatus::Converted);
  assert!(looks_like_x64_font(&fs::read(path).unwrap()));
}

#[test]
fn excluded_asset_kinds_are_not_converted() {
  let dir = tempdir().unwrap();
  let path = dir.path().join("asset.font");
  let source = legacy_font();
  fs::write(&path, &source).unwrap();

  let manifest = convert_with_progress_excluding(
    ConvertOptions {
      root: dir.path().to_path_buf(),
      jobs: 1,
      write_report: false,
      dry_run: false,
    },
    &[AssetKind::Font],
    CancelToken::default(),
    ProgressReporter::default(),
  )
  .unwrap();

  assert_eq!(manifest.entries[0].status, EntryStatus::Excluded);
  assert_eq!(manifest.summary.excluded, 1);
  assert_eq!(fs::read(path).unwrap(), source);
}

#[test]
fn reviewed_manifest_skips_scan_phase() {
  let dir = tempdir().unwrap();
  let path = dir.path().join("asset.font");
  fs::write(&path, legacy_font()).unwrap();
  let mut reviewed = scan_with_progress(
    ScanOptions {
      root: dir.path().to_path_buf(),
      jobs: 1,
      write_report: false,
    },
    CancelToken::default(),
    ProgressReporter::default(),
  )
  .unwrap();
  reviewed.report_path = Some("old-scan-report.json".to_string());
  let events = Arc::new(Mutex::new(Vec::new()));
  let reporter_events = Arc::clone(&events);

  let manifest = convert_reviewed_with_progress_excluding(
    ConvertOptions {
      root: dir.path().to_path_buf(),
      jobs: 1,
      write_report: false,
      dry_run: false,
    },
    reviewed,
    &[],
    CancelToken::default(),
    ProgressReporter::new(move |event| reporter_events.lock().unwrap().push(event)),
  )
  .unwrap();

  assert_eq!(manifest.entries[0].status, EntryStatus::Converted);
  assert!(manifest.report_path.is_none());
  assert!(
    events
      .lock()
      .unwrap()
      .iter()
      .all(|event| event.phase != ProgressPhase::Scan)
  );
  assert!(looks_like_x64_font(&fs::read(path).unwrap()));
}

#[test]
fn reviewed_manifest_rejects_invalid_entries() {
  let dir = tempdir().unwrap();
  fs::write(dir.path().join("asset.font"), legacy_font()).unwrap();
  let reviewed = scan_with_progress(
    ScanOptions {
      root: dir.path().to_path_buf(),
      jobs: 1,
      write_report: false,
    },
    CancelToken::default(),
    ProgressReporter::default(),
  )
  .unwrap();
  let options = || ConvertOptions {
    root: dir.path().to_path_buf(),
    jobs: 1,
    write_report: false,
    dry_run: false,
  };

  let mut traversal = reviewed.clone();
  traversal.entries[0].relative_path = "../outside.font".to_string();
  assert!(
    convert_reviewed_with_progress_excluding(
      options(),
      traversal,
      &[],
      CancelToken::default(),
      ProgressReporter::default(),
    )
    .is_err()
  );

  let mut invalid_index = reviewed;
  invalid_index.entries[0].index = 1;
  assert!(
    convert_reviewed_with_progress_excluding(
      options(),
      invalid_index,
      &[],
      CancelToken::default(),
      ProgressReporter::default(),
    )
    .is_err()
  );
}

#[test]
fn convert_commits_staged_outputs_in_manifest_order() {
  let dir = tempdir().unwrap();
  fs::write(dir.path().join("a.font"), legacy_font()).unwrap();
  fs::write(dir.path().join("b.font"), legacy_font()).unwrap();
  let events = Arc::new(Mutex::new(Vec::new()));
  let reporter_events = Arc::clone(&events);

  let manifest = convert_with_progress(
    ConvertOptions {
      root: dir.path().to_path_buf(),
      jobs: 4,
      write_report: false,
      dry_run: false,
    },
    CancelToken::default(),
    ProgressReporter::new(move |event| reporter_events.lock().unwrap().push(event)),
  )
  .unwrap();

  let commit_paths: Vec<_> = events
    .lock()
    .unwrap()
    .iter()
    .filter(|event| event.phase == ProgressPhase::Commit)
    .map(|event| event.current_path.clone().unwrap_or_default())
    .collect();
  assert_eq!(commit_paths, ["a.font", "b.font"]);
  assert_eq!(manifest.summary.converted, 2);
}

#[test]
fn staged_failure_does_not_block_successful_commit() {
  let dir = tempdir().unwrap();
  let good = dir.path().join("a.font");
  let bad = dir.path().join("b.font");
  fs::write(&good, legacy_font()).unwrap();
  fs::write(&bad, legacy_font()).unwrap();
  let corrupt_after_scan = bad.clone();

  let manifest = convert_with_progress(
    ConvertOptions {
      root: dir.path().to_path_buf(),
      jobs: 4,
      write_report: false,
      dry_run: false,
    },
    CancelToken::default(),
    ProgressReporter::new(move |event| {
      if event.phase == ProgressPhase::Scan && event.processed == event.total {
        fs::write(&corrupt_after_scan, b"broken").unwrap();
      }
    }),
  )
  .unwrap();

  assert_eq!(manifest.status, RunStatus::CompletedWithFailures);
  assert_eq!(manifest.summary.converted, 1);
  assert_eq!(manifest.summary.failed, 1);
  assert!(looks_like_x64_font(&fs::read(good).unwrap()));
  assert_eq!(fs::read(bad).unwrap(), b"broken");
  assert_eq!(temp_leftovers(dir.path()), 0);
}

#[test]
fn cancel_before_commit_discards_staged_outputs() {
  let dir = tempdir().unwrap();
  let first = dir.path().join("a.font");
  let second = dir.path().join("b.font");
  fs::write(&first, legacy_font()).unwrap();
  fs::write(&second, legacy_font()).unwrap();
  let cancel = CancelToken::default();
  let reporter_cancel = cancel.clone();

  let manifest = convert_with_progress(
    ConvertOptions {
      root: dir.path().to_path_buf(),
      jobs: 4,
      write_report: false,
      dry_run: false,
    },
    cancel,
    ProgressReporter::new(move |event| {
      if event.phase == ProgressPhase::Stage && event.processed == event.total {
        reporter_cancel.cancel();
      }
    }),
  )
  .unwrap();

  assert_eq!(manifest.status, RunStatus::Cancelled);
  assert_eq!(manifest.summary.cancelled, 2);
  assert!(!looks_like_x64_font(&fs::read(first).unwrap()));
  assert!(!looks_like_x64_font(&fs::read(second).unwrap()));
  assert_eq!(temp_leftovers(dir.path()), 0);
}

#[test]
fn cancel_during_commit_stops_future_replacements_without_rollback() {
  let dir = tempdir().unwrap();
  let first = dir.path().join("a.font");
  let second = dir.path().join("b.font");
  fs::write(&first, legacy_font()).unwrap();
  fs::write(&second, legacy_font()).unwrap();
  let cancel = CancelToken::default();
  let reporter_cancel = cancel.clone();

  let manifest = convert_with_progress(
    ConvertOptions {
      root: dir.path().to_path_buf(),
      jobs: 4,
      write_report: false,
      dry_run: false,
    },
    cancel,
    ProgressReporter::new(move |event| {
      if event.phase == ProgressPhase::Commit && event.processed == 1 {
        reporter_cancel.cancel();
      }
    }),
  )
  .unwrap();

  assert_eq!(manifest.status, RunStatus::Cancelled);
  assert_eq!(manifest.summary.converted, 1);
  assert_eq!(manifest.summary.cancelled, 1);
  assert!(looks_like_x64_font(&fs::read(first).unwrap()));
  assert!(!looks_like_x64_font(&fs::read(second).unwrap()));
  assert_eq!(temp_leftovers(dir.path()), 0);
}

#[test]
fn cancellation_marks_remaining_work() {
  let dir = tempdir().unwrap();
  fs::write(dir.path().join("asset.font"), legacy_font()).unwrap();
  let cancel = CancelToken::default();
  cancel.cancel();

  let manifest = convert(
    ConvertOptions {
      root: dir.path().to_path_buf(),
      jobs: 1,
      write_report: false,
      dry_run: false,
    },
    cancel,
  )
  .unwrap();

  assert_eq!(manifest.status, RunStatus::Cancelled);
  assert_eq!(manifest.summary.cancelled, 1);
}

#[test]
fn dry_run_converts_and_discards_staged_outputs() {
  let dir = tempdir().unwrap();
  let font = dir.path().join("asset.font");
  let original = legacy_font();
  fs::write(&font, &original).unwrap();
  let events = Arc::new(Mutex::new(Vec::new()));
  let reporter_events = Arc::clone(&events);

  let manifest = convert_with_progress(
    ConvertOptions {
      root: dir.path().to_path_buf(),
      jobs: 4,
      write_report: false,
      dry_run: true,
    },
    CancelToken::default(),
    ProgressReporter::new(move |event| reporter_events.lock().unwrap().push(event)),
  )
  .unwrap();

  assert!(manifest.dry_run);
  assert!(!manifest.non_restorable);
  assert_eq!(manifest.summary.converted, 1);
  assert_eq!(manifest.entries[0].status, EntryStatus::Converted);
  assert!(manifest.entries[0].converted_checksum.is_some());
  assert_eq!(fs::read(&font).unwrap(), original);
  assert_eq!(temp_leftovers(dir.path()), 0);
  assert!(
    events
      .lock()
      .unwrap()
      .iter()
      .any(|event| event.phase == ProgressPhase::Cleanup)
  );
}

fn temp_leftovers(path: &Path) -> usize {
  fs::read_dir(path)
    .unwrap()
    .filter_map(|entry| entry.ok())
    .filter(|entry| entry.file_name().to_string_lossy().contains(".pd2x64-tmp"))
    .count()
}
