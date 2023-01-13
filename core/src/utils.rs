use std::fmt::Debug;
use std::ops::Sub;

#[derive(Debug)]
pub struct DiffStore<T>
where
  T: Copy + Debug + PartialOrd + Sub,
{
  previous: T,
}

impl<T> DiffStore<T>
where
  T: Copy + Debug + PartialOrd + Sub<Output = T>,
{
  pub const fn new(init: T) -> Self {
    Self { previous: init }
  }

  pub fn update(&mut self, new: T) -> T {
    let ret = new - self.previous;

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
}
