use std::fmt::Debug;
use std::ops::Sub;

#[derive(Debug)]
pub(crate) struct DiffStore<T>
where
  T: Copy + Debug + PartialOrd + Sub,
{
  previous: T,
}

impl<T> DiffStore<T>
where
  T: Copy + Debug + PartialOrd + Sub<Output = T>,
{
  pub(crate) fn new(init: T) -> Self {
    Self { previous: init }
  }

  pub(crate) fn update(&mut self, new: T) -> T {
    let ret = new - self.previous;

    self.previous = new;

    ret
  }
}
