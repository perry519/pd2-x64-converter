use super::*;

#[test]
fn massunit_conversion_widens_legacy_records() {
  let converted = convert(&legacy_massunit(), "fixture.massunit").unwrap();

  assert_eq!(read_u64(&converted, 0, "unit count").unwrap(), 1);
  assert_eq!(
    read_u64(&converted, 16, "record offset").unwrap(),
    MASSUNIT_X64_HEADER_SIZE as u64
  );
  assert_eq!(
    read_u64(&converted, MASSUNIT_X64_HEADER_SIZE, "name hash").unwrap(),
    0x1122_3344_5566_7788
  );
  assert_eq!(
    read_u64(
      &converted,
      MASSUNIT_X64_HEADER_SIZE + 32,
      "placement offset"
    )
    .unwrap(),
    (MASSUNIT_X64_HEADER_SIZE + MASSUNIT_X64_RECORD_SIZE) as u64
  );
  assert!(converted.ends_with(&MASSUNIT_SENTINEL));
}

fn legacy_massunit() -> Vec<u8> {
  let mut data = vec![0; MASSUNIT_X32_HEADER_SIZE + MASSUNIT_X32_RECORD_SIZE];
  put_u32(&mut data, 0, 1);
  put_u32(&mut data, 4, 1);
  put_u32(&mut data, 8, MASSUNIT_X32_HEADER_SIZE as u32);
  put_u64(&mut data, MASSUNIT_X32_HEADER_SIZE, 0x1122_3344_5566_7788);
  put_u32(&mut data, MASSUNIT_X32_HEADER_SIZE + 8, 1);
  put_u32(&mut data, MASSUNIT_X32_HEADER_SIZE + 12, 1);
  put_u32(&mut data, MASSUNIT_X32_HEADER_SIZE + 16, 1);
  put_u32(
    &mut data,
    MASSUNIT_X32_HEADER_SIZE + 20,
    (MASSUNIT_X32_HEADER_SIZE + MASSUNIT_X32_RECORD_SIZE) as u32,
  );
  data.extend_from_slice(&[0xab; MASSUNIT_PLACEMENT_SIZE]);
  data
}

fn put_u32(data: &mut [u8], offset: usize, value: u32) {
  data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(data: &mut [u8], offset: usize, value: u64) {
  data[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}
