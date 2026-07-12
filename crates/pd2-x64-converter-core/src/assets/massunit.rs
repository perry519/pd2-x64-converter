use crate::error::{Error, Result, invalid};
use crate::manifest::LayoutState;

use super::{checked_size, push_u64, read_u32, read_u64, require_range, usize_from_u64};

const MASSUNIT_SENTINEL: [u8; 4] = [0xd1, 0xee, 0x0b, 0xcc];
const MASSUNIT_X32_HEADER_SIZE: usize = 16;
const MASSUNIT_X64_HEADER_SIZE: usize = 32;
const MASSUNIT_X32_RECORD_SIZE: usize = 32;
const MASSUNIT_X64_RECORD_SIZE: usize = 48;
const MASSUNIT_PLACEMENT_SIZE: usize = 28;

pub(super) fn classify(data: &[u8], label: &str) -> Result<LayoutState> {
  let parsed = parse_massunit(data, label)?;
  if parsed.pointer_size == 8 && parsed.had_sentinel {
    Ok(LayoutState::AlreadyX64)
  } else {
    Ok(LayoutState::SupportedX32)
  }
}

#[derive(Clone, Copy)]
struct MassUnitRecord {
  name_hash: u64,
  enabled_count: u64,
  placement_count: u64,
  placement_capacity: u64,
  placement_offset: usize,
}

struct MassUnitRaw {
  pointer_size: usize,
  unit_count: u64,
  unit_capacity: u64,
  records_offset: usize,
  records: Vec<MassUnitRecord>,
  placements: Vec<u8>,
  had_sentinel: bool,
}

pub(super) fn convert(data: &[u8], label: &str) -> Result<Vec<u8>> {
  let parsed = parse_massunit(data, label)?;
  let converted = if parsed.pointer_size == 8 {
    let mut out = data.to_vec();
    if !parsed.had_sentinel {
      out.extend_from_slice(&MASSUNIT_SENTINEL);
    }
    out
  } else {
    build_massunit_x64(&parsed)?
  };
  let reparsed = parse_massunit_x64(&converted, &format!("{label}: converted"))?;
  if reparsed.pointer_size != 8 || !reparsed.had_sentinel {
    return invalid!("{label}: converted massunit is missing x64 sentinel");
  }
  Ok(converted)
}

fn massunit_payload_end(data: &[u8]) -> (usize, bool) {
  if data.ends_with(&MASSUNIT_SENTINEL) {
    (data.len() - MASSUNIT_SENTINEL.len(), true)
  } else {
    (data.len(), false)
  }
}

fn parse_massunit(data: &[u8], label: &str) -> Result<MassUnitRaw> {
  let mut errors = Vec::new();
  for parser in [parse_massunit_x64, parse_massunit_x32] {
    match parser(data, label) {
      Ok(parsed) => return Ok(parsed),
      Err(error) => errors.push(error.to_string()),
    }
  }
  invalid!(
    "{label}: unsupported massunit layout; {}",
    errors.join("; ")
  )
}

fn parse_massunit_x32(data: &[u8], label: &str) -> Result<MassUnitRaw> {
  require_range(
    data,
    0,
    MASSUNIT_X32_HEADER_SIZE,
    &format!("{label}: x32 header"),
  )?;
  let unit_count = read_u32(data, 0, label)? as u64;
  let unit_capacity = read_u32(data, 4, label)? as u64;
  let records_offset = read_u32(data, 8, label)? as usize;
  let (payload_end, had_sentinel) = massunit_payload_end(data);

  if unit_count == 0 {
    if payload_end != MASSUNIT_X32_HEADER_SIZE {
      return invalid!("{label}: x32 empty massunit has trailing payload");
    }
    return Ok(MassUnitRaw {
      pointer_size: 4,
      unit_count,
      unit_capacity,
      records_offset,
      records: Vec::new(),
      placements: Vec::new(),
      had_sentinel,
    });
  }

  let records_size = checked_size(unit_count, MASSUNIT_X32_RECORD_SIZE, label)?;
  let records_end = records_offset
    .checked_add(records_size)
    .ok_or_else(|| Error::Invalid(format!("{label}: x32 unit records overflow")))?;
  require_range(
    data,
    records_offset,
    records_size,
    &format!("{label}: x32 unit records"),
  )?;
  if records_end > payload_end {
    return invalid!("{label}: x32 unit records overlap sentinel");
  }

  let mut records = Vec::with_capacity(usize_from_u64(unit_count, label)?);
  for index in 0..usize_from_u64(unit_count, label)? {
    let offset = records_offset + index * MASSUNIT_X32_RECORD_SIZE;
    let name_hash = read_u64(data, offset, label)?;
    let enabled_count = read_u32(data, offset + 8, label)? as u64;
    let placement_count = read_u32(data, offset + 12, label)? as u64;
    let placement_capacity = read_u32(data, offset + 16, label)? as u64;
    let placement_offset = read_u32(data, offset + 20, label)? as usize;
    let placement_capacity = if placement_capacity == 0 {
      placement_count
    } else {
      placement_capacity
    };
    if placement_capacity < placement_count {
      return invalid!(
        "{label}: x32 record {index} capacity {placement_capacity} < count {placement_count}"
      );
    }
    let placement_size = checked_size(placement_count, MASSUNIT_PLACEMENT_SIZE, label)?;
    require_range(
      data,
      placement_offset,
      placement_size,
      &format!("{label}: x32 placements {index}"),
    )?;
    if placement_offset < records_end || placement_offset + placement_size > payload_end {
      return invalid!("{label}: x32 placements {index} outside placement blob");
    }
    records.push(MassUnitRecord {
      name_hash,
      enabled_count,
      placement_count,
      placement_capacity,
      placement_offset,
    });
  }

  Ok(MassUnitRaw {
    pointer_size: 4,
    unit_count,
    unit_capacity: if unit_capacity == 0 {
      unit_count
    } else {
      unit_capacity
    },
    records_offset,
    records,
    placements: data[records_end..payload_end].to_vec(),
    had_sentinel,
  })
}

fn parse_massunit_x64(data: &[u8], label: &str) -> Result<MassUnitRaw> {
  require_range(
    data,
    0,
    MASSUNIT_X64_HEADER_SIZE,
    &format!("{label}: x64 header"),
  )?;
  let unit_count = read_u64(data, 0, label)?;
  let unit_capacity = read_u64(data, 8, label)?;
  let records_offset = usize_from_u64(read_u64(data, 16, label)?, label)?;
  let (payload_end, had_sentinel) = massunit_payload_end(data);

  if unit_count == 0 {
    if payload_end != MASSUNIT_X64_HEADER_SIZE {
      return invalid!("{label}: x64 empty massunit has trailing payload");
    }
    return Ok(MassUnitRaw {
      pointer_size: 8,
      unit_count,
      unit_capacity,
      records_offset,
      records: Vec::new(),
      placements: Vec::new(),
      had_sentinel,
    });
  }

  let records_size = checked_size(unit_count, MASSUNIT_X64_RECORD_SIZE, label)?;
  let records_end = records_offset
    .checked_add(records_size)
    .ok_or_else(|| Error::Invalid(format!("{label}: x64 unit records overflow")))?;
  require_range(
    data,
    records_offset,
    records_size,
    &format!("{label}: x64 unit records"),
  )?;
  if records_end > payload_end {
    return invalid!("{label}: x64 unit records overlap sentinel");
  }

  let mut records = Vec::with_capacity(usize_from_u64(unit_count, label)?);
  for index in 0..usize_from_u64(unit_count, label)? {
    let offset = records_offset + index * MASSUNIT_X64_RECORD_SIZE;
    let name_hash = read_u64(data, offset, label)?;
    let enabled_count = read_u64(data, offset + 8, label)?;
    let placement_count = read_u64(data, offset + 16, label)?;
    let placement_capacity = read_u64(data, offset + 24, label)?;
    let placement_offset = usize_from_u64(read_u64(data, offset + 32, label)?, label)?;
    if placement_capacity < placement_count {
      return invalid!(
        "{label}: x64 record {index} capacity {placement_capacity} < count {placement_count}"
      );
    }
    let placement_size = checked_size(placement_count, MASSUNIT_PLACEMENT_SIZE, label)?;
    require_range(
      data,
      placement_offset,
      placement_size,
      &format!("{label}: x64 placements {index}"),
    )?;
    if placement_offset < records_end || placement_offset + placement_size > payload_end {
      return invalid!("{label}: x64 placements {index} outside placement blob");
    }
    records.push(MassUnitRecord {
      name_hash,
      enabled_count,
      placement_count,
      placement_capacity,
      placement_offset,
    });
  }

  Ok(MassUnitRaw {
    pointer_size: 8,
    unit_count,
    unit_capacity,
    records_offset,
    records,
    placements: data[records_end..payload_end].to_vec(),
    had_sentinel,
  })
}

fn build_massunit_x64(massunit: &MassUnitRaw) -> Result<Vec<u8>> {
  if massunit.unit_count == 0 {
    let mut out = vec![0; MASSUNIT_X64_HEADER_SIZE];
    out.extend_from_slice(&MASSUNIT_SENTINEL);
    return Ok(out);
  }

  let old_records_end = massunit
    .records_offset
    .checked_add(checked_size(
      massunit.unit_count,
      MASSUNIT_X32_RECORD_SIZE,
      "massunit",
    )?)
    .ok_or_else(|| Error::Invalid("massunit: x32 record table overflow".to_string()))?;
  let new_records_offset = MASSUNIT_X64_HEADER_SIZE;
  let new_records_end = new_records_offset
    .checked_add(checked_size(
      massunit.unit_count,
      MASSUNIT_X64_RECORD_SIZE,
      "massunit",
    )?)
    .ok_or_else(|| Error::Invalid("massunit: x64 record table overflow".to_string()))?;
  let offset_delta = new_records_end
    .checked_sub(old_records_end)
    .ok_or_else(|| Error::Invalid("massunit: record table moved backwards".to_string()))?;

  let mut out = Vec::new();
  push_u64(&mut out, massunit.unit_count);
  push_u64(
    &mut out,
    if massunit.unit_capacity == 0 {
      massunit.unit_count
    } else {
      massunit.unit_capacity
    },
  );
  push_u64(&mut out, new_records_offset as u64);
  push_u64(&mut out, 0);
  for record in &massunit.records {
    push_u64(&mut out, record.name_hash);
    push_u64(&mut out, record.enabled_count);
    push_u64(&mut out, record.placement_count);
    push_u64(
      &mut out,
      if record.placement_capacity == 0 {
        record.placement_count
      } else {
        record.placement_capacity
      },
    );
    push_u64(
      &mut out,
      record
        .placement_offset
        .checked_add(offset_delta)
        .ok_or_else(|| Error::Invalid("massunit: placement offset overflow".to_string()))?
        as u64,
    );
    push_u64(&mut out, 0);
  }
  out.extend_from_slice(&massunit.placements);
  out.extend_from_slice(&MASSUNIT_SENTINEL);
  Ok(out)
}

#[cfg(test)]
#[path = "massunit_tests.rs"]
mod tests;
