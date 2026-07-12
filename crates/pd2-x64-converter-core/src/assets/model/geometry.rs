use crate::assets::{push_u32, read_u32, require_range, write_u32_at};
use crate::error::Result;

use super::container::{
  GEOMETRY_TYPE, Section, TOPOLOGY_TYPE, checked_add, checked_mul, invalid, usize_from_u32,
};

pub(super) const TRIANGLE_LIST: u32 = 3;

pub(super) struct Geometry<'a> {
  pub(super) vertex_count: usize,
  pub(super) descriptors: Vec<(u32, u32)>,
  pub(super) channels: Vec<&'a [u8]>,
  pub(super) trailer: &'a [u8],
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ModelLayout {
  X32,
  X64,
}

pub(super) fn parse_geometry<'a>(payload: &'a [u8], label: &str) -> Result<Geometry<'a>> {
  require_range(payload, 0, 8, &format!("{label}: geometry header"))?;
  let vertex_count = usize_from_u32(read_u32(payload, 0, label)?, label)?;
  let channel_count = usize_from_u32(read_u32(payload, 4, label)?, label)?;
  let descriptor_size = checked_mul(channel_count, 8, label)?;
  require_range(
    payload,
    8,
    descriptor_size,
    &format!("{label}: geometry descriptors"),
  )?;

  let mut descriptors = Vec::with_capacity(channel_count);
  for index in 0..channel_count {
    let offset = 8 + index * 8;
    descriptors.push((
      read_u32(payload, offset, label)?,
      read_u32(payload, offset + 4, label)?,
    ));
  }

  let mut offset = 8 + descriptor_size;
  let mut channels = Vec::with_capacity(channel_count);
  for &(channel_type, component) in &descriptors {
    let width = channel_width(channel_type).ok_or_else(|| {
      invalid(
        label,
        format!("unsupported geometry channel type {channel_type} for component {component}"),
      )
    })?;
    let size = checked_mul(vertex_count, width, label)?;
    require_range(
      payload,
      offset,
      size,
      &format!("{label}: geometry channel {channel_type}/{component}"),
    )?;
    channels.push(&payload[offset..offset + size]);
    offset = checked_add(offset, size, label)?;
  }
  let trailer = &payload[offset..];
  Ok(Geometry {
    vertex_count,
    descriptors,
    channels,
    trailer,
  })
}

pub(super) fn validate_geometry_trailers(
  sections: &[Section<'_>],
  layout: ModelLayout,
  label: &str,
) -> Result<()> {
  for (index, section) in sections
    .iter()
    .enumerate()
    .filter(|(_, section)| section.type_id == GEOMETRY_TYPE)
  {
    let geometry_label = format!("{label}: Geometry {}", section.ref_id);
    let trailer = parse_geometry(section.payload, &geometry_label)?.trailer;
    if matches!(trailer.len(), 0 | 8) {
      continue;
    }
    let duplicates_next = trailer
      .get(8..)
      .zip(sections.get(index + 1))
      .is_some_and(|(prefix, next)| matches_section_prefix(prefix, next));
    if layout != ModelLayout::X32 || !duplicates_next {
      return Err(invalid(
        &geometry_label,
        format!("unexpected {}-byte geometry trailer", trailer.len()),
      ));
    }
  }
  Ok(())
}

fn matches_section_prefix(prefix: &[u8], section: &Section<'_>) -> bool {
  let Ok(size) = u32::try_from(section.payload.len()) else {
    return false;
  };
  let expected = [
    section.type_id.to_le_bytes(),
    section.ref_id.to_le_bytes(),
    size.to_le_bytes(),
  ]
  .into_iter()
  .flatten()
  .chain(section.payload.iter().copied())
  .take(prefix.len());
  prefix.iter().copied().eq(expected)
}

pub(super) fn detect_layout(sections: &[Section<'_>], label: &str) -> Result<ModelLayout> {
  let mut legacy = false;
  let mut x64 = false;
  let mut geometries = 0usize;
  for section in sections
    .iter()
    .filter(|section| section.type_id == GEOMETRY_TYPE)
  {
    geometries += 1;
    let geometry = parse_geometry(
      section.payload,
      &format!("{label}: Geometry {}", section.ref_id),
    )?;
    for &(channel_type, component) in &geometry.descriptors {
      legacy |= (channel_type == 2 && (7..=14).contains(&component))
        || (channel_type == 7 && component == 15)
        || (channel_type == 3 && matches!(component, 2 | 17 | 20 | 21));
      x64 |= (channel_type == 9 && (7..=14).contains(&component))
        || (channel_type == 7 && component == 17)
        || (channel_type == 3 && component == 19)
        || (channel_type == 8 && matches!(component, 2 | 22 | 23));
    }
  }
  match (geometries, legacy, x64) {
    (0, _, _) => Err(invalid(label, "model has no Geometry sections")),
    (_, true, true) => Err(invalid(
      label,
      "model mixes legacy and x64 geometry declarations",
    )),
    (_, false, false) => Err(invalid(label, "could not identify model geometry layout")),
    (_, true, false) => Ok(ModelLayout::X32),
    (_, false, true) => Ok(ModelLayout::X64),
  }
}

pub(super) fn validate_topologies(sections: &[Section<'_>], label: &str) -> Result<()> {
  for section in sections
    .iter()
    .filter(|section| section.type_id == TOPOLOGY_TYPE)
  {
    validate_topology(
      section.payload,
      &format!("{label}: Topology {}", section.ref_id),
    )?;
  }
  Ok(())
}

pub(super) fn validate_topology(payload: &[u8], label: &str) -> Result<()> {
  require_range(payload, 0, 8, &format!("{label}: topology header"))?;
  let primitive_type = read_u32(payload, 0, label)?;
  let index_count = usize_from_u32(read_u32(payload, 4, label)?, label)?;
  if !matches!(primitive_type, 0 | TRIANGLE_LIST) {
    return Err(invalid(
      label,
      format!("unsupported topology primitive {primitive_type}"),
    ));
  }
  if !index_count.is_multiple_of(3) {
    return Err(invalid(
      label,
      format!("triangle-list index count {index_count} is not divisible by 3"),
    ));
  }
  let indices_size = checked_mul(index_count, 2, label)?;
  let grouping_offset = checked_add(8, indices_size, label)?;
  require_range(
    payload,
    grouping_offset,
    4,
    &format!("{label}: topology group count"),
  )?;
  let grouping_count = usize_from_u32(read_u32(payload, grouping_offset, label)?, label)?;
  let groupings_offset = checked_add(grouping_offset, 4, label)?;
  require_range(
    payload,
    groupings_offset,
    grouping_count,
    &format!("{label}: topology groupings"),
  )?;
  let trailer = &payload[groupings_offset + grouping_count..];
  if !matches!(trailer.len(), 0 | 8) {
    return Err(invalid(
      label,
      format!("unexpected {}-byte topology trailer", trailer.len()),
    ));
  }
  Ok(())
}

fn channel_width(channel_type: u32) -> Option<usize> {
  match channel_type {
    1 | 5 | 6 | 8 | 9 => Some(4),
    2 | 7 => Some(8),
    3 => Some(12),
    4 => Some(16),
    _ => None,
  }
}

pub(super) fn convert_topology(payload: &[u8], label: &str) -> Result<Vec<u8>> {
  validate_topology(payload, label)?;
  let mut converted = payload.to_vec();
  if read_u32(payload, 0, label)? == 0 {
    write_u32_at(&mut converted, 0, TRIANGLE_LIST);
  }
  Ok(converted)
}

pub(super) fn convert_geometry(payload: &[u8], label: &str) -> Result<Vec<u8>> {
  let geometry = parse_geometry(payload, label)?;
  let mut descriptors = Vec::with_capacity(geometry.descriptors.len());
  let mut channels = Vec::with_capacity(geometry.channels.len());
  for (&(channel_type, component), &channel) in geometry.descriptors.iter().zip(&geometry.channels)
  {
    let mut converted_type = channel_type;
    let converted_component = if component >= 15 {
      component
        .checked_add(2)
        .ok_or_else(|| invalid(label, "geometry component overflow"))?
    } else {
      component
    };
    let converted = if channel_type == 2 && (7..=14).contains(&component) {
      converted_type = 9;
      convert_uv_channel(channel, label)?
    } else if channel_type == 3 && matches!(component, 2 | 20 | 21) {
      converted_type = 8;
      pack_normal_channel(channel, label)?
    } else {
      channel.to_vec()
    };
    descriptors.push((converted_type, converted_component));
    channels.push(converted);
  }
  let mut out = Vec::with_capacity(payload.len());
  push_u32(
    &mut out,
    u32::try_from(geometry.vertex_count)
      .map_err(|_| invalid(label, "vertex count does not fit u32"))?,
  );
  push_u32(
    &mut out,
    u32::try_from(descriptors.len())
      .map_err(|_| invalid(label, "channel count does not fit u32"))?,
  );
  for (channel_type, component) in descriptors {
    push_u32(&mut out, channel_type);
    push_u32(&mut out, component);
  }
  for channel in channels {
    out.extend_from_slice(&channel);
  }
  out.extend_from_slice(&geometry.trailer[..geometry.trailer.len().min(8)]);
  Ok(out)
}

fn convert_uv_channel(channel: &[u8], label: &str) -> Result<Vec<u8>> {
  if !channel.len().is_multiple_of(8) {
    return Err(invalid(label, "misaligned float2 UV channel"));
  }
  let mut out = Vec::with_capacity(channel.len() / 2);
  for value in channel.chunks_exact(4) {
    let value = f32::from_le_bytes(
      value
        .try_into()
        .map_err(|_| invalid(label, "truncated UV value"))?,
    );
    out.extend_from_slice(&f32_to_f16(value).to_le_bytes());
  }
  Ok(out)
}

fn pack_normal_channel(channel: &[u8], label: &str) -> Result<Vec<u8>> {
  if !channel.len().is_multiple_of(12) {
    return Err(invalid(label, "misaligned float3 normal channel"));
  }
  let mut out = Vec::with_capacity(channel.len() / 3);
  for normal in channel.chunks_exact(12) {
    let mut values = [0.0; 3];
    for (value, offset) in values.iter_mut().zip([0, 4, 8]) {
      *value = f32::from_le_bytes(
        normal[offset..offset + 4]
          .try_into()
          .map_err(|_| invalid(label, "truncated normal value"))?,
      );
    }
    for value in values.into_iter().rev() {
      let scaled = (f64::from(value) + 1.0) * 127.5;
      out.push(if scaled.is_nan() {
        255
      } else {
        scaled.clamp(0.0, 255.0) as u8
      });
    }
    out.push(0);
  }
  Ok(out)
}

pub(super) fn f32_to_f16(value: f32) -> u16 {
  let bits = value.to_bits();
  let sign = ((bits >> 16) & 0x8000) as u16;
  let exponent = ((bits >> 23) & 0xff) as i32;
  let mantissa = bits & 0x7f_ffff;
  if exponent == 0xff {
    return sign | if mantissa == 0 { 0x7c00 } else { 0x7e00 };
  }

  let half_exponent = exponent - 127 + 15;
  if half_exponent >= 31 {
    return sign | 0x7c00;
  }
  if half_exponent <= 0 {
    if half_exponent < -10 {
      return sign;
    }
    let mantissa = mantissa | 0x80_0000;
    let shift = (14 - half_exponent) as u32;
    let mut half = mantissa >> shift;
    let remainder = mantissa & ((1u32 << shift) - 1);
    let halfway = 1u32 << (shift - 1);
    if remainder > halfway || (remainder == halfway && half & 1 != 0) {
      half += 1;
    }
    return sign | half as u16;
  }

  let mut half_exponent = half_exponent as u16;
  let mut half_mantissa = (mantissa >> 13) as u16;
  let remainder = mantissa & 0x1fff;
  if remainder > 0x1000 || (remainder == 0x1000 && half_mantissa & 1 != 0) {
    half_mantissa += 1;
    if half_mantissa == 0x400 {
      half_mantissa = 0;
      half_exponent += 1;
      if half_exponent == 31 {
        return sign | 0x7c00;
      }
    }
  }
  sign | (half_exponent << 10) | half_mantissa
}
