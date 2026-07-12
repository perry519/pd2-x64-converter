use super::*;

#[test]
fn scriptdata_empty_x32_converts_to_x64_headers() {
  let converted = convert(&empty_scriptdata_x32(), "empty.world", ".world").unwrap();
  let parsed = parse_scriptdata(&converted, "converted").unwrap();

  assert_eq!(parsed.pointer_size, 8);
  assert_eq!(parsed.count_size, 8);
  assert_eq!(converted.len(), 204);
}

fn empty_scriptdata_x32() -> Vec<u8> {
  let mut data = vec![0; 4 + 6 * 16 + 4];
  for index in 0..6 {
    let offset = 4 + index * 16;
    put_u32(&mut data, offset, 0);
    put_u32(&mut data, offset + 4, 0);
    put_u32(&mut data, offset + 8, 0);
    put_u32(&mut data, offset + 12, 0);
  }
  put_u32(&mut data, 4 + 6 * 16, 0);
  data
}

fn put_u32(data: &mut [u8], offset: usize, value: u32) {
  data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
