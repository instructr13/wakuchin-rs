use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
  #[error("invalid workers: {0}")]
  InvalidWorkers(usize),

  #[error("times cannot be zero")]
  TimesIsZero,

  #[error(transparent)]
  WorkerError(#[from] crate::worker::Error),
}
