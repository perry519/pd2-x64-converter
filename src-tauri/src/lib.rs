use pd2_x64_converter_core::{
  AssetKind, CancelToken, ConvertOptions, EntryStatus, ProgressEvent, ProgressPhase,
  ProgressReporter, RunManifest, RunStatus, ScanOptions,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use std::{fs, mem};
use tauri::{AppHandle, Emitter, State};

const PROGRESS_EVENT: &str = "pd2x64-progress";
const PROGRESS_EMIT_INTERVAL: Duration = Duration::from_millis(100);
const MAX_PROGRESS_BATCH_EVENTS: usize = 512;
const DEFAULT_JOBS: usize = 4;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanRequest {
  pub root: PathBuf,
  #[serde(default = "default_jobs")]
  pub jobs: usize,
  #[serde(default)]
  pub write_report: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvertRequest {
  pub root: PathBuf,
  #[serde(default = "default_jobs")]
  pub jobs: usize,
  #[serde(default)]
  pub write_report: bool,
  #[serde(default)]
  pub dry_run: bool,
  #[serde(default)]
  pub excluded_asset_kinds: Vec<AssetKind>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportReportRequest {
  pub manifest: RunManifest,
  pub target_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InputPath {
  pub path: PathBuf,
  pub is_file: bool,
}

fn default_jobs() -> usize {
  DEFAULT_JOBS
}

fn available_jobs() -> usize {
  std::thread::available_parallelism()
    .map(usize::from)
    .unwrap_or(1)
}

#[tauri::command]
fn max_jobs() -> usize {
  available_jobs()
}

fn clamp_jobs(jobs: usize) -> usize {
  jobs.clamp(1, available_jobs())
}

#[tauri::command]
async fn inspect_paths(paths: Vec<PathBuf>) -> Result<Vec<InputPath>, String> {
  tauri::async_runtime::spawn_blocking(move || inspect_paths_inner(paths))
    .await
    .map_err(|error| error.to_string())?
}

fn inspect_paths_inner(paths: Vec<PathBuf>) -> Result<Vec<InputPath>, String> {
  paths
    .into_iter()
    .map(|path| {
      if !path.is_file() && !path.is_dir() {
        return Err(format!("{} is not a file or directory", path.display()));
      }
      Ok(InputPath {
        is_file: path.is_file(),
        path,
      })
    })
    .collect()
}

#[derive(Default)]
struct ActiveTokens {
  scan: Mutex<Option<CancelToken>>,
  convert: Mutex<Option<CancelToken>>,
  reviewed: Mutex<HashMap<PathBuf, RunManifest>>,
}

#[cfg(feature = "e2e")]
#[derive(Default)]
struct E2EFolderPickerState {
  path: Mutex<Option<String>>,
}

impl ActiveTokens {
  fn set_scan(&self, token: CancelToken) {
    *self.scan.lock().unwrap() = Some(token);
  }

  fn set_convert(&self, token: CancelToken) {
    *self.convert.lock().unwrap() = Some(token);
  }

  fn clear_scan(&self, token: &CancelToken) {
    clear_token(&self.scan, token);
  }

  fn clear_convert(&self, token: &CancelToken) {
    clear_token(&self.convert, token);
  }

  fn cancel_scan(&self) {
    if let Some(token) = self.scan.lock().unwrap().as_ref() {
      token.cancel();
    }
  }

  fn cancel_convert(&self) {
    if let Some(token) = self.convert.lock().unwrap().as_ref() {
      token.cancel();
    }
  }

  fn clear_reviewed(&self, root: &PathBuf) {
    self.reviewed.lock().unwrap().remove(root);
  }

  fn set_reviewed(&self, root: PathBuf, manifest: RunManifest) {
    self.reviewed.lock().unwrap().insert(root, manifest);
  }

  fn take_reviewed(&self, root: &PathBuf) -> Option<RunManifest> {
    self.reviewed.lock().unwrap().remove(root)
  }
}

fn clear_token(slot: &Mutex<Option<CancelToken>>, token: &CancelToken) {
  let mut active = slot.lock().unwrap();
  if active
    .as_ref()
    .is_some_and(|active| active.is_same_source(token))
  {
    *active = None;
  }
}

fn progress_reporter(app: Option<AppHandle>) -> ProgressReporter {
  app.map_or_else(ProgressReporter::default, |app| {
    let batcher = Mutex::new(ProgressBatcher::new(
      PROGRESS_EMIT_INTERVAL,
      MAX_PROGRESS_BATCH_EVENTS,
    ));
    ProgressReporter::new(move |event| {
      if let Some(events) = batcher.lock().unwrap().push(event, Instant::now()) {
        let _ = app.emit(PROGRESS_EVENT, events);
      }
    })
  })
}

struct ProgressBatcher {
  interval: Duration,
  max_events: usize,
  last_emit: Option<Instant>,
  pending: Vec<ProgressEvent>,
}

impl ProgressBatcher {
  fn new(interval: Duration, max_events: usize) -> Self {
    Self {
      interval,
      max_events: max_events.max(1),
      last_emit: None,
      pending: Vec::new(),
    }
  }

  fn push(&mut self, event: ProgressEvent, now: Instant) -> Option<Vec<ProgressEvent>> {
    let is_completion = event.total > 0 && event.processed >= event.total;
    let is_process = event.phase == ProgressPhase::Process;
    if self.pending.len() >= self.max_events {
      self.pending.remove(0);
    }
    self.pending.push(event);

    let last_emit = self.last_emit.get_or_insert(now);
    if !is_process && !is_completion && now.saturating_duration_since(*last_emit) < self.interval {
      return None;
    }

    self.last_emit = Some(now);
    Some(mem::take(&mut self.pending))
  }
}

#[tauri::command(rename_all = "snake_case")]
async fn scan_folder(
  app: AppHandle,
  tokens: State<'_, ActiveTokens>,
  request: ScanRequest,
) -> Result<RunManifest, String> {
  scan_folder_inner(request, Some(app), &tokens).await
}

async fn scan_folder_inner(
  request: ScanRequest,
  app: Option<AppHandle>,
  tokens: &ActiveTokens,
) -> Result<RunManifest, String> {
  let root = request.root.clone();
  tokens.clear_reviewed(&root);
  let token = CancelToken::default();
  let worker_token = token.clone();
  tokens.set_scan(token.clone());
  let reporter = progress_reporter(app);
  let result = tauri::async_runtime::spawn_blocking(move || {
    pd2_x64_converter_core::scan_with_progress(
      ScanOptions {
        root: request.root,
        jobs: clamp_jobs(request.jobs),
        write_report: request.write_report,
      },
      worker_token,
      reporter,
    )
  })
  .await;
  let result = match result {
    Ok(result) => result.map_err(|error| error.to_string()),
    Err(error) => Err(error.to_string()),
  };
  if let Ok(manifest) = &result
    && manifest.status == RunStatus::Scanned
  {
    tokens.set_reviewed(root, manifest.clone());
  }
  tokens.clear_scan(&token);
  result
}

#[tauri::command(rename_all = "snake_case")]
async fn convert_folder(
  app: AppHandle,
  tokens: State<'_, ActiveTokens>,
  request: ConvertRequest,
) -> Result<RunManifest, String> {
  convert_folder_inner(request, Some(app), &tokens).await
}

async fn convert_folder_inner(
  request: ConvertRequest,
  app: Option<AppHandle>,
  tokens: &ActiveTokens,
) -> Result<RunManifest, String> {
  let reviewed = tokens
    .take_reviewed(&request.root)
    .ok_or_else(|| "scan results are no longer available; scan the input again".to_string())?;
  let token = CancelToken::default();
  let worker_token = token.clone();
  tokens.set_convert(token.clone());
  let reporter = progress_reporter(app);
  let result = tauri::async_runtime::spawn_blocking(move || {
    pd2_x64_converter_core::convert_reviewed_with_progress_excluding(
      ConvertOptions {
        root: request.root,
        jobs: clamp_jobs(request.jobs),
        write_report: request.write_report,
        dry_run: request.dry_run,
      },
      reviewed,
      &request.excluded_asset_kinds,
      worker_token,
      reporter,
    )
  })
  .await;
  let result = match result {
    Ok(result) => result.map_err(|error| error.to_string()),
    Err(error) => Err(error.to_string()),
  };
  tokens.clear_convert(&token);
  result
}

#[tauri::command(rename_all = "snake_case")]
fn cancel_scan(tokens: State<'_, ActiveTokens>) {
  cancel_scan_inner(&tokens);
}

fn cancel_scan_inner(tokens: &ActiveTokens) {
  tokens.cancel_scan();
}

#[tauri::command(rename_all = "snake_case")]
fn cancel_convert(tokens: State<'_, ActiveTokens>) {
  cancel_convert_inner(&tokens);
}

fn cancel_convert_inner(tokens: &ActiveTokens) {
  tokens.cancel_convert();
}

#[tauri::command(rename_all = "snake_case")]
fn export_report(request: ExportReportRequest) -> Result<RunManifest, String> {
  let mut manifest = request.manifest;
  manifest.report_path = Some(request.target_path.display().to_string());
  let mut report = manifest.clone();
  report
    .entries
    .retain(|entry| entry.status != EntryStatus::Unsupported);
  let bytes = serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?;
  fs::write(&request.target_path, bytes).map_err(|error| error.to_string())?;
  Ok(manifest)
}

#[cfg(feature = "e2e")]
#[tauri::command(rename_all = "snake_case")]
fn e2e_set_pick_folder(state: State<'_, E2EFolderPickerState>, path: Option<String>) {
  *state.path.lock().unwrap() = path;
}

#[cfg(feature = "e2e")]
#[tauri::command(rename_all = "snake_case")]
fn e2e_pick_folder(state: State<'_, E2EFolderPickerState>) -> Option<String> {
  state.path.lock().unwrap().take()
}

pub fn destructive_write_warning() -> String {
  pd2_x64_converter_core::destructive_write_warning()
}

pub fn run() {
  let builder = tauri::Builder::default()
    .manage(ActiveTokens::default())
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_opener::init())
    .plugin(tauri_plugin_store::Builder::new().build());

  #[cfg(feature = "e2e")]
  let builder = builder
    .manage(E2EFolderPickerState::default())
    .plugin(tauri_plugin_wdio_webdriver::init());

  #[cfg(feature = "e2e")]
  builder
    .invoke_handler(tauri::generate_handler![
      scan_folder,
      convert_folder,
      cancel_scan,
      cancel_convert,
      export_report,
      inspect_paths,
      max_jobs,
      e2e_set_pick_folder,
      e2e_pick_folder
    ])
    .run(tauri::generate_context!())
    .expect("error while running PD2 x64 Converter");

  #[cfg(not(feature = "e2e"))]
  builder
    .invoke_handler(tauri::generate_handler![
      scan_folder,
      convert_folder,
      cancel_scan,
      cancel_convert,
      export_report,
      inspect_paths,
      max_jobs
    ])
    .run(tauri::generate_context!())
    .expect("error while running PD2 x64 Converter");
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
