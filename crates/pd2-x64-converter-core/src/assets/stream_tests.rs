use super::*;

#[test]
fn stream_conversion_rewrites_ima_to_ptadpcm() {
  let converted = convert(&legacy_stream([1; 16], [2; 12]), "fixture.stream").unwrap();
  let chunks = read_riff_chunks(&converted, "converted.stream").unwrap();
  let fmt = parse_stream_fmt(chunks.get(b"fmt ").unwrap(), "converted fmt").unwrap();
  let mut expected_data = vec![0; 5];
  expected_data.extend_from_slice(&[0x77; 31]);

  assert_eq!(fmt.format_tag, WWISE_PTADPCM_FORMAT);
  assert_eq!(chunks.get(b"hash").unwrap().as_slice(), &[1; 16]);
  assert_eq!(chunks.get(b"junk").unwrap().as_slice(), &[2; 12]);
  assert_eq!(chunks.get(b"data").unwrap().as_slice(), expected_data);
}

#[test]
fn converts_non_silent_frame_to_expected_ptadpcm_bytes() {
  let mut ima_frame = [0; WWISE_ADPCM_FRAME_SIZE];
  ima_frame[4..].fill(0x77);
  let converted = convert(
    &legacy_stream_with_frame([1; 16], [2; 12], ima_frame),
    "fixture.stream",
  )
  .unwrap();
  let chunks = read_riff_chunks(&converted, "converted.stream").unwrap();
  let expected_data = [
    0x00, 0x00, 0x0d, 0x00, 0x05, 0xa8, 0xeb, 0xdc, 0xdc, 0x5d, 0x74, 0x79, 0x78, 0x77, 0x77, 0x77,
    0x77, 0x76, 0x77, 0x77, 0x77, 0x77, 0x77, 0x77, 0x77, 0x77, 0x77, 0x77, 0x77, 0x77, 0x77, 0x77,
    0x77, 0x77, 0x77, 0x77,
  ];

  assert_eq!(chunks.get(b"data").unwrap().as_slice(), expected_data);
}

fn legacy_stream(hash: [u8; 16], junk: [u8; 12]) -> Vec<u8> {
  legacy_stream_with_frame(hash, junk, [0; WWISE_ADPCM_FRAME_SIZE])
}

fn legacy_stream_with_frame(
  hash: [u8; 16],
  junk: [u8; 12],
  frame: [u8; WWISE_ADPCM_FRAME_SIZE],
) -> Vec<u8> {
  let mut fmt = Vec::new();
  push_u16(&mut fmt, WWISE_IMA_FORMAT);
  push_u16(&mut fmt, 1);
  push_u32(&mut fmt, 48_000);
  push_u32(&mut fmt, 48_000 * WWISE_ADPCM_FRAME_SIZE as u32 / 64);
  push_u16(&mut fmt, WWISE_ADPCM_FRAME_SIZE as u16);
  push_u16(&mut fmt, 4);
  push_u16(&mut fmt, 0);
  build_riff_stream(&fmt, &hash, &junk, &frame)
}
