use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::Result;
use crate::files::{METADATA_DIR, relative};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunManifest {
  pub run_id: String,
  pub root: String,
  pub started_at_unix: u64,
  pub finished_at_unix: Option<u64>,
  pub status: RunStatus,
  pub dry_run: bool,
  pub non_restorable: bool,
  pub destructive_write_warning: String,
  pub report_path: Option<String>,
  pub summary: Summary,
  pub entries: Vec<ManifestEntry>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Summary {
  pub planned: usize,
  #[serde(default)]
  pub excluded: usize,
  pub converted: usize,
  pub already_x64: usize,
  pub unsupported: usize,
  pub warning: usize,
  pub failed: usize,
  pub cancelled: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
  pub index: usize,
  pub relative_path: String,
  pub asset_kind: AssetKind,
  pub layout_state: LayoutState,
  pub status: EntryStatus,
  pub warning: Option<String>,
  pub error: Option<String>,
  pub original_checksum: Option<String>,
  pub converted_checksum: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
  Scanned,
  Completed,
  CompletedWithFailures,
  Cancelled,
  Failed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
  Font,
  Animation,
  MassUnit,
  Model,
  Stream,
  SoundBank,
  ScriptData,
  TextureDependency,
  UnsupportedUnknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LayoutState {
  SupportedX32,
  AlreadyX64,
  TextScriptData,
  UnsupportedKnown,
  UnsupportedUnknown,
  InvalidSupported,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Ord, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub enum EntryStatus {
  Planned,
  Excluded,
  Converted,
  AlreadyX64,
  Unsupported,
  Warning,
  Failed,
  Cancelled,
}

pub fn destructive_write_warning() -> String {
  "Supported assets will be overwritten in place. This converter does not create backups or restore points.".to_string()
}

pub fn dry_run_warning() -> String {
  "Dry run: supported assets are converted into temporary files only. Original files are not replaced.".to_string()
}

pub(crate) fn write_manifest(root: &Path, manifest: &mut RunManifest) -> Result<()> {
  let report_dir = root.join(METADATA_DIR).join("runs");
  fs::create_dir_all(&report_dir)?;
  let report_path = report_dir.join(format!("{}.json", manifest.run_id));
  manifest.report_path = Some(relative(root, &report_path)?);
  let bytes = serde_json::to_vec_pretty(manifest)?;
  fs::write(report_path, bytes)?;
  Ok(())
}

pub(crate) fn summarize(entries: &[ManifestEntry]) -> Summary {
  let mut counts: BTreeMap<EntryStatus, usize> = BTreeMap::new();
  for entry in entries {
    *counts.entry(entry.status).or_default() += 1;
  }
  Summary {
    planned: *counts.get(&EntryStatus::Planned).unwrap_or(&0),
    excluded: *counts.get(&EntryStatus::Excluded).unwrap_or(&0),
    converted: *counts.get(&EntryStatus::Converted).unwrap_or(&0),
    already_x64: *counts.get(&EntryStatus::AlreadyX64).unwrap_or(&0),
    unsupported: *counts.get(&EntryStatus::Unsupported).unwrap_or(&0),
    warning: *counts.get(&EntryStatus::Warning).unwrap_or(&0),
    failed: *counts.get(&EntryStatus::Failed).unwrap_or(&0),
    cancelled: *counts.get(&EntryStatus::Cancelled).unwrap_or(&0),
  }
}

pub(crate) fn now_unix() -> u64 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map(|duration| duration.as_secs())
    .unwrap_or_default()
}
