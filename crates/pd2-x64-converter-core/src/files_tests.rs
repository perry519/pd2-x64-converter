use super::*;
use std::fs;
use tempfile::tempdir;

#[test]
fn write_replace_uses_unique_temp_and_cleans_up() {
  let dir = tempdir().unwrap();
  let path = dir.path().join("asset.font");
  let old_tmp = dir.path().join("asset.font.pd2x64-tmp");
  fs::write(&path, b"old").unwrap();
  fs::write(&old_tmp, b"stale").unwrap();

  write_replace(&path, b"new").unwrap();

  assert_eq!(fs::read(&path).unwrap(), b"new");
  assert_eq!(fs::read(&old_tmp).unwrap(), b"stale");
  let leftovers = fs::read_dir(dir.path())
    .unwrap()
    .filter_map(|entry| entry.ok())
    .filter(|entry| entry.file_name().to_string_lossy().contains(".pd2x64-tmp"))
    .count();
  assert_eq!(leftovers, 1);
}
