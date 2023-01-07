use std::borrow::Cow;

use serde::{Deserialize, Serialize};

/// Kind of progress data.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ProgressKind {
  /// Worker is idle, do nothing.
  Idle(IdleDetail),

  /// Worker is processing something.
  Processing(ProcessingDetail),

  /// Worker finished all tasks.
  Done(DoneDetail),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IdleDetail {
  /// Worker id. 1-indexed, 0 means single worker (sequential).
  pub id: usize,

  /// Total number of workers.
  pub total_workers: usize,
}

/// Detail of processing progress.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProcessingDetail {
  /// Worker id. 1-indexed, 0 means single worker (sequential).
  pub id: usize,

  /// Current processing wakuchin chars.
  pub wakuchin: Cow<'static, str>,

  /// Current processing index.
  pub current: usize,

  /// Total number of wakuchin chars to process _in this worker_.
  pub total: usize,

  /// Total number of workers.
  pub total_workers: usize,
}

impl ProcessingDetail {
  pub(crate) fn new(
    id: usize,
    wakuchin: impl Into<Cow<'static, str>>,
    current: usize,
    total: usize,
    total_workers: usize,
  ) -> Self {
    Self {
      id,
      wakuchin: wakuchin.into(),
      current,
      total,
      total_workers,
    }
  }
}

/// Detail of done progress.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DoneDetail {
  /// Worker id. 1-indexed, 0 means single worker (sequential).
  pub id: usize,

  /// Total number of wakuchin chars to process _in this worker_.
  pub total: usize,

  /// Total number of workers.
  pub total_workers: usize,
}

/// Progress data that you will use in progress_handler.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Progress(pub ProgressKind);
