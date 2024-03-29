use std::io;

use thiserror::Error;

/// Error type for wakuchin.
#[derive(Debug, Error)]
pub enum WakuchinError {
  #[error("cancelled")]
  Cancelled,
  /// You may specified bad number of times.
  #[error("times cannot be zero")]
  TimesIsZero,
  #[error("'{0}': unknown result output format")]
  UnknownResultOutputFormat(String),
  #[error("error while serializing result: {0}")]
  SerializeError(#[from] io::Error),
  #[error(transparent)]
  Other(#[from] anyhow::Error),
}
