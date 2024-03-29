use std::time::Duration;

use anyhow::Result;

use crate::{progress::Progress, result::HitCount};

pub mod empty;
pub mod msgpack;

pub trait ProgressHandler: Send {
  #[inline]
  fn before_start(&mut self, _total_workers: usize) -> Result<()> {
    Ok(())
  }

  fn handle(
    &mut self,
    progresses: &[Progress],
    hit_counts: &[HitCount],
    elapsed_time: Duration,
    current_diff: usize,
    all_done: bool,
  ) -> Result<()>;

  #[inline]
  fn after_finish(&mut self) -> Result<()> {
    Ok(())
  }

  #[inline]
  fn on_accidential_stop(&mut self) -> Result<()> {
    self.after_finish()
  }
}
