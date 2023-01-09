use std::time::Duration;

use anyhow::Result;

use crate::{progress::Progress, result::HitCount};

pub mod empty;
pub mod msgpack;

pub trait ProgressHandler: Send {
  fn before_start(&mut self) -> Result<()> {
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

  fn after_finish(&mut self) -> Result<()> {
    Ok(())
  }

  fn on_accidential_stop(&mut self) -> Result<()> {
    self.after_finish()
  }
}
