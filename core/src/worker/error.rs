use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
  #[error("insufficient workers: {0}")]
  InsufficientWorkers(usize),
}
