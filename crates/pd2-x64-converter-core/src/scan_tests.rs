use super::*;
use crate::assets::legacy_font;
use crate::files::METADATA_DIR;
use std::fs;
use tempfile::tempdir;

#[test]
fn scans_a_single_file() {
  let dir = tempdir().unwrap();
  let path = dir.path().join("asset.font");
  fs::write(&path, legacy_font()).unwrap();

  let manifest = scan(ScanOptions {
    root: path.clone(),
    jobs: 1,
    write_report: false,
  })
  .unwrap();

  assert_eq!(manifest.root, path.display().to_string());
  assert_eq!(manifest.entries.len(), 1);
  assert_eq!(manifest.entries[0].relative_path, "asset.font");
  assert_eq!(manifest.entries[0].status, EntryStatus::Planned);
}

#[test]
fn scan_jobs_four_matches_serial_order_and_summary() {
  let dir = tempdir().unwrap();
  fs::write(dir.path().join("b.texture"), b"texture").unwrap();
  fs::write(dir.path().join("a.font"), legacy_font()).unwrap();
  fs::create_dir(dir.path().join(METADATA_DIR)).unwrap();
  fs::write(
    dir.path().join(METADATA_DIR).join("ignored.font"),
    legacy_font(),
  )
  .unwrap();

  let serial = scan(ScanOptions {
    root: dir.path().to_path_buf(),
    jobs: 1,
    write_report: false,
  })
  .unwrap();
  let parallel = scan(ScanOptions {
    root: dir.path().to_path_buf(),
    jobs: 4,
    write_report: false,
  })
  .unwrap();

  let serial_entries: Vec<_> = serial
    .entries
    .iter()
    .map(|entry| (entry.relative_path.as_str(), entry.status))
    .collect();
  let parallel_entries: Vec<_> = parallel
    .entries
    .iter()
    .map(|entry| (entry.relative_path.as_str(), entry.status))
    .collect();
  assert_eq!(parallel_entries, serial_entries);
  assert_eq!(parallel.summary.planned, 1);
  assert_eq!(parallel.summary.unsupported, 1);
}

#[test]
fn reporter_panic_does_not_fail_scan() {
  let dir = tempdir().unwrap();
  fs::write(dir.path().join("asset.texture"), b"texture").unwrap();

  let manifest = scan_with_progress(
    ScanOptions {
      root: dir.path().to_path_buf(),
      jobs: 1,
      write_report: false,
    },
    CancelToken::default(),
    ProgressReporter::new(|_| panic!("progress sink failed")),
  )
  .unwrap();

  assert_eq!(manifest.summary.unsupported, 1);
}

#[test]
fn cancellation_after_scan_marks_the_asset_cancelled() {
  let dir = tempdir().unwrap();
  fs::write(dir.path().join("asset.font"), legacy_font()).unwrap();
  let cancel = CancelToken::default();
  let reporter_cancel = cancel.clone();

  let manifest = scan_with_progress(
    ScanOptions {
      root: dir.path().to_path_buf(),
      jobs: 1,
      write_report: false,
    },
    cancel,
    ProgressReporter::new(move |event| {
      if event.phase == ProgressPhase::Scan {
        reporter_cancel.cancel();
      }
    }),
  )
  .unwrap();

  assert_eq!(manifest.entries[0].status, EntryStatus::Cancelled);
  assert_eq!(manifest.summary.cancelled, 1);
}

#[test]
fn scan_classifies_soundbank_layouts() {
  let dir = tempdir().unwrap();
  fs::write(dir.path().join("legacy.bnk"), soundbank(88)).unwrap();
  fs::write(dir.path().join("already.bnk"), soundbank(145)).unwrap();
  fs::write(dir.path().join("broken.bnk"), b"BKHD").unwrap();

  let manifest = scan(ScanOptions {
    root: dir.path().to_path_buf(),
    jobs: 1,
    write_report: false,
  })
  .unwrap();

  let legacy = manifest
    .entries
    .iter()
    .find(|entry| entry.relative_path == "legacy.bnk")
    .unwrap();
  assert_eq!(legacy.status, EntryStatus::Planned);
  assert_eq!(legacy.layout_state, LayoutState::SupportedX32);
  assert_eq!(
    serde_json::to_value(legacy.asset_kind).unwrap(),
    serde_json::json!("sound_bank")
  );

  let already = manifest
    .entries
    .iter()
    .find(|entry| entry.relative_path == "already.bnk")
    .unwrap();
  assert_eq!(already.asset_kind, AssetKind::SoundBank);
  assert_eq!(already.status, EntryStatus::AlreadyX64);
  assert_eq!(already.layout_state, LayoutState::AlreadyX64);

  let broken = manifest
    .entries
    .iter()
    .find(|entry| entry.relative_path == "broken.bnk")
    .unwrap();
  assert_eq!(broken.asset_kind, AssetKind::SoundBank);
  assert_eq!(broken.status, EntryStatus::Warning);
  assert_eq!(broken.layout_state, LayoutState::InvalidSupported);
  assert!(broken.warning.is_some());
}

fn soundbank(version: u32) -> Vec<u8> {
  let mut bank = Vec::new();
  let mut bkhd = vec![0; if version == 145 { 40 } else { 24 }];
  bkhd[..4].copy_from_slice(&version.to_le_bytes());
  push_chunk(&mut bank, b"BKHD", &bkhd);
  let mut hirc = Vec::new();
  hirc.extend_from_slice(&1_u32.to_le_bytes());
  hirc.push(4);
  hirc.extend_from_slice(&(if version == 145 { 5_u32 } else { 8 }).to_le_bytes());
  hirc.extend_from_slice(&1_u32.to_le_bytes());
  if version == 145 {
    hirc.push(0);
  } else {
    hirc.extend_from_slice(&0_u32.to_le_bytes());
  }
  push_chunk(&mut bank, b"HIRC", &hirc);
  bank
}

fn push_chunk(bank: &mut Vec<u8>, id: &[u8; 4], payload: &[u8]) {
  bank.extend_from_slice(id);
  bank.extend_from_slice(&u32::try_from(payload.len()).unwrap().to_le_bytes());
  bank.extend_from_slice(payload);
}
