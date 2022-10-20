use std::time::Duration;

use anyhow::Result;

use crate::{progress::Progress, result::HitCounter};

use super::ProgressHandler;

pub struct EmptyProgressHandler {}

impl EmptyProgressHandler {
  pub fn new() -> Self {
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
    _: &[HitCounter],
    _: Duration,
    _: usize,
    _: bool,
  ) -> Result<()> {
    Ok(())
  }
}
