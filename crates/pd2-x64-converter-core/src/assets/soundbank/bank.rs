use super::codec::{Reader, copy_rtpc_graph, invalid, put_u16, put_u32, read_u32, reserve};
use super::hirc::{convert_hirc, validate_hirc_bounds};
use crate::error::{Error, Result};
use crate::manifest::LayoutState;

const BKHD: [u8; 4] = *b"BKHD";
const DIDX: [u8; 4] = *b"DIDX";
const DATA: [u8; 4] = *b"DATA";
const HIRC: [u8; 4] = *b"HIRC";
const STID: [u8; 4] = *b"STID";
const STMG: [u8; 4] = *b"STMG";
const ENVS: [u8; 4] = *b"ENVS";

pub(in crate::assets) fn classify(data: &[u8], label: &str) -> Result<LayoutState> {
  let chunks = parse_chunks(data, label)?;
  layout_state(&chunks, label)
}

pub(in crate::assets) fn convert(data: &[u8], label: &str) -> Result<Vec<u8>> {
  let chunks = parse_chunks(data, label)?;
  if layout_state(&chunks, label)? != LayoutState::SupportedX32 {
    return invalid(format!(
      "{label}: only Wwise soundbank version 88 can be converted"
    ));
  }
  let feedback = read_u32(required_chunk(&chunks, BKHD, label)?, 12, label)? != 0;
  let add_audio_devices = has_init_chunks(&chunks);
  let mut output = Vec::new();
  reserve(&mut output, data.len(), label)?;
  for chunk in chunks {
    match chunk.id {
      BKHD => push_chunk(
        &mut output,
        chunk.id,
        &convert_bkhd(chunk.payload, label)?,
        label,
      )?,
      HIRC => push_chunk(
        &mut output,
        chunk.id,
        &convert_hirc(chunk.payload, feedback, add_audio_devices, label)?,
        label,
      )?,
      STMG => push_chunk(
        &mut output,
        chunk.id,
        &convert_stmg(chunk.payload, label)?,
        label,
      )?,
      ENVS => push_chunk(
        &mut output,
        chunk.id,
        &convert_envs(chunk.payload, label)?,
        label,
      )?,
      STID => {}
      _ => push_chunk(&mut output, chunk.id, chunk.payload, label)?,
    }
  }
  Ok(output)
}

fn layout_state(chunks: &[Chunk<'_>], label: &str) -> Result<LayoutState> {
  let bkhd = required_chunk(chunks, BKHD, label)?;
  let hirc = optional_chunk(chunks, HIRC, label)?;
  match read_u32(bkhd, 0, label)? {
    88 => {
      validate_v88_bkhd(bkhd, label)?;
      validate_v88_chunks(chunks, label)?;
      if let Some(hirc) = hirc {
        let feedback = read_u32(bkhd, 12, label)? != 0;
        convert_hirc(hirc, feedback, has_init_chunks(chunks), label)?;
      }
      Ok(LayoutState::SupportedX32)
    }
    145 => {
      validate_v145_bkhd(bkhd, label)?;
      validate_common_chunks(chunks, label)?;
      if let Some(hirc) = hirc {
        validate_hirc_bounds(hirc, label)?;
      }
      Ok(LayoutState::AlreadyX64)
    }
    version => invalid(format!(
      "{label}: unsupported Wwise soundbank version {version}"
    )),
  }
}

fn has_init_chunks(chunks: &[Chunk<'_>]) -> bool {
  chunks.iter().any(|chunk| chunk.id == STMG) && chunks.iter().any(|chunk| chunk.id == ENVS)
}

fn validate_v145_bkhd(payload: &[u8], label: &str) -> Result<()> {
  if !matches!(payload.len(), 40 | 44 | 48) {
    return invalid(format!(
      "{label}: BKHD has size {}, expected an observed v145 size of 40, 44, or 48",
      payload.len()
    ));
  }
  if payload[40..].iter().any(|byte| *byte != 0) {
    return invalid(format!(
      "{label}: BKHD has an unsupported nonzero alignment tail"
    ));
  }
  Ok(())
}

fn validate_v88_bkhd(payload: &[u8], label: &str) -> Result<()> {
  if !matches!(payload.len(), 20 | 24 | 28 | 32) {
    return invalid(format!(
      "{label}: BKHD has size {}, expected an observed v88 size of 20, 24, 28, or 32",
      payload.len()
    ));
  }
  if payload[20..].iter().any(|byte| *byte != 0) {
    return invalid(format!("{label}: BKHD has an unsupported nonzero tail"));
  }
  Ok(())
}

fn validate_v88_chunks(chunks: &[Chunk<'_>], label: &str) -> Result<()> {
  for chunk in chunks {
    match chunk.id {
      BKHD | HIRC | DIDX | DATA | STID | STMG | ENVS => {}
      id => {
        return invalid(format!(
          "{label}: unsupported soundbank chunk {}",
          chunk_name(id)
        ));
      }
    }
  }

  validate_common_chunks(chunks, label)?;
  if let Some(stmg) = optional_chunk(chunks, STMG, label)? {
    convert_stmg(stmg, label)?;
  }
  if let Some(envs) = optional_chunk(chunks, ENVS, label)? {
    convert_envs(envs, label)?;
  }
  Ok(())
}

fn validate_common_chunks(chunks: &[Chunk<'_>], label: &str) -> Result<()> {
  let didx = optional_chunk(chunks, DIDX, label)?;
  let data = optional_chunk(chunks, DATA, label)?;
  match (didx, data) {
    (Some(didx), Some(data)) => validate_didx(didx, data, label)?,
    (Some(_), None) => return invalid(format!("{label}: DIDX chunk requires DATA")),
    (None, Some(_)) => return invalid(format!("{label}: DATA chunk requires DIDX")),
    (None, None) => {}
  }
  if let Some(stid) = optional_chunk(chunks, STID, label)? {
    validate_stid(stid, label)?;
  }
  Ok(())
}

fn validate_didx(didx: &[u8], data: &[u8], label: &str) -> Result<()> {
  if !didx.len().is_multiple_of(12) {
    return invalid(format!("{label}: DIDX size is not a multiple of 12"));
  }
  for (index, entry) in didx.chunks_exact(12).enumerate() {
    let offset = usize::try_from(read_u32(entry, 4, label)?).map_err(|_| {
      Error::Invalid(format!(
        "{label}: DIDX entry {index} offset does not fit usize"
      ))
    })?;
    let size = usize::try_from(read_u32(entry, 8, label)?).map_err(|_| {
      Error::Invalid(format!(
        "{label}: DIDX entry {index} size does not fit usize"
      ))
    })?;
    let end = offset
      .checked_add(size)
      .ok_or_else(|| Error::Invalid(format!("{label}: DIDX entry {index} range overflow")))?;
    if end > data.len() {
      return invalid(format!(
        "{label}: DIDX entry {index} range {offset}..{end} exceeds DATA size {}",
        data.len()
      ));
    }
  }
  Ok(())
}

fn validate_stid(payload: &[u8], label: &str) -> Result<()> {
  let mut reader = Reader::new(payload, label);
  reader.u32()?;
  let count = reader.u32()?;
  for _ in 0..count {
    reader.u32()?;
    let len = reader.u8()? as usize;
    reader.bytes(len)?;
  }
  reader.finish("STID")
}

fn convert_stmg(payload: &[u8], label: &str) -> Result<Vec<u8>> {
  let mut reader = Reader::new(payload, label);
  let mut out = Vec::new();
  put_u16(&mut out, 0);
  out.extend_from_slice(reader.bytes(4)?);
  put_u16(&mut out, reader.u16()?);
  put_u16(&mut out, 50);

  let state_groups = reader.u32()?;
  put_u32(&mut out, state_groups);
  for _ in 0..state_groups {
    put_u32(&mut out, reader.u32()?);
    put_u32(&mut out, reader.u32()?);
    let transitions = reader.u32()?;
    put_u32(&mut out, transitions);
    out.extend_from_slice(reader.bytes(transitions as usize * 12)?);
  }

  let switch_groups = reader.u32()?;
  put_u32(&mut out, switch_groups);
  for _ in 0..switch_groups {
    put_u32(&mut out, reader.u32()?);
    put_u32(&mut out, reader.u32()?);
    out.push(0);
    copy_rtpc_graph(&mut reader, &mut out)?;
  }

  let parameters = reader.u32()?;
  put_u32(&mut out, parameters);
  for _ in 0..parameters {
    out.extend_from_slice(reader.bytes(8)?);
    out.extend_from_slice(&[0; 13]);
  }
  put_u32(&mut out, 0);
  reader.finish("STMG")?;
  Ok(out)
}

fn convert_envs(payload: &[u8], label: &str) -> Result<Vec<u8>> {
  let mut reader = Reader::new(payload, label);
  let mut out = Vec::new();
  for _ in 0..2 {
    copy_env_curve(&mut reader, &mut out)?;
    copy_env_curve(&mut reader, &mut out)?;
    out.extend_from_slice(&[0, 0]);
    put_u16(&mut out, 2);
    out.extend_from_slice(&0_f32.to_le_bytes());
    out.extend_from_slice(&0_f32.to_le_bytes());
    put_u32(&mut out, 4);
    out.extend_from_slice(&100_f32.to_le_bytes());
    out.extend_from_slice(&100_f32.to_le_bytes());
    put_u32(&mut out, 4);
  }
  reader.finish("ENVS")?;
  Ok(out)
}

fn copy_env_curve(reader: &mut Reader<'_, '_>, out: &mut Vec<u8>) -> Result<()> {
  out.push(reader.u8()?);
  out.push(reader.u8()?);
  let points = reader.u16()?;
  put_u16(out, points);
  out.extend_from_slice(reader.bytes(points as usize * 12)?);
  Ok(())
}

#[derive(Clone, Copy)]
struct Chunk<'a> {
  id: [u8; 4],
  payload: &'a [u8],
}

fn parse_chunks<'a>(data: &'a [u8], label: &str) -> Result<Vec<Chunk<'a>>> {
  let mut chunks = Vec::new();
  let mut reader = Reader::new(data, label);
  while !reader.is_empty() {
    let id: [u8; 4] = reader
      .bytes(4)?
      .try_into()
      .map_err(|_| Error::Invalid(format!("{label}: truncated soundbank chunk identifier")))?;
    let size = usize::try_from(reader.u32()?)
      .map_err(|_| Error::Invalid(format!("{label}: chunk size does not fit usize")))?;
    chunks.push(Chunk {
      id,
      payload: reader.bytes(size)?,
    });
  }
  Ok(chunks)
}

fn optional_chunk<'a>(chunks: &[Chunk<'a>], id: [u8; 4], label: &str) -> Result<Option<&'a [u8]>> {
  let mut matches = chunks.iter().filter(|chunk| chunk.id == id);
  let chunk = matches.next();
  if matches.next().is_some() {
    return invalid(format!("{label}: duplicate {} chunk", chunk_name(id)));
  }
  Ok(chunk.map(|chunk| chunk.payload))
}

fn required_chunk<'a>(chunks: &[Chunk<'a>], id: [u8; 4], label: &str) -> Result<&'a [u8]> {
  let mut matches = chunks.iter().filter(|chunk| chunk.id == id);
  let chunk = matches.next().ok_or_else(|| {
    Error::Invalid(format!(
      "{label}: missing required {} chunk",
      chunk_name(id)
    ))
  })?;
  if matches.next().is_some() {
    return invalid(format!(
      "{label}: duplicate required {} chunk",
      chunk_name(id)
    ));
  }
  Ok(chunk.payload)
}

fn convert_bkhd(payload: &[u8], label: &str) -> Result<Vec<u8>> {
  validate_v88_bkhd(payload, label)?;
  let mut output = Vec::with_capacity(40);
  put_u32(&mut output, 145);
  output.extend_from_slice(&payload[4..8]);
  put_u32(
    &mut output,
    wwise_hash(v88_language(read_u32(payload, 8, label)?, label)?),
  );
  put_u32(&mut output, 16);
  output.extend_from_slice(&payload[16..20]);
  put_u32(&mut output, 0);
  output.extend_from_slice(&[0; 16]);
  Ok(output)
}

fn v88_language(id: u32, label: &str) -> Result<&'static str> {
  const LANGUAGES: [&str; 38] = [
    "SFX",
    "Arabic",
    "Bulgarian",
    "Chinese(HK)",
    "Chinese(PRC)",
    "Chinese(Taiwan)",
    "Czech",
    "Danish",
    "Dutch",
    "English(Australia)",
    "English(India)",
    "English(UK)",
    "English(US)",
    "Finnish",
    "French(Canada)",
    "French(France)",
    "German",
    "Greek",
    "Hebrew",
    "Hungarian",
    "Indonesian",
    "Italian",
    "Japanese",
    "Korean",
    "Latin",
    "Norwegian",
    "Polish",
    "Portuguese(Brazil)",
    "Portuguese(Portugal)",
    "Romanian",
    "Russian",
    "Slovenian",
    "Spanish(Mexico)",
    "Spanish(Spain)",
    "Spanish(US)",
    "Swedish",
    "Turkish",
    "Ukrainian",
  ];
  LANGUAGES
    .get(id as usize)
    .copied()
    .ok_or_else(|| Error::Invalid(format!("{label}: unsupported v88 language ID {id}")))
}

fn wwise_hash(value: &str) -> u32 {
  value.bytes().fold(2_166_136_261, |hash, byte| {
    hash.wrapping_mul(16_777_619) ^ u32::from(byte.to_ascii_lowercase())
  })
}

fn push_chunk(output: &mut Vec<u8>, id: [u8; 4], payload: &[u8], label: &str) -> Result<()> {
  let size = u32::try_from(payload.len())
    .map_err(|_| Error::Invalid(format!("{label}: {} chunk is too large", chunk_name(id))))?;
  reserve(output, payload.len().saturating_add(8), label)?;
  output.extend_from_slice(&id);
  put_u32(output, size);
  output.extend_from_slice(payload);
  Ok(())
}

fn chunk_name(id: [u8; 4]) -> String {
  String::from_utf8_lossy(&id).into_owned()
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
