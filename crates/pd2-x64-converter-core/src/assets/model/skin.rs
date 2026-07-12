use std::collections::{BTreeMap, BTreeSet};

use crate::assets::{read_u32, require_range};
use crate::error::Result;

use super::container::{
  Section, checked_add, checked_mul, invalid, parse_meshes, sections_by_ref, usize_from_u32,
};
use super::geometry::{ModelLayout, parse_geometry};

const BONE_WEIGHT_EPSILON: f32 = 1e-5;

pub(super) struct SkinBones {
  pub(super) bone_node_refs: Vec<u32>,
}

pub(super) fn validate_skin_bindings(
  sections: &[Section<'_>],
  layout: ModelLayout,
  label: &str,
) -> Result<()> {
  let by_ref = sections_by_ref(sections, label)?;
  let mut geometries_by_skin: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
  let mut skin_by_geometry = BTreeMap::new();
  for mesh in parse_meshes(sections, label)?.values() {
    if mesh.skin_ref == 0 {
      continue;
    }
    if let Some(previous) = skin_by_geometry.insert(mesh.geometry_ref, mesh.skin_ref)
      && previous != mesh.skin_ref
    {
      return Err(invalid(
        label,
        format!(
          "Geometry {} is bound to multiple SkinBones sections",
          mesh.geometry_ref
        ),
      ));
    }
    geometries_by_skin
      .entry(mesh.skin_ref)
      .or_default()
      .insert(mesh.geometry_ref);
  }

  for (skin_ref, geometry_refs) in geometries_by_skin {
    let skin_section = by_ref[&skin_ref];
    let skin = parse_skin_bones(
      skin_section.payload,
      &format!("{label}: SkinBones {skin_ref}"),
    )?;
    let mut used = BTreeSet::new();
    for geometry_ref in &geometry_refs {
      used.extend(geometry_used_bones(
        by_ref[geometry_ref].payload,
        layout,
        &format!("{label}: Geometry {geometry_ref}"),
      )?);
    }
    if let Some(&last) = used.last()
      && usize::from(last) >= skin.bone_node_refs.len()
    {
      return Err(invalid(
        label,
        format!(
          "SkinBones {skin_ref} uses node {last} but only has {} nodes",
          skin.bone_node_refs.len()
        ),
      ));
    }
  }
  Ok(())
}

pub(super) fn parse_skin_bones(payload: &[u8], label: &str) -> Result<SkinBones> {
  require_range(payload, 0, 4, &format!("{label}: bone-set count"))?;
  let set_count = usize_from_u32(read_u32(payload, 0, label)?, label)?;
  require_range(
    payload,
    4,
    checked_mul(set_count, 4, label)?,
    &format!("{label}: minimum bone-set headers"),
  )?;
  let mut offset = 4usize;
  let mut max_bone_index = None;
  for index in 0..set_count {
    require_range(
      payload,
      offset,
      4,
      &format!("{label}: bone set {index} count"),
    )?;
    let count = usize_from_u32(read_u32(payload, offset, label)?, label)?;
    offset = checked_add(offset, 4, label)?;
    let size = checked_mul(count, 4, label)?;
    require_range(payload, offset, size, &format!("{label}: bone set {index}"))?;
    for item in 0..count {
      let bone = usize_from_u32(read_u32(payload, offset + item * 4, label)?, label)?;
      max_bone_index = Some(max_bone_index.map_or(bone, |current: usize| current.max(bone)));
    }
    offset = checked_add(offset, size, label)?;
  }
  require_range(
    payload,
    offset,
    8,
    &format!("{label}: root and bone-node count"),
  )?;
  read_u32(payload, offset, label)?;
  let node_count = usize_from_u32(read_u32(payload, offset + 4, label)?, label)?;
  offset = checked_add(offset, 8, label)?;
  let node_size = checked_mul(node_count, 4, label)?;
  require_range(
    payload,
    offset,
    node_size,
    &format!("{label}: bone-node references"),
  )?;
  let mut bone_node_refs = Vec::with_capacity(node_count);
  for index in 0..node_count {
    bone_node_refs.push(read_u32(payload, offset + index * 4, label)?);
  }
  offset = checked_add(offset, node_size, label)?;
  let records_size = checked_mul(node_count, 64, label)?;
  require_range(
    payload,
    offset,
    checked_add(records_size, 64, label)?,
    &format!("{label}: bind records"),
  )?;
  offset = checked_add(offset, records_size, label)?;
  offset = checked_add(offset, 64, label)?;
  if offset != payload.len() {
    return Err(invalid(
      label,
      format!(
        "unexpected {} trailing SkinBones bytes",
        payload.len() - offset
      ),
    ));
  }
  if max_bone_index.is_some_and(|index| index >= node_count) {
    return Err(invalid(
      label,
      "SkinBones palette contains an out-of-range node index",
    ));
  }
  Ok(SkinBones { bone_node_refs })
}

pub(super) fn geometry_used_bones(
  payload: &[u8],
  layout: ModelLayout,
  label: &str,
) -> Result<BTreeSet<u16>> {
  let geometry = parse_geometry(payload, label)?;
  let (index_component, weight_component) = match layout {
    ModelLayout::X32 => (15, 17),
    ModelLayout::X64 => (17, 19),
  };
  let Some(index_position) = geometry
    .descriptors
    .iter()
    .position(|descriptor| *descriptor == (7, index_component))
  else {
    return Ok(BTreeSet::new());
  };
  let mut weight_positions = geometry
    .descriptors
    .iter()
    .enumerate()
    .filter(|(_, descriptor)| matches!(descriptor.0, 2 | 3) && descriptor.1 == weight_component);
  let (weight_position, weight_descriptor) = weight_positions
    .next()
    .ok_or_else(|| invalid(label, "bone indices have no matching weight channel"))?;
  let weight_type = weight_descriptor.0;
  if weight_positions.next().is_some() {
    return Err(invalid(
      label,
      "bone indices have ambiguous weight channels",
    ));
  }
  let indices = geometry.channels[index_position];
  let weights = geometry.channels[weight_position];
  let weight_count = usize::try_from(weight_type)
    .map_err(|_| invalid(label, "bone weight component count does not fit usize"))?;
  let expected_indices = geometry
    .vertex_count
    .checked_mul(8)
    .ok_or_else(|| invalid(label, "bone index channel size overflow"))?;
  let expected_weights = geometry
    .vertex_count
    .checked_mul(weight_count * 4)
    .ok_or_else(|| invalid(label, "bone weight channel size overflow"))?;
  if indices.len() != expected_indices || weights.len() != expected_weights {
    return Err(invalid(
      label,
      "bone weight/index channels have incompatible lengths",
    ));
  }
  let mut used = BTreeSet::new();
  for vertex in 0..geometry.vertex_count {
    let mut vertex_weights = [0.0f32; 4];
    for (component, weight) in vertex_weights[..weight_count].iter_mut().enumerate() {
      let offset = vertex * weight_count * 4 + component * 4;
      *weight = f32::from_le_bytes(
        weights[offset..offset + 4]
          .try_into()
          .map_err(|_| invalid(label, "truncated bone weight channel"))?,
      );
    }
    if weight_type == 3 {
      vertex_weights[3] = 1.0 - vertex_weights[..3].iter().sum::<f32>();
    }
    if vertex_weights.iter().any(|weight| {
      !weight.is_finite() || *weight < -BONE_WEIGHT_EPSILON || *weight > 1.0 + BONE_WEIGHT_EPSILON
    }) || (vertex_weights.iter().sum::<f32>() - 1.0).abs() > BONE_WEIGHT_EPSILON
    {
      return Err(invalid(
        label,
        format!("invalid bone weights at vertex {vertex}"),
      ));
    }
    for (component, weight) in vertex_weights.into_iter().enumerate() {
      if weight > BONE_WEIGHT_EPSILON {
        let offset = vertex * 8 + component * 2;
        used.insert(u16::from_le_bytes(
          indices[offset..offset + 2]
            .try_into()
            .map_err(|_| invalid(label, "truncated bone index channel"))?,
        ));
      }
    }
  }
  Ok(used)
}
