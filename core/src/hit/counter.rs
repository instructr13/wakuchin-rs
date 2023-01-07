use std::borrow::Cow;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use flume::{Receiver, TryRecvError};
use tokio::task::JoinSet;
use tokio::time::sleep;
use tokio::{runtime::Handle, task::AbortHandle};

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
pub(crate) struct HitCounter {
  pub(crate) count_stopped: Arc<AtomicBool>,
  store: HitStore,
  hit_rx: Receiver<Hit>,
}

impl HitCounter {
  pub(crate) fn new(hit_rx: Receiver<Hit>) -> Self {
    Self {
      count_stopped: Arc::new(AtomicBool::new(false)),
      store: HitStore::new(),
      hit_rx,
    }
  }

  pub(crate) fn run(
    &self,
    set: &mut JoinSet<()>,
    handle: &Handle,
  ) -> AbortHandle {
    set.spawn_on(
      {
        let count_stopped = self.count_stopped.clone();
        let store = self.store.clone();
        let hit_rx = self.hit_rx.clone();

        async move {
          loop {
            match hit_rx.try_recv() {
              Ok(hit) => {
                store.add(hit.chars);
              }
              Err(TryRecvError::Disconnected) => {
                count_stopped.store(true, Ordering::Release);

                break;
              }
              Err(TryRecvError::Empty) => {
                sleep(Duration::from_millis(5)).await;
              }
            }
          }
        }
      },
      handle,
    )
  }

  pub(crate) fn get_all(&self) -> HitCounterEntry {
    HitCounterEntry::new(self.store.get_all())
  }
}

pub(crate) struct SyncHitCounter {
  store: SyncHitStore,
}

impl SyncHitCounter {
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
