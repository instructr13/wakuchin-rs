use std::time::Duration;

use crate::pattern::Pattern;

use crate::error::WakuchinError;
use crate::handlers::empty::EmptyProgressHandler;
use crate::handlers::ProgressHandler;
use crate::result::WakuchinResult;
use crate::worker::{exact, run_par, run_seq};

type Result<T> = std::result::Result<T, WakuchinError>;

pub struct ResearchBuilder<Tries, Times, TPattern> {
  tries: Tries,
  times: Times,
  regex: TPattern,
  progress_handler: Box<dyn ProgressHandler>,
  progress_interval: Duration,
  workers: usize,
}

impl ResearchBuilder<(), (), ()> {
  #[must_use]
  pub fn new() -> Self {
    Self {
      tries: (),
      times: (),
      regex: (),
      progress_handler: Box::new(EmptyProgressHandler::new()),
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

impl<Tries, Times, TPattern> ResearchBuilder<Tries, Times, TPattern> {
  pub fn tries(self, tries: usize) -> ResearchBuilder<usize, Times, TPattern> {
    ResearchBuilder {
      tries,
      times: self.times,
      regex: self.regex,
      progress_handler: self.progress_handler,
      progress_interval: self.progress_interval,
      workers: self.workers,
    }
  }

  pub fn times(self, times: usize) -> ResearchBuilder<Tries, usize, TPattern> {
    ResearchBuilder {
      tries: self.tries,
      times,
      regex: self.regex,
      progress_handler: self.progress_handler,
      progress_interval: self.progress_interval,
      workers: self.workers,
    }
  }

  pub fn regex(self, regex: Pattern) -> ResearchBuilder<Tries, Times, Pattern> {
    ResearchBuilder {
      tries: self.tries,
      times: self.times,
      regex,
      progress_handler: self.progress_handler,
      progress_interval: self.progress_interval,
      workers: self.workers,
    }
  }

  pub fn progress_handler(
    mut self,
    progress_handler: Box<dyn ProgressHandler>,
  ) -> Self {
    self.progress_handler = progress_handler;

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

impl ResearchBuilder<usize, usize, Pattern> {
  #[inline]
  pub fn run_par(self) -> Result<WakuchinResult> {
    run_par(
      self.tries,
      self.times,
      &self.regex,
      self.progress_handler,
      self.progress_interval,
      self.workers,
    )
  }

  #[inline]
  pub fn exact(self) -> Result<WakuchinResult> {
    exact(self.times, &self.regex)
  }

  #[inline]
  pub fn run_seq(self) -> Result<WakuchinResult> {
    run_seq(
      self.tries,
      self.times,
      &self.regex,
      self.progress_handler,
      self.progress_interval,
    )
  }
}
