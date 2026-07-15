use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn scan_json_reports_sorted_candidates() {
  let dir = tempdir().unwrap();
  fs::write(dir.path().join("b.model"), b"model").unwrap();
  fs::write(dir.path().join("a.font"), legacy_font()).unwrap();

  let output = Command::new(env!("CARGO_BIN_EXE_pd2-x64-converter-cli"))
    .arg("scan")
    .arg(dir.path())
    .arg("--jobs")
    .arg("4")
    .arg("--json")
    .output()
    .unwrap();

  assert!(
    output.status.success(),
    "stderr={}",
    String::from_utf8_lossy(&output.stderr)
  );
  let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
  assert_eq!(json["summary"]["planned"], 1);
  assert_eq!(json["summary"]["unsupported"], 0);
  assert_eq!(json["entries"].as_array().unwrap().len(), 1);
  assert_eq!(json["entries"][0]["relative_path"], "a.font");
}

#[test]
fn convert_warns_and_keeps_unsupported() {
  let dir = tempdir().unwrap();
  fs::write(dir.path().join("asset.font"), legacy_font()).unwrap();
  fs::write(dir.path().join("asset.texture"), b"texture").unwrap();

  let output = Command::new(env!("CARGO_BIN_EXE_pd2-x64-converter-cli"))
    .arg("convert")
    .arg(dir.path())
    .arg("--json")
    .output()
    .unwrap();

  assert!(
    output.status.success(),
    "stderr={}",
    String::from_utf8_lossy(&output.stderr)
  );
  let stderr = String::from_utf8_lossy(&output.stderr);
  assert!(stderr.contains("overwritten in place"));
  assert_eq!(
    fs::read(dir.path().join("asset.texture")).unwrap(),
    b"texture"
  );
  let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
  assert_eq!(json["non_restorable"], true);
  assert_eq!(json["summary"]["converted"], 1);
}

#[test]
fn convert_dry_run_leaves_supported_files_unchanged() {
  let dir = tempdir().unwrap();
  let font = legacy_font();
  fs::write(dir.path().join("asset.font"), &font).unwrap();

  let output = Command::new(env!("CARGO_BIN_EXE_pd2-x64-converter-cli"))
    .arg("convert")
    .arg(dir.path())
    .arg("--dry-run")
    .arg("--json")
    .output()
    .unwrap();

  assert!(
    output.status.success(),
    "stderr={}",
    String::from_utf8_lossy(&output.stderr)
  );
  let stderr = String::from_utf8_lossy(&output.stderr);
  assert!(stderr.contains("Dry run"));
  assert!(!stderr.contains("overwritten in place"));
  assert_eq!(fs::read(dir.path().join("asset.font")).unwrap(), font);
  let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
  assert_eq!(json["dry_run"], true);
  assert_eq!(json["non_restorable"], false);
  assert_eq!(json["summary"]["converted"], 1);
}

#[test]
fn convert_does_not_write_report_by_default() {
  let dir = tempdir().unwrap();
  fs::write(dir.path().join("asset.font"), legacy_font()).unwrap();

  let output = Command::new(env!("CARGO_BIN_EXE_pd2-x64-converter-cli"))
    .arg("convert")
    .arg(dir.path())
    .arg("--dry-run")
    .output()
    .unwrap();

  assert!(
    output.status.success(),
    "stderr={}",
    String::from_utf8_lossy(&output.stderr)
  );
  assert!(
    !dir.path().join(".pd2-x64-converter").exists(),
    "convert should not write a report unless --report is set"
  );
}

#[test]
fn convert_writes_report_when_requested() {
  let dir = tempdir().unwrap();
  fs::write(dir.path().join("asset.font"), legacy_font()).unwrap();

  let output = Command::new(env!("CARGO_BIN_EXE_pd2-x64-converter-cli"))
    .arg("convert")
    .arg(dir.path())
    .arg("--dry-run")
    .arg("--report")
    .output()
    .unwrap();

  assert!(
    output.status.success(),
    "stderr={}",
    String::from_utf8_lossy(&output.stderr)
  );
  assert!(dir.path().join(".pd2-x64-converter").join("runs").exists());
}

fn legacy_font() -> Vec<u8> {
  let mut data = vec![0; 92];
  put_u32(&mut data, 0, 1);
  put_u32(&mut data, 4, 1);
  put_u32(&mut data, 8, 92);
  put_u32(&mut data, 20, 1);
  put_u32(&mut data, 24, 1);
  put_u32(&mut data, 28, 104);
  put_u32(&mut data, 68, 112);
  put_u32(&mut data, 76, 512);
  put_u32(&mut data, 80, 256);
  data.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
  data.extend_from_slice(&[0, 0]);
  data.extend_from_slice(&[65, 0, 0, 0, 0, 0, 0, 0]);
  data.extend_from_slice(b"metadata-zS07");
  data
}

fn put_u32(data: &mut [u8], offset: usize, value: u32) {
  data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
