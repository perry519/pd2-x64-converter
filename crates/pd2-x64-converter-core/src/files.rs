use sha2::{Digest, Sha256};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use tempfile::TempPath;

use crate::error::{Error, Result};

pub(crate) const METADATA_DIR: &str = ".pd2-x64-converter";

pub(crate) fn normalized_root(root: &Path) -> Result<PathBuf> {
  if !root.exists() {
    return Err(Error::Invalid(format!("{} does not exist", root.display())));
  }
  if !root.is_dir() && !root.is_file() {
    return Err(Error::Invalid(format!(
      "{} is not a file or directory",
      root.display()
    )));
  }
  Ok(root.to_path_buf())
}

pub(crate) fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
  if dir.is_file() {
    out.push(dir.to_path_buf());
    return Ok(());
  }

  let mut entries = fs::read_dir(dir)?.collect::<io::Result<Vec<_>>>()?;
  entries.sort_by_key(|entry| entry.file_name());

  for entry in entries {
    let path = entry.path();
    let file_type = entry.file_type()?;
    if file_type.is_symlink() {
      continue;
    }
    if file_type.is_dir() {
      if entry.file_name() == METADATA_DIR {
        continue;
      }
      collect_files(&path, out)?;
    } else if file_type.is_file() {
      out.push(path);
    }
  }
  Ok(())
}

pub(crate) fn base_root(input: &Path) -> &Path {
  if input.is_file() {
    input.parent().unwrap_or_else(|| Path::new("."))
  } else {
    input
  }
}

#[cfg(test)]
fn write_replace(path: &Path, data: &[u8]) -> Result<()> {
  let temp_path = write_staged(path, data)?;
  commit_staged(path, temp_path)
}

pub(crate) fn write_staged(path: &Path, data: &[u8]) -> Result<TempPath> {
  let parent = path.parent().unwrap_or_else(|| Path::new("."));
  let file_name = path
    .file_name()
    .and_then(|value| value.to_str())
    .unwrap_or("asset");
  let mut file = tempfile::Builder::new()
    .prefix(&format!(".{file_name}."))
    .suffix(".pd2x64-tmp")
    .tempfile_in(parent)?;
  file.write_all(data)?;
  file.as_file().sync_all()?;
  Ok(file.into_temp_path())
}

pub(crate) fn commit_staged(path: &Path, temp_path: TempPath) -> Result<()> {
  temp_path
    .persist(path)
    .map(|_| ())
    .map_err(|error| Error::Io(error.error))
}

pub(crate) fn relative(root: &Path, path: &Path) -> Result<String> {
  let stripped = path
    .strip_prefix(root)
    .map_err(|_| Error::Invalid(format!("{} is outside {}", path.display(), root.display())))?;
  Ok(
    stripped
      .components()
      .map(|component| component.as_os_str().to_string_lossy())
      .collect::<Vec<_>>()
      .join("/"),
  )
}

pub(crate) fn suffix(path: &Path) -> Option<String> {
  path
    .file_name()
    .and_then(|name| name.to_str())
    .and_then(|name| name.rsplit_once('.').map(|(_, ext)| format!(".{ext}")))
}

pub(crate) fn checksum(data: &[u8]) -> String {
  format!("{:x}", Sha256::digest(data))
}

#[cfg(test)]
#[path = "files_tests.rs"]
mod tests;
