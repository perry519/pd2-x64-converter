use super::node::{
  convert_base, convert_initial_rtpc, convert_property_bundle, convert_state, copy_children,
  copy_ranged_bundle, copy_u32s, map_property, narrow_u8,
};
use crate::assets::soundbank::codec::{Reader, copy_rtpc_graph, put_u16, put_u32};
use crate::error::{Error, Result};

pub(super) fn convert_sound(
  reader: &mut Reader<'_, '_>,
  out: &mut Vec<u8>,
  feedback: bool,
) -> Result<()> {
  convert_source(reader, out)?;
  convert_base(reader, out, feedback)
}

fn convert_source(reader: &mut Reader<'_, '_>, out: &mut Vec<u8>) -> Result<()> {
  let plugin = reader.u32()?;
  put_u32(out, plugin);
  let source_type = reader.u32()?;
  if source_type > 2 {
    return reader.fail(format!("invalid v88 sound source type {source_type}"));
  }
  out.push(if source_type == 0 { 0 } else { 2 });
  let source_id = reader.u32()?;
  put_u32(out, source_id);
  reader.u32()?;
  let in_memory_size = if source_type == 1 {
    0
  } else {
    reader.u32()?;
    reader.u32()?
  };
  put_u32(out, in_memory_size);
  let old_bits = reader.u8()?;
  out.push((old_bits & 1) | u8::from(source_type == 2) << 1 | (old_bits & 2) << 6);
  if matches!(plugin & 0xf, 2 | 5) {
    let size = reader.u32()?;
    put_u32(out, size);
    out.extend_from_slice(reader.bytes(size as usize)?);
  }
  Ok(())
}

pub(super) fn convert_bus(reader: &mut Reader<'_, '_>, out: &mut Vec<u8>) -> Result<()> {
  let parent = reader.u32()?;
  put_u32(out, parent);
  if parent == 0 {
    put_u32(out, 0xe611_314a);
  }
  convert_property_bundle(reader, out, None)?;

  reader.u8()?;
  reader.u8()?;
  let kill_newest = reader.u8()? & 1;
  let virtual_behavior = reader.u8()? & 1;
  let max_instances = reader.u16()?;
  let override_instances = reader.u8()? & 1;
  let channel_mask = reader.u16()?;
  reader.bytes(2)?;
  let is_hdr = reader.u8()? & 1;
  let exponential_release = reader.u8()? & 1;

  out.push(1);
  out.push(0x15);
  put_u32(out, 0);
  out.push(kill_newest | virtual_behavior << 1 | override_instances << 2);
  put_u16(out, max_instances);
  let channels = channel_mask.count_ones();
  put_u32(
    out,
    channels | u32::from(channels != 0) << 8 | u32::from(channel_mask) << 12,
  );
  out.push(is_hdr | exponential_release << 1);

  out.extend_from_slice(reader.bytes(4)?);
  out.extend_from_slice(reader.bytes(4)?);
  let ducks = reader.u32()?;
  put_u32(out, ducks);
  for _ in 0..ducks {
    out.extend_from_slice(reader.bytes(17)?);
    out.push(map_property(reader.u8()?)?);
  }

  let effects = reader.u8()?;
  out.push(effects);
  if effects != 0 {
    out.push(reader.u8()?);
    out.extend_from_slice(reader.bytes(effects as usize * 7)?);
  }
  put_u32(out, 0);
  out.push(0);
  out.push(0);
  out.push(0);
  convert_initial_rtpc(reader, out)?;
  out.extend_from_slice(&convert_state(reader)?);
  Ok(())
}

fn convert_music_node(
  reader: &mut Reader<'_, '_>,
  out: &mut Vec<u8>,
  feedback: bool,
) -> Result<()> {
  out.push(0);
  convert_base(reader, out, feedback)?;
  copy_children(reader, out)?;
  out.extend_from_slice(reader.bytes(23)?);
  let stingers = reader.u32()?;
  put_u32(out, stingers);
  out.extend_from_slice(reader.bytes(stingers as usize * 24)?);
  Ok(())
}

pub(super) fn convert_music_segment(
  reader: &mut Reader<'_, '_>,
  out: &mut Vec<u8>,
  feedback: bool,
) -> Result<()> {
  convert_music_node(reader, out, feedback)?;
  out.extend_from_slice(reader.bytes(8)?);
  let markers = reader.u32()?;
  put_u32(out, markers);
  for _ in 0..markers {
    out.extend_from_slice(reader.bytes(12)?);
    let name_size = reader.u32()? as usize;
    out.extend_from_slice(reader.bytes(name_size)?);
    out.push(0);
  }
  Ok(())
}

pub(super) fn convert_music_track(
  reader: &mut Reader<'_, '_>,
  out: &mut Vec<u8>,
  feedback: bool,
) -> Result<()> {
  out.push(0);
  let sources = reader.u32()?;
  put_u32(out, sources);
  for _ in 0..sources {
    convert_source(reader, out)?;
  }

  let playlist = reader.u32()?;
  put_u32(out, playlist);
  for _ in 0..playlist {
    out.extend_from_slice(reader.bytes(8)?);
    put_u32(out, 0);
    out.extend_from_slice(reader.bytes(32)?);
  }
  if playlist != 0 {
    put_u32(out, reader.u32()?);
  }

  let automation = reader.u32()?;
  put_u32(out, automation);
  for _ in 0..automation {
    out.extend_from_slice(reader.bytes(8)?);
    let points = reader.u32()?;
    put_u32(out, points);
    out.extend_from_slice(reader.bytes(points as usize * 12)?);
  }

  convert_base(reader, out, feedback)?;
  reader.u32()?;
  out.push(0);
  out.extend_from_slice(reader.bytes(4)?);
  Ok(())
}

fn convert_music_transition_node(
  reader: &mut Reader<'_, '_>,
  out: &mut Vec<u8>,
  feedback: bool,
) -> Result<()> {
  convert_music_node(reader, out, feedback)?;
  let rules = reader.u32()?;
  put_u32(out, rules);
  for _ in 0..rules {
    let sources = reader.u32()?;
    put_u32(out, sources);
    copy_u32s(reader, out, sources)?;
    let destinations = reader.u32()?;
    put_u32(out, destinations);
    copy_u32s(reader, out, destinations)?;
    out.extend_from_slice(reader.bytes(21)?);
    out.extend_from_slice(reader.bytes(20)?);
    put_u16(out, 0);
    put_u16(out, reader.u16()?);
    out.push(reader.u8()?);
    out.push(reader.u8()?);
    let transition_object = reader.u8()?;
    out.push(transition_object);
    if transition_object != 0 {
      out.extend_from_slice(reader.bytes(30)?);
    }
  }
  Ok(())
}

pub(super) fn convert_music_switch(
  reader: &mut Reader<'_, '_>,
  out: &mut Vec<u8>,
  feedback: bool,
) -> Result<()> {
  convert_music_transition_node(reader, out, feedback)?;
  out.extend_from_slice(reader.bytes(reader.remaining())?);
  Ok(())
}

pub(super) fn convert_music_random_sequence(
  reader: &mut Reader<'_, '_>,
  out: &mut Vec<u8>,
  feedback: bool,
) -> Result<()> {
  convert_music_transition_node(reader, out, feedback)?;
  let declared = reader.u32()?;
  if declared == 0 {
    return reader.fail("music playlist contains no root item".to_owned());
  }
  put_u32(out, declared);
  let mut pending = 1_u32;
  let mut seen = 0_u32;
  while pending != 0 {
    seen = seen
      .checked_add(1)
      .ok_or_else(|| Error::Invalid(format!("{}: music playlist size overflow", reader.label)))?;
    if seen > declared {
      return reader.fail("music playlist tree exceeds its declared item count".to_owned());
    }
    out.extend_from_slice(reader.bytes(8)?);
    let children = reader.u32()?;
    put_u32(out, children);
    out.extend_from_slice(reader.bytes(6)?);
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(reader.bytes(8)?);
    pending = pending
      .checked_sub(1)
      .and_then(|value| value.checked_add(children))
      .ok_or_else(|| Error::Invalid(format!("{}: music playlist size overflow", reader.label)))?;
  }
  if seen != declared {
    return reader.fail(format!(
      "music playlist tree has {seen} items, expected {declared}"
    ));
  }
  Ok(())
}

pub(super) fn convert_action(reader: &mut Reader<'_, '_>, out: &mut Vec<u8>) -> Result<()> {
  let action_type = reader.u16()?;
  let mut target_type = action_type;
  let mut id_ext = reader.u32()?;
  let id_ext_4 = reader.u8()?;
  let mut initial = Vec::new();
  convert_property_bundle(reader, &mut initial, None)?;
  copy_ranged_bundle(reader, &mut initial)?;
  let mut tail = Vec::new();
  match action_type {
    0x0102 | 0x0103 | 0x0105 | 0x0109 => {
      if action_type == 0x0109 {
        target_type = 0x0105;
      }
      tail.push(reader.u8()?);
      tail.push(6);
      convert_action_exceptions(reader, &mut tail)?;
    }
    0x0202 | 0x0203 | 0x0204 | 0x0302 | 0x0303 | 0x0304 => {
      tail.push(reader.u8()?);
      tail.push(reader.u8()?);
      convert_action_exceptions(reader, &mut tail)?;
    }
    0x0403 => {
      tail.push(reader.u8()?);
      put_u32(&mut tail, reader.u32()?);
      put_u32(&mut tail, 0);
    }
    0x0c02 | 0x0e03 => {
      tail.push(reader.u8()?);
      tail.extend_from_slice(reader.bytes(13)?);
      convert_action_exceptions(reader, &mut tail)?;
    }
    0x1204 | 0x1901 => {
      let group = reader.u32()?;
      let state = reader.u32()?;
      id_ext = state;
      put_u32(&mut tail, group);
      put_u32(&mut tail, state);
    }
    0x1302 | 0x1303 | 0x1402 => {
      tail.push(reader.u8()?);
      tail.push(0);
      tail.extend_from_slice(reader.bytes(13)?);
      convert_action_exceptions(reader, &mut tail)?;
    }
    0x1a02 => {
      tail.push(reader.u8()?);
      tail.push(reader.u8()?);
      convert_action_exceptions(reader, &mut tail)?;
    }
    0x1c02 | 0x1c03 => {}
    _ => return reader.fail(format!("unsupported v88 action type 0x{action_type:04x}")),
  }

  put_u16(out, target_type);
  put_u32(out, id_ext);
  out.push(id_ext_4);
  out.extend_from_slice(&initial);
  out.extend_from_slice(&tail);
  Ok(())
}

fn convert_action_exceptions(reader: &mut Reader<'_, '_>, out: &mut Vec<u8>) -> Result<()> {
  let count = reader.u32()?;
  out.push(narrow_u8(count, reader, "action exception count")?);
  for _ in 0..count {
    put_u32(out, reader.u32()?);
    out.push(reader.u8()?);
  }
  Ok(())
}

pub(super) fn convert_event(reader: &mut Reader<'_, '_>, out: &mut Vec<u8>) -> Result<()> {
  let count = reader.u32()?;
  out.push(narrow_u8(count, reader, "event action count")?);
  copy_u32s(reader, out, count)
}

pub(super) fn convert_random_sequence(
  reader: &mut Reader<'_, '_>,
  out: &mut Vec<u8>,
  feedback: bool,
) -> Result<()> {
  convert_base(reader, out, feedback)?;
  out.extend_from_slice(reader.bytes(6)?);
  out.extend_from_slice(reader.bytes(12)?);
  out.extend_from_slice(reader.bytes(5)?);
  let mut flags = 0;
  for bit in 0..5 {
    flags |= (reader.u8()? & 1) << bit;
  }
  out.push(flags);
  copy_children(reader, out)?;
  let count = reader.u16()?;
  put_u16(out, count);
  out.extend_from_slice(reader.bytes(count as usize * 8)?);
  Ok(())
}

pub(super) fn convert_switch(
  reader: &mut Reader<'_, '_>,
  out: &mut Vec<u8>,
  feedback: bool,
) -> Result<()> {
  convert_base(reader, out, feedback)?;
  let group_type = reader.u32()?;
  out.push(narrow_u8(group_type, reader, "switch group type")?);
  put_u32(out, reader.u32()?);
  put_u32(out, reader.u32()?);
  out.push(reader.u8()?);
  copy_children(reader, out)?;
  let groups = reader.u32()?;
  put_u32(out, groups);
  for _ in 0..groups {
    put_u32(out, reader.u32()?);
    let items = reader.u32()?;
    put_u32(out, items);
    copy_u32s(reader, out, items)?;
  }
  let params = reader.u32()?;
  put_u32(out, params);
  for _ in 0..params {
    put_u32(out, reader.u32()?);
    let first = reader.u8()?;
    let continuous = reader.u8()?;
    out.push((first & 1) | (continuous & 1) << 1);
    let mode = reader.u32()?;
    out.push(narrow_u8(mode, reader, "on-switch mode")? & 1);
    put_u32(out, reader.u32()?);
    put_u32(out, reader.u32()?);
  }
  Ok(())
}

pub(super) fn convert_actor_mixer(
  reader: &mut Reader<'_, '_>,
  out: &mut Vec<u8>,
  feedback: bool,
) -> Result<()> {
  convert_base(reader, out, feedback)?;
  copy_children(reader, out)
}

pub(super) fn convert_layer_container(
  reader: &mut Reader<'_, '_>,
  out: &mut Vec<u8>,
  feedback: bool,
) -> Result<()> {
  convert_base(reader, out, feedback)?;
  copy_children(reader, out)?;
  let layers = reader.u32()?;
  put_u32(out, layers);
  for _ in 0..layers {
    put_u32(out, reader.u32()?);
    convert_initial_rtpc(reader, out)?;
    put_u32(out, reader.u32()?);
    out.push(0);
    let children = reader.u32()?;
    put_u32(out, children);
    for _ in 0..children {
      put_u32(out, reader.u32()?);
      let points = reader.u32()?;
      put_u32(out, points);
      let graph_size = usize::try_from(points)
        .map_err(|_| {
          Error::Invalid(format!(
            "{}: layer RTPC graph size does not fit usize",
            reader.label
          ))
        })?
        .checked_mul(12)
        .ok_or_else(|| {
          Error::Invalid(format!("{}: layer RTPC graph size overflow", reader.label))
        })?;
      out.extend_from_slice(reader.bytes(graph_size)?);
    }
  }
  out.push(0);
  Ok(())
}

pub(super) fn convert_effect(reader: &mut Reader<'_, '_>, out: &mut Vec<u8>) -> Result<()> {
  put_u32(out, reader.u32()?);
  let parameter_size = reader.u32()?;
  put_u32(out, parameter_size);
  out.extend_from_slice(reader.bytes(parameter_size as usize)?);
  let media = reader.u8()?;
  out.push(media);
  for _ in 0..media {
    out.push(reader.u8()?);
    put_u32(out, reader.u32()?);
  }
  convert_initial_rtpc(reader, out)?;
  out.extend_from_slice(&[0; 4]);
  Ok(())
}

pub(super) fn convert_attenuation(reader: &mut Reader<'_, '_>, out: &mut Vec<u8>) -> Result<()> {
  out.push(1);
  let cone = reader.u8()?;
  out.push(cone);
  if cone == 1 {
    out.extend_from_slice(reader.bytes(16)?);
    put_u32(out, 0);
  } else if cone != 0 {
    return reader.fail(format!("invalid attenuation cone flag {cone}"));
  }
  let curves = reader.bytes(5)?;
  out.extend_from_slice(&curves[..4]);
  out.push(0xff);
  out.push(curves[4]);
  out.push(0xff);
  out.extend_from_slice(&[0xfe; 12]);
  let defined = reader.u8()?;
  out.push(defined);
  for _ in 0..defined {
    out.push(reader.u8()?);
    copy_rtpc_graph(reader, out)?;
  }
  convert_initial_rtpc(reader, out)
}
