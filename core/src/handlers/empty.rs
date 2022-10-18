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
    _: &[crate::progress::Progress],
    _: &[crate::result::HitCounter],
    _: std::time::Duration,
    _: usize,
    _: bool,
  ) -> anyhow::Result<()> {
    Ok(())
  }
}
