use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use arc_swap::ArcSwap;
use atomig::{Atom, Atomic};
use parking_lot::Mutex;

use crate::hit::HitTally;
use crate::progress::{
  DoneDetail, IdleDetail, ProcessingDetail, Progress, ProgressKind,
};
use crate::result::Hit;

/// What a worker is currently doing.
///
/// Stored in an `atomig::Atomic` so the state stays a real enum rather than a
/// bare `AtomicU8` with hand-written constants and a catch-all match arm.
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
#[derive(Debug)]
pub struct WorkerProgress {
  /// Tries this worker has finished.
  processed: AtomicUsize,

  /// Tries this worker has taken responsibility for.
  claimed: AtomicUsize,

  state: Atomic<WorkerState>,
  sample: ArcSwap<String>,
  tally: Mutex<HitTally>,
}

impl Default for WorkerProgress {
  fn default() -> Self {
    Self {
      processed: AtomicUsize::new(0),
      claimed: AtomicUsize::new(0),
      state: Atomic::new(WorkerState::default()),
      sample: ArcSwap::from_pointee(String::new()),
      tally: Mutex::new(HitTally::default()),
    }
  }
}

impl WorkerProgress {
  pub fn claim(&self, tries: usize) {
    self.claimed.fetch_add(tries, Ordering::Relaxed);
    self.state.store(WorkerState::Processing, Ordering::Release);
  }

  /// Records progress within the current claim.
  pub fn publish(&self, processed: usize, candidate: &str) {
    self.processed.store(processed, Ordering::Relaxed);
    self.sample.store(Arc::new(candidate.to_owned()));
  }

  pub fn record_hits(&self, hits: &[Hit]) {
    if hits.is_empty() {
      return;
    }

    let mut tally = self.tally.lock();

    for hit in hits {
      tally.add(&hit.chars);
    }
  }

  pub fn merge_hits_into(&self, out: &mut HitTally) {
    out.merge_from(&self.tally.lock());
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
          self.sample.load().as_str().to_owned(),
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
}

#[cfg(test)]
mod test {
  use crate::hit::HitTally;
  use crate::progress::{Progress, ProgressKind};
  use crate::result::Hit;

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

  #[test]
  fn hits_merge_out_of_the_worker_tally() {
    let progress = WorkerProgress::default();

    progress.record_hits(&[
      Hit::new(0, "WKCN"),
      Hit::new(3, "WKCN"),
      Hit::new(9, "NCKW"),
    ]);

    let mut tally = HitTally::default();

    progress.merge_hits_into(&mut tally);

    let counts = tally.into_hit_counts();

    assert_eq!(counts.len(), 2);
    assert_eq!(counts[0].chars, "NCKW");
    assert_eq!(counts[0].hits, 1);
    assert_eq!(counts[1].chars, "WKCN");
    assert_eq!(counts[1].hits, 2);
  }
}
