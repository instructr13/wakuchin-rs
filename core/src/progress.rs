/// Kind of progress data.
#[derive(Clone, Debug)]
pub enum ProgressKind {
  /// Worker is idle.
  Idle(usize, usize),

  /// Worker is processing something.
  Processing(ProcessingDetail),

  /// Worker finished all tasks.
  Done(DoneDetail),
}

/// Detail of processing progress.
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct ProcessingDetail {
  /// Worker ID. 1-indexed, 0 means single worker (sequential).
  pub id: usize,

  /// Current processing wakuchin chars.
  pub wakuchin: String,

  /// Current processing index.
  pub current: usize,

  /// Total number of wakuchin chars to process.
  pub total: usize,

  /// Total number of workers.
  pub total_workers: usize,
}

impl ProcessingDetail {
  /// Create new processing detail.
  ///
  /// # Arguments
  ///
  /// * `id` - Worker ID. 1-indexed, 0 means single worker (sequential).
  /// * `wakuchin` - Current processing wakuchin chars.
  /// * `current` - Current processing index.
  /// * `total` - Total number of wakuchin chars to process.
  /// * `total_workers` - Total number of workers.
  ///
  /// # Returns
  ///
  /// * `ProcessingDetail` - New processing detail.
  pub fn new(
    id: usize,
    wakuchin: impl Into<String>,
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
#[derive(Clone, Debug)]
pub struct DoneDetail {
  /// Worker ID. 1-indexed, 0 means single worker (sequential).
  pub id: usize,

  /// Total number of wakuchin chars to process.
  pub total: usize,

  /// Total number of workers.
  pub total_workers: usize,
}

/// Progress data that you will use in progress_handler.
#[derive(Clone, Debug)]
pub struct Progress(pub ProgressKind);
