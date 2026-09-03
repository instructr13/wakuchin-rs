/// Tracks the delta between successive absolute readings.
///
/// Used to turn a running "tries processed" total into a per-refresh rate.
/// `update` saturates rather than wrapping: the aggregate is assembled from
/// several independently-updated counters, so a later reading can legitimately
/// come in below an earlier one.
#[derive(Debug)]
pub struct DiffStore {
  previous: usize,
}

impl DiffStore {
  pub const fn new(init: usize) -> Self {
    Self { previous: init }
  }

  pub fn update(&mut self, new: usize) -> usize {
    let ret = new.saturating_sub(self.previous);

    self.previous = new;

    ret
  }
}

#[cfg(test)]
mod test {
  use crate::utils::DiffStore;

  #[test]
  fn test_diff_store() {
    let mut store = DiffStore::new(0);

    assert_eq!(store.update(1), 1);
    assert_eq!(store.update(2), 1);
    assert_eq!(store.update(3), 1);
    assert_eq!(store.update(4), 1);
    assert_eq!(store.update(5), 1);
  }

  #[test]
  fn test_diff_store_saturates_on_a_lower_reading() {
    let mut store = DiffStore::new(0);

    assert_eq!(store.update(10), 10);
    assert_eq!(store.update(4), 0);
    assert_eq!(store.update(12), 8);
  }
}
