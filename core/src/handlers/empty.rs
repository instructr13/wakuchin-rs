use std::time::Duration;

use anyhow::Result;

use crate::{progress::Progress, result::HitCount};

use super::ProgressHandler;

pub struct EmptyProgressHandler {}

impl EmptyProgressHandler {
  #[must_use]
  pub const fn new() -> Self {
    Self {}
  }
}

impl Default for EmptyProgressHandler {
  fn default() -> Self {
    Self::new()
  }
}

impl ProgressHandler for EmptyProgressHandler {
  fn handle(
    &mut self,
    _: &[Progress],
    _: &[HitCount],
    _: Duration,
    _: usize,
    _: bool,
  ) -> Result<()> {
    Ok(())
  }
}
