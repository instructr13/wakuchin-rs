use std::borrow::Cow;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use dashmap::DashMap;

#[derive(Clone)]
pub(crate) struct HitStore {
  map: Arc<DashMap<Cow<'static, str>, AtomicUsize>>,
}

impl HitStore {
  #[inline]
  pub(crate) fn new() -> Self {
    Self {
      map: Arc::new(DashMap::new()),
    }
  }

  #[inline]
  pub(crate) fn add(&self, chars: impl Into<Cow<'static, str>>) {
    self
      .map
      .entry(chars.into())
      .or_insert_with(|| AtomicUsize::new(0))
      .fetch_add(1, Ordering::Relaxed);
  }

  #[inline]
  pub(crate) fn get_all(&self) -> Vec<(Cow<'static, str>, usize)> {
    self
      .map
      .iter()
      .map(|entry| (entry.key().clone(), entry.value().load(Ordering::Relaxed)))
      .collect()
  }
}

pub(crate) struct SyncHitStore {
  map: DashMap<Cow<'static, str>, usize>,
}

impl SyncHitStore {
  #[inline]
  pub(crate) fn new() -> Self {
    Self {
      map: DashMap::new(),
    }
  }

  #[inline]
  pub(crate) fn add(&self, chars: impl Into<Cow<'static, str>>) {
    self
      .map
      .entry(chars.into())
      .and_modify(|hits| *hits += 1)
      .or_insert(1);
  }

  #[inline]
  pub(crate) fn get_all(&self) -> Vec<(Cow<'static, str>, usize)> {
    self
      .map
      .iter()
      .map(|entry| (entry.key().clone(), *entry.value()))
      .collect()
  }
}
