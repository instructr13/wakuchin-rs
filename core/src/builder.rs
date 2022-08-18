use std::error::Error;
use std::time::Duration;

use regex::Regex;

use crate::progress::Progress;
use crate::result::{HitCounter, WakuchinResult};
use crate::worker::{run_par, run_seq};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
type ProgressHandler =
  Box<dyn Fn(&[Progress], &[HitCounter], Duration, usize, bool) + Sync + Send>;

fn empty_progress_handler(
) -> fn(&[Progress], &[HitCounter], Duration, usize, bool) {
  |_, _, _, _, _| {}
}

pub struct ResearchBuilder<Tries, Times, TRegex> {
  tries: Tries,
  times: Times,
  regex: TRegex,
  progress_handler: ProgressHandler,
  progress_interval: Duration,
  workers: usize,
}

impl ResearchBuilder<(), (), ()> {
  pub fn new() -> Self {
    Self {
      tries: (),
      times: (),
      regex: (),
      progress_handler: Box::new(empty_progress_handler()),
      progress_interval: Duration::from_millis(500),
      workers: 0,
    }
  }
}

impl Default for ResearchBuilder<(), (), ()> {
  fn default() -> Self {
    Self::new()
  }
}

impl<Tries, Times, TRegex> ResearchBuilder<Tries, Times, TRegex> {
  pub fn tries(self, tries: usize) -> ResearchBuilder<usize, Times, TRegex> {
    ResearchBuilder {
      tries,
      times: self.times,
      regex: self.regex,
      progress_handler: self.progress_handler,
      progress_interval: self.progress_interval,
      workers: self.workers,
    }
  }

  pub fn times(self, times: usize) -> ResearchBuilder<Tries, usize, TRegex> {
    ResearchBuilder {
      tries: self.tries,
      times,
      regex: self.regex,
      progress_handler: self.progress_handler,
      progress_interval: self.progress_interval,
      workers: self.workers,
    }
  }

  pub fn regex(self, regex: Regex) -> ResearchBuilder<Tries, Times, Regex> {
    ResearchBuilder {
      tries: self.tries,
      times: self.times,
      regex,
      progress_handler: self.progress_handler,
      progress_interval: self.progress_interval,
      workers: self.workers,
    }
  }

  pub fn progress_handler<F>(mut self, progress_handler: F) -> Self
  where
    F: Fn(&[Progress], &[HitCounter], Duration, usize, bool),
    F: Sync + Send + 'static,
  {
    self.progress_handler = Box::new(progress_handler);

    self
  }

  pub fn progress_interval(mut self, progress_interval: Duration) -> Self {
    self.progress_interval = progress_interval;

    self
  }

  pub fn workers(mut self, workers: usize) -> Self {
    self.workers = workers;

    self
  }
}

impl ResearchBuilder<usize, usize, Regex> {
  pub async fn run_par(self) -> Result<WakuchinResult> {
    run_par(
      self.tries,
      self.times,
      self.regex,
      self.progress_handler,
      self.progress_interval,
      self.workers,
    )
    .await
  }

  pub fn run_seq(self) -> Result<WakuchinResult> {
    run_seq(
      self.tries,
      self.times,
      self.regex,
      self.progress_handler,
      self.progress_interval,
    )
  }
}
