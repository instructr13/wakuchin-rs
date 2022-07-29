use std::sync::Arc;
use std::time::Instant;

use dashmap::DashMap;
use flume::{
  bounded as channel, unbounded as channel_unbounded, Receiver, Sender,
  TryRecvError,
};
use itertools::Itertools;
use tokio::sync::watch;
use tokio::task::JoinError;
use tokio::time::{sleep, Duration};

use crate::progress::{HitCounter, ProcessingDetail, Progress, ProgressKind};
use crate::result::Hit;

struct ThreadRenderInner {
  hit_rx: Receiver<Hit>,
  internal_hit_tx: Sender<Hit>,
  stop_tx: Sender<bool>,
}

pub(crate) struct ThreadRender<F>
where
  F: Fn(&[Progress], &[HitCounter], bool) + Copy + Send + Sync + 'static,
{
  hit_counter: DashMap<String, HitCounter>,
  inner: Arc<ThreadRenderInner>,
  internal_hit_rx: Receiver<Hit>,
  progress_channels: Vec<watch::Receiver<Progress>>,
  progress_handler: Arc<F>,
  stop_rx: Receiver<bool>,
}

impl ThreadRenderInner {
  pub(crate) async fn wait_for_hit(&self) {
    loop {
      let hit = self.hit_rx.try_recv();

      match hit {
        Ok(hit) => {
          self.internal_hit_tx.send_async(hit).await.unwrap();
        }
        Err(TryRecvError::Disconnected) => {
          self.stop_tx.send_async(true).await.unwrap();

          break;
        }
        Err(TryRecvError::Empty) => {
          sleep(Duration::from_millis(5)).await;
        }
      }
    }
  }
}

impl<F> ThreadRender<F>
where
  F: Fn(&[Progress], &[HitCounter], bool) + Copy + Send + Sync + 'static,
{
  pub(crate) fn new(
    hit_rx: Receiver<Hit>,
    progress_channels: Vec<watch::Receiver<Progress>>,
    progress_handler: F,
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
      progress_handler: Arc::new(progress_handler),
      stop_rx,
    }
  }

  pub(crate) async fn start_render_progress(&self, interval: Duration) {
    let progress_handler = self.progress_handler.clone();
    let mut start_time = Instant::now();
    let mut workers = None;

    loop {
      if self.stop_rx.try_recv().is_ok() {
        progress_handler(
          &(0..self.progress_channels.len())
            .map(|id| Progress(ProgressKind::Done(id, workers.unwrap_or(1))))
            .collect_vec(),
          &self
            .hit_counter
            .iter()
            .map(|ref_| ref_.value().clone())
            .collect_vec(),
          true,
        );

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

      progress_handler(
        &self
          .progress_channels
          .iter()
          .map(|rx| {
            let progress = rx.borrow().clone();

            if workers.is_none() {
              match progress {
                Progress(ProgressKind::Idle(_, total)) => {
                  workers = Some(total);
                }
                Progress(ProgressKind::Processing(ProcessingDetail {
                  total_workers,
                  ..
                })) => {
                  workers = Some(total_workers);
                }
                _ => {}
              }
            }

            progress
          })
          .collect_vec(),
        &self
          .hit_counter
          .iter()
          .map(|ref_| ref_.value().clone())
          .collect_vec(),
        false,
      );

      start_time = Instant::now();
    }
  }

  pub(crate) async fn start(self) -> Result<(), JoinError> {
    let inner = self.inner.clone();

    let hit_handle =
      tokio::task::Builder::new()
        .name("Hit Receiver")
        .spawn(async move {
          inner.wait_for_hit().await;
        });

    let progress_handle = tokio::task::Builder::new()
      .name("Progress Notifier")
      .spawn(async move {
        self.start_render_progress(Duration::from_millis(300)).await;
      });

    for handle in vec![progress_handle, hit_handle] {
      handle.await.unwrap();
    }

    Ok(())
  }
}

pub(crate) struct Render<F>
where
  F: Fn(&[Progress], &[HitCounter], bool),
{
  hit_counter: DashMap<String, HitCounter>,
  progress_handler: F,
  start_time: Instant,
}

impl<F> Render<F>
where
  F: Fn(&[Progress], &[HitCounter], bool),
{
  pub(crate) fn new(progress_handler: F) -> Self {
    Self {
      hit_counter: DashMap::new(),
      progress_handler,
      start_time: Instant::now(),
    }
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
  ) {
    if interval.is_zero() {
      (self.progress_handler)(
        &[progress],
        &self
          .hit_counter
          .iter()
          .map(|ref_| ref_.value().clone())
          .collect_vec(),
        all_done,
      );

      return;
    }

    if self.start_time.elapsed() <= interval {
      return;
    }

    if matches!(progress, Progress(ProgressKind::Done(_, _))) {
      return;
    }

    (self.progress_handler)(
      &[progress],
      &self
        .hit_counter
        .iter()
        .map(|ref_| ref_.value().clone())
        .collect_vec(),
      all_done,
    );

    self.start_time = Instant::now();
  }
}
