use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::Result;
use dashmap::DashMap;
use flume::{
  bounded as channel, unbounded as channel_unbounded, Receiver, Sender,
  TryRecvError,
};
use itertools::Itertools;
use tokio::sync::{watch, RwLock};
use tokio::time::{sleep, Duration};

use crate::handlers::ProgressHandler;
use crate::progress::{DoneDetail, ProcessingDetail, Progress, ProgressKind};
use crate::result::{Hit, HitCount};
use crate::utils::DiffStore;

fn hit_map_to_count(map: &DashMap<String, usize>) -> Vec<HitCount> {
  map
    .iter()
    .map(|ref_| HitCount {
      chars: ref_.key().clone(),
      hits: *ref_.value(),
    })
    .collect()
}

struct ThreadRenderInner {
  accidential_stop_rx: watch::Receiver<bool>,
  hit_counter: DashMap<String, usize>,
  internal_hit_rx: Receiver<Hit>,
  progress_channels: Vec<watch::Receiver<Progress>>,
  progress_handler: Arc<Mutex<Box<dyn ProgressHandler>>>,
  stop_rx: Receiver<bool>,
  total: usize,
  total_workers: usize,
}

impl ThreadRenderInner {
  fn hits(&self) -> Vec<HitCount> {
    hit_map_to_count(&self.hit_counter)
  }

  async fn start_render_progress(&mut self, interval: Duration) -> Result<()> {
    let mut start_time = Instant::now();
    let mut current_diff = DiffStore::new(0_usize);
    let mut current_ = 0;

    let mut progress_handler = self.progress_handler.lock().unwrap();

    loop {
      let accidential_stop = self.accidential_stop_rx.borrow();

      if *accidential_stop {
        return progress_handler.on_accidential_stop();
      }

      if self.stop_rx.try_recv().is_ok() {
        progress_handler.handle(
          &(0..self.progress_channels.len())
            .map(|id| {
              Progress(ProgressKind::Done(DoneDetail {
                id,
                total: self.total,
                total_workers: self.total_workers,
              }))
            })
            .collect_vec(),
          &self.hits(),
          interval,
          0,
          true,
        )?;

        break;
      }

      if !self.internal_hit_rx.is_empty() {
        while let Ok(hit) = self.internal_hit_rx.try_recv() {
          *self.hit_counter.entry(hit.chars.clone()).or_insert(0) += 1;
        }
      }

      if start_time.elapsed() < interval {
        continue;
      }

      let progressses = &self
        .progress_channels
        .iter()
        .map(|rx| {
          let progress = rx.borrow();

          match *progress {
            Progress(ProgressKind::Processing(ProcessingDetail {
              current,
              ..
            })) => {
              current_ += current;
            }
            Progress(ProgressKind::Done(DoneDetail { total, .. })) => {
              current_ += total;
            }
            _ => {}
          }

          progress.clone()
        })
        .collect_vec();

      progress_handler.handle(
        progressses,
        &self.hits(),
        interval,
        current_diff.update(current_),
        false,
      )?;

      current_ = 0;
      start_time = Instant::now();
    }

    Ok(())
  }
}

pub(crate) struct ThreadRender {
  inner: Arc<RwLock<ThreadRenderInner>>,
  hit_rx: Receiver<Hit>,
  internal_hit_tx: Sender<Hit>,
  stop_tx: Sender<bool>,
}

impl ThreadRender {
  pub(crate) fn new(
    accidential_stop_rx: watch::Receiver<bool>,
    hit_rx: Receiver<Hit>,
    progress_channels: Vec<watch::Receiver<Progress>>,
    progress_handler: Arc<Mutex<Box<dyn ProgressHandler>>>,
    total: usize,
    total_workers: usize,
  ) -> Self {
    let (internal_hit_tx, internal_hit_rx) = channel_unbounded();
    let (stop_tx, stop_rx) = channel(1);

    Self {
      inner: Arc::new(RwLock::new(ThreadRenderInner {
        accidential_stop_rx,
        hit_counter: DashMap::new(),
        internal_hit_rx,
        progress_channels,
        progress_handler,
        stop_rx,
        total,
        total_workers,
      })),
      stop_tx,
      hit_rx,
      internal_hit_tx,
    }
  }

  pub(crate) async fn hits(&self) -> Vec<HitCount> {
    let inner = self.inner.read().await;

    inner.hits()
  }

  pub(crate) async fn wait_for_hit(&self) {
    loop {
      let hit = self.hit_rx.try_recv();

      match hit {
        Ok(hit) => {
          self
            .internal_hit_tx
            .send_async(hit)
            .await
            .expect("internal hit channel sending failed");
        }
        Err(TryRecvError::Disconnected) => {
          self
            .stop_tx
            .send_async(true)
            .await
            .expect("stop channel sending failed");

          break;
        }
        Err(TryRecvError::Empty) => {
          sleep(Duration::from_millis(5)).await;
        }
      }
    }
  }

  pub(crate) async fn start_render_progress(
    &self,
    interval: Duration,
  ) -> Result<()> {
    let mut inner = self.inner.write().await;

    inner.start_render_progress(interval).await
  }
}

pub(crate) struct Render {
  current_diff: DiffStore<usize>,
  hit_counter: DashMap<String, usize>,
  progress_handler: Rc<RefCell<dyn ProgressHandler>>,
  start_time: Instant,
}

impl Render {
  pub(crate) fn new(
    progress_handler: Rc<RefCell<dyn ProgressHandler>>,
  ) -> Self {
    Self {
      current_diff: DiffStore::new(0),
      hit_counter: DashMap::new(),
      progress_handler,
      start_time: Instant::now(),
    }
  }

  pub(crate) fn hits(&self) -> Vec<HitCount> {
    hit_map_to_count(&self.hit_counter)
  }

  pub(crate) fn handle_hit(&self, hit: &Hit) {
    // Insert hit to hit counter with specific char entry
    *self.hit_counter.entry(hit.chars.clone()).or_insert(0) += 1;
  }

  pub(crate) fn render_progress(
    &mut self,
    interval: Duration,
    progress: Progress,
    all_done: bool,
  ) -> Result<()> {
    if interval.is_zero() {
      let mut progress_handler = self.progress_handler.borrow_mut();

      progress_handler.handle(
        &[progress],
        &self.hits(),
        interval,
        0,
        all_done,
      )?;

      return Ok(());
    }

    if self.start_time.elapsed() <= interval {
      return Ok(());
    }

    if matches!(progress, Progress(ProgressKind::Done(_))) {
      return Ok(());
    }

    let mut progress_handler = self.progress_handler.borrow_mut();

    let current_diff =
      if let Progress(ProgressKind::Processing(ProcessingDetail {
        current,
        ..
      })) = &progress
      {
        self.current_diff.update(*current)
      } else {
        0
      };

    progress_handler.handle(
      &[progress],
      &self.hits(),
      interval,
      current_diff,
      all_done,
    )?;

    self.start_time = Instant::now();

    Ok(())
  }
}
