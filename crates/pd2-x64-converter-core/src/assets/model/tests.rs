use crate::assets::{push_u32, write_u32_at};

use super::container::{
  GEOMETRY_TYPE, MAGIC, MODEL_TYPE, PASS_THROUGH_GEOMETRY_TYPE, SKIN_BONES_TYPE, TOPOLOGY_TYPE,
  parse_sections,
};
use super::geometry::{TRIANGLE_LIST, f32_to_f16, parse_geometry};
use super::skin::parse_skin_bones;
use super::{classify, convert};

#[test]
fn converts_skinned_model_and_is_idempotent() {
  let mut source = x32_model(true);
  let primitive_offset = set_mesh_primitives(&mut source, 0);

  let converted = convert(&source, "fixture.model").unwrap();
  assert_eq!(convert(&converted, "fixture.model").unwrap(), converted);

  let sections = parse_sections(&converted, "converted").unwrap();
  let geometry = parse_geometry(
    sections
      .iter()
      .find(|section| section.type_id == GEOMETRY_TYPE)
      .unwrap()
      .payload,
    "geometry",
  )
  .unwrap();
  assert_eq!(
    geometry.descriptors,
    [(3, 1), (9, 7), (7, 17), (3, 19), (8, 2), (8, 22), (8, 23)]
  );
  assert_eq!(&geometry.channels[0][..12], &floats(&[1.0, 2.0, 3.0]));
  assert_eq!(&geometry.channels[1][..4], &[0x00, 0x38, 0x00, 0xb8]);
  assert_eq!(&geometry.channels[2][..8], &u16s(&[2, 3, 0, 0]));
  assert_eq!(&geometry.channels[3][..12], &floats(&[0.75, 0.25, 0.0]));
  assert_eq!(&geometry.channels[4][..4], &[0x1f, 0xbf, 0x9f, 0]);
  assert_eq!(geometry.trailer, 0x1111_2222_3333_4444u64.to_le_bytes());

  let topology = sections
    .iter()
    .find(|section| section.type_id == TOPOLOGY_TYPE)
    .unwrap();
  assert_eq!(&topology.payload[..4], &TRIANGLE_LIST.to_le_bytes());
  assert_eq!(&topology.payload[8..14], &u16s(&[0, 1, 2]));
  assert_eq!(
    &topology.payload[topology.payload.len() - 8..],
    &0x5555_6666_7777_8888u64.to_le_bytes()
  );
  let model = sections
    .iter()
    .find(|section| section.type_id == MODEL_TYPE)
    .unwrap();
  assert_eq!(
    &model.payload[primitive_offset..primitive_offset + 4],
    &TRIANGLE_LIST.to_le_bytes()
  );
  let skin = parse_skin_bones(
    sections
      .iter()
      .find(|section| section.type_id == SKIN_BONES_TYPE)
      .unwrap()
      .payload,
    "skin",
  )
  .unwrap();
  assert_eq!(skin.bone_node_refs, (100..108).collect::<Vec<_>>());
  assert!(sections.iter().any(|section| {
    section.type_id == 0x0ffc_d100 && section.payload == 0x1000u64.to_le_bytes()
  }));
}

#[test]
fn converts_rigid_two_weight_skinning() {
  let source = x32_model_with_weight_type(true, 2);
  let converted = convert(&source, "rigid.model").unwrap();
  let sections = parse_sections(&converted, "rigid.model").unwrap();
  let geometry = parse_geometry(
    sections
      .iter()
      .find(|section| section.type_id == GEOMETRY_TYPE)
      .unwrap()
      .payload,
    "rigid geometry",
  )
  .unwrap();

  assert!(geometry.descriptors.contains(&(7, 17)));
  assert!(geometry.descriptors.contains(&(2, 19)));
  assert_eq!(&geometry.channels[2][..8], &u16s(&[2, 3, 0, 0]));
}

#[test]
fn preserves_five_set_skin_bones() {
  let source = x32_model_with_skin_sets(5);
  let source_skin = skin_payload(&source).to_vec();

  let converted = convert(&source, "vehicle.model").unwrap();

  assert_eq!(skin_payload(&converted), source_skin);
}

#[test]
fn trims_geometry_trailer_that_duplicates_the_next_section() {
  let source = model_with_duplicated_next_section_prefix();
  let converted = convert(&source, "overlap.model").unwrap();
  let geometry = parse_geometry(
    parse_sections(&converted, "overlap.model")
      .unwrap()
      .iter()
      .find(|section| section.type_id == GEOMETRY_TYPE)
      .unwrap()
      .payload,
    "geometry",
  )
  .unwrap();
  assert_eq!(geometry.trailer.len(), 8);
}

#[test]
fn rejects_malformed_mixed_and_unsupported_topology() {
  let mut bad_magic = x32_model(false);
  bad_magic[0] = 0;
  assert!(
    classify(&bad_magic, "bad.model")
      .unwrap_err()
      .to_string()
      .contains("magic")
  );

  let mut bad_size = x32_model(false);
  bad_size[4..8].copy_from_slice(&1u32.to_le_bytes());
  assert!(
    classify(&bad_size, "bad.model")
      .unwrap_err()
      .to_string()
      .contains("declared")
  );

  let mut mixed = x32_model(false);
  let weight = find_descriptor(&mixed, 3, 17);
  mixed[weight + 4..weight + 8].copy_from_slice(&19u32.to_le_bytes());
  assert!(
    classify(&mixed, "mixed.model")
      .unwrap_err()
      .to_string()
      .contains("mixes")
  );

  for primitive in [1u32, 2, 4] {
    let mut unsupported = x32_model(false);
    let topology = find_section_payload(&unsupported, TOPOLOGY_TYPE);
    unsupported[topology..topology + 4].copy_from_slice(&primitive.to_le_bytes());
    assert!(
      classify(&unsupported, "unsupported.model")
        .unwrap_err()
        .to_string()
        .contains(&format!("unsupported topology primitive {primitive}"))
    );
  }
}

#[test]
fn rejects_truncated_unknown_and_invalid_skin_data() {
  let mut truncated = x32_model(true);
  truncated.pop();
  let size = truncated.len() as u32;
  truncated[4..8].copy_from_slice(&size.to_le_bytes());
  assert!(
    classify(&truncated, "truncated.model")
      .unwrap_err()
      .to_string()
      .contains("invalid range")
  );

  let mut unknown = x32_model(false);
  let position = find_descriptor(&unknown, 3, 1);
  unknown[position..position + 4].copy_from_slice(&99u32.to_le_bytes());
  assert!(
    classify(&unknown, "unknown.model")
      .unwrap_err()
      .to_string()
      .contains("unsupported geometry channel type")
  );

  let mut bad_weights = x32_model(true);
  let geometry = find_section_payload(&bad_weights, GEOMETRY_TYPE);
  let weights = geometry + 8 + 7 * 8 + 36 + 24 + 24;
  bad_weights[weights..weights + 4].copy_from_slice(&f32::NAN.to_le_bytes());
  assert!(
    classify(&bad_weights, "weights.model")
      .unwrap_err()
      .to_string()
      .contains("invalid bone weights")
  );

  let mut bad_two_weights = x32_model_with_weight_type(true, 2);
  let geometry = find_section_payload(&bad_two_weights, GEOMETRY_TYPE);
  let weights = geometry + 8 + 7 * 8 + 36 + 24 + 24;
  bad_two_weights[weights..weights + 8].copy_from_slice(&floats(&[0.75, 0.75]));
  assert!(
    classify(&bad_two_weights, "two-weights.model")
      .unwrap_err()
      .to_string()
      .contains("invalid bone weights")
  );

  let mut bad_palette = x32_model(true);
  let skin = find_section_payload(&bad_palette, SKIN_BONES_TYPE);
  bad_palette[skin + 8..skin + 12].copy_from_slice(&u32::MAX.to_le_bytes());
  assert!(
    classify(&bad_palette, "palette.model")
      .unwrap_err()
      .to_string()
      .contains("palette contains an out-of-range")
  );
}

#[test]
fn rejects_unrecognized_geometry_trailer() {
  let mut sections = owned_sections(&x32_model(false));
  let geometry = sections
    .iter_mut()
    .find(|section| section.0 == GEOMETRY_TYPE)
    .unwrap();
  geometry.2.extend_from_slice(&[0; 4]);
  let source = build_container(&sections);

  assert!(
    classify(&source, "bad-trailer.model")
      .unwrap_err()
      .to_string()
      .contains("unexpected 12-byte geometry trailer")
  );
}

#[test]
fn half_encoder_matches_ieee_edges() {
  assert_eq!(f32_to_f16(0.0), 0x0000);
  assert_eq!(f32_to_f16(-0.0), 0x8000);
  assert_eq!(f32_to_f16(1.0), 0x3c00);
  assert_eq!(f32_to_f16(f32::INFINITY), 0x7c00);
  assert_eq!(f32_to_f16(f32::NEG_INFINITY), 0xfc00);
  assert_eq!(f32_to_f16(f32::NAN) & 0x7fff, 0x7e00);
  assert_eq!(f32_to_f16(2f32.powi(-24)), 0x0001);
  assert_eq!(f32_to_f16(65504.0), 0x7bff);
}

pub(crate) fn x32_model(skinned: bool) -> Vec<u8> {
  x32_model_with_weight_type_and_skin_sets(skinned, 3, 2)
}

fn x32_model_with_weight_type(skinned: bool, weight_type: u32) -> Vec<u8> {
  x32_model_with_weight_type_and_skin_sets(skinned, weight_type, 2)
}

fn x32_model_with_skin_sets(skin_sets: u32) -> Vec<u8> {
  x32_model_with_weight_type_and_skin_sets(true, 3, skin_sets)
}

fn x32_model_with_weight_type_and_skin_sets(
  skinned: bool,
  weight_type: u32,
  skin_sets: u32,
) -> Vec<u8> {
  let descriptors = [
    (3, 1),
    (2, 7),
    (7, 15),
    (weight_type, 17),
    (3, 2),
    (3, 20),
    (3, 21),
  ];
  let mut geometry = Vec::new();
  push_u32(&mut geometry, 3);
  push_u32(&mut geometry, descriptors.len() as u32);
  for (channel_type, component) in descriptors {
    push_u32(&mut geometry, channel_type);
    push_u32(&mut geometry, component);
  }
  geometry.extend(floats(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]));
  geometry.extend(floats(&[0.5, -0.5, 0.25, 0.75, -0.25, 1.0]));
  geometry.extend(if weight_type == 2 {
    u16s(&[2, 3, 0, 0, 2, 4, 0, 0, 2, 5, 6, 0])
  } else {
    u16s(&[2, 3, 0, 0, 4, 0, 0, 0, 1, 5, 6, 0])
  });
  if weight_type == 2 {
    geometry.extend(floats(&[1.0, 0.0, 1.0, 0.0, 1.0, 0.0]));
  } else {
    geometry.extend(floats(&[0.75, 0.25, 0.0, 1.0, 0.0, 0.0, 0.5, 0.25, 0.25]));
  }
  geometry.extend(floats(&[0.25, 0.5, -0.75, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0]));
  geometry.extend(floats(&[0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0]));
  geometry.extend(floats(&[1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0]));
  geometry.extend(0x1111_2222_3333_4444u64.to_le_bytes());

  let mut topology = Vec::new();
  push_u32(&mut topology, TRIANGLE_LIST);
  push_u32(&mut topology, 3);
  topology.extend(u16s(&[0, 1, 2]));
  push_u32(&mut topology, 0);
  topology.extend(0x5555_6666_7777_8888u64.to_le_bytes());

  let mut sections = vec![
    (0x0ffc_d100, 100, 0x1000u64.to_le_bytes().to_vec()),
    (GEOMETRY_TYPE, 1, geometry),
    (TOPOLOGY_TYPE, 2, topology),
  ];
  if skinned {
    for index in 1..8u32 {
      sections.push((
        0x0ffc_d100,
        100 + index,
        (0x1000u64 + u64::from(index)).to_le_bytes().to_vec(),
      ));
    }
    sections.push((SKIN_BONES_TYPE, 4, skin_bones_fixture(skin_sets)));
  }
  sections.extend([
    (
      PASS_THROUGH_GEOMETRY_TYPE,
      3,
      [1u32.to_le_bytes(), 2u32.to_le_bytes()].concat(),
    ),
    (
      MODEL_TYPE,
      5,
      model_binding_fixture(if skinned { 4 } else { 0 }),
    ),
  ]);
  build_container(&sections)
}

fn skin_bones_fixture(set_count: u32) -> Vec<u8> {
  let mut out = Vec::new();
  push_u32(&mut out, set_count);
  for _ in 0..set_count {
    push_u32(&mut out, 8);
    for index in 0..8 {
      push_u32(&mut out, index);
    }
  }
  push_u32(&mut out, 100);
  push_u32(&mut out, 8);
  for node in 100..108 {
    push_u32(&mut out, node);
  }
  for index in 0..8u8 {
    out.extend_from_slice(&[index; 64]);
  }
  out.extend_from_slice(&[b'P'; 64]);
  out
}

fn skin_payload(data: &[u8]) -> &[u8] {
  parse_sections(data, "model")
    .unwrap()
    .into_iter()
    .find(|section| section.type_id == SKIN_BONES_TYPE)
    .unwrap()
    .payload
}

fn model_binding_fixture(skin_ref: u32) -> Vec<u8> {
  let mut out = Vec::new();
  out.extend_from_slice(&0x9999_8888_7777_6666u64.to_le_bytes());
  push_u32(&mut out, 0);
  out.extend_from_slice(&[0; 76]);
  push_u32(&mut out, 0);
  push_u32(&mut out, TRIANGLE_LIST);
  push_u32(&mut out, 3);
  push_u32(&mut out, 2);
  push_u32(&mut out, 1);
  out.extend_from_slice(&[0; 20]);
  push_u32(&mut out, 0);
  push_u32(&mut out, 0);
  push_u32(&mut out, 0);
  out.extend_from_slice(&[0; 32]);
  push_u32(&mut out, skin_ref);
  out
}

fn build_container(sections: &[(u32, u32, Vec<u8>)]) -> Vec<u8> {
  let mut out = Vec::new();
  push_u32(&mut out, MAGIC);
  push_u32(&mut out, 0);
  push_u32(&mut out, sections.len() as u32);
  for (type_id, ref_id, payload) in sections {
    push_u32(&mut out, *type_id);
    push_u32(&mut out, *ref_id);
    push_u32(&mut out, payload.len() as u32);
    out.extend_from_slice(payload);
  }
  let size = out.len() as u32;
  write_u32_at(&mut out, 4, size);
  out
}

fn owned_sections(data: &[u8]) -> Vec<(u32, u32, Vec<u8>)> {
  parse_sections(data, "fixture")
    .unwrap()
    .into_iter()
    .map(|section| (section.type_id, section.ref_id, section.payload.to_vec()))
    .collect()
}

fn model_with_duplicated_next_section_prefix() -> Vec<u8> {
  let mut sections = owned_sections(&x32_model(true));
  let geometry = sections
    .iter()
    .position(|section| section.0 == GEOMETRY_TYPE)
    .unwrap();
  let next = &sections[geometry + 1];
  let mut prefix = Vec::new();
  push_u32(&mut prefix, next.0);
  push_u32(&mut prefix, next.1);
  push_u32(&mut prefix, next.2.len() as u32);
  prefix.extend_from_slice(&next.2[..12]);
  sections[geometry].2.extend_from_slice(&prefix);
  build_container(&sections)
}

fn floats(values: &[f32]) -> Vec<u8> {
  values
    .iter()
    .flat_map(|value| value.to_le_bytes())
    .collect()
}

fn u16s(values: &[u16]) -> Vec<u8> {
  values
    .iter()
    .flat_map(|value| value.to_le_bytes())
    .collect()
}

fn set_mesh_primitives(data: &mut [u8], primitive: u32) -> usize {
  let topology = find_section_payload(data, TOPOLOGY_TYPE);
  data[topology..topology + 4].copy_from_slice(&primitive.to_le_bytes());
  let model = find_section_payload(data, MODEL_TYPE);
  let parameters = u32::from_le_bytes(data[model + 8..model + 12].try_into().unwrap()) as usize;
  let primitive_offset = 12 + parameters * 4 + 80;
  let model = model + primitive_offset;
  data[model..model + 4].copy_from_slice(&primitive.to_le_bytes());
  primitive_offset
}

fn find_section_payload(data: &[u8], wanted: u32) -> usize {
  let mut offset = 12;
  for _ in 0..u32::from_le_bytes(data[8..12].try_into().unwrap()) {
    let type_id = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
    let size = u32::from_le_bytes(data[offset + 8..offset + 12].try_into().unwrap()) as usize;
    if type_id == wanted {
      return offset + 12;
    }
    offset += 12 + size;
  }
  panic!("section not found")
}

fn find_descriptor(data: &[u8], channel_type: u32, component: u32) -> usize {
  let geometry = find_section_payload(data, GEOMETRY_TYPE);
  let count = u32::from_le_bytes(data[geometry + 4..geometry + 8].try_into().unwrap()) as usize;
  (0..count)
    .map(|index| geometry + 8 + index * 8)
    .find(|offset| {
      u32::from_le_bytes(data[*offset..*offset + 4].try_into().unwrap()) == channel_type
        && u32::from_le_bytes(data[*offset + 4..*offset + 8].try_into().unwrap()) == component
    })
    .unwrap()
}
