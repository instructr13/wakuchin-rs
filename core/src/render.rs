use std::borrow::Cow;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use instant::Instant;
use itertools::Itertools;
use tokio::sync::watch;

use crate::handlers::ProgressHandler;
use crate::hit::counter::{HitCounter, ThreadHitCounter};
use crate::progress::{DoneDetail, ProcessingDetail, Progress, ProgressKind};
use crate::result::HitCount;
use crate::utils::DiffStore;

pub(crate) struct ThreadRender {
  is_stopped_accidentially: Arc<AtomicBool>,
  counter: ThreadHitCounter,
  progress_channels: Vec<watch::Receiver<Progress>>,
  progress_handler: Box<dyn ProgressHandler>,
  total: usize,
  total_workers: usize,
}

impl ThreadRender {
  pub(crate) fn new(
    is_stopped_accidentially: Arc<AtomicBool>,
    counter: ThreadHitCounter,
    progress_channels: Vec<watch::Receiver<Progress>>,
    progress_handler: Box<dyn ProgressHandler>,
    total: usize,
    total_workers: usize,
  ) -> Self {
    Self {
      is_stopped_accidentially,
      counter,
      progress_channels,
      progress_handler,
      total,
      total_workers,
    }
  }

  fn hits(&self) -> Vec<HitCount> {
    self.counter.get_all().into_hit_counts()
  }

  pub(crate) fn invoke_before_start(&mut self) -> Result<()> {
    self.progress_handler.before_start()
  }

  pub(crate) fn run(&mut self, interval: Duration) -> Result<()> {
    self.invoke_before_start()?;

    let mut start_time = Instant::now();
    let mut current_diff = DiffStore::new(0_usize);
    let mut current_ = 0;

    loop {
      if self.is_stopped_accidentially.load(Ordering::SeqCst) {
        return self.invoke_on_accidential_stop();
      }

      if self.counter.count_stopped.load(Ordering::Acquire) {
        self.progress_handler.handle(
          &(0..self.progress_channels.len())
            .map(|id| {
              Progress(ProgressKind::Done(DoneDetail {
                id,
                total: self.total,
                total_workers: self.total_workers,
              }))
            })
            .collect_vec(),
          &self.hits(),
          interval,
          0,
          true,
        )?;

        break;
      }

      if start_time.elapsed() < interval {
        continue;
      }

      let progressses = &self
        .progress_channels
        .iter()
        .map(|rx| {
          let progress = rx.borrow();

          match *progress {
            Progress(ProgressKind::Processing(ProcessingDetail {
              current,
              ..
            })) => {
              current_ += current;
            }
            Progress(ProgressKind::Done(DoneDetail { total, .. })) => {
              current_ += total;
            }
            _ => {}
          }

          progress.clone()
        })
        .collect_vec();

      self.progress_handler.handle(
        progressses,
        &self.hits(),
        interval,
        current_diff.update(current_),
        false,
      )?;

      current_ = 0;
      start_time = Instant::now();
    }

    self.invoke_after_finish()?;

    Ok(())
  }

  pub(crate) fn invoke_on_accidential_stop(&mut self) -> Result<()> {
    self.progress_handler.on_accidential_stop()
  }

  pub(crate) fn invoke_after_finish(&mut self) -> Result<()> {
    self.progress_handler.after_finish()
  }
}

pub(crate) struct Render {
  current_diff: DiffStore<usize>,
  counter: HitCounter,
  progress_handler: Box<dyn ProgressHandler>,
  start_time: Instant,
}

impl Render {
  pub(crate) fn new(progress_handler: Box<dyn ProgressHandler>) -> Self {
    Self {
      current_diff: DiffStore::new(0),
      counter: HitCounter::new(),
      progress_handler,
      start_time: Instant::now(),
    }
  }

  #[inline]
  pub(crate) fn hits(&self) -> Vec<HitCount> {
    self.counter.get_all().into_hit_counts()
  }

  #[inline]
  pub(crate) fn handle_hit(&self, chars: impl Into<Cow<'static, str>>) {
    // Insert hit to hit counter with specific char entry
    self.counter.add(chars);
  }

  pub(crate) fn invoke_before_start(&mut self) -> Result<()> {
    self.progress_handler.before_start()
  }

  pub(crate) fn render_progress(
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

  pub(crate) fn invoke_on_accidential_stop(&mut self) -> Result<()> {
    self.progress_handler.on_accidential_stop()
  }

  pub(crate) fn invoke_after_finish(&mut self) -> Result<()> {
    self.progress_handler.after_finish()
  }
}
