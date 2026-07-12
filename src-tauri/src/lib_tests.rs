use super::*;
use pd2_x64_converter_core::ProgressPhase;
use std::fs;
use tempfile::tempdir;

#[test]
fn jobs_are_clamped_to_the_available_parallelism() {
  let available = max_jobs();

  assert_eq!(clamp_jobs(0), 1);
  assert_eq!(clamp_jobs(usize::MAX), available);
  assert_eq!(clamp_jobs(available), available);
}

#[test]
fn inspect_paths_classifies_files_and_folders() {
  let dir = tempdir().unwrap();
  let file = dir.path().join("asset.font");
  fs::write(&file, b"font").unwrap();

  let paths = inspect_paths_inner(vec![file.clone(), dir.path().to_path_buf()]).unwrap();

  assert_eq!(
    paths,
    [
      InputPath {
        path: file,
        is_file: true,
      },
      InputPath {
        path: dir.path().to_path_buf(),
        is_file: false,
      },
    ]
  );
}

#[test]
fn convert_command_does_not_write_report_by_default() {
  let dir = tempdir().unwrap();
  fs::write(dir.path().join("asset.texture"), b"texture").unwrap();
  let tokens = ActiveTokens::default();
  tauri::async_runtime::block_on(scan_folder_inner(
    ScanRequest {
      root: dir.path().to_path_buf(),
      jobs: 1,
      write_report: false,
    },
    None,
    &tokens,
  ))
  .unwrap();

  let manifest = tauri::async_runtime::block_on(convert_folder_inner(
    ConvertRequest {
      root: dir.path().to_path_buf(),
      jobs: 4,
      write_report: false,
      dry_run: false,
      excluded_asset_kinds: Vec::new(),
    },
    None,
    &tokens,
  ))
  .unwrap();

  assert!(manifest.report_path.is_none());
  assert!(!dir.path().join(".pd2-x64-converter").join("runs").exists());
  assert!(tokens.take_reviewed(&dir.path().to_path_buf()).is_none());
}

#[test]
fn export_report_writes_json_to_requested_path() {
  let dir = tempdir().unwrap();
  fs::write(dir.path().join("asset.texture"), b"texture").unwrap();
  let tokens = ActiveTokens::default();
  let manifest = tauri::async_runtime::block_on(scan_folder_inner(
    ScanRequest {
      root: dir.path().to_path_buf(),
      jobs: 1,
      write_report: false,
    },
    None,
    &tokens,
  ))
  .unwrap();
  let target_path = dir.path().join("report.json");

  let exported = export_report(ExportReportRequest {
    manifest,
    target_path: target_path.clone(),
  })
  .unwrap();
  let json: serde_json::Value = serde_json::from_slice(&fs::read(target_path).unwrap()).unwrap();

  assert_eq!(
    exported.report_path.as_deref(),
    json["report_path"].as_str()
  );
  assert_eq!(json["summary"]["unsupported"], 1);
  assert!(json["entries"].as_array().unwrap().is_empty());
  assert_eq!(exported.entries.len(), 1);
}

#[test]
fn cancel_commands_flip_active_tokens() {
  let tokens = ActiveTokens::default();
  let scan = CancelToken::default();
  let convert = CancelToken::default();
  tokens.set_scan(scan.clone());
  tokens.set_convert(convert.clone());

  cancel_scan_inner(&tokens);
  cancel_convert_inner(&tokens);

  assert!(scan.is_cancelled());
  assert!(convert.is_cancelled());
}

#[test]
fn progress_batcher_flushes_process_events_and_batches_terminal_updates() {
  let mut batcher = ProgressBatcher::new(Duration::from_millis(100), 512);
  let start = Instant::now();

  let first_process = batcher
    .push(
      progress_event(ProgressPhase::Process, 0, 2, "a.unit"),
      start,
    )
    .unwrap();
  assert_eq!(first_process.len(), 1);
  assert_eq!(first_process[0].phase, ProgressPhase::Process);

  assert!(
    batcher
      .push(
        progress_event(ProgressPhase::Stage, 1, 2, "a.unit"),
        start + Duration::from_millis(50)
      )
      .is_none()
  );

  let process_batch = batcher
    .push(
      progress_event(ProgressPhase::Process, 1, 2, "b.unit"),
      start + Duration::from_millis(60),
    )
    .unwrap();
  assert_eq!(process_batch.len(), 2);
  assert_eq!(process_batch[0].phase, ProgressPhase::Stage);
  assert_eq!(process_batch[1].phase, ProgressPhase::Process);

  assert!(
    batcher
      .push(
        progress_event(ProgressPhase::Stage, 1, 2, "b.unit"),
        start + Duration::from_millis(110),
      )
      .is_none()
  );
  let completion_batch = batcher
    .push(
      progress_event(ProgressPhase::Commit, 2, 2, "b.unit"),
      start + Duration::from_millis(120),
    )
    .unwrap();
  assert_eq!(completion_batch.len(), 2);
  assert_eq!(completion_batch[0].phase, ProgressPhase::Stage);
  assert_eq!(completion_batch[1].phase, ProgressPhase::Commit);
}

#[test]
fn progress_batcher_keeps_recent_events_bounded() {
  let mut batcher = ProgressBatcher::new(Duration::from_millis(100), 3);
  let start = Instant::now();

  for index in 0..5 {
    assert!(
      batcher
        .push(
          progress_event(ProgressPhase::Stage, index, 10, &format!("{index}.unit")),
          start
        )
        .is_none()
    );
  }

  let batch = batcher
    .push(
      progress_event(ProgressPhase::Stage, 5, 10, "5.unit"),
      start + Duration::from_millis(100),
    )
    .unwrap();

  assert_eq!(batch.len(), 3);
  assert_eq!(batch[0].current_path.as_deref(), Some("3.unit"));
  assert_eq!(batch[1].current_path.as_deref(), Some("4.unit"));
  assert_eq!(batch[2].current_path.as_deref(), Some("5.unit"));
}

fn progress_event(
  phase: ProgressPhase,
  processed: usize,
  total: usize,
  current_path: &str,
) -> ProgressEvent {
  ProgressEvent {
    phase,
    processed,
    total,
    current_path: Some(current_path.to_string()),
    message: None,
  }
}
