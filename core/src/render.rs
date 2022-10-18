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
use tokio::sync::watch;
use tokio::time::{sleep, Duration};

use crate::handlers::ProgressHandler;
use crate::progress::{DoneDetail, ProcessingDetail, Progress, ProgressKind};
use crate::result::{Hit, HitCounter};
use crate::utils::DiffStore;

pub(crate) struct ThreadRenderInner {
  hit_rx: Receiver<Hit>,
  internal_hit_tx: Sender<Hit>,
  stop_tx: Sender<bool>,
}

pub(crate) struct ThreadRender {
  hit_counter: DashMap<String, HitCounter>,
  pub(crate) inner: Arc<ThreadRenderInner>,
  internal_hit_rx: Receiver<Hit>,
  progress_channels: Vec<watch::Receiver<Progress>>,
  progress_handler: Arc<Mutex<Box<dyn ProgressHandler>>>,
  stop_rx: Receiver<bool>,
  total: usize,
  total_workers: usize,
}

impl ThreadRenderInner {
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
}

impl ThreadRender {
  pub(crate) fn new(
    hit_rx: Receiver<Hit>,
    progress_channels: Vec<watch::Receiver<Progress>>,
    progress_handler: Arc<Mutex<Box<dyn ProgressHandler>>>,
    total: usize,
    total_workers: usize,
  ) -> Self {
    let (internal_hit_tx, internal_hit_rx) = channel_unbounded();
    let (stop_tx, stop_rx) = channel(1);

    Self {
      hit_counter: DashMap::new(),
      inner: Arc::new(ThreadRenderInner {
        internal_hit_tx,
        hit_rx,
        stop_tx,
      }),
      internal_hit_rx,
      progress_channels,
      progress_handler,
      stop_rx,
      total,
      total_workers,
    }
  }

  pub(crate) fn hits(&self) -> Vec<HitCounter> {
    self
      .hit_counter
      .iter()
      .map(|ref_| ref_.value().clone())
      .collect()
  }

  pub(crate) async fn start_render_progress(
    &mut self,
    interval: Duration,
  ) -> Result<()> {
    let mut start_time = Instant::now();
    let mut current_diff = DiffStore::new(0_usize);
    let mut current_ = 0;

    let mut progress_handler = self.progress_handler.lock().unwrap();

    loop {
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
          self
            .hit_counter
            .entry(hit.chars.clone())
            .or_insert(HitCounter {
              chars: hit.chars,
              hits: 0,
            })
            .hits += 1;
        }
      }

      if start_time.elapsed() < interval {
        continue;
      }

      let progressses = &self
        .progress_channels
        .iter()
        .map(|rx| {
          let progress = rx.borrow().clone();

          match progress {
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

          progress
        })
        .collect_vec();

      progress_handler.handle(
        progressses,
        &self
          .hit_counter
          .iter()
          .map(|ref_| ref_.value().clone())
          .collect_vec(),
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

pub(crate) struct Render {
  current_diff: DiffStore<usize>,
  hit_counter: DashMap<String, HitCounter>,
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

  pub(crate) fn hits(&self) -> Vec<HitCounter> {
    self
      .hit_counter
      .iter()
      .map(|ref_| ref_.value().clone())
      .collect()
  }

  pub(crate) fn handle_hit(&self, hit: &Hit) {
    self
      .hit_counter
      .entry(hit.chars.clone())
      .or_insert(HitCounter {
        chars: hit.chars.clone(),
        hits: 0,
      })
      .hits += 1;
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
        &self
          .hit_counter
          .iter()
          .map(|ref_| ref_.value().clone())
          .collect_vec(),
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

    progress_handler.handle(
      &[progress.clone()],
      &self
        .hit_counter
        .iter()
        .map(|ref_| ref_.value().clone())
        .collect_vec(),
      interval,
      if let Progress(ProgressKind::Processing(ProcessingDetail {
        current,
        ..
      })) = progress
      {
        self.current_diff.update(current)
      } else {
        0
      },
      all_done,
    )?;

    self.start_time = Instant::now();

    Ok(())
  }
}
