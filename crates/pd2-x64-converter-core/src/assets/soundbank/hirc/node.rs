use crate::assets::soundbank::codec::{Reader, copy_rtpc_graph, put_i32, put_u16, put_u32};
use crate::error::{Error, Result};

pub(super) fn convert_base(
  reader: &mut Reader<'_, '_>,
  out: &mut Vec<u8>,
  feedback: bool,
) -> Result<()> {
  let fx = convert_fx(reader)?;
  let bus = reader.u32()?;
  let parent = reader.u32()?;
  let priority = (reader.u8()? & 1) | (reader.u8()? & 1) << 1;
  let properties = read_property_bundle(reader)?;
  let ranged = read_ranged_bundle(reader)?;
  let positioning = convert_positioning(reader)?;
  let aux = convert_aux(reader)?;
  let advanced = convert_advanced(reader)?;
  let state = convert_state(reader)?;
  let rtpc = convert_initial_rtpc_to_vec(reader)?;
  if feedback {
    reader.u32()?;
  }

  out.extend_from_slice(&fx);
  out.extend_from_slice(&[0, 0]);
  out.push(0);
  put_u32(out, bus);
  put_u32(out, parent);
  out.push(priority);
  write_property_bundle(out, &properties, positioning.attenuation_id)?;
  out.extend_from_slice(&ranged);
  out.extend_from_slice(&positioning.bytes);
  out.extend_from_slice(&aux);
  out.extend_from_slice(&advanced);
  out.extend_from_slice(&state);
  out.extend_from_slice(&rtpc);
  Ok(())
}

fn convert_fx(reader: &mut Reader<'_, '_>) -> Result<Vec<u8>> {
  let mut out = Vec::new();
  let override_parent = reader.u8()?;
  let count = reader.u8()?;
  out.extend_from_slice(&[override_parent, count]);
  if count > 0 {
    out.push(reader.u8()?);
    out.extend_from_slice(reader.bytes(count as usize * 7)?);
  }
  Ok(out)
}

struct Properties {
  ids: Vec<u8>,
  values: Vec<[u8; 4]>,
}

fn read_property_bundle(reader: &mut Reader<'_, '_>) -> Result<Properties> {
  let count = reader.u8()? as usize;
  let ids = reader.bytes(count)?.to_vec();
  let mut values = Vec::with_capacity(count);
  for _ in 0..count {
    values.push(
      reader
        .bytes(4)?
        .try_into()
        .map_err(|_| Error::Invalid(format!("{}: invalid property value", reader.label)))?,
    );
  }
  Ok(Properties { ids, values })
}

pub(super) fn convert_property_bundle(
  reader: &mut Reader<'_, '_>,
  out: &mut Vec<u8>,
  attenuation: Option<u32>,
) -> Result<()> {
  let properties = read_property_bundle(reader)?;
  write_property_bundle(out, &properties, attenuation)
}

fn write_property_bundle(
  out: &mut Vec<u8>,
  properties: &Properties,
  attenuation: Option<u32>,
) -> Result<()> {
  let count = properties.ids.len() + usize::from(attenuation.is_some());
  out.push(
    u8::try_from(count)
      .map_err(|_| Error::Invalid("soundbank property bundle exceeds 255 entries".to_owned()))?,
  );
  for &id in &properties.ids {
    out.push(map_property(id)?);
  }
  if attenuation.is_some() {
    out.push(0x46);
  }
  for value in &properties.values {
    out.extend_from_slice(value);
  }
  if let Some(id) = attenuation {
    out.extend_from_slice(&f32::from_bits(id).to_le_bytes());
  }
  Ok(())
}

pub(super) fn map_property(id: u8) -> Result<u8> {
  const MAP: [u8; 45] = [
    0x00, 0x01, 0x02, 0x03, 0x05, 0x07, 0x08, 0x3a, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
    0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x1a, 0x3b, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20,
    0x21, 0x06, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c,
  ];
  MAP
    .get(id as usize)
    .copied()
    .ok_or_else(|| Error::Invalid(format!("unsupported v88 property identifier {id}")))
}

fn read_ranged_bundle(reader: &mut Reader<'_, '_>) -> Result<Vec<u8>> {
  let mut out = Vec::new();
  copy_ranged_bundle(reader, &mut out)?;
  Ok(out)
}

pub(super) fn copy_ranged_bundle(reader: &mut Reader<'_, '_>, out: &mut Vec<u8>) -> Result<()> {
  let count = reader.u8()?;
  out.push(count);
  out.extend_from_slice(reader.bytes(count as usize)?);
  out.extend_from_slice(reader.bytes(count as usize * 8)?);
  Ok(())
}

struct Positioning {
  bytes: Vec<u8>,
  attenuation_id: Option<u32>,
}

fn convert_positioning(reader: &mut Reader<'_, '_>) -> Result<Positioning> {
  let old_bits = reader.u8()?;
  if old_bits & 1 == 0 {
    return Ok(Positioning {
      bytes: vec![0],
      attenuation_id: None,
    });
  }
  let is_2d = reader.u8()? == 1;
  let is_3d = reader.u8()? == 1;
  if is_2d {
    reader.u8()?;
  }
  let mut attenuation = None;
  let mut kind = None;
  let mut dynamic = 0;
  let mut follow = 0;
  let mut automation = Vec::new();
  if is_3d {
    let position_kind = reader.u32()?;
    if position_kind > 3 {
      return reader.fail(format!("invalid v88 positioning type {position_kind}"));
    }
    kind = Some(position_kind);
    attenuation = Some(reader.u32()?);
    reader.u8()?;
    if position_kind == 1 {
      dynamic = reader.u8()? & 1;
    } else {
      let path_mode = reader.u32()?;
      automation.push(narrow_u8(path_mode, reader, "path mode")?);
      reader.u8()?;
      let transition = reader.u32()?;
      let transition = i32::try_from(transition)
        .map_err(|_| Error::Invalid(format!("{}: transition time exceeds i32", reader.label)))?;
      put_i32(&mut automation, transition);
      follow = reader.u8()? & 1;
      let vertices = reader.u32()?;
      put_u32(&mut automation, vertices);
      automation.extend_from_slice(reader.bytes(vertices as usize * 16)?);
      let playlist = reader.u32()?;
      put_u32(&mut automation, playlist);
      automation.extend_from_slice(reader.bytes(playlist as usize * 8)?);
      for _ in 0..playlist {
        automation.extend_from_slice(reader.bytes(8)?);
        put_u32(&mut automation, 0);
      }
    }
  }

  let has_routing = is_2d || is_3d;
  let has_automation = matches!(kind, Some(value) if value != 1);
  let mut bits = 1 | u8::from(has_routing) << 1;
  if has_automation {
    bits |= 1 << 5;
  }
  let mut out = vec![bits];
  if has_routing {
    let mut bits_3d = 0;
    if attenuation.is_some() || is_2d {
      bits_3d |= 1 << 3;
    }
    if kind == Some(1) {
      bits_3d |= dynamic << 4;
    } else if kind.is_some() {
      bits_3d |= follow << 5;
    }
    out.push(bits_3d);
    if has_automation {
      out.extend_from_slice(&automation);
    }
  }
  Ok(Positioning {
    bytes: out,
    attenuation_id: attenuation,
  })
}

fn convert_aux(reader: &mut Reader<'_, '_>) -> Result<Vec<u8>> {
  let game_override = reader.u8()? & 1;
  let game = reader.u8()? & 1;
  let user_override = reader.u8()? & 1;
  let has_aux = reader.u8()? & 1;
  let mut out = vec![game_override | game << 1 | user_override << 2 | has_aux << 3];
  if has_aux == 1 {
    out.extend_from_slice(reader.bytes(16)?);
  }
  put_u32(&mut out, 0);
  Ok(out)
}

fn convert_advanced(reader: &mut Reader<'_, '_>) -> Result<Vec<u8>> {
  let queue = reader.u8()?;
  let kill = reader.u8()? & 1;
  let virtual_behavior = reader.u8()? & 1;
  let max = reader.u16()?;
  reader.u8()?;
  let below = reader.u8()?;
  let max_override = reader.u8()? & 1;
  let voice_override = reader.u8()? & 1;
  let hdr = reader.u8()? & 1;
  let analysis = reader.u8()? & 1;
  let normalize = reader.u8()? & 1;
  let envelope = reader.u8()? & 1;
  let mut out = vec![
    kill | virtual_behavior << 1 | max_override << 3 | voice_override << 4,
    queue,
  ];
  put_u16(&mut out, max);
  out.push(below);
  out.push(hdr | analysis << 1 | normalize << 2 | envelope << 3);
  Ok(out)
}

pub(super) fn convert_state(reader: &mut Reader<'_, '_>) -> Result<Vec<u8>> {
  let groups = reader.u32()?;
  let mut out = vec![0, narrow_u8(groups, reader, "state group count")?];
  for _ in 0..groups {
    put_u32(&mut out, reader.u32()?);
    out.push(reader.u8()?);
    let states = reader.u16()?;
    out.push(
      u8::try_from(states)
        .map_err(|_| Error::Invalid(format!("{}: state count exceeds 255", reader.label)))?,
    );
    out.extend_from_slice(reader.bytes(states as usize * 8)?);
  }
  Ok(out)
}

fn convert_initial_rtpc_to_vec(reader: &mut Reader<'_, '_>) -> Result<Vec<u8>> {
  let mut out = Vec::new();
  convert_initial_rtpc(reader, &mut out)?;
  Ok(out)
}

pub(super) fn convert_initial_rtpc(reader: &mut Reader<'_, '_>, out: &mut Vec<u8>) -> Result<()> {
  let count = reader.u16()?;
  put_u16(out, count);
  for _ in 0..count {
    put_u32(out, reader.u32()?);
    out.extend_from_slice(&[0, 0]);
    let parameter = reader.u32()?;
    out.push(map_rtpc(parameter, reader)?);
    put_u32(out, reader.u32()?);
    out.push(reader.u8()?);
    copy_rtpc_graph(reader, out)?;
  }
  Ok(())
}

fn map_rtpc(id: u32, reader: &Reader<'_, '_>) -> Result<u8> {
  let mapped = match id {
    0 => 0,
    1 => 1,
    2 => 2,
    3 => 3,
    4 => 5,
    5 => 15,
    6 => 6,
    8 => 17,
    9 => 16,
    10 => 23,
    11 => 24,
    12 => 25,
    13 => 26,
    14 => 27,
    15 => 39,
    16 => 40,
    17 => 41,
    18 => 42,
    19 => 38,
    20 => 18,
    21 => 19,
    22 => 43,
    23 => 45,
    24 => 29,
    25 => 30,
    26 => 31,
    27 => 32,
    28 => 33,
    32 => 34,
    33 => 35,
    34 => 36,
    35 => 37,
    36 => 7,
    37 => 20,
    38 => 21,
    60 => 43,
    62 => 45,
    _ => return reader.fail(format!("unsupported v88 RTPC parameter {id}")),
  };
  Ok(mapped)
}

pub(super) fn copy_children(reader: &mut Reader<'_, '_>, out: &mut Vec<u8>) -> Result<()> {
  let count = reader.u32()?;
  put_u32(out, count);
  copy_u32s(reader, out, count)
}

pub(super) fn copy_u32s(reader: &mut Reader<'_, '_>, out: &mut Vec<u8>, count: u32) -> Result<()> {
  out.extend_from_slice(reader.bytes(count as usize * 4)?);
  Ok(())
}

pub(super) fn narrow_u8(value: u32, reader: &Reader<'_, '_>, name: &str) -> Result<u8> {
  u8::try_from(value).map_err(|_| Error::Invalid(format!("{}: {name} exceeds 255", reader.label)))
}
