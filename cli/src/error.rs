use std::io;
use std::path::Path;

use crossterm::style::Stylize;
use format_serde_error::SerdeError;
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum AppError {
  #[error("'{}': {source}", .path.to_string_lossy())]
  ConfigIoError {
    path: Box<Path>,
    #[source]
    source: io::Error,
  },
  #[error("'{}': Not supported config type", .path.to_string_lossy())]
  ConfigTypeNotSupported { path: Box<Path> },
  #[error("error when parsing config file:
   {} {}{line}{column}{source}",
    "-->".blue().bold(),
    .path.to_string_lossy(),
    line = .line.map(|l| format!(":{}", l)).unwrap_or_default(),
    column = .column.map(|c| format!(":{}", c)).unwrap_or_default())]
  ConfigDeserializeError {
    path: Box<Path>,
    line: Option<usize>,
    column: Option<usize>,
    #[source]
    source: SerdeError,
  },
  #[error(transparent)]
  Other(anyhow::Error),
}

impl From<io::Error> for AppError {
  fn from(e: io::Error) -> Self {
    Self::Other(e.into())
  }
}

impl From<anyhow::Error> for AppError {
  fn from(e: anyhow::Error) -> Self {
    Self::Other(e)
  }
}

pub(crate) type Result<T> = anyhow::Result<T, AppError>;
