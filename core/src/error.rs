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
  #[error("could not install the interrupt handler: {0}")]
  SignalHandler(io::Error),
  #[error("{0}")]
  Pattern(String),
  #[error(transparent)]
  Other(#[from] anyhow::Error),
}
