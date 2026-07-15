use super::*;
use crate::assets::read_u64;

#[test]
fn animation_empty_x32_converts_to_x64_header() {
  let converted = convert(&legacy_empty_x32(), "empty.animation").unwrap();

  assert_eq!(read_u32(&converted, 0, "magic").unwrap(), ANIMATION_MAGIC);
  assert_eq!(
    read_u64(&converted, 16, "size").unwrap(),
    ANIMATION_X64_HEADER_SIZE as u64
  );
  assert_eq!(converted.len(), ANIMATION_X64_HEADER_SIZE);
  parse_animation_x64(&converted, "converted").unwrap();
  assert_eq!(
    classify(&converted, "converted").unwrap(),
    LayoutState::AlreadyX64
  );
}
