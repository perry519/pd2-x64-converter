use super::*;
use crate::assets::read_u64;

#[test]
fn font_conversion_expands_legacy_records() {
  let converted = convert(&legacy_font(), "fixture.font").unwrap();

  assert_eq!(read_u64(&converted, 0, "glyph count").unwrap(), 1);
  assert_eq!(
    read_u64(&converted, 16, "glyph offset").unwrap(),
    FONT_X64_HEADER_SIZE as u64
  );
  assert_eq!(
    &converted[FONT_X64_HEADER_SIZE..FONT_X64_HEADER_SIZE + 12],
    &[1, 0x0f, 2, 3, 4, 5, 6, 0xcc, 7, 8, 9, 10]
  );
  assert!(converted.ends_with(b"zS07"));
}

fn legacy_font() -> Vec<u8> {
  let mut data = vec![0; 92];
  put_u32(&mut data, 0, 1);
  put_u32(&mut data, 4, 1);
  put_u32(&mut data, 8, 92);
  put_u32(&mut data, 20, 1);
  put_u32(&mut data, 24, 1);
  put_u32(&mut data, 28, 104);
  put_u32(&mut data, 68, 112);
  put_u32(&mut data, 76, 512);
  put_u32(&mut data, 80, 256);
  data.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
  data.extend_from_slice(&[0, 0]);
  data.extend_from_slice(&[65, 0, 0, 0, 0, 0, 0, 0]);
  data.extend_from_slice(b"metadata-zS07");
  data
}

fn put_u32(data: &mut [u8], offset: usize, value: u32) {
  data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
