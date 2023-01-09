use std::borrow::Cow;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use flume::Receiver;

use crate::result::{Hit, HitCount};

use super::store::{HitStore, SyncHitStore};

pub(crate) struct HitCounterEntry {
  entry: Vec<(Cow<'static, str>, usize)>,
}

impl HitCounterEntry {
  #[inline]
  pub(crate) fn new(entry: Vec<(Cow<'static, str>, usize)>) -> Self {
    Self { entry }
  }

  #[inline]
  pub(crate) fn into_hit_counts(self) -> Vec<HitCount> {
    self
      .entry
      .into_iter()
      .map(|(chars, hits)| HitCount { chars, hits })
      .collect()
  }
}

#[derive(Clone)]
pub(crate) struct ThreadHitCounter {
  pub(crate) count_stopped: Arc<AtomicBool>,
  store: HitStore,
  hit_rx: Receiver<Hit>,
}

impl ThreadHitCounter {
  pub(crate) fn new(hit_rx: Receiver<Hit>) -> Self {
    Self {
      count_stopped: Arc::new(AtomicBool::new(false)),
      store: HitStore::new(),
      hit_rx,
    }
  }

  pub(crate) fn run(&self) {
    for hit in &self.hit_rx {
      self.store.add(hit.chars);
    }

    self.count_stopped.store(true, Ordering::Release);
  }

  #[inline]
  pub(crate) fn get_all(&self) -> HitCounterEntry {
    HitCounterEntry::new(self.store.get_all())
  }
}

pub(crate) struct HitCounter {
  store: SyncHitStore,
}

impl HitCounter {
  #[inline]
  pub(crate) fn new() -> Self {
    Self {
      store: SyncHitStore::new(),
    }
  }

  #[inline]
  pub(crate) fn add(&self, chars: impl Into<Cow<'static, str>>) {
    self.store.add(chars)
  }

  #[inline]
  pub(crate) fn get_all(&self) -> HitCounterEntry {
    HitCounterEntry::new(self.store.get_all())
  }
}
