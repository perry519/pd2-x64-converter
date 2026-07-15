use std::fmt::{self, Display};
use std::io;

macro_rules! invalid {
    ($($arg:tt)*) => {
        Err($crate::error::Error::Invalid(format!($($arg)*)))
    };
}
pub(crate) use invalid;

#[derive(Debug)]
pub enum Error {
  Io(io::Error),
  Json(serde_json::Error),
  Invalid(String),
  InvalidInput { context: String, source: Box<Error> },
}

impl Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Io(error) => write!(f, "{error}"),
      Self::Json(error) => write!(f, "{error}"),
      Self::Invalid(message) => f.write_str(message),
      Self::InvalidInput { context, source } => write!(f, "{context}: {source}"),
    }
  }
}

impl std::error::Error for Error {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    match self {
      Self::Io(error) => Some(error),
      Self::Json(error) => Some(error),
      Self::Invalid(_) => None,
      Self::InvalidInput { source, .. } => Some(source),
    }
  }
}

impl From<io::Error> for Error {
  fn from(value: io::Error) -> Self {
    Self::Io(value)
  }
}

impl From<serde_json::Error> for Error {
  fn from(value: serde_json::Error) -> Self {
    Self::Json(value)
  }
}

pub type Result<T> = std::result::Result<T, Error>;
