use std::fmt::Debug;
use std::ops::Sub;

pub(crate) struct DiffStore<T>
where
  T: Copy + Debug + PartialOrd + Sub,
{
  inner: T,
}

impl<T> DiffStore<T>
where
  T: Copy + Debug + PartialOrd + Sub<Output = T>,
{
  pub(crate) fn new(init: T) -> Self {
    Self { inner: init }
  }

  pub(crate) fn update(&mut self, new: T) -> T {
    let ret = new - self.inner;

    self.inner = new;

    ret
  }
}
