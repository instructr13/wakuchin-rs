use std::io;

use thiserror::Error;
use tokio::task::JoinError;

/// Error type for wakuchin.
#[derive(Debug, Error)]
pub enum WakuchinError {
  /// You may specified bad number of times.
  #[error("times cannot be zero")]
  TimesIsZero,
  #[error(transparent)]
  WorkerError(#[from] JoinError),
  #[error("error while serializing result: {0}")]
  SerializeError(#[from] io::Error),
  #[error(transparent)]
  Other(#[from] anyhow::Error),
}
