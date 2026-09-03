use std::io;
use std::path::Path;

use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;
use wakuchin::error::WakuchinError;

/// A config file that could not be deserialized, carrying its source text so
/// that miette can render the offending region.
#[derive(Debug, Error, Diagnostic)]
#[error("could not parse config file")]
#[diagnostic(code(wakuchin::config::parse))]
pub struct ConfigParseError {
  #[source_code]
  src: NamedSource<String>,
  /// `None` when the parser could not attribute the error to a location.
  #[label(primary, "{reason}")]
  span: Option<SourceSpan>,
  reason: String,
}

impl ConfigParseError {
  pub fn new(
    path: &Path,
    contents: String,
    span: Option<SourceSpan>,
    reason: impl Into<String>,
  ) -> Self {
    Self {
      src: NamedSource::new(path.to_string_lossy(), contents),
      span,
      reason: reason.into(),
    }
  }

  /// The message the parser reported, without the location miette renders.
  #[cfg(test)]
  pub fn reason(&self) -> &str {
    &self.reason
  }
}

#[derive(Debug, Error, Diagnostic)]
pub enum AppError {
  #[error("could not read config file '{}'", .path.to_string_lossy())]
  #[diagnostic(code(wakuchin::config::io))]
  ConfigIoError {
    path: Box<Path>,
    #[source]
    source: io::Error,
  },
  #[error("'{}': Not supported config type", .path.to_string_lossy())]
  #[diagnostic(
    code(wakuchin::config::unsupported_type),
    help("supported config file extensions are: json, yaml, yml, toml")
  )]
  ConfigTypeNotSupported { path: Box<Path> },
  #[error(transparent)]
  #[diagnostic(transparent)]
  ConfigParse(#[from] ConfigParseError),
  #[error(transparent)]
  Wakuchin(#[from] WakuchinError),
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

impl From<dialoguer::Error> for AppError {
  fn from(e: dialoguer::Error) -> Self {
    Self::Other(e.into())
  }
}

pub type Result<T> = anyhow::Result<T, AppError>;
