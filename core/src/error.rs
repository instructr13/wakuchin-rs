use std::io;

use thiserror::Error;

/// Error type for wakuchin.
#[derive(Debug, Error)]
pub enum WakuchinError {
  #[error("cancelled")]
  Cancelled,
  /// You may specify bad number of times.
  #[error("times cannot be zero")]
  TimesIsZero,
  #[error(
    "times cannot exceed {max}, but {times} was given: beyond that the number \
     of distinct wakuchin arrangements no longer fits in a 64-bit integer, so \
     the size of the space being searched cannot be represented"
  )]
  TimesTooLarge { times: usize, max: usize },
  #[error(
    "the candidate space for times = {times} is too large to enumerate, so \
     there is no exact answer to give"
  )]
  SpaceTooLarge { times: usize },
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
