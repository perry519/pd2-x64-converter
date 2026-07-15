use flate2::read::ZlibDecoder;

#[cfg(test)]
use flate2::{Compression, write::ZlibEncoder};
#[cfg(test)]
use std::io::Write;

use crate::error::{Error, Result, invalid};
use crate::manifest::LayoutState;

#[cfg(test)]
use super::push_u32;
use super::{
  align8, checked_size, p32_bytes, p64_bytes, read_array, read_cstr, read_u32, read_u64,
  require_range, usize_from_u64, write_u32_at, write_u64_at,
};

const ANIMATION_MAGIC: u32 = 0x0883_CC85;
const ANIMATION_X32_HEADER_SIZE: usize = 60;
const ANIMATION_X64_HEADER_SIZE: usize = 120;

pub(super) fn classify(data: &[u8], label: &str) -> Result<LayoutState> {
  if parse_animation_x64(data, label).is_ok() {
    return Ok(LayoutState::AlreadyX64);
  }
  if parse_animation_x32(data, label).is_ok() {
    return Ok(LayoutState::SupportedX32);
  }

  let raw = decompress_animation(data, label)?;
  if parse_animation_x64(&raw, label).is_ok() {
    return invalid!("{label}: wrapped x64 animations are not supported");
  }
  parse_animation_x32(&raw, label)?;
  Ok(LayoutState::SupportedX32)
}

pub(super) fn convert(data: &[u8], label: &str) -> Result<Vec<u8>> {
  if parse_animation_x64(data, label).is_ok() {
    return invalid!("{label}: animation is already raw x64");
  }
  if let Ok(parsed) = parse_animation_x32(data, label) {
    return build_animation_x64(&parsed);
  }

  let raw = decompress_animation(data, label).map_err(|source| Error::InvalidInput {
    context: format!("{label}: unsupported animation input"),
    source: Box::new(source),
  })?;
  if parse_animation_x64(&raw, label).is_ok() {
    return invalid!("{label}: wrapped x64 animations are not supported");
  }
  let parsed = parse_animation_x32(&raw, label).map_err(|source| Error::InvalidInput {
    context: format!("{label}: unsupported animation input"),
    source: Box::new(source),
  })?;
  build_animation_x64(&parsed)
}

#[derive(Debug)]
struct Event {
  time: [u8; 4],
  name: Vec<u8>,
}

#[derive(Debug)]
struct Motion {
  fmt: u32,
  unknown: u32,
  count: u32,
  data: Vec<u8>,
}

#[derive(Debug)]
struct Animation32 {
  unknown64: [u8; 8],
  duration: [u8; 4],
  names: Vec<Vec<u8>>,
  events: Vec<Event>,
  positions: Vec<Motion>,
  rotations: Vec<Motion>,
}

fn decompress_animation(data: &[u8], label: &str) -> Result<Vec<u8>> {
  if data.len() < 4 {
    return invalid!("{label}: animation is too small");
  }
  let trailer_offset = data.len() - 4;
  let expected_size = read_u32(data, trailer_offset, label)? as usize;
  let mut decoder = ZlibDecoder::new(&data[..trailer_offset]);
  let mut raw = Vec::new();
  std::io::Read::read_to_end(&mut decoder, &mut raw)?;
  if raw.len() != expected_size {
    return invalid!(
      "{label}: trailer size {expected_size} does not match decompressed size {}",
      raw.len()
    );
  }
  Ok(raw)
}

#[cfg(test)]
fn compress_animation(raw: &[u8]) -> Result<Vec<u8>> {
  let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
  encoder.write_all(raw)?;
  let mut compressed = encoder.finish()?;
  push_u32(&mut compressed, raw.len() as u32);
  Ok(compressed)
}

fn parse_animation_x32(raw: &[u8], label: &str) -> Result<Animation32> {
  require_range(raw, 0, ANIMATION_X32_HEADER_SIZE, label)?;
  let magic = read_u32(raw, 0, label)?;
  if magic != ANIMATION_MAGIC {
    return invalid!("{label}: unexpected animation magic {magic:#x}");
  }
  let embedded_size = read_u32(raw, 12, label)? as usize;
  if embedded_size != 0 && embedded_size != raw.len() {
    return invalid!(
      "{label}: embedded file size {embedded_size} does not match {}",
      raw.len()
    );
  }

  let name_count = read_u32(raw, 20, label)? as usize;
  let name_offset = read_u32(raw, 24, label)? as usize;
  let unknown_count = read_u32(raw, 28, label)? as usize;
  let unknown_offset = read_u32(raw, 32, label)? as usize;
  let event_count = read_u32(raw, 36, label)? as usize;
  let event_offset = read_u32(raw, 40, label)? as usize;
  let position_count = read_u32(raw, 44, label)? as usize;
  let position_offset = read_u32(raw, 48, label)? as usize;
  let rotation_count = read_u32(raw, 52, label)? as usize;
  let rotation_offset = read_u32(raw, 56, label)? as usize;

  if unknown_count != 0 {
    return invalid!("{label}: unknowns table count {unknown_count} is not supported");
  }
  for (section, count, offset, width) in [
    ("object names", name_count, name_offset, 4),
    ("events", event_count, event_offset, 8),
    ("positions", position_count, position_offset, 8),
    ("rotations", rotation_count, rotation_offset, 8),
  ] {
    require_range(raw, offset, count * width, &format!("{label}: {section}"))?;
  }
  require_range(raw, unknown_offset, 0, &format!("{label}: unknowns"))?;

  let mut names = Vec::with_capacity(name_count);
  for index in 0..name_count {
    names.push(read_cstr(
      raw,
      read_u32(raw, name_offset + index * 4, label)? as usize,
      &format!("{label}: object name {index}"),
    )?);
  }

  let mut events = Vec::with_capacity(event_count);
  for index in 0..event_count {
    let entry = event_offset + index * 8;
    events.push(Event {
      time: read_array(raw, entry, label)?,
      name: read_cstr(
        raw,
        read_u32(raw, entry + 4, label)? as usize,
        &format!("{label}: event {index}"),
      )?,
    });
  }

  let mut positions = Vec::with_capacity(position_count);
  for index in 0..position_count {
    let entry = position_offset + index * 8;
    let fmt = read_u32(raw, entry, label)?;
    let stride = position_stride(fmt)
      .ok_or_else(|| Error::Invalid(format!("{label}: unsupported position format {fmt:#x}")))?;
    let (unknown, count, data) = parse_motion_block(
      raw,
      read_u32(raw, entry + 4, label)? as usize,
      stride,
      &format!("{label}: position {index}"),
    )?;
    positions.push(Motion {
      fmt,
      unknown,
      count,
      data,
    });
  }

  let mut rotations = Vec::with_capacity(rotation_count);
  for index in 0..rotation_count {
    let entry = rotation_offset + index * 8;
    let fmt = read_u32(raw, entry, label)?;
    let stride = rotation_stride(fmt)
      .ok_or_else(|| Error::Invalid(format!("{label}: unsupported rotation format {fmt:#x}")))?;
    let (unknown, count, data) = parse_motion_block(
      raw,
      read_u32(raw, entry + 4, label)? as usize,
      stride,
      &format!("{label}: rotation {index}"),
    )?;
    rotations.push(Motion {
      fmt,
      unknown,
      count,
      data,
    });
  }

  Ok(Animation32 {
    unknown64: read_array(raw, 4, label)?,
    duration: read_array(raw, 16, label)?,
    names,
    events,
    positions,
    rotations,
  })
}

fn parse_animation_x64(raw: &[u8], label: &str) -> Result<()> {
  require_range(raw, 0, ANIMATION_X64_HEADER_SIZE, label)?;
  let magic = read_u32(raw, 0, label)?;
  if magic != ANIMATION_MAGIC {
    return invalid!("{label}: unexpected animation magic {magic:#x}");
  }
  if read_u32(raw, 12, label)? != 0 || read_u64(raw, 24, label)? != 0 {
    return invalid!("{label}: unsupported x64 animation header");
  }
  let embedded_size = usize_from_u64(read_u64(raw, 16, label)?, label)?;
  if embedded_size != raw.len() {
    return invalid!(
      "{label}: embedded file size {embedded_size} does not match {}",
      raw.len()
    );
  }
  if read_u32(raw, 36, label)? != 0 {
    return invalid!("{label}: unsupported x64 animation duration padding");
  }

  let mut sections = [(0_u64, 0_usize); 5];
  let mut header_offset = 40;
  for section in &mut sections {
    let count = read_u64(raw, header_offset, label)?;
    let offset = usize_from_u64(read_u64(raw, header_offset + 8, label)?, label)?;
    *section = (count, offset);
    header_offset += 16;
  }
  let [
    (name_count, name_offset),
    (unknown_count, unknown_offset),
    (event_count, event_offset),
    (position_count, position_offset),
    (rotation_count, rotation_offset),
  ] = sections;

  require_range(
    raw,
    name_offset,
    checked_size(name_count, 8, &format!("{label}: x64 object names"))?,
    &format!("{label}: x64 object names"),
  )?;
  for index in 0..usize_from_u64(name_count, label)? {
    read_cstr(
      raw,
      usize_from_u64(read_u64(raw, name_offset + index * 8, label)?, label)?,
      &format!("{label}: x64 object name {index}"),
    )?;
  }

  if unknown_count != 0 {
    return invalid!("{label}: unknowns table count {unknown_count} is not supported");
  }
  require_range(raw, unknown_offset, 0, &format!("{label}: x64 unknowns"))?;

  require_range(
    raw,
    event_offset,
    checked_size(event_count, 16, &format!("{label}: x64 events"))?,
    &format!("{label}: x64 events"),
  )?;
  for index in 0..usize_from_u64(event_count, label)? {
    let entry = event_offset + index * 16;
    read_cstr(
      raw,
      usize_from_u64(read_u64(raw, entry + 8, label)?, label)?,
      &format!("{label}: x64 event {index}"),
    )?;
  }

  parse_motion_table_x64(
    raw,
    position_count,
    position_offset,
    position_stride,
    &format!("{label}: x64 position"),
  )?;
  parse_motion_table_x64(
    raw,
    rotation_count,
    rotation_offset,
    rotation_stride,
    &format!("{label}: x64 rotation"),
  )?;
  Ok(())
}

fn build_animation_x64(animation: &Animation32) -> Result<Vec<u8>> {
  let mut out = vec![0; ANIMATION_X64_HEADER_SIZE];

  let names_offset = out.len();
  out.resize(out.len() + animation.names.len() * 8, 0);
  append_strings(&mut out, &animation.names, names_offset);
  align8(&mut out);

  let unknowns_offset = out.len();
  let events_offset = unknowns_offset;
  out.resize(out.len() + animation.events.len() * 16, 0);
  for (index, event) in animation.events.iter().enumerate() {
    let name_offset = out.len();
    let record_offset = events_offset + index * 16;
    out[record_offset..record_offset + 4].copy_from_slice(&event.time);
    write_u32_at(&mut out, record_offset + 4, 0);
    write_u64_at(&mut out, record_offset + 8, name_offset as u64);
    out.extend_from_slice(&event.name);
  }
  align8(&mut out);

  let positions_offset = out.len();
  out.resize(out.len() + animation.positions.len() * 16, 0);
  append_motion_section(&mut out, &animation.positions, positions_offset);

  let rotations_offset = out.len();
  out.resize(out.len() + animation.rotations.len() * 16, 0);
  append_motion_section(&mut out, &animation.rotations, rotations_offset);

  out[0..4].copy_from_slice(&p32_bytes(ANIMATION_MAGIC));
  out[4..12].copy_from_slice(&animation.unknown64);
  out[12..16].copy_from_slice(&p32_bytes(0));
  let output_len = out.len() as u64;
  out[16..24].copy_from_slice(&p64_bytes(output_len));
  out[24..32].copy_from_slice(&p64_bytes(0));
  out[32..36].copy_from_slice(&animation.duration);
  out[36..40].copy_from_slice(&p32_bytes(0));

  let mut header_offset = 40;
  for (count, offset) in [
    (animation.names.len(), names_offset),
    (0, unknowns_offset),
    (animation.events.len(), events_offset),
    (animation.positions.len(), positions_offset),
    (animation.rotations.len(), rotations_offset),
  ] {
    out[header_offset..header_offset + 8].copy_from_slice(&p64_bytes(count as u64));
    out[header_offset + 8..header_offset + 16].copy_from_slice(&p64_bytes(offset as u64));
    header_offset += 16;
  }

  Ok(out)
}

fn append_strings(buffer: &mut Vec<u8>, strings: &[Vec<u8>], table_offset: usize) {
  for (index, value) in strings.iter().enumerate() {
    let pointer = buffer.len();
    buffer[table_offset + index * 8..table_offset + index * 8 + 8]
      .copy_from_slice(&p64_bytes(pointer as u64));
    buffer.extend_from_slice(value);
  }
}

fn append_motion_section(buffer: &mut Vec<u8>, motions: &[Motion], table_offset: usize) {
  for (index, motion) in motions.iter().enumerate() {
    let block_offset = buffer.len();
    let data_offset = block_offset + 24;
    let record_offset = table_offset + index * 16;
    write_u32_at(buffer, record_offset, motion.fmt);
    write_u32_at(buffer, record_offset + 4, 0);
    write_u64_at(buffer, record_offset + 8, block_offset as u64);
    buffer.extend_from_slice(&p64_bytes(motion.unknown as u64));
    buffer.extend_from_slice(&p64_bytes(motion.count as u64));
    buffer.extend_from_slice(&p64_bytes(data_offset as u64));
    buffer.extend_from_slice(&motion.data);
    align8(buffer);
  }
}

fn parse_motion_block(
  raw: &[u8],
  offset: usize,
  stride: usize,
  label: &str,
) -> Result<(u32, u32, Vec<u8>)> {
  require_range(raw, offset, 12, label)?;
  let unknown = read_u32(raw, offset, label)?;
  let count = read_u32(raw, offset + 4, label)?;
  let data_offset = read_u32(raw, offset + 8, label)? as usize;
  let data_size = count as usize * stride;
  require_range(raw, data_offset, data_size, label)?;
  Ok((
    unknown,
    count,
    raw[data_offset..data_offset + data_size].to_vec(),
  ))
}

fn parse_motion_table_x64(
  raw: &[u8],
  count: u64,
  offset: usize,
  stride_for_format: fn(u32) -> Option<usize>,
  label: &str,
) -> Result<()> {
  require_range(raw, offset, checked_size(count, 16, label)?, label)?;
  for index in 0..usize_from_u64(count, label)? {
    let entry = offset + index * 16;
    let fmt = read_u32(raw, entry, label)?;
    let stride = stride_for_format(fmt)
      .ok_or_else(|| Error::Invalid(format!("{label} {index}: unsupported format {fmt:#x}")))?;
    parse_motion_block_x64(
      raw,
      usize_from_u64(read_u64(raw, entry + 8, label)?, label)?,
      stride,
      &format!("{label} {index}"),
    )?;
  }
  Ok(())
}

fn parse_motion_block_x64(raw: &[u8], offset: usize, stride: usize, label: &str) -> Result<()> {
  require_range(raw, offset, 24, label)?;
  let count = read_u64(raw, offset + 8, label)?;
  let data_offset = usize_from_u64(read_u64(raw, offset + 16, label)?, label)?;
  require_range(raw, data_offset, checked_size(count, stride, label)?, label)
}

fn position_stride(fmt: u32) -> Option<usize> {
  match fmt {
    0x1dc6_86e0 => Some(12),
    0x1196_cfb2 => Some(20),
    0x9d3d_e40d => Some(16),
    _ => None,
  }
}

fn rotation_stride(fmt: u32) -> Option<usize> {
  match fmt {
    0x9dfb_92b6 => Some(20),
    0x96ec_b85c => Some(8),
    0xe91a_69ba => Some(24),
    _ => None,
  }
}

#[cfg(test)]
pub(crate) fn legacy_empty_x32() -> Vec<u8> {
  let mut raw = vec![0; ANIMATION_X32_HEADER_SIZE];
  let raw_len = raw.len() as u32;
  put_u32(&mut raw, 0, ANIMATION_MAGIC);
  put_u32(&mut raw, 12, raw_len);
  for offset in [24, 32, 40, 48, 56] {
    put_u32(&mut raw, offset, ANIMATION_X32_HEADER_SIZE as u32);
  }
  compress_animation(&raw).unwrap()
}

#[cfg(test)]
fn put_u32(data: &mut [u8], offset: usize, value: u32) {
  data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
#[path = "animation_tests.rs"]
mod tests;
