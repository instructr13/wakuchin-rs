use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use atomig::{Atom, Atomic};

use crate::progress::{
  DoneDetail, IdleDetail, ProcessingDetail, Progress, ProgressKind,
};

/// What a worker is currently doing.
#[derive(Atom, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum WorkerState {
  #[default]
  Idle,
  Processing,
  Done,
}

/// Where one worker has got to, published by the worker and sampled by the renderer.
#[repr(align(128))]
#[derive(Debug, Default)]
pub struct WorkerProgress {
  /// Tries this worker has finished.
  processed: AtomicUsize,

  /// Tries this worker has taken responsibility for.
  claimed: AtomicUsize,

  state: Atomic<WorkerState>,
  sample: Mutex<String>,
}

impl WorkerProgress {
  pub fn claim(&self, tries: usize) {
    self.claimed.fetch_add(tries, Ordering::Relaxed);
    self.state.store(WorkerState::Processing, Ordering::Release);
  }

  /// Records progress within the current claim. `candidate` is best-effort.
  pub fn publish(&self, processed: usize, candidate: &str) {
    self.processed.store(processed, Ordering::Relaxed);

    if let Ok(mut sample) = self.sample.try_lock() {
      sample.clear();
      sample.push_str(candidate);
    }
  }

  pub fn finish(&self, processed: usize) {
    self.processed.store(processed, Ordering::Relaxed);
    self.state.store(WorkerState::Done, Ordering::Release);
  }

  /// Builds the public progress value for this worker. `id` is 1-based; 0 is
  /// reserved for sequential runs.
  pub fn snapshot(&self, id: usize) -> Progress {
    let processed = self.processed.load(Ordering::Relaxed);

    Progress(match self.state.load(Ordering::Acquire) {
      WorkerState::Idle => ProgressKind::Idle(IdleDetail { id }),
      WorkerState::Done => ProgressKind::Done(DoneDetail {
        id,
        total: processed,
      }),
      WorkerState::Processing => {
        ProgressKind::Processing(ProcessingDetail::new(
          id,
          self.candidate(),
          processed,
          self.claimed.load(Ordering::Relaxed),
        ))
      }
    })
  }

  /// Tries this worker has finished, for the aggregate progress bar.
  pub fn processed(&self) -> usize {
    self.processed.load(Ordering::Relaxed)
  }

  fn candidate(&self) -> String {
    self
      .sample
      .lock()
      .map(|sample| sample.clone())
      .unwrap_or_default()
  }
}

#[cfg(test)]
mod test {
  use crate::progress::{Progress, ProgressKind};

  use super::WorkerProgress;

  #[test]
  fn starts_idle() {
    let progress = WorkerProgress::default();

    assert!(matches!(
      progress.snapshot(1),
      Progress(ProgressKind::Idle(detail)) if detail.id == 1
    ));
    assert_eq!(progress.processed(), 0);
  }

  #[test]
  fn reports_the_claim_as_the_total_while_processing() {
    let progress = WorkerProgress::default();

    progress.claim(4096);
    progress.publish(100, "WKCN");
    progress.claim(4096);

    let Progress(ProgressKind::Processing(detail)) = progress.snapshot(2)
    else {
      panic!("expected Processing");
    };

    assert_eq!(detail.id, 2);
    assert_eq!(detail.current, 100);
    assert_eq!(detail.total, 8192);
    assert_eq!(detail.wakuchin, "WKCN");
  }

  #[test]
  fn done_reports_what_was_actually_processed() {
    let progress = WorkerProgress::default();

    progress.claim(4096);
    progress.finish(4096);

    let Progress(ProgressKind::Done(detail)) = progress.snapshot(3) else {
      panic!("expected Done");
    };

    assert_eq!(detail.id, 3);
    assert_eq!(detail.total, 4096);
  }
}
