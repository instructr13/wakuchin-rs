use std::cmp::min;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::Result;
use instant::Instant;

use crate::gate::Countdown;
use crate::handlers::ProgressHandler;
use crate::hit::HitTally;
use crate::progress::{ProcessingDetail, Progress, ProgressKind};
use crate::result::HitCount;
use crate::slot::WorkerProgress;
use crate::utils::DiffStore;

pub struct ThreadRender<'run> {
  cancelled: &'run AtomicBool,
  finished: &'run Countdown,
  workers: &'run [WorkerProgress],
  progress_handler: Box<dyn ProgressHandler>,
  /// Reused across refreshes so a tick does not allocate fresh buffers.
  progresses: Vec<Progress>,
  tally: HitTally,
  hits: Vec<HitCount>,
}

impl<'run> ThreadRender<'run> {
  pub fn new(
    cancelled: &'run AtomicBool,
    finished: &'run Countdown,
    workers: &'run [WorkerProgress],
    progress_handler: Box<dyn ProgressHandler>,
  ) -> Self {
    Self {
      cancelled,
      finished,
      progresses: Vec::with_capacity(workers.len()),
      workers,
      progress_handler,
      tally: HitTally::default(),
      hits: Vec::new(),
    }
  }

  #[inline]
  pub fn invoke_before_start(&mut self) -> Result<()> {
    self.progress_handler.before_start(self.workers.len())
  }

  /// Longest single sleep while waiting for the next refresh.
  const POLL_SLICE: Duration = Duration::from_millis(10);

  pub fn run(&mut self, interval: Duration) -> Result<()> {
    self.invoke_before_start()?;

    let mut current_diff = DiffStore::new(0);
    let mut last_tick = Instant::now();

    loop {
      if self.cancelled.load(Ordering::Relaxed) {
        return self.invoke_on_accidental_stop();
      }

      if self.finished.is_done() {
        self.refresh(last_tick.elapsed(), &mut current_diff, true)?;

        break;
      }

      let elapsed = last_tick.elapsed();

      if elapsed < interval {
        self
          .finished
          .wait(min(interval - elapsed, Self::POLL_SLICE));

        continue;
      }

      self.refresh(elapsed, &mut current_diff, false)?;

      last_tick = Instant::now();
    }

    self.invoke_after_finish()?;

    Ok(())
  }

  /// Samples every worker slot and hands the snapshot to the handler.
  fn refresh(
    &mut self,
    elapsed: Duration,
    current_diff: &mut DiffStore,
    all_done: bool,
  ) -> Result<()> {
    self.progresses.clear();
    self.tally.clear();

    let mut current = 0;

    for (index, worker) in self.workers.iter().enumerate() {
      // Worker ids are 1-based everywhere in the public API; 0 means a
      // sequential run.
      self.progresses.push(worker.snapshot(index + 1));

      current += worker.processed();

      worker.merge_hits_into(&mut self.tally);
    }

    self.tally.write_hit_counts(&mut self.hits);

    self.progress_handler.handle(
      &self.progresses,
      &self.hits,
      elapsed,
      current_diff.update(current),
      all_done,
    )
  }

  pub fn invoke_on_accidental_stop(&mut self) -> Result<()> {
    self.progress_handler.on_accidental_stop()
  }

  pub fn invoke_after_finish(&mut self) -> Result<()> {
    self.progress_handler.after_finish()
  }
}

/// Drives the progress handler for a sequential run.
pub struct Render {
  current_diff: DiffStore,
  tally: HitTally,
  hits: Vec<HitCount>,
  progress_handler: Box<dyn ProgressHandler>,
  start_time: Instant,
}

impl Render {
  pub fn new(progress_handler: Box<dyn ProgressHandler>) -> Self {
    Self {
      current_diff: DiffStore::new(0),
      tally: HitTally::default(),
      hits: Vec::new(),
      progress_handler,
      start_time: Instant::now(),
    }
  }

  pub fn into_hit_counts(self) -> Vec<HitCount> {
    self.tally.into_hit_counts()
  }

  /// Folds a chunk's hit counts into the running tally.
  #[inline]
  pub fn record_hits(&mut self, chunk: &HitTally) {
    self.tally.merge_from(chunk);
  }

  #[inline]
  pub fn invoke_before_start(&mut self) -> Result<()> {
    self.progress_handler.before_start(1)
  }

  /// Refreshes the display if `interval` has elapsed since the last one.
  pub fn due(&self, interval: Duration) -> bool {
    interval.is_zero() || self.start_time.elapsed() > interval
  }

  pub fn render_progress(
    &mut self,
    interval: Duration,
    progress: Progress,
    all_done: bool,
  ) -> Result<()> {
    let elapsed = self.start_time.elapsed();

    let current_diff =
      if let Progress(ProgressKind::Processing(ProcessingDetail {
        current,
        ..
      })) = &progress
      {
        self.current_diff.update(*current)
      } else {
        0
      };

    self.tally.write_hit_counts(&mut self.hits);

    self.progress_handler.handle(
      &[progress],
      &self.hits,
      elapsed,
      current_diff,
      all_done,
    )?;

    if !interval.is_zero() {
      self.start_time = Instant::now();
    }

    Ok(())
  }

  #[inline]
  pub fn invoke_on_accidental_stop(&mut self) -> Result<()> {
    self.progress_handler.on_accidental_stop()
  }

  #[inline]
  pub fn invoke_after_finish(&mut self) -> Result<()> {
    self.progress_handler.after_finish()
  }
}
