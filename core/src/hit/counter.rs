use std::borrow::Cow;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use flume::Receiver;

use crate::result::{Hit, HitCount};

use super::store::{AtomicHitStore, HitStore};

pub struct HitCounterEntry {
  entry: Vec<(Cow<'static, str>, usize)>,
}

impl HitCounterEntry {
  #[inline]
  pub fn new(entry: Vec<(Cow<'static, str>, usize)>) -> Self {
    Self { entry }
  }

  #[inline]
  pub fn into_hit_counts(self) -> Vec<HitCount> {
    self
      .entry
      .into_iter()
      .map(|(chars, hits)| HitCount { chars, hits })
      .collect()
  }
}

#[derive(Clone)]
pub struct ThreadHitCounter {
  pub count_stopped: Arc<AtomicBool>,
  store: AtomicHitStore,
  hit_rx: Receiver<Hit>,
}

impl ThreadHitCounter {
  pub fn new(hit_rx: Receiver<Hit>) -> Self {
    Self {
      count_stopped: Arc::new(AtomicBool::new(false)),
      store: AtomicHitStore::new(),
      hit_rx,
    }
  }

  pub fn run(&self) {
    for hit in &self.hit_rx {
      self.store.add(hit.chars);
    }

    self.count_stopped.store(true, Ordering::Release);
  }

  #[inline]
  pub fn get_all(&self) -> HitCounterEntry {
    HitCounterEntry::new(self.store.get_all())
  }
}

pub struct HitCounter {
  store: HitStore,
}

impl HitCounter {
  #[inline]
  pub fn new() -> Self {
    Self {
      store: HitStore::new(),
    }
  }

  #[inline]
  pub fn add(&self, chars: impl Into<Cow<'static, str>>) {
    self.store.add(chars);
  }

  #[inline]
  pub fn get_all(&self) -> HitCounterEntry {
    HitCounterEntry::new(self.store.get_all())
  }
}
