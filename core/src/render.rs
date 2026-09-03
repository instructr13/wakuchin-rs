use std::borrow::Cow;
use std::cmp::min;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use instant::Instant;

use crate::handlers::ProgressHandler;
use crate::hit::counter::{HitCounter, ThreadHitCounter};
use crate::progress::{ProcessingDetail, Progress, ProgressKind};
use crate::result::HitCount;
use crate::slot::WorkerProgress;
use crate::utils::DiffStore;

pub struct ThreadRender<'slots> {
  is_stopped_accidentially: Arc<AtomicBool>,
  counter: ThreadHitCounter,
  workers: &'slots [WorkerProgress],
  progress_handler: Box<dyn ProgressHandler>,
  /// Reused across refreshes so a tick does not allocate a fresh vector.
  progresses: Vec<Progress>,
}

impl<'slots> ThreadRender<'slots> {
  pub fn new(
    is_stopped_accidentially: Arc<AtomicBool>,
    counter: ThreadHitCounter,
    workers: &'slots [WorkerProgress],
    progress_handler: Box<dyn ProgressHandler>,
  ) -> Self {
    Self {
      is_stopped_accidentially,
      counter,
      progresses: Vec::with_capacity(workers.len()),
      workers,
      progress_handler,
    }
  }

  #[inline]
  pub fn invoke_before_start(&mut self) -> Result<()> {
    self.progress_handler.before_start(self.workers.len())
  }

  /// Longest single sleep while waiting for the next refresh.
  ///
  /// This loop used to `continue` without sleeping, which pinned a core for the
  /// entire run and read the clock tens of millions of times a second. Sleeping
  /// in slices keeps the refresh cadence while still noticing completion and
  /// cancellation promptly, and costs about a hundred wakeups a second.
  const POLL_SLICE: Duration = Duration::from_millis(10);

  pub fn run(&mut self, interval: Duration) -> Result<()> {
    self.invoke_before_start()?;

    let mut current_diff = DiffStore::new(0);
    let mut last_tick = Instant::now();

    loop {
      if self.is_stopped_accidentially.load(Ordering::Relaxed) {
        return self.invoke_on_accidential_stop();
      }

      if self.counter.done.is_done() {
        self.refresh(last_tick.elapsed(), &mut current_diff, true)?;

        break;
      }

      let elapsed = last_tick.elapsed();

      if elapsed < interval {
        self.counter.done.wait(min(interval - elapsed, Self::POLL_SLICE));

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

    let mut current = 0;

    for (index, worker) in self.workers.iter().enumerate() {
      // Worker ids are 1-based everywhere in the public API; 0 means a
      // sequential run.
      self.progresses.push(worker.snapshot(index + 1));

      current += worker.processed();
    }

    let hits = self.counter.get_all().into_hit_counts();

    self.progress_handler.handle(
      &self.progresses,
      &hits,
      elapsed,
      current_diff.update(current),
      all_done,
    )
  }

  pub fn invoke_on_accidential_stop(&mut self) -> Result<()> {
    self.progress_handler.on_accidential_stop()
  }

  pub fn invoke_after_finish(&mut self) -> Result<()> {
    self.progress_handler.after_finish()
  }
}

pub struct Render {
  current_diff: DiffStore,
  counter: HitCounter,
  progress_handler: Box<dyn ProgressHandler>,
  start_time: Instant,
}

impl Render {
  pub fn new(progress_handler: Box<dyn ProgressHandler>) -> Self {
    Self {
      current_diff: DiffStore::new(0),
      counter: HitCounter::new(),
      progress_handler,
      start_time: Instant::now(),
    }
  }

  #[inline]
  pub fn hits(&self) -> Vec<HitCount> {
    self.counter.get_all().into_hit_counts()
  }

  #[inline]
  pub fn handle_hit(&self, chars: impl Into<Cow<'static, str>>) {
    // Insert hit to hit counter with specific char entry
    self.counter.add(chars);
  }

  #[inline]
  pub fn invoke_before_start(&mut self) -> Result<()> {
    self.progress_handler.before_start(1)
  }

  pub fn render_progress(
    &mut self,
    interval: Duration,
    progress: Progress,
    all_done: bool,
  ) -> Result<()> {
    if interval.is_zero() {
      self.progress_handler.handle(
        &[progress],
        &self.hits(),
        interval,
        0,
        all_done,
      )?;

      return Ok(());
    }

    if self.start_time.elapsed() <= interval {
      return Ok(());
    }

    if matches!(progress, Progress(ProgressKind::Done(_))) {
      return Ok(());
    }

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

    self.progress_handler.handle(
      &[progress],
      &self.hits(),
      interval,
      current_diff,
      all_done,
    )?;

    self.start_time = Instant::now();

    Ok(())
  }

  #[inline]
  pub fn invoke_on_accidential_stop(&mut self) -> Result<()> {
    self.progress_handler.on_accidential_stop()
  }

  #[inline]
  pub fn invoke_after_finish(&mut self) -> Result<()> {
    self.progress_handler.after_finish()
  }
}
