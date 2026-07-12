use crate::error::{Error, Result};

pub(super) fn reserve(output: &mut Vec<u8>, additional: usize, label: &str) -> Result<()> {
  output
    .try_reserve(additional)
    .map_err(|_| Error::Invalid(format!("{label}: output allocation is too large")))
}

pub(super) fn read_u32(data: &[u8], offset: usize, label: &str) -> Result<u32> {
  let mut reader = Reader::new(
    data
      .get(offset..)
      .ok_or_else(|| Error::Invalid(format!("{label}: integer offset out of bounds")))?,
    label,
  );
  reader.u32()
}

pub(super) fn put_u16(out: &mut Vec<u8>, value: u16) {
  out.extend_from_slice(&value.to_le_bytes());
}

pub(super) fn put_u32(out: &mut Vec<u8>, value: u32) {
  out.extend_from_slice(&value.to_le_bytes());
}

pub(super) fn put_i32(out: &mut Vec<u8>, value: i32) {
  out.extend_from_slice(&value.to_le_bytes());
}

pub(super) fn invalid<T>(message: String) -> Result<T> {
  Err(Error::Invalid(message))
}

pub(super) struct Reader<'a, 'label> {
  data: &'a [u8],
  offset: usize,
  pub(super) label: &'label str,
}

impl<'a, 'label> Reader<'a, 'label> {
  pub(super) fn new(data: &'a [u8], label: &'label str) -> Self {
    Self {
      data,
      offset: 0,
      label,
    }
  }

  pub(super) fn bytes(&mut self, len: usize) -> Result<&'a [u8]> {
    let end = self
      .offset
      .checked_add(len)
      .ok_or_else(|| Error::Invalid(format!("{}: soundbank offset overflow", self.label)))?;
    let bytes = self
      .data
      .get(self.offset..end)
      .ok_or_else(|| Error::Invalid(format!("{}: truncated soundbank data", self.label)))?;
    self.offset = end;
    Ok(bytes)
  }

  pub(super) fn u8(&mut self) -> Result<u8> {
    Ok(self.bytes(1)?[0])
  }

  pub(super) fn u16(&mut self) -> Result<u16> {
    let bytes: [u8; 2] = self
      .bytes(2)?
      .try_into()
      .map_err(|_| Error::Invalid(format!("{}: truncated 16-bit integer", self.label)))?;
    Ok(u16::from_le_bytes(bytes))
  }

  pub(super) fn u32(&mut self) -> Result<u32> {
    let bytes: [u8; 4] = self
      .bytes(4)?
      .try_into()
      .map_err(|_| Error::Invalid(format!("{}: truncated 32-bit integer", self.label)))?;
    Ok(u32::from_le_bytes(bytes))
  }

  pub(super) fn finish(&self, section: &str) -> Result<()> {
    if self.offset != self.data.len() {
      return self.fail(format!(
        "{section} has {} trailing bytes",
        self.data.len() - self.offset
      ));
    }
    Ok(())
  }

  pub(super) fn is_empty(&self) -> bool {
    self.offset == self.data.len()
  }

  pub(super) fn remaining(&self) -> usize {
    self.data.len() - self.offset
  }

  pub(super) fn fail<T>(&self, message: String) -> Result<T> {
    invalid(format!("{}: {message}", self.label))
  }
}

pub(super) fn copy_rtpc_graph(reader: &mut Reader<'_, '_>, out: &mut Vec<u8>) -> Result<()> {
  let points = reader.u16()?;
  put_u16(out, points);
  out.extend_from_slice(reader.bytes(points as usize * 12)?);
  Ok(())
}
