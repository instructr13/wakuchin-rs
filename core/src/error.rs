use thiserror::Error;

/// Error type for wakuchin.
#[derive(Debug, Error)]
pub enum Error {
  /// You may specified bad number of times.
  #[error("times cannot be zero")]
  TimesIsZero,

  /// Errors related to workers, only for parallelism.
  #[error(transparent)]
  WorkerError(#[from] crate::worker::Error),
}
