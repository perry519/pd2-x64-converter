use std::collections::BTreeMap;

use crate::error::{Error, Result, invalid};
use crate::manifest::LayoutState;

use super::{push_u16, push_u32, read_array, read_u16, read_u32, require_range};

const WWISE_IMA_FORMAT: u16 = 0x0002;
const WWISE_PTADPCM_FORMAT: u16 = 0x8311;
const WWISE_ADPCM_FRAME_SIZE: usize = 0x24;
const WWISE_PTADPCM_EXTRA: [u8; 6] = [0x00, 0x00, 0x02, 0x31, 0x00, 0x00];
const IMA_STEP_SIZE_TABLE: [i32; 89] = [
  7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66, 73,
  80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408, 449, 494,
  544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272, 2499,
  2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630, 9493, 10442, 11487,
  12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794, 32767,
];
const IMA_INDEX_TABLE: [i32; 16] = [-1, -1, -1, -1, 2, 4, 6, 8, -1, -1, -1, -1, 2, 4, 6, 8];
const PTADPCM_STEPS: [[i32; 16]; 12] = [
  [-14, -10, -7, -5, -3, -2, -1, 0, 0, 1, 2, 3, 5, 7, 10, 14],
  [
    -28, -20, -14, -10, -7, -5, -3, -1, 1, 3, 5, 7, 10, 14, 20, 28,
  ],
  [
    -56, -40, -28, -20, -14, -10, -6, -2, 2, 6, 10, 14, 20, 28, 40, 56,
  ],
  [
    -112, -80, -56, -40, -28, -20, -12, -4, 4, 12, 20, 28, 40, 56, 80, 112,
  ],
  [
    -224, -160, -112, -80, -56, -40, -24, -8, 8, 24, 40, 56, 80, 112, 160, 224,
  ],
  [
    -448, -320, -224, -160, -112, -80, -48, -16, 16, 48, 80, 112, 160, 224, 320, 448,
  ],
  [
    -896, -640, -448, -320, -224, -160, -96, -32, 32, 96, 160, 224, 320, 448, 640, 896,
  ],
  [
    -1792, -1280, -896, -640, -448, -320, -192, -64, 64, 192, 320, 448, 640, 896, 1280, 1792,
  ],
  [
    -3584, -2560, -1792, -1280, -896, -640, -384, -128, 128, 384, 640, 896, 1280, 1792, 2560, 3584,
  ],
  [
    -7168, -5120, -3584, -2560, -1792, -1280, -768, -256, 256, 768, 1280, 1792, 2560, 3584, 5120,
    7168,
  ],
  [
    -14336, -10240, -7168, -5120, -3584, -2560, -1536, -512, 512, 1536, 2560, 3584, 5120, 7168,
    10240, 14336,
  ],
  [
    -28672, -20480, -14336, -10240, -7168, -5120, -3072, -1024, 1024, 3072, 5120, 7168, 10240,
    14336, 20480, 28672,
  ],
];
const PTADPCM_NEXT_INDEXES: [[usize; 16]; 12] = [
  [2, 2, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 2, 2],
  [3, 3, 2, 2, 1, 1, 1, 0, 0, 1, 1, 1, 2, 2, 3, 3],
  [4, 4, 3, 3, 2, 2, 2, 1, 1, 2, 2, 2, 3, 3, 4, 4],
  [5, 5, 4, 4, 3, 3, 3, 2, 2, 3, 3, 3, 4, 4, 5, 5],
  [6, 6, 5, 5, 4, 4, 4, 3, 3, 4, 4, 4, 5, 5, 6, 6],
  [7, 7, 6, 6, 5, 5, 5, 4, 4, 5, 5, 5, 6, 6, 7, 7],
  [8, 8, 7, 7, 6, 6, 6, 5, 5, 6, 6, 6, 7, 7, 8, 8],
  [9, 9, 8, 8, 7, 7, 7, 6, 6, 7, 7, 7, 8, 8, 9, 9],
  [10, 10, 9, 9, 8, 8, 8, 7, 7, 8, 8, 8, 9, 9, 10, 10],
  [11, 11, 10, 10, 9, 9, 9, 8, 8, 9, 9, 9, 10, 10, 11, 11],
  [11, 11, 11, 11, 10, 10, 10, 9, 9, 10, 10, 10, 11, 11, 11, 11],
  [
    11, 11, 11, 11, 11, 11, 11, 10, 10, 11, 11, 11, 11, 11, 11, 11,
  ],
];

pub(super) fn classify(data: &[u8], label: &str) -> Result<LayoutState> {
  let chunks = read_riff_chunks(data, label)?;
  let fmt = chunks
    .get(b"fmt ")
    .ok_or_else(|| Error::Invalid(format!("{label}: missing fmt chunk")))?;
  let payload = chunks
    .get(b"data")
    .ok_or_else(|| Error::Invalid(format!("{label}: missing data chunk")))?;
  let fmt = parse_stream_fmt(fmt, label)?;
  if fmt.channels == 0 {
    return invalid!("{label}: invalid channel count");
  }
  if fmt.format_tag == WWISE_PTADPCM_FORMAT {
    return Ok(LayoutState::AlreadyX64);
  }
  if fmt.format_tag != WWISE_IMA_FORMAT {
    return invalid!(
      "{label}: unsupported Wwise stream codec 0x{:04x}",
      fmt.format_tag
    );
  }
  validate_wwise_ima_layout(
    payload,
    fmt.channels,
    fmt.block_align,
    fmt.bits_per_sample,
    label,
  )?;
  Ok(LayoutState::SupportedX32)
}

#[derive(Clone, Copy)]
struct StreamFmt {
  format_tag: u16,
  channels: u16,
  sample_rate: u32,
  block_align: u16,
  bits_per_sample: u16,
}

pub(super) fn convert(data: &[u8], label: &str) -> Result<Vec<u8>> {
  let chunks = read_riff_chunks(data, label)?;
  let fmt = chunks
    .get(b"fmt ")
    .ok_or_else(|| Error::Invalid(format!("{label}: missing fmt chunk")))?;
  let payload = chunks
    .get(b"data")
    .ok_or_else(|| Error::Invalid(format!("{label}: missing data chunk")))?;
  let fmt = parse_stream_fmt(fmt, label)?;
  if fmt.channels == 0 {
    return invalid!("{label}: invalid channel count");
  }
  if fmt.format_tag == WWISE_PTADPCM_FORMAT {
    return Ok(data.to_vec());
  }
  if fmt.format_tag != WWISE_IMA_FORMAT {
    return invalid!(
      "{label}: unsupported Wwise stream codec 0x{:04x}",
      fmt.format_tag
    );
  }
  validate_wwise_ima_layout(
    payload,
    fmt.channels,
    fmt.block_align,
    fmt.bits_per_sample,
    label,
  )?;

  let converted_payload = convert_wwise_ima_data_to_ptadpcm(payload, fmt.channels, label)?;
  let (hash_chunk, junk_chunk) = stream_metadata_chunks(&chunks, &converted_payload);
  Ok(build_riff_stream(
    &build_ptadpcm_fmt(fmt.channels, fmt.sample_rate),
    &hash_chunk,
    &junk_chunk,
    &converted_payload,
  ))
}

fn read_riff_chunks(data: &[u8], label: &str) -> Result<BTreeMap<[u8; 4], Vec<u8>>> {
  if data.len() < 12 || &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
    return invalid!("{label}: not a RIFF/WAVE stream");
  }
  let riff_size = read_u32(data, 4, label)? as usize;
  if riff_size.checked_add(8).is_none_or(|end| end > data.len()) {
    return invalid!("{label}: RIFF size exceeds file size");
  }

  let mut chunks = BTreeMap::new();
  let mut offset = 12;
  while offset + 8 <= data.len() {
    let chunk_id = read_array(data, offset, label)?;
    let chunk_size = read_u32(data, offset + 4, label)? as usize;
    let chunk_start = offset + 8;
    require_range(data, chunk_start, chunk_size, label)?;
    chunks.insert(
      chunk_id,
      data[chunk_start..chunk_start + chunk_size].to_vec(),
    );
    offset = chunk_start + chunk_size + (chunk_size & 1);
  }
  if offset != data.len() {
    return invalid!("{label}: trailing partial RIFF chunk");
  }
  Ok(chunks)
}

fn parse_stream_fmt(fmt: &[u8], label: &str) -> Result<StreamFmt> {
  if fmt.len() < 18 {
    return invalid!("{label}: fmt chunk is too small");
  }
  let extra_size = read_u16(fmt, 16, label)? as usize;
  if fmt.len() != 18 + extra_size {
    return invalid!("{label}: fmt extra size mismatch");
  }
  Ok(StreamFmt {
    format_tag: read_u16(fmt, 0, label)?,
    channels: read_u16(fmt, 2, label)?,
    sample_rate: read_u32(fmt, 4, label)?,
    block_align: read_u16(fmt, 12, label)?,
    bits_per_sample: read_u16(fmt, 14, label)?,
  })
}

fn validate_wwise_ima_layout(
  payload: &[u8],
  channels: u16,
  block_align: u16,
  bits_per_sample: u16,
  label: &str,
) -> Result<()> {
  let expected_block_align = WWISE_ADPCM_FRAME_SIZE
    .checked_mul(usize::from(channels))
    .ok_or_else(|| Error::Invalid(format!("{label}: stream block align overflow")))?;
  if bits_per_sample != 4 || usize::from(block_align) != expected_block_align {
    return invalid!("{label}: unsupported Wwise IMA layout");
  }
  if !payload.len().is_multiple_of(expected_block_align) {
    return invalid!(
      "{label}: stream data is not aligned to {expected_block_align}-byte Wwise ADPCM blocks"
    );
  }
  Ok(())
}

fn convert_wwise_ima_data_to_ptadpcm(data: &[u8], channels: u16, label: &str) -> Result<Vec<u8>> {
  let block_size = WWISE_ADPCM_FRAME_SIZE
    .checked_mul(usize::from(channels))
    .ok_or_else(|| Error::Invalid(format!("{label}: stream block size overflow")))?;
  if !data.len().is_multiple_of(block_size) {
    return invalid!("{label}: stream data is not aligned to {block_size}-byte Wwise ADPCM blocks");
  }

  let mut out = Vec::with_capacity(data.len());
  for block_offset in (0..data.len()).step_by(block_size) {
    for channel in 0..usize::from(channels) {
      let frame_offset = block_offset + channel * WWISE_ADPCM_FRAME_SIZE;
      let samples =
        decode_wwise_ima_frame(&data[frame_offset..frame_offset + WWISE_ADPCM_FRAME_SIZE])?;
      out.extend_from_slice(&encode_ptadpcm_frame(&samples)?);
    }
  }
  Ok(out)
}

fn decode_wwise_ima_frame(frame: &[u8]) -> Result<Vec<i16>> {
  if frame.len() != WWISE_ADPCM_FRAME_SIZE {
    return invalid!("Wwise IMA frame must be 36 bytes");
  }
  let mut sample = i16::from_le_bytes(
    frame[0..2]
      .try_into()
      .expect("Wwise IMA frame length was checked"),
  ) as i32;
  let mut step_index = usize::from(frame[2].min(88));
  let mut samples = Vec::with_capacity(64);
  samples.push(sample as i16);
  for sample_index in 1..64 {
    let packed = frame[4 + (sample_index - 1) / 2];
    let nibble = if sample_index & 1 == 0 {
      packed >> 4
    } else {
      packed
    } & 0x0f;
    let step = IMA_STEP_SIZE_TABLE[step_index];
    let mut delta = ((i32::from(nibble & 0x07) * 2 + 1) * step) >> 3;
    if nibble & 0x08 != 0 {
      delta = -delta;
    }
    sample = clamp_i16(sample + delta);
    step_index = (step_index as i32 + IMA_INDEX_TABLE[usize::from(nibble)]).clamp(0, 88) as usize;
    samples.push(sample as i16);
  }
  Ok(samples)
}

fn encode_ptadpcm_frame(samples: &[i16]) -> Result<Vec<u8>> {
  if samples.len() != 64 {
    return invalid!("PTADPCM frame requires 64 samples");
  }

  let mut hist2 = i32::from(samples[0]);
  let mut hist1 = i32::from(samples[1]);
  let mut index = choose_ptadpcm_start_index(samples);
  let mut frame = Vec::with_capacity(WWISE_ADPCM_FRAME_SIZE);
  frame.extend_from_slice(&samples[0].to_le_bytes());
  frame.extend_from_slice(&samples[1].to_le_bytes());
  frame.push(index as u8);

  let mut nibbles = Vec::with_capacity(62);
  for target in samples.iter().skip(2).map(|sample| i32::from(*sample)) {
    let delta = target - (2 * hist1 - hist2);
    let nibble = nearest_ptadpcm_nibble(index, delta);
    let step = ptadpcm_step(index, nibble);
    let next_index = ptadpcm_next_index(index, nibble);
    let sample = clamp_i16(step + 2 * hist1 - hist2);
    nibbles.push(nibble as u8);
    hist2 = hist1;
    hist1 = sample;
    index = next_index;
  }

  for pair in nibbles.chunks_exact(2) {
    frame.push(pair[0] | (pair[1] << 4));
  }
  Ok(frame)
}

fn choose_ptadpcm_start_index(samples: &[i16]) -> usize {
  if samples.len() < 3 {
    return 0;
  }
  let delta = i32::from(samples[2]) - (2 * i32::from(samples[1]) - i32::from(samples[0]));
  let mut best_index = 0;
  let mut best_error = (ptadpcm_step(0, nearest_ptadpcm_nibble(0, delta)) - delta).abs();
  for index in 1..PTADPCM_STEPS.len() {
    let nibble = nearest_ptadpcm_nibble(index, delta);
    let error = (ptadpcm_step(index, nibble) - delta).abs();
    if error < best_error {
      best_index = index;
      best_error = error;
    }
  }
  best_index
}

fn nearest_ptadpcm_nibble(index: usize, delta: i32) -> usize {
  let row = &PTADPCM_STEPS[index];
  let position = row.partition_point(|step| *step < delta);
  if position == 0 {
    return 0;
  }
  if position >= row.len() {
    return row.len() - 1;
  }
  if (row[position] - delta).abs() < (row[position - 1] - delta).abs() {
    position
  } else {
    position - 1
  }
}

fn ptadpcm_step(index: usize, nibble: usize) -> i32 {
  PTADPCM_STEPS[index][nibble]
}

fn ptadpcm_next_index(index: usize, nibble: usize) -> usize {
  PTADPCM_NEXT_INDEXES[index][nibble]
}

fn build_ptadpcm_fmt(channels: u16, sample_rate: u32) -> Vec<u8> {
  let block_align = WWISE_ADPCM_FRAME_SIZE * usize::from(channels);
  let avg_bytes_per_second = sample_rate.saturating_mul(block_align as u32) / 64;
  let mut fmt = Vec::with_capacity(18 + WWISE_PTADPCM_EXTRA.len());
  push_u16(&mut fmt, WWISE_PTADPCM_FORMAT);
  push_u16(&mut fmt, channels);
  push_u32(&mut fmt, sample_rate);
  push_u32(&mut fmt, avg_bytes_per_second);
  push_u16(&mut fmt, block_align as u16);
  push_u16(&mut fmt, 4);
  push_u16(&mut fmt, WWISE_PTADPCM_EXTRA.len() as u16);
  fmt.extend_from_slice(&WWISE_PTADPCM_EXTRA);
  fmt
}

fn build_riff_stream(fmt: &[u8], hash_chunk: &[u8], junk_chunk: &[u8], data: &[u8]) -> Vec<u8> {
  let mut body = b"WAVE".to_vec();
  for (chunk_id, payload) in [
    (b"fmt ", fmt),
    (b"hash", hash_chunk),
    (b"junk", junk_chunk),
    (b"data", data),
  ] {
    body.extend_from_slice(chunk_id);
    push_u32(&mut body, payload.len() as u32);
    body.extend_from_slice(payload);
    if payload.len() & 1 != 0 {
      body.push(0);
    }
  }
  let mut out = b"RIFF".to_vec();
  push_u32(&mut out, body.len() as u32);
  out.extend_from_slice(&body);
  out
}

fn stream_metadata_chunks(
  source_chunks: &BTreeMap<[u8; 4], Vec<u8>>,
  converted_data: &[u8],
) -> (Vec<u8>, Vec<u8>) {
  let hash_chunk = source_chunks.get(b"hash");
  let hash_chunk = if hash_chunk.is_some_and(|chunk| chunk.len() == 16) {
    hash_chunk.expect("hash presence was checked").clone()
  } else {
    md5::compute(converted_data).0.to_vec()
  };

  let junk_chunk = source_chunks
    .get(b"junk")
    .or_else(|| source_chunks.get(b"JUNK"));
  let junk_chunk = if junk_chunk.is_some_and(|chunk| chunk.len() == 12) {
    junk_chunk.expect("junk presence was checked").clone()
  } else {
    vec![0; 12]
  };

  (hash_chunk, junk_chunk)
}

fn clamp_i16(value: i32) -> i32 {
  value.clamp(i32::from(i16::MIN), i32::from(i16::MAX))
}

#[cfg(test)]
#[path = "stream_tests.rs"]
mod tests;
