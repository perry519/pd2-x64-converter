use crate::error::{Result, invalid};
use crate::manifest::LayoutState;

use super::{
  checked_size, push_u32, push_u64, read_cstr, read_u32, read_u64, require_range, usize_from_u64,
  write_u64_at,
};

const SCRIPT_DATA_SUFFIXES: &[&str] = &[
  ".sequence_manager",
  ".continent",
  ".environment",
  ".world",
  ".mission",
  ".continents",
  ".cover_data",
  ".nav_data",
  ".world_cameras",
  ".world_sounds",
];

pub(super) fn is_suffix(value: &str) -> bool {
  SCRIPT_DATA_SUFFIXES.contains(&value)
}

pub(super) fn classify(data: &[u8], label: &str) -> Result<LayoutState> {
  if is_text_scriptdata(data) {
    return Ok(LayoutState::TextScriptData);
  }
  let parsed = parse_scriptdata(data, label)?;
  if parsed.pointer_size == 8 && parsed.count_size == 8 {
    Ok(LayoutState::AlreadyX64)
  } else {
    Ok(LayoutState::SupportedX32)
  }
}

#[derive(Clone, Copy)]
struct RawVector {
  count: u64,
  capacity: u64,
  offset: usize,
  next_offset: usize,
}

#[derive(Clone, Copy)]
struct ScriptDataLayout {
  pointer_size: usize,
  count_size: usize,
}

struct ScriptDataTable {
  meta: u64,
  count: u64,
  capacity: u64,
  entries: Vec<u8>,
}

struct ScriptDataRaw {
  pointer_size: usize,
  count_size: usize,
  vector_headers: [RawVector; 6],
  root: u32,
  numbers: Vec<u8>,
  strings: Vec<Vec<u8>>,
  vectors: Vec<u8>,
  quaternions: Vec<u8>,
  idstrings: Vec<u8>,
  tables: Vec<ScriptDataTable>,
}

pub(super) fn is_text_scriptdata(data: &[u8]) -> bool {
  data
    .iter()
    .copied()
    .find(|byte| !byte.is_ascii_whitespace())
    == Some(b'<')
}

pub(super) fn convert(data: &[u8], label: &str, extension: &str) -> Result<Vec<u8>> {
  if is_text_scriptdata(data) {
    let _ = extension;
    return Ok(data.to_vec());
  }

  let parsed = parse_scriptdata(data, label)?;
  let converted = if parsed.pointer_size == 8 && parsed.count_size == 8 {
    data.to_vec()
  } else {
    build_scriptdata_x64(&parsed)
  };
  let reparsed = parse_scriptdata(&converted, &format!("{label}: converted"))?;
  if reparsed.pointer_size != 8 || reparsed.count_size != 8 {
    return invalid!("{label}: converted ScriptData still uses legacy-width vectors");
  }
  Ok(converted)
}

fn parse_scriptdata(data: &[u8], label: &str) -> Result<ScriptDataRaw> {
  let mut errors = Vec::new();
  for layout in [
    ScriptDataLayout {
      pointer_size: 8,
      count_size: 8,
    },
    ScriptDataLayout {
      pointer_size: 8,
      count_size: 4,
    },
    ScriptDataLayout {
      pointer_size: 4,
      count_size: 4,
    },
  ] {
    match parse_scriptdata_layout(data, label, layout) {
      Ok(parsed) => return Ok(parsed),
      Err(error) => errors.push(format!(
        "{} / {}: {error}",
        layout.pointer_size * 8,
        layout.count_size * 8
      )),
    }
  }
  invalid!(
    "{label}: unsupported ScriptData layout; {}",
    errors.join("; ")
  )
}

fn parse_scriptdata_layout(
  data: &[u8],
  label: &str,
  layout: ScriptDataLayout,
) -> Result<ScriptDataRaw> {
  let numbers_header = read_scriptdata_vector(
    data,
    layout.pointer_size,
    layout,
    &format!("{label}: numbers header"),
  )?;
  let strings_header = read_scriptdata_vector(
    data,
    numbers_header.next_offset,
    layout,
    &format!("{label}: strings header"),
  )?;
  let vectors_header = read_scriptdata_vector(
    data,
    strings_header.next_offset,
    layout,
    &format!("{label}: vectors header"),
  )?;
  let quaternions_header = read_scriptdata_vector(
    data,
    vectors_header.next_offset,
    layout,
    &format!("{label}: quaternions header"),
  )?;
  let idstrings_header = read_scriptdata_vector(
    data,
    quaternions_header.next_offset,
    layout,
    &format!("{label}: idstrings header"),
  )?;
  let tables_header = read_scriptdata_vector(
    data,
    idstrings_header.next_offset,
    layout,
    &format!("{label}: tables header"),
  )?;
  let root_offset = tables_header.next_offset;
  require_range(data, root_offset, 4, &format!("{label}: root reference"))?;
  let root = read_u32(data, root_offset, label)?;

  let headers = [
    numbers_header,
    strings_header,
    vectors_header,
    quaternions_header,
    idstrings_header,
    tables_header,
  ];
  let counts = headers.map(|header| header.count);
  validate_scriptdata_ref(root, counts, &format!("{label}: root reference"))?;

  let numbers = read_scriptdata_payload(
    data,
    numbers_header.offset,
    checked_size(numbers_header.count, 4, label)?,
    &format!("{label}: numbers"),
  )?;
  let vectors = read_scriptdata_payload(
    data,
    vectors_header.offset,
    checked_size(vectors_header.count, 12, label)?,
    &format!("{label}: vectors"),
  )?;
  let quaternions = read_scriptdata_payload(
    data,
    quaternions_header.offset,
    checked_size(quaternions_header.count, 16, label)?,
    &format!("{label}: quaternions"),
  )?;
  let idstrings = read_scriptdata_payload(
    data,
    idstrings_header.offset,
    checked_size(idstrings_header.count, 8, label)?,
    &format!("{label}: idstrings"),
  )?;

  let string_record_size = if layout.pointer_size == 4 { 8 } else { 16 };
  let string_records = read_scriptdata_payload(
    data,
    strings_header.offset,
    checked_size(strings_header.count, string_record_size, label)?,
    &format!("{label}: string records"),
  )?;
  let mut strings = Vec::with_capacity(strings_header.count as usize);
  for index in 0..strings_header.count as usize {
    let record_offset = index * string_record_size;
    let string_offset = if layout.pointer_size == 4 {
      read_u32(&string_records, record_offset + 4, label)? as usize
    } else {
      usize_from_u64(read_u64(&string_records, record_offset + 8, label)?, label)?
    };
    strings.push(read_cstr(
      data,
      string_offset,
      &format!("{label}: string {index}"),
    )?);
  }

  let table_record_size = if layout.pointer_size == 4 {
    20
  } else if layout.count_size == 4 {
    32
  } else {
    40
  };
  let table_records = read_scriptdata_payload(
    data,
    tables_header.offset,
    checked_size(tables_header.count, table_record_size, label)?,
    &format!("{label}: table records"),
  )?;
  let mut tables = Vec::with_capacity(tables_header.count as usize);
  for index in 0..tables_header.count as usize {
    let record_offset = index * table_record_size;
    let (meta, count, capacity, entries_offset) = if layout.pointer_size == 4 {
      (
        read_u32(&table_records, record_offset, label)? as u64,
        read_u32(&table_records, record_offset + 4, label)? as u64,
        read_u32(&table_records, record_offset + 8, label)? as u64,
        read_u32(&table_records, record_offset + 12, label)? as u64,
      )
    } else if layout.count_size == 4 {
      (
        read_u64(&table_records, record_offset, label)?,
        read_u32(&table_records, record_offset + 8, label)? as u64,
        read_u32(&table_records, record_offset + 12, label)? as u64,
        read_u64(&table_records, record_offset + 16, label)?,
      )
    } else {
      (
        read_u64(&table_records, record_offset, label)?,
        read_u64(&table_records, record_offset + 8, label)?,
        read_u64(&table_records, record_offset + 16, label)?,
        read_u64(&table_records, record_offset + 24, label)?,
      )
    };

    if meta != 0xffff_ffff && meta != 0xffff_ffff_ffff_ffff && meta >= strings_header.count {
      return invalid!("{label}: table {index} meta string index {meta} out of range");
    }
    let entries = read_scriptdata_payload(
      data,
      usize_from_u64(entries_offset, label)?,
      checked_size(count, 8, label)?,
      &format!("{label}: table {index} entries"),
    )?;
    for entry_index in 0..count as usize {
      let pair_offset = entry_index * 8;
      validate_scriptdata_ref(
        read_u32(&entries, pair_offset, label)?,
        counts,
        &format!("{label}: table {index} key {entry_index}"),
      )?;
      validate_scriptdata_ref(
        read_u32(&entries, pair_offset + 4, label)?,
        counts,
        &format!("{label}: table {index} value {entry_index}"),
      )?;
    }
    tables.push(ScriptDataTable {
      meta,
      count,
      capacity,
      entries,
    });
  }

  Ok(ScriptDataRaw {
    pointer_size: layout.pointer_size,
    count_size: layout.count_size,
    vector_headers: headers,
    root,
    numbers,
    strings,
    vectors,
    quaternions,
    idstrings,
    tables,
  })
}

fn read_scriptdata_vector(
  data: &[u8],
  offset: usize,
  layout: ScriptDataLayout,
  label: &str,
) -> Result<RawVector> {
  if layout.pointer_size == 4 {
    require_range(data, offset, 16, label)?;
    return Ok(RawVector {
      count: read_u32(data, offset, label)? as u64,
      capacity: read_u32(data, offset + 4, label)? as u64,
      offset: read_u32(data, offset + 8, label)? as usize,
      next_offset: offset + 16,
    });
  }
  if layout.count_size == 4 {
    require_range(data, offset, 24, label)?;
    return Ok(RawVector {
      count: read_u32(data, offset, label)? as u64,
      capacity: read_u32(data, offset + 4, label)? as u64,
      offset: usize_from_u64(read_u64(data, offset + 8, label)?, label)?,
      next_offset: offset + 24,
    });
  }
  require_range(data, offset, 32, label)?;
  Ok(RawVector {
    count: read_u64(data, offset, label)?,
    capacity: read_u64(data, offset + 8, label)?,
    offset: usize_from_u64(read_u64(data, offset + 16, label)?, label)?,
    next_offset: offset + 32,
  })
}

fn read_scriptdata_payload(
  data: &[u8],
  offset: usize,
  size: usize,
  label: &str,
) -> Result<Vec<u8>> {
  require_range(data, offset, size, label)?;
  Ok(data[offset..offset + size].to_vec())
}

fn build_scriptdata_x64(scriptdata: &ScriptDataRaw) -> Vec<u8> {
  let mut out = Vec::new();
  push_u64(&mut out, 0);

  let mut header_offsets = Vec::new();
  for _ in 0..6 {
    header_offsets.push(out.len());
    out.resize(out.len() + 32, 0);
  }
  push_u32(&mut out, scriptdata.root);

  let numbers_offset = out.len();
  out.extend_from_slice(&scriptdata.numbers);
  write_scriptdata_vector_header(
    &mut out,
    header_offsets[0],
    scriptdata.vector_headers[0].count,
    scriptdata.vector_headers[0].capacity,
    numbers_offset,
  );

  let string_records_offset = out.len();
  out.resize(out.len() + scriptdata.strings.len() * 16, 0);
  write_scriptdata_vector_header(
    &mut out,
    header_offsets[1],
    scriptdata.vector_headers[1].count,
    scriptdata.vector_headers[1].capacity,
    string_records_offset,
  );
  for (index, value) in scriptdata.strings.iter().enumerate() {
    let string_offset = out.len();
    let record_offset = string_records_offset + index * 16;
    write_u64_at(&mut out, record_offset, 0);
    write_u64_at(&mut out, record_offset + 8, string_offset as u64);
    out.extend_from_slice(value);
  }

  let vectors_offset = out.len();
  out.extend_from_slice(&scriptdata.vectors);
  write_scriptdata_vector_header(
    &mut out,
    header_offsets[2],
    scriptdata.vector_headers[2].count,
    scriptdata.vector_headers[2].capacity,
    vectors_offset,
  );

  let quaternions_offset = out.len();
  out.extend_from_slice(&scriptdata.quaternions);
  write_scriptdata_vector_header(
    &mut out,
    header_offsets[3],
    scriptdata.vector_headers[3].count,
    scriptdata.vector_headers[3].capacity,
    quaternions_offset,
  );

  let idstrings_offset = out.len();
  out.extend_from_slice(&scriptdata.idstrings);
  write_scriptdata_vector_header(
    &mut out,
    header_offsets[4],
    scriptdata.vector_headers[4].count,
    scriptdata.vector_headers[4].capacity,
    idstrings_offset,
  );

  let table_records_offset = out.len();
  out.resize(out.len() + scriptdata.tables.len() * 40, 0);
  write_scriptdata_vector_header(
    &mut out,
    header_offsets[5],
    scriptdata.vector_headers[5].count,
    scriptdata.vector_headers[5].capacity,
    table_records_offset,
  );
  for (index, table) in scriptdata.tables.iter().enumerate() {
    let entries_offset = out.len();
    let record_offset = table_records_offset + index * 40;
    write_u64_at(&mut out, record_offset, table.meta & 0xffff_ffff);
    write_u64_at(&mut out, record_offset + 8, table.count);
    write_u64_at(&mut out, record_offset + 16, table.capacity);
    write_u64_at(&mut out, record_offset + 24, entries_offset as u64);
    write_u64_at(&mut out, record_offset + 32, 0);
    out.extend_from_slice(&table.entries);
  }

  out
}

fn write_scriptdata_vector_header(
  out: &mut [u8],
  offset: usize,
  count: u64,
  capacity: u64,
  contents_offset: usize,
) {
  write_u64_at(out, offset, count);
  write_u64_at(out, offset + 8, capacity);
  write_u64_at(out, offset + 16, contents_offset as u64);
  write_u64_at(out, offset + 24, 0);
}

fn validate_scriptdata_ref(value: u32, counts: [u64; 6], label: &str) -> Result<()> {
  let item_type = value >> 24;
  let index = (value & 0x00ff_ffff) as u64;
  if matches!(item_type, 0..=2) {
    return Ok(());
  }
  let count = match item_type {
    3 => counts[0],
    4 => counts[1],
    5 => counts[2],
    6 => counts[3],
    7 => counts[4],
    8 => counts[5],
    _ => return invalid!("{label}: unsupported ScriptData reference type {item_type}"),
  };
  if index >= count {
    return invalid!(
      "{label}: ScriptData reference index {index} out of range for type {item_type} count {count}"
    );
  }
  Ok(())
}

#[cfg(test)]
#[path = "scriptdata_tests.rs"]
mod tests;
