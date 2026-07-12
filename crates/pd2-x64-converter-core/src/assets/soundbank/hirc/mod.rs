mod node;
mod objects;

use super::codec::{Reader, invalid, put_u32, reserve};
use crate::error::{Error, Result};
use objects::{
  convert_action, convert_actor_mixer, convert_attenuation, convert_bus, convert_effect,
  convert_event, convert_layer_container, convert_music_random_sequence, convert_music_segment,
  convert_music_switch, convert_music_track, convert_random_sequence, convert_sound,
  convert_switch,
};

pub(super) fn convert_hirc(
  payload: &[u8],
  feedback: bool,
  add_audio_devices: bool,
  label: &str,
) -> Result<Vec<u8>> {
  let mut reader = Reader::new(payload, label);
  let count = reader.u32()?;
  if count == 0 {
    return invalid(format!("{label}: HIRC chunk contains no objects"));
  }
  let mut output = Vec::new();
  reserve(
    &mut output,
    payload
      .len()
      .checked_mul(2)
      .ok_or_else(|| Error::Invalid(format!("{label}: HIRC output size overflow")))?,
    label,
  )?;
  put_u32(
    &mut output,
    count
      .checked_add(u32::from(add_audio_devices) * 2)
      .ok_or_else(|| Error::Invalid(format!("{label}: HIRC object count overflow")))?,
  );
  if add_audio_devices {
    push_default_audio_devices(&mut output);
  }
  for index in 0..count {
    let kind = reader.u8()?;
    let size = reader.u32()?;
    if size < 4 {
      return invalid(format!(
        "{label}: HIRC item {index} is too short for its object ID"
      ));
    }
    let input_size = usize::try_from(size)
      .map_err(|_| Error::Invalid(format!("{label}: HIRC item {index} size is invalid")))?;
    let mut item = Reader::new(reader.bytes(input_size)?, label);
    let id = item.u32()?;
    let mut body = Vec::new();
    reserve(
      &mut body,
      input_size.checked_mul(2).ok_or_else(|| {
        Error::Invalid(format!("{label}: HIRC item {index} output size overflow"))
      })?,
      label,
    )?;
    let target_kind = match kind {
      2 => {
        convert_sound(&mut item, &mut body, feedback)?;
        2
      }
      3 => {
        convert_action(&mut item, &mut body)?;
        3
      }
      4 => {
        convert_event(&mut item, &mut body)?;
        4
      }
      5 => {
        convert_random_sequence(&mut item, &mut body, feedback)?;
        5
      }
      6 => {
        convert_switch(&mut item, &mut body, feedback)?;
        6
      }
      7 => {
        convert_actor_mixer(&mut item, &mut body, feedback)?;
        7
      }
      8 => {
        convert_bus(&mut item, &mut body)?;
        8
      }
      9 => {
        convert_layer_container(&mut item, &mut body, feedback)?;
        9
      }
      10 => {
        convert_music_segment(&mut item, &mut body, feedback)?;
        10
      }
      11 => {
        convert_music_track(&mut item, &mut body, feedback)?;
        11
      }
      12 => {
        convert_music_switch(&mut item, &mut body, feedback)?;
        12
      }
      13 => {
        convert_music_random_sequence(&mut item, &mut body, feedback)?;
        13
      }
      14 => {
        convert_attenuation(&mut item, &mut body)?;
        14
      }
      18 => {
        convert_effect(&mut item, &mut body)?;
        16
      }
      19 => {
        convert_effect(&mut item, &mut body)?;
        17
      }
      20 => {
        convert_bus(&mut item, &mut body)?;
        18
      }
      _ => {
        return invalid(format!(
          "{label}: unsupported HIRC object type {kind} at item {index}"
        ));
      }
    };
    item.finish("HIRC item")?;
    output.push(target_kind);
    let target_size = u32::try_from(
      body
        .len()
        .checked_add(4)
        .ok_or_else(|| Error::Invalid(format!("{label}: HIRC item {index} size overflow")))?,
    )
    .map_err(|_| Error::Invalid(format!("{label}: HIRC item {index} is too large")))?;
    put_u32(&mut output, target_size);
    put_u32(&mut output, id);
    output.extend_from_slice(&body);
  }
  reader.finish("HIRC")?;
  Ok(output)
}

fn push_default_audio_devices(out: &mut Vec<u8>) {
  out.push(21);
  put_u32(out, 20);
  put_u32(out, 2_317_455_096);
  put_u32(out, 0x00b5_0007);
  put_u32(out, 0);
  out.extend_from_slice(&[0; 8]);

  out.push(21);
  put_u32(out, 32);
  put_u32(out, 3_859_886_410);
  put_u32(out, 0x00ae_0007);
  put_u32(out, 12);
  out.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0x20, 0]);
  out.extend_from_slice(&[0; 8]);
}

pub(super) fn validate_hirc_bounds(payload: &[u8], label: &str) -> Result<()> {
  let mut reader = Reader::new(payload, label);
  let count = reader.u32()?;
  if count == 0 {
    return invalid(format!("{label}: HIRC chunk contains no objects"));
  }
  for index in 0..count {
    reader.u8()?;
    let size = reader.u32()?;
    if size < 4 {
      return invalid(format!(
        "{label}: HIRC item {index} is too short for its object ID"
      ));
    }
    reader.bytes(size as usize)?;
  }
  reader.finish("HIRC")
}
