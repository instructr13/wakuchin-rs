use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use anyhow::Result;

use crate::{progress::Progress, result::HitCount};

pub mod empty;
pub mod msgpack;

pub trait ProgressHandler: RefCellWrapper + Sync + Send + 'static {
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

pub trait RefCellWrapper {
  fn wrap_in_refcell(self: Box<Self>) -> Rc<RefCell<dyn ProgressHandler>>;
}

impl<T: ProgressHandler + 'static> RefCellWrapper for T {
  fn wrap_in_refcell(self: Box<Self>) -> Rc<RefCell<dyn ProgressHandler>> {
    Rc::new(RefCell::new(*self))
  }
}
