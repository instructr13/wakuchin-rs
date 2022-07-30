use thiserror::Error;

/// Error type for wakuchin with parallelism.
#[derive(Debug, Error)]
pub enum Error {
  /// Means that the number of workers is too small.
  #[error("insufficient workers: {0}")]
  InsufficientWorkers(usize),
}
