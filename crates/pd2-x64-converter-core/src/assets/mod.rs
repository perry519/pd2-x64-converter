use std::path::Path;

use crate::error::{Error, Result, invalid};
use crate::files::suffix;
use crate::manifest::{AssetKind, LayoutState};

pub(crate) mod animation;
pub(crate) mod font;
pub(crate) mod massunit;
pub(crate) mod model;
pub(crate) mod scriptdata;
pub(crate) mod soundbank;
pub(crate) mod stream;

pub(crate) fn classify_font(data: &[u8]) -> std::result::Result<LayoutState, String> {
  font::classify(data)
}

pub(crate) fn classify_animation(data: &[u8], label: &str) -> Result<LayoutState> {
  animation::classify(data, label)
}

pub(crate) fn classify_massunit(data: &[u8], label: &str) -> Result<LayoutState> {
  massunit::classify(data, label)
}

pub(crate) fn classify_model(data: &[u8], label: &str) -> Result<LayoutState> {
  model::classify(data, label)
}

pub(crate) fn classify_stream(data: &[u8], label: &str) -> Result<LayoutState> {
  stream::classify(data, label)
}

pub(crate) fn classify_soundbank(data: &[u8], label: &str) -> Result<LayoutState> {
  soundbank::classify(data, label)
}

pub(crate) fn is_scriptdata_suffix(value: &str) -> bool {
  scriptdata::is_suffix(value)
}

pub(crate) fn classify_scriptdata(data: &[u8], label: &str) -> Result<LayoutState> {
  scriptdata::classify(data, label)
}

pub(crate) fn convert(path: &Path, asset_kind: AssetKind, data: &[u8]) -> Result<Vec<u8>> {
  let label = path.display().to_string();
  Ok(match asset_kind {
    AssetKind::Font => font::convert(data, &label)?,
    AssetKind::Animation => animation::convert(data, &label)?,
    AssetKind::MassUnit => massunit::convert(data, &label)?,
    AssetKind::Model => model::convert(data, &label)?,
    AssetKind::Stream => stream::convert(data, &label)?,
    AssetKind::SoundBank => soundbank::convert(data, &label)?,
    AssetKind::ScriptData => {
      scriptdata::convert(data, &label, suffix(path).as_deref().unwrap_or(""))?
    }
    _ => {
      return Err(Error::Invalid(format!(
        "{} is not convertible",
        path.display()
      )));
    }
  })
}

#[cfg(test)]
pub(crate) fn legacy_font() -> Vec<u8> {
  let mut data = vec![0; 92];
  put_u32_at(&mut data, 0, 1);
  put_u32_at(&mut data, 4, 1);
  put_u32_at(&mut data, 8, 92);
  put_u32_at(&mut data, 20, 1);
  put_u32_at(&mut data, 24, 1);
  put_u32_at(&mut data, 28, 104);
  put_u32_at(&mut data, 68, 112);
  put_u32_at(&mut data, 76, 512);
  put_u32_at(&mut data, 80, 256);
  data.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
  data.extend_from_slice(&[0, 0]);
  data.extend_from_slice(&[65, 0, 0, 0, 0, 0, 0, 0]);
  data.extend_from_slice(b"metadata-zS07");
  data
}

#[cfg(test)]
pub(crate) fn looks_like_x64_font(data: &[u8]) -> bool {
  font::looks_like_x64_font(data)
}

#[cfg(test)]
fn put_u32_at(data: &mut [u8], offset: usize, value: u32) {
  data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn checked_size(count: u64, width: usize, label: &str) -> Result<usize> {
  usize_from_u64(count, label)?
    .checked_mul(width)
    .ok_or_else(|| Error::Invalid(format!("{label}: byte count overflow")))
}

fn require_range(data: &[u8], offset: usize, size: usize, label: &str) -> Result<()> {
  if offset.checked_add(size).is_none_or(|end| end > data.len()) {
    return invalid!(
      "{label}: invalid range offset={offset} size={size} file_size={}",
      data.len()
    );
  }
  Ok(())
}

fn read_u16(data: &[u8], offset: usize, label: &str) -> Result<u16> {
  require_range(data, offset, 2, label)?;
  Ok(u16::from_le_bytes(read_array(data, offset, label)?))
}

fn read_u32(data: &[u8], offset: usize, label: &str) -> Result<u32> {
  require_range(data, offset, 4, label)?;
  Ok(u32::from_le_bytes(read_array(data, offset, label)?))
}

fn read_u64(data: &[u8], offset: usize, label: &str) -> Result<u64> {
  require_range(data, offset, 8, label)?;
  Ok(u64::from_le_bytes(read_array(data, offset, label)?))
}

fn read_u64_lossy(data: &[u8], offset: usize) -> Option<u64> {
  let bytes: [u8; 8] = data.get(offset..offset + 8)?.try_into().ok()?;
  Some(u64::from_le_bytes(bytes))
}

fn read_array<const N: usize>(data: &[u8], offset: usize, label: &str) -> Result<[u8; N]> {
  require_range(data, offset, N, label)?;
  Ok(
    data[offset..offset + N]
      .try_into()
      .expect("range was checked"),
  )
}

fn read_cstr(data: &[u8], offset: usize, label: &str) -> Result<Vec<u8>> {
  require_range(data, offset, 1, label)?;
  let end = data[offset..]
    .iter()
    .position(|byte| *byte == 0)
    .map(|position| offset + position)
    .ok_or_else(|| Error::Invalid(format!("{label}: unterminated string at {offset}")))?;
  Ok(data[offset..=end].to_vec())
}

fn usize_from_u64(value: u64, label: &str) -> Result<usize> {
  usize::try_from(value)
    .map_err(|_| Error::Invalid(format!("{label}: offset/count {value} does not fit usize")))
}

fn align8(buffer: &mut Vec<u8>) {
  buffer.resize(buffer.len() + ((8 - buffer.len() % 8) % 8), 0);
}

fn push_u16(out: &mut Vec<u8>, value: u16) {
  out.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
  out.extend_from_slice(&p32_bytes(value));
}

fn push_u64(out: &mut Vec<u8>, value: u64) {
  out.extend_from_slice(&p64_bytes(value));
}

fn write_u32_at(out: &mut [u8], offset: usize, value: u32) {
  out[offset..offset + 4].copy_from_slice(&p32_bytes(value));
}

fn write_u64_at(out: &mut [u8], offset: usize, value: u64) {
  out[offset..offset + 8].copy_from_slice(&p64_bytes(value));
}

fn p32_bytes(value: u32) -> [u8; 4] {
  value.to_le_bytes()
}

fn p64_bytes(value: u64) -> [u8; 8] {
  value.to_le_bytes()
}
