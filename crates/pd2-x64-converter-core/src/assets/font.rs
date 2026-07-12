use crate::error::{Error, Result, invalid};
use crate::manifest::LayoutState;

use super::{push_u32, push_u64, read_u32, read_u64_lossy, require_range};

const FONT_X64_HEADER_SIZE: usize = 168;

pub(super) fn classify(data: &[u8]) -> std::result::Result<LayoutState, String> {
  match convert(data, "font") {
    Ok(_) => Ok(LayoutState::SupportedX32),
    Err(error) if looks_like_x64_font(data) => {
      let _ = error;
      Ok(LayoutState::AlreadyX64)
    }
    Err(error) => Err(error.to_string()),
  }
}

pub(super) fn looks_like_x64_font(data: &[u8]) -> bool {
  if data.len() < FONT_X64_HEADER_SIZE {
    return false;
  }
  let glyph_offset = read_u64_lossy(data, 16);
  let metadata_offset = read_u64_lossy(data, 128);
  glyph_offset == Some(FONT_X64_HEADER_SIZE as u64)
    && metadata_offset.is_some_and(|offset| offset as usize <= data.len())
    && data.ends_with(b"zS07")
}

pub(super) fn convert(data: &[u8], label: &str) -> Result<Vec<u8>> {
  if data.len() < 92 {
    return invalid!("{label}: too small for a legacy x32 .font header");
  }

  let glyph_count = read_u32(data, 0, label)? as usize;
  let glyph_capacity = read_u32(data, 4, label)? as usize;
  let glyph_offset = read_u32(data, 8, label)? as usize;
  let codepoint_count = read_u32(data, 20, label)? as usize;
  let codepoint_capacity = read_u32(data, 24, label)? as usize;
  let codepoint_offset = read_u32(data, 28, label)? as usize;
  let metadata_offset = read_u32(data, 68, label)? as usize;

  if glyph_count != glyph_capacity {
    return invalid!("{label}: glyph size/capacity mismatch {glyph_count}/{glyph_capacity}");
  }
  if codepoint_count != codepoint_capacity {
    return invalid!(
      "{label}: codepoint size/capacity mismatch {codepoint_count}/{codepoint_capacity}"
    );
  }
  if !(92 <= glyph_offset
    && glyph_offset < codepoint_offset
    && codepoint_offset <= metadata_offset
    && metadata_offset <= data.len())
  {
    return invalid!("{label}: invalid section offsets");
  }

  let glyph_bytes = glyph_count
    .checked_mul(10)
    .ok_or_else(|| Error::Invalid(format!("{label}: glyph byte count overflow")))?;
  let glyph_padding = (4 - (glyph_bytes % 4)) % 4;
  let expected_codepoint_offset = glyph_offset + glyph_bytes + glyph_padding;
  if codepoint_offset != expected_codepoint_offset {
    return invalid!(
      "{label}: unexpected codepoint offset {codepoint_offset}, expected {expected_codepoint_offset}"
    );
  }
  require_range(data, glyph_offset + glyph_bytes, glyph_padding, label)?;
  if data[glyph_offset + glyph_bytes..codepoint_offset] != vec![0; glyph_padding] {
    return invalid!("{label}: non-zero glyph padding");
  }

  let codepoint_bytes = codepoint_count
    .checked_mul(8)
    .ok_or_else(|| Error::Invalid(format!("{label}: codepoint byte count overflow")))?;
  let codepoint_end = codepoint_offset + codepoint_bytes;
  if codepoint_end > metadata_offset {
    return invalid!("{label}: codepoint table overlaps metadata");
  }

  let glyphs_old = &data[glyph_offset..glyph_offset + glyph_bytes];
  let codepoints = &data[codepoint_offset..codepoint_end];
  let kernings = &data[codepoint_end..metadata_offset];
  if !kernings.len().is_multiple_of(12) {
    return invalid!("{label}: inferred kerning block is not 12-byte aligned");
  }
  let metadata = &data[metadata_offset..];
  if !metadata.ends_with(b"zS07") {
    return invalid!("{label}: metadata tail does not end with zS07");
  }

  let mut glyphs_new = Vec::with_capacity(glyph_count * 12);
  for index in 0..glyph_count {
    let record = &glyphs_old[index * 10..(index + 1) * 10];
    glyphs_new.extend_from_slice(&[
      record[0], 0x0f, record[1], record[2], record[3], record[4], record[5], 0xcc,
    ]);
    glyphs_new.extend_from_slice(&record[6..10]);
  }

  let glyph_offset_new = FONT_X64_HEADER_SIZE;
  let codepoint_offset_new = glyph_offset_new + glyphs_new.len();
  let kerning_offset_new = codepoint_offset_new + codepoints.len();
  let metadata_offset_new = kerning_offset_new + kernings.len();
  let kerning_count = kernings.len() / 12;

  let mut header = Vec::with_capacity(FONT_X64_HEADER_SIZE);
  push_u64(&mut header, glyph_count as u64);
  push_u64(&mut header, glyph_count as u64);
  push_u64(&mut header, glyph_offset_new as u64);
  push_u64(&mut header, 0);
  push_u64(&mut header, 0);
  push_u64(&mut header, codepoint_count as u64);
  push_u64(&mut header, codepoint_count as u64);
  push_u64(&mut header, codepoint_offset_new as u64);
  push_u64(&mut header, 0);
  push_u64(&mut header, 1);
  push_u64(&mut header, 0);
  push_u64(&mut header, kerning_count as u64);
  push_u64(&mut header, kerning_count as u64);
  push_u64(
    &mut header,
    if kerning_count == 0 {
      0
    } else {
      kerning_offset_new as u64
    },
  );
  push_u64(&mut header, 0);
  push_u64(&mut header, 1);
  push_u64(&mut header, 0);
  push_u64(&mut header, metadata_offset_new as u64);
  header.extend_from_slice(&data[72..92]);
  push_u32(&mut header, 0);
  if header.len() != FONT_X64_HEADER_SIZE {
    return invalid!("{label}: internal font header size bug");
  }

  let mut converted = header;
  converted.extend_from_slice(&glyphs_new);
  converted.extend_from_slice(codepoints);
  converted.extend_from_slice(kernings);
  converted.extend_from_slice(metadata);
  Ok(converted)
}

#[cfg(test)]
#[path = "font_tests.rs"]
mod tests;
