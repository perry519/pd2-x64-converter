use super::*;

#[test]
fn converts_v88_header_fields_to_v145_semantics() {
  let mut source = bank(&[(4, 1, 0_u32.to_le_bytes().to_vec())]);
  source[12..16].copy_from_slice(&595_414_150_u32.to_le_bytes());
  source[16..20].copy_from_slice(&12_u32.to_le_bytes());
  source[20..24].copy_from_slice(&1_u32.to_le_bytes());
  source[24..28].copy_from_slice(&357_u32.to_le_bytes());

  let converted = convert(&source, "fixture.bnk").unwrap();
  let chunks = parse_chunks(&converted, "converted.bnk").unwrap();
  let bkhd = required_chunk(&chunks, BKHD, "converted.bnk").unwrap();

  assert_eq!(read_u32(bkhd, 4, "converted.bnk").unwrap(), 595_414_150);
  assert_eq!(read_u32(bkhd, 8, "converted.bnk").unwrap(), 684_519_430);
  assert_eq!(read_u32(bkhd, 12, "converted.bnk").unwrap(), 16);
  assert_eq!(read_u32(bkhd, 16, "converted.bnk").unwrap(), 357);
}

#[test]
fn converts_observed_v88_header_sizes_and_header_only_banks() {
  for size in [20, 24, 28, 32] {
    let mut source = Vec::new();
    let mut bkhd = vec![0; size];
    bkhd[..4].copy_from_slice(&88_u32.to_le_bytes());
    bkhd[4..8].copy_from_slice(&42_u32.to_le_bytes());
    bkhd[16..20].copy_from_slice(&357_u32.to_le_bytes());
    push_chunk_for_test(&mut source, BKHD, &bkhd);

    assert_eq!(
      classify(&source, "header-only.bnk").unwrap(),
      LayoutState::SupportedX32
    );
    let converted = convert(&source, "header-only.bnk").unwrap();
    assert_eq!(
      classify(&converted, "converted.bnk").unwrap(),
      LayoutState::AlreadyX64
    );
    assert_eq!(parse_chunks(&converted, "converted.bnk").unwrap().len(), 1);
  }

  let mut source = Vec::new();
  let mut bkhd = vec![0; 24];
  bkhd[..4].copy_from_slice(&88_u32.to_le_bytes());
  bkhd[20] = 1;
  push_chunk_for_test(&mut source, BKHD, &bkhd);
  assert!(classify(&source, "unknown-tail.bnk").is_err());
}

#[test]
fn accepts_observed_native_v145_header_sizes() {
  for size in [40, 44, 48] {
    let mut source = Vec::new();
    let mut bkhd = vec![0; size];
    bkhd[..4].copy_from_slice(&145_u32.to_le_bytes());
    bkhd[24..40].fill(0xaa);
    push_chunk_for_test(&mut source, BKHD, &bkhd);
    assert_eq!(
      classify(&source, "native.bnk").unwrap(),
      LayoutState::AlreadyX64
    );
  }

  let mut source = Vec::new();
  let mut bkhd = vec![0; 44];
  bkhd[..4].copy_from_slice(&145_u32.to_le_bytes());
  bkhd[40] = 1;
  push_chunk_for_test(&mut source, BKHD, &bkhd);
  assert!(classify(&source, "unknown-native-tail.bnk").is_err());
}

#[test]
fn converts_event_count_and_preserves_opaque_chunks() {
  let mut event = Vec::new();
  put_u32(&mut event, 2);
  put_u32(&mut event, 11);
  put_u32(&mut event, 12);
  let source = bank(&[(4, 99, event)]);
  let mut source = source;
  let mut didx = Vec::new();
  put_u32(&mut didx, 42);
  put_u32(&mut didx, 0);
  put_u32(&mut didx, 3);
  push_chunk_for_test(&mut source, DIDX, &didx);
  push_chunk_for_test(&mut source, DATA, &[1, 2, 3]);

  let converted = convert(&source, "fixture.bnk").unwrap();
  let chunks = parse_chunks(&converted, "converted.bnk").unwrap();
  assert_eq!(chunks[2].id, DIDX);
  assert_eq!(chunks[2].payload, didx);
  assert_eq!(chunks[3].id, DATA);
  assert_eq!(chunks[3].payload, [1, 2, 3]);
  assert_eq!(
    required_chunk(&chunks, HIRC, "converted.bnk").unwrap(),
    [
      1, 0, 0, 0, 4, 13, 0, 0, 0, 99, 0, 0, 0, 2, 11, 0, 0, 0, 12, 0, 0, 0
    ]
  );
}

#[test]
fn omits_legacy_stid_like_native_v145_banks() {
  let mut source = bank(&[(4, 1, 0_u32.to_le_bytes().to_vec())]);
  let mut stid = Vec::new();
  put_u32(&mut stid, 1);
  put_u32(&mut stid, 1);
  put_u32(&mut stid, 42);
  stid.push(4);
  stid.extend_from_slice(b"test");
  push_chunk_for_test(&mut source, STID, &stid);

  let converted = convert(&source, "fixture.bnk").unwrap();
  let chunks = parse_chunks(&converted, "converted.bnk").unwrap();
  assert!(chunks.iter().all(|chunk| chunk.id != STID));
}

#[test]
fn rejects_malformed_or_unsupported_banks_without_panicking() {
  let mut invalid_chunk_size = b"BKHD".to_vec();
  put_u32(&mut invalid_chunk_size, u32::MAX);

  let mut unsupported_version = bank(&[(4, 1, 0_u32.to_le_bytes().to_vec())]);
  unsupported_version[8..12].copy_from_slice(&113_u32.to_le_bytes());

  let mut duplicate_bkhd = bank(&[(4, 1, 0_u32.to_le_bytes().to_vec())]);
  let mut bkhd = vec![0; 24];
  bkhd[..4].copy_from_slice(&88_u32.to_le_bytes());
  push_chunk_for_test(&mut duplicate_bkhd, BKHD, &bkhd);

  let mut duplicate_hirc = bank(&[(4, 1, 0_u32.to_le_bytes().to_vec())]);
  push_chunk_for_test(&mut duplicate_hirc, HIRC, &[1, 0, 0, 0]);

  let mut unknown_chunk = bank(&[(4, 1, 0_u32.to_le_bytes().to_vec())]);
  push_chunk_for_test(&mut unknown_chunk, *b"JUNK", &[]);

  let mut invalid_hirc_count = bank(&[(4, 1, 0_u32.to_le_bytes().to_vec())]);
  invalid_hirc_count[40..44].copy_from_slice(&2_u32.to_le_bytes());

  let mut invalid_didx = bank(&[(4, 1, 0_u32.to_le_bytes().to_vec())]);
  let mut didx = Vec::new();
  put_u32(&mut didx, 42);
  put_u32(&mut didx, 2);
  put_u32(&mut didx, 4);
  push_chunk_for_test(&mut invalid_didx, DIDX, &didx);
  push_chunk_for_test(&mut invalid_didx, DATA, &[1, 2, 3]);

  for (name, bytes) in [
    ("truncated header", b"BKHD".to_vec()),
    ("invalid chunk size", invalid_chunk_size),
    ("unsupported version", unsupported_version),
    ("duplicate BKHD", duplicate_bkhd),
    ("duplicate HIRC", duplicate_hirc),
    ("unknown chunk", unknown_chunk),
    ("invalid HIRC count", invalid_hirc_count),
    ("unsupported HIRC", bank(&[(22, 1, Vec::new())])),
    ("out-of-range DIDX", invalid_didx),
  ] {
    assert!(classify(&bytes, name).is_err(), "{name} was accepted");
  }
}

#[test]
fn converts_every_supported_hirc_type() {
  let base = minimal_base();

  let mut random = base.clone();
  random.extend_from_slice(&[0; 23]);
  random.extend_from_slice(&[1, 0, 1, 0, 1]);
  put_u32(&mut random, 0);
  put_u16(&mut random, 0);

  let mut switch = base.clone();
  put_u32(&mut switch, 0);
  put_u32(&mut switch, 30);
  put_u32(&mut switch, 0);
  switch.push(0);
  put_u32(&mut switch, 0);
  put_u32(&mut switch, 0);
  put_u32(&mut switch, 0);

  let mut actor = base;
  put_u32(&mut actor, 0);

  let mut attenuation = vec![0];
  attenuation.extend_from_slice(&[0; 5]);
  attenuation.push(0);
  put_u16(&mut attenuation, 0);

  let source = bank(&[
    (5, 4, random),
    (6, 5, switch),
    (7, 6, actor),
    (14, 7, attenuation),
  ]);
  let converted = convert(&source, "fixture.bnk").unwrap();
  let chunks = parse_chunks(&converted, "converted.bnk").unwrap();
  let items = converted_items(required_chunk(&chunks, HIRC, "converted.bnk").unwrap());

  assert_eq!(
    items.iter().map(|item| item.0).collect::<Vec<_>>(),
    [5, 6, 7, 14]
  );
  let converted_base_len = items[2].2.len() - 4;
  assert_eq!(items[0].2[converted_base_len + 23], 0b1_0101);
  assert_eq!(items[1].2[converted_base_len], 0);
  assert_eq!(&items[3].2[..2], [1, 0]);
}

#[test]
fn proves_streamed_and_embedded_media_size_conversion() {
  let mut streamed = Vec::new();
  put_u32(&mut streamed, 0x0002_0001);
  put_u32(&mut streamed, 1);
  put_u32(&mut streamed, 10);
  put_u32(&mut streamed, 10);
  streamed.push(0);
  streamed.extend_from_slice(&minimal_base());

  let mut embedded = Vec::new();
  put_u32(&mut embedded, 0x0002_0001);
  put_u32(&mut embedded, 2);
  put_u32(&mut embedded, 20);
  put_u32(&mut embedded, 20);
  put_u32(&mut embedded, 64);
  put_u32(&mut embedded, 1_234);
  embedded.push(0);
  embedded.extend_from_slice(&minimal_base());

  let converted = convert(&bank(&[(2, 11, streamed), (2, 22, embedded)]), "media.bnk").unwrap();
  let chunks = parse_chunks(&converted, "converted.bnk").unwrap();
  let items = converted_items(required_chunk(&chunks, HIRC, "converted.bnk").unwrap());
  assert_eq!(&items[0].2[9..13], 0_u32.to_le_bytes());
  assert_eq!(&items[1].2[9..13], 1_234_u32.to_le_bytes());
}

#[test]
fn converts_layer_and_effect_hirc_types() {
  let mut layer = minimal_base();
  put_u32(&mut layer, 0);
  put_u32(&mut layer, 1);
  put_u32(&mut layer, 10);
  put_u16(&mut layer, 0);
  put_u32(&mut layer, 20);
  put_u32(&mut layer, 1);
  put_u32(&mut layer, 30);
  put_u32(&mut layer, 1);
  layer.extend_from_slice(&[0; 12]);

  let mut effect = Vec::new();
  put_u32(&mut effect, 0x0065_0002);
  put_u32(&mut effect, 3);
  effect.extend_from_slice(&[1, 2, 3]);
  effect.push(1);
  effect.push(4);
  put_u32(&mut effect, 40);
  put_u16(&mut effect, 0);

  let source = bank(&[(9, 1, layer), (18, 2, effect.clone()), (19, 3, effect)]);
  let converted = convert(&source, "fixture.bnk").unwrap();
  let chunks = parse_chunks(&converted, "converted.bnk").unwrap();
  let items = converted_items(required_chunk(&chunks, HIRC, "converted.bnk").unwrap());

  assert_eq!(
    items.iter().map(|item| item.0).collect::<Vec<_>>(),
    [9, 16, 17]
  );
  assert_eq!(items[0].2.last(), Some(&0));
  assert_eq!(&items[1].2[items[1].2.len() - 4..], [0; 4]);
  assert_eq!(&items[2].2[items[2].2.len() - 4..], [0; 4]);
}

#[test]
fn converts_stop_and_bypass_actions() {
  let mut stop = Vec::new();
  put_u16(&mut stop, 0x0102);
  stop.extend_from_slice(&[0; 7]);
  stop.push(0);
  put_u32(&mut stop, 1);
  put_u32(&mut stop, 10);
  stop.push(1);

  let mut bypass = Vec::new();
  put_u16(&mut bypass, 0x1a02);
  bypass.extend_from_slice(&[0; 7]);
  bypass.extend_from_slice(&[1, 2]);
  put_u32(&mut bypass, 0);

  let source = bank(&[(3, 1, stop), (3, 2, bypass)]);
  let converted = convert(&source, "fixture.bnk").unwrap();
  let chunks = parse_chunks(&converted, "converted.bnk").unwrap();
  let items = converted_items(required_chunk(&chunks, HIRC, "converted.bnk").unwrap());

  assert_eq!(items[0].2[10], 6);
  assert_eq!(items[0].2[11], 1);
  assert_eq!(&items[1].2[9..12], [1, 2, 0]);
}

#[test]
fn converts_every_observed_action_layout() {
  let stop = [0, 0, 0, 0, 0];
  let pause = [0, 4, 0, 0, 0, 0];
  let play = [0, 42, 0, 0, 0];
  let mut value = vec![4, 1];
  value.extend_from_slice(&[0; 16]);
  let state = [10, 0, 0, 0, 20, 0, 0, 0];
  let bypass = [1, 2, 0, 0, 0, 0];
  let opcodes = [
    0x0102, 0x0103, 0x0105, 0x0109, 0x0202, 0x0203, 0x0204, 0x0302, 0x0303, 0x0304, 0x0403, 0x0c02,
    0x0e03, 0x1204, 0x1302, 0x1303, 0x1402, 0x1901, 0x1a02, 0x1c02, 0x1c03,
  ];
  let items = opcodes
    .iter()
    .enumerate()
    .map(|(index, opcode)| {
      let tail: &[u8] = match opcode & 0xff00 {
        0x0100 => &stop,
        0x0200 | 0x0300 => &pause,
        0x0400 => &play,
        0x0c00 | 0x0e00 | 0x1300 | 0x1400 => &value,
        0x1200 | 0x1900 => &state,
        0x1a00 => &bypass,
        0x1c00 => &[],
        _ => unreachable!(),
      };
      (3, index as u32 + 1, action(*opcode, tail))
    })
    .collect::<Vec<_>>();

  let converted = convert(&bank(&items), "actions.bnk").unwrap();
  let chunks = parse_chunks(&converted, "converted.bnk").unwrap();
  let converted = converted_items(required_chunk(&chunks, HIRC, "converted.bnk").unwrap());
  let target_opcodes = converted
    .iter()
    .map(|item| u16::from_le_bytes(item.2[..2].try_into().unwrap()))
    .collect::<Vec<_>>();
  let mut expected = opcodes;
  expected[3] = 0x0105;
  assert_eq!(target_opcodes, expected);
  assert_eq!(&converted[13].2[2..6], 20_u32.to_le_bytes());
  assert_eq!(&converted[17].2[2..6], 20_u32.to_le_bytes());
}

#[test]
fn action_conversion_matches_native_music_bank() {
  let source = bank(&[
    (3, 1, hex("0901000000000000000401000000131ed41f00")),
    (3, 2, hex("011900000000000000920986e79a07ad48")),
    (
      3,
      3,
      hex("0213e0187ee300010f204e000000040100000000000000000000000000000000"),
    ),
  ]);
  let converted = convert(&source, "music-actions.bnk").unwrap();
  let chunks = parse_chunks(&converted, "converted.bnk").unwrap();
  let items = converted_items(required_chunk(&chunks, HIRC, "converted.bnk").unwrap());
  assert_eq!(items[0].2, hex("050100000000000000040601131ed41f00"));
  assert_eq!(items[1].2, hex("01199a07ad48000000920986e79a07ad48"));
  assert_eq!(
    items[2].2,
    hex("0213e0187ee3000110204e00000004000100000000000000000000000000")
  );
}

#[test]
fn converts_observed_stmg_and_envs_layouts() {
  let mut stmg = vec![0; 6];
  stmg.extend_from_slice(&0_u32.to_le_bytes());
  stmg.extend_from_slice(&0_u32.to_le_bytes());
  stmg.extend_from_slice(&0_u32.to_le_bytes());
  let envs = vec![0; 16];
  let mut source = bank(&[(4, 1, 0_u32.to_le_bytes().to_vec())]);
  push_chunk_for_test(&mut source, STMG, &stmg);
  push_chunk_for_test(&mut source, ENVS, &envs);

  let converted = convert(&source, "init.bnk").unwrap();
  let chunks = parse_chunks(&converted, "converted.bnk").unwrap();
  assert_eq!(
    chunks.iter().map(|chunk| chunk.id).collect::<Vec<_>>(),
    [BKHD, HIRC, STMG, ENVS]
  );
  let target_stmg = required_chunk(&chunks, STMG, "converted.bnk").unwrap();
  assert_eq!(target_stmg.len(), 26);
  assert_eq!(&target_stmg[..2], [0, 0]);
  assert_eq!(&target_stmg[8..10], 50_u16.to_le_bytes());
  assert_eq!(&target_stmg[22..], [0; 4]);
  let target_envs = required_chunk(&chunks, ENVS, "converted.bnk").unwrap();
  assert_eq!(target_envs.len(), 72);
  assert_eq!(&target_envs[8..12], [0, 0, 2, 0]);
  assert_eq!(&target_envs[24..28], 100_f32.to_le_bytes());
  assert_eq!(&target_envs[28..32], 100_f32.to_le_bytes());
}

#[test]
fn converts_bus_aux_bus_and_init_audio_devices() {
  let mut source = bank(&[(8, 1, minimal_bus()), (20, 2, minimal_bus())]);
  let mut stmg = vec![0; 6];
  stmg.extend_from_slice(&[0; 12]);
  push_chunk_for_test(&mut source, STMG, &stmg);
  push_chunk_for_test(&mut source, ENVS, &[0; 16]);

  let converted = convert(&source, "init.bnk").unwrap();
  let chunks = parse_chunks(&converted, "converted.bnk").unwrap();
  let items = converted_items(required_chunk(&chunks, HIRC, "converted.bnk").unwrap());
  assert_eq!(
    items.iter().map(|item| item.0).collect::<Vec<_>>(),
    [21, 21, 8, 18]
  );
  assert_eq!(items[0].1, 2_317_455_096);
  assert_eq!(items[0].2, hex("0700b500000000000000000000000000"));
  assert_eq!(items[1].1, 3_859_886_410);
  assert_eq!(
    items[1].2,
    hex("0700ae000c0000000000000000000000000120000000000000000000")
  );
  assert_eq!(&items[2].2[..9], hex("000000004a3111e600"));
  assert_eq!(&items[3].2[..9], hex("000000004a3111e600"));
}

#[test]
fn converts_observed_music_hirc_layouts() {
  let node = minimal_music_node();
  let mut segment = node.clone();
  segment.extend_from_slice(&[0; 8]);
  segment.extend_from_slice(&0_u32.to_le_bytes());

  let mut track = Vec::new();
  track.extend_from_slice(&0_u32.to_le_bytes());
  track.extend_from_slice(&0_u32.to_le_bytes());
  track.extend_from_slice(&0_u32.to_le_bytes());
  track.extend_from_slice(&minimal_base());
  track.extend_from_slice(&0_u32.to_le_bytes());
  track.extend_from_slice(&0_u32.to_le_bytes());

  let mut music_switch = node.clone();
  music_switch.extend_from_slice(&0_u32.to_le_bytes());
  let mut random = music_switch.clone();
  random.extend_from_slice(&1_u32.to_le_bytes());
  random.extend_from_slice(&[0; 26]);

  let source = bank(&[
    (10, 1, segment),
    (11, 2, track),
    (12, 3, music_switch),
    (13, 4, random),
  ]);
  let converted = convert(&source, "music.bnk").unwrap();
  let chunks = parse_chunks(&converted, "converted.bnk").unwrap();
  let items = converted_items(required_chunk(&chunks, HIRC, "converted.bnk").unwrap());
  assert_eq!(
    items.iter().map(|item| item.0).collect::<Vec<_>>(),
    [10, 11, 12, 13]
  );
  assert_eq!(items[1].2[0], 0);
  assert_eq!(&items[1].2[items[1].2.len() - 5..], [0; 5]);
  assert_eq!(items[3].2.len(), items[2].2.len() + 4 + 30);
}

fn minimal_base() -> Vec<u8> {
  let mut base = vec![0, 0];
  base.extend_from_slice(&[0; 8]);
  base.extend_from_slice(&[0, 0]);
  base.extend_from_slice(&[0, 0]);
  base.push(0);
  base.extend_from_slice(&[0; 4]);
  base.extend_from_slice(&[0; 13]);
  put_u32(&mut base, 0);
  put_u16(&mut base, 0);
  base
}

fn minimal_bus() -> Vec<u8> {
  let mut bus = vec![0; 4];
  bus.push(0);
  bus.extend_from_slice(&[0; 4]);
  bus.extend_from_slice(&[0; 2]);
  bus.push(0);
  bus.extend_from_slice(&[0; 2]);
  bus.extend_from_slice(&[0; 4]);
  bus.extend_from_slice(&[0; 12]);
  bus.push(0);
  bus.extend_from_slice(&[0; 2]);
  bus.extend_from_slice(&[0; 4]);
  bus
}

fn minimal_music_node() -> Vec<u8> {
  let mut node = minimal_base();
  node.extend_from_slice(&0_u32.to_le_bytes());
  node.extend_from_slice(&[0; 23]);
  node.extend_from_slice(&0_u32.to_le_bytes());
  node
}

fn action(opcode: u16, tail: &[u8]) -> Vec<u8> {
  let mut action = Vec::new();
  put_u16(&mut action, opcode);
  put_u32(&mut action, 0);
  action.extend_from_slice(&[0, 0, 0]);
  action.extend_from_slice(tail);
  action
}

fn hex(value: &str) -> Vec<u8> {
  value
    .as_bytes()
    .chunks_exact(2)
    .map(|pair| {
      let digit = |byte| match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => panic!("invalid hex fixture"),
      };
      digit(pair[0]) << 4 | digit(pair[1])
    })
    .collect()
}

fn converted_items(hirc: &[u8]) -> Vec<(u8, u32, Vec<u8>)> {
  let mut reader = Reader::new(hirc, "converted fixture");
  let count = reader.u32().unwrap();
  let mut items = Vec::new();
  for _ in 0..count {
    let kind = reader.u8().unwrap();
    let size = reader.u32().unwrap();
    let mut item = Reader::new(reader.bytes(size as usize).unwrap(), "converted fixture");
    let id = item.u32().unwrap();
    items.push((kind, id, item.bytes(size as usize - 4).unwrap().to_vec()));
  }
  reader.finish("HIRC").unwrap();
  items
}

fn bank(items: &[(u8, u32, Vec<u8>)]) -> Vec<u8> {
  let mut bank = Vec::new();
  let mut bkhd = vec![0; 24];
  bkhd[..4].copy_from_slice(&88_u32.to_le_bytes());
  push_chunk_for_test(&mut bank, BKHD, &bkhd);
  let mut hirc = Vec::new();
  put_u32(&mut hirc, items.len() as u32);
  for (kind, id, body) in items {
    hirc.push(*kind);
    put_u32(&mut hirc, body.len() as u32 + 4);
    put_u32(&mut hirc, *id);
    hirc.extend_from_slice(body);
  }
  push_chunk_for_test(&mut bank, HIRC, &hirc);
  bank
}

fn push_chunk_for_test(bank: &mut Vec<u8>, id: [u8; 4], payload: &[u8]) {
  bank.extend_from_slice(&id);
  put_u32(bank, payload.len() as u32);
  bank.extend_from_slice(payload);
}
