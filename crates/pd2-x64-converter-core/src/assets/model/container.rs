use std::borrow::Cow;
use std::collections::BTreeMap;

use crate::assets::{push_u32, read_u32, read_u64, require_range, write_u32_at};
use crate::error::{Error, Result};

use super::geometry::TRIANGLE_LIST;

pub(super) const MAGIC: u32 = 0xffff_ffff;
pub(super) const MODEL_TYPE: u32 = 0x6221_2d88;
pub(super) const GEOMETRY_TYPE: u32 = 0x7ab0_72d3;
pub(super) const TOPOLOGY_TYPE: u32 = 0x4c50_7a13;
pub(super) const PASS_THROUGH_GEOMETRY_TYPE: u32 = 0xe3a3_b1ca;
pub(super) const SKIN_BONES_TYPE: u32 = 0x65cc_1825;

#[derive(Clone, Copy)]
pub(super) struct Section<'a> {
  pub(super) type_id: u32,
  pub(super) ref_id: u32,
  pub(super) payload: &'a [u8],
}

#[derive(Clone, Copy)]
pub(super) struct ModelMesh {
  pub(super) geometry_ref: u32,
  pub(super) skin_ref: u32,
}

pub(super) fn sections_by_ref<'a>(
  sections: &[Section<'a>],
  label: &str,
) -> Result<BTreeMap<u32, Section<'a>>> {
  let mut by_ref = BTreeMap::new();
  for section in sections {
    if by_ref.insert(section.ref_id, *section).is_some() {
      return Err(invalid(label, "duplicate model section reference"));
    }
  }
  Ok(by_ref)
}

pub(super) fn section_of_type<'a>(
  sections: &BTreeMap<u32, Section<'a>>,
  ref_id: u32,
  type_id: u32,
  kind: &str,
  label: &str,
) -> Result<Section<'a>> {
  let section = sections.get(&ref_id).ok_or_else(|| {
    invalid(
      label,
      format!("invalid {kind} reference {ref_id}: section is missing"),
    )
  })?;
  if section.type_id != type_id {
    return Err(invalid(
      label,
      format!(
        "invalid {kind} reference {ref_id}: section type 0x{:08x}, expected 0x{type_id:08x}",
        section.type_id
      ),
    ));
  }
  Ok(*section)
}

pub(super) fn parse_meshes(
  sections: &[Section<'_>],
  label: &str,
) -> Result<BTreeMap<u64, ModelMesh>> {
  let by_ref = sections_by_ref(sections, label)?;
  let mut meshes = BTreeMap::new();
  for section in sections
    .iter()
    .filter(|section| section.type_id == MODEL_TYPE)
  {
    require_range(
      section.payload,
      0,
      12,
      &format!("{label}: model {}", section.ref_id),
    )?;
    let name_hash = read_u64(section.payload, 0, label)?;
    let parameters = usize_from_u32(read_u32(section.payload, 8, label)?, label)?;
    let primitive_offset = checked_add(12 + checked_mul(parameters, 4, label)?, 80, label)?;
    require_range(
      section.payload,
      primitive_offset,
      16,
      &format!("{label}: model {} header", section.ref_id),
    )?;
    if read_u32(section.payload, primitive_offset, label)? == 6 {
      continue;
    }
    let producer_ref = read_u32(section.payload, primitive_offset + 4, label)?;
    let atom_count = usize_from_u32(
      read_u32(section.payload, primitive_offset + 12, label)?,
      label,
    )?;
    let atom_offset = primitive_offset + 16;
    require_range(
      section.payload,
      atom_offset,
      checked_add(checked_mul(atom_count, 20, label)?, 48, label)?,
      &format!("{label}: model {} body", section.ref_id),
    )?;
    let skin_ref = read_u32(section.payload, atom_offset + atom_count * 20 + 44, label)?;
    let producer = section_of_type(
      &by_ref,
      producer_ref,
      PASS_THROUGH_GEOMETRY_TYPE,
      "geometry producer",
      label,
    )?;
    require_range(
      producer.payload,
      0,
      8,
      &format!("{label}: geometry producer {producer_ref}"),
    )?;
    let geometry_ref = read_u32(producer.payload, 0, label)?;
    let topology_ref = read_u32(producer.payload, 4, label)?;
    section_of_type(&by_ref, geometry_ref, GEOMETRY_TYPE, "Geometry", label)?;
    section_of_type(&by_ref, topology_ref, TOPOLOGY_TYPE, "Topology", label)?;
    if skin_ref != 0 {
      section_of_type(&by_ref, skin_ref, SKIN_BONES_TYPE, "SkinBones", label)?;
    }
    if meshes
      .insert(
        name_hash,
        ModelMesh {
          geometry_ref,
          skin_ref,
        },
      )
      .is_some()
    {
      return Err(invalid(
        label,
        format!("duplicate model name 0x{name_hash:016x}"),
      ));
    }
  }
  Ok(meshes)
}

pub(super) fn parse_sections<'a>(data: &'a [u8], label: &str) -> Result<Vec<Section<'a>>> {
  require_range(data, 0, 12, &format!("{label}: model header"))?;
  let magic = read_u32(data, 0, label)?;
  if magic != MAGIC {
    return Err(invalid(
      label,
      format!("unsupported .model magic 0x{magic:08x}"),
    ));
  }
  let declared_size = usize_from_u32(read_u32(data, 4, label)?, label)?;
  if declared_size != data.len() {
    return Err(invalid(
      label,
      format!(
        "declared model size {declared_size} != file size {}",
        data.len()
      ),
    ));
  }
  let section_count = usize_from_u32(read_u32(data, 8, label)?, label)?;
  require_range(
    data,
    12,
    checked_mul(section_count, 12, label)?,
    &format!("{label}: minimum section headers"),
  )?;
  let mut sections = Vec::with_capacity(section_count);
  let mut offset = 12usize;
  for index in 0..section_count {
    require_range(
      data,
      offset,
      12,
      &format!("{label}: section {index} header"),
    )?;
    let type_id = read_u32(data, offset, label)?;
    let ref_id = read_u32(data, offset + 4, label)?;
    let size = usize_from_u32(read_u32(data, offset + 8, label)?, label)?;
    offset = checked_add(offset, 12, label)?;
    require_range(
      data,
      offset,
      size,
      &format!("{label}: section {index} payload"),
    )?;
    sections.push(Section {
      type_id,
      ref_id,
      payload: &data[offset..offset + size],
    });
    offset = checked_add(offset, size, label)?;
  }
  if offset != data.len() {
    return Err(invalid(
      label,
      format!("trailing bytes after {section_count} model sections"),
    ));
  }
  Ok(sections)
}

pub(super) fn rebuild_model<'a>(
  sections: &[Section<'a>],
  capacity: usize,
  label: &str,
  mut payload_for: impl FnMut(Section<'a>) -> Result<Cow<'a, [u8]>>,
) -> Result<Vec<u8>> {
  let mut out = Vec::with_capacity(capacity);
  push_u32(&mut out, MAGIC);
  push_u32(&mut out, 0);
  push_u32(
    &mut out,
    u32::try_from(sections.len())
      .map_err(|_| invalid(label, "model section count does not fit u32"))?,
  );
  for &section in sections {
    let payload = payload_for(section)?;
    push_u32(&mut out, section.type_id);
    push_u32(&mut out, section.ref_id);
    push_u32(
      &mut out,
      u32::try_from(payload.len())
        .map_err(|_| invalid(label, "model section payload does not fit u32"))?,
    );
    out.extend_from_slice(&payload);
  }
  let size = u32::try_from(out.len()).map_err(|_| invalid(label, "model is too large"))?;
  write_u32_at(&mut out, 4, size);
  Ok(out)
}

pub(super) fn convert_model_primitive(payload: &[u8], label: &str) -> Result<Vec<u8>> {
  require_range(payload, 0, 12, &format!("{label}: persistent object"))?;
  let parameters = usize_from_u32(read_u32(payload, 8, label)?, label)?;
  let primitive_offset = checked_add(12 + checked_mul(parameters, 4, label)?, 80, label)?;
  require_range(payload, primitive_offset, 4, &format!("{label}: primitive"))?;
  let mut converted = payload.to_vec();
  if read_u32(payload, primitive_offset, label)? == 0 {
    write_u32_at(&mut converted, primitive_offset, TRIANGLE_LIST);
  }
  Ok(converted)
}

pub(super) fn checked_add(left: usize, right: usize, label: &str) -> Result<usize> {
  left
    .checked_add(right)
    .ok_or_else(|| invalid(label, "model offset overflow"))
}

pub(super) fn checked_mul(left: usize, right: usize, label: &str) -> Result<usize> {
  left
    .checked_mul(right)
    .ok_or_else(|| invalid(label, "model byte count overflow"))
}

pub(super) fn usize_from_u32(value: u32, label: &str) -> Result<usize> {
  usize::try_from(value).map_err(|_| invalid(label, "model count does not fit usize"))
}

pub(super) fn invalid(label: &str, message: impl std::fmt::Display) -> Error {
  Error::Invalid(format!("{label}: {message}"))
}
