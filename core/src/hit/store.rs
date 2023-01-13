use std::borrow::Cow;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use dashmap::DashMap;

#[derive(Clone)]
pub struct AtomicHitStore {
  map: Arc<DashMap<Cow<'static, str>, AtomicUsize>>,
}

impl AtomicHitStore {
  #[inline]
  pub fn new() -> Self {
    Self {
      map: Arc::new(DashMap::new()),
    }
  }

  #[inline]
  pub fn add(&self, chars: impl Into<Cow<'static, str>>) {
    self
      .map
      .entry(chars.into())
      .or_insert_with(|| AtomicUsize::new(0))
      .fetch_add(1, Ordering::Relaxed);
  }

  #[inline]
  pub fn get_all(&self) -> Vec<(Cow<'static, str>, usize)> {
    self
      .map
      .iter()
      .map(|entry| (entry.key().clone(), entry.value().load(Ordering::Relaxed)))
      .collect()
  }
}

pub struct HitStore {
  map: DashMap<Cow<'static, str>, usize>,
}

impl HitStore {
  #[inline]
  pub fn new() -> Self {
    Self {
      map: DashMap::new(),
    }
  }

  #[inline]
  pub fn add(&self, chars: impl Into<Cow<'static, str>>) {
    self
      .map
      .entry(chars.into())
      .and_modify(|hits| *hits += 1)
      .or_insert(1);
  }

  #[inline]
  pub fn get_all(&self) -> Vec<(Cow<'static, str>, usize)> {
    self
      .map
      .iter()
      .map(|entry| (entry.key().clone(), *entry.value()))
      .collect()
  }
}
