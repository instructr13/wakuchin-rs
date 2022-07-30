//! Wakuchin researcher main functions

mod error;

pub use crate::worker::error::Error;

use std::time::Duration;

use divide_range::RangeDivisions;
use flume::unbounded as channel;
use itertools::Itertools;
use regex::Regex;
use tokio::sync::watch::channel as progress_channel;

use crate::{
  check,
  error::Error as NormalError,
  gen,
  progress::{
    DoneDetail, HitCounter, ProcessingDetail, Progress, ProgressKind,
  },
  render::{Render, ThreadRender},
  result::{Hit, WakuchinResult},
};

/// Research wakuchin with parallelism.
///
/// * `tries` - number of tries
/// * `times` - wakuchin times n
/// * `regex` - regular expression to detect hit
/// * `progress_handler` - handler function to handle progress
///   * `progress` - progress information of all workers
///   * `hit_counter` - hit counter, see `HitCounter`
///   * `interval` - interval of progress refresh you were specified, usable for calculation of speed
///   * `current_diff` - `current - previous` of progress, usable for calculation of speed
///   * `all_done` - if all workers are done, usable for finalization
/// * `workers` - number of workers you want to use, default to number of logical cores
///
/// # Returns
///
/// * `WakuchinResult` - the result of the research
///
/// # Examples
///
/// ```rust
/// use std::time::Duration;
///
/// use regex::Regex;
///
/// use wakuchin::worker::run_par;
///
/// let result = run_par(10, 1, Regex::new(r"WKCN").unwrap(), |_, counters, _, _, _| {
///   println!("total hits: {}", counters.iter().map(|c| c.hits).sum::<usize>());
/// }, Duration::from_secs(1), Some(4));
/// ```
pub async fn run_par<F>(
  tries: usize,
  times: usize,
  regex: Regex,
  progress_handler: F,
  interval: Duration,
  workers: Option<usize>,
) -> anyhow::Result<WakuchinResult, NormalError>
where
  F: Fn(&[Progress], &[HitCounter], Duration, usize, bool)
    + Copy
    + Send
    + Sync
    + 'static,
{
  if tries == 0 {
    return Ok(WakuchinResult {
      tries: 0,
      hits_n: 0,
      hits: Vec::new(),
    });
  }

  if times == 0 {
    return Err(NormalError::TimesIsZero);
  }

  let workers = {
    let workers = workers.unwrap_or_else(num_cpus::get);

    if workers > 5 {
      workers - 2 // to work progress render thread and hit notifier thread
    } else {
      workers
    }
  };

  if tries < workers {
    return Err(NormalError::WorkerError(Error::InsufficientWorkers(
      workers,
    )));
  }

  let (hit_tx, hit_rx) = channel();

  let (progress_tx_vec, progress_rx_vec): (Vec<_>, Vec<_>) = (0..workers)
    .map(|id| progress_channel(Progress(ProgressKind::Idle(id + 1, workers))))
    .unzip();

  let render = ThreadRender::new(hit_rx, progress_rx_vec, progress_handler);

  let render_handle = tokio::spawn(async move { render.start(interval).await });

  let handles = (0..tries)
    .divide_evenly_into(workers)
    .zip(progress_tx_vec.into_iter())
    .enumerate()
    .map(|(id, (wakuchins, progress_tx))| {
      let hit_tx = hit_tx.clone();
      let regex = regex.clone();
      let total = wakuchins.len();

      tokio::spawn(async move {
        let mut hits = Vec::new();

        for (i, wakuchin) in wakuchins.map(|_| gen(times)).enumerate() {
          progress_tx
            .send(Progress(ProgressKind::Processing(ProcessingDetail::new(
              id + 1,
              &wakuchin,
              i,
              total,
              workers,
            ))))
            .expect("progress channel is unavailable");

          if check(&wakuchin, &regex) {
            let hit = Hit::new(i, &wakuchin);

            hit_tx
              .send_async(hit.clone())
              .await
              .expect("hit channel is unavailable");

            hits.push(hit);
          }
        }

        drop(hit_tx);

        progress_tx
          .send(Progress(ProgressKind::Done(DoneDetail {
            id: id + 1,
            total,
            total_workers: workers,
          })))
          .expect("progress channel is unavailable");

        hits
      })
    })
    .collect_vec();

  drop(hit_tx);

  render_handle.await.unwrap().unwrap();

  let mut hits = Vec::new();

  for handle in handles {
    hits.push(handle.await.unwrap());
  }

  let hits = hits.into_iter().flatten().collect_vec();

  Ok(WakuchinResult {
    tries,
    hits_n: hits.len(),
    hits,
  })
}

/// Research wakuchin with sequential.
/// This function is useful when you don't use multi-core processors.
///
/// # Arguments
///
/// * `tries` - number of tries
/// * `times` - wakuchin times n
/// * `regex` - regular expression to detect hit
/// * `progress_handler` - handler function to handle progress
///   * `progress` - progress information of all workers
///   * `hit_counter` - hit counter, see `HitCounter`
///   * `interval` - interval of progress refresh you were specified, usable for calculation of speed
///   * `current_diff` - `current - previous` of progress, usable for calculation of speed
///   * `all_done` - if all workers are done, usable for finalization
///
/// # Returns
///
/// * `WakuchinResult` - the result of the research
///
/// # Examples
///
/// ```rust
/// use std::time::Duration;
///
/// use regex::Regex;
///
/// use wakuchin::worker::run_seq;
///
/// let result = run_seq(10, 1, Regex::new(r"WKCN").unwrap(), |_, counters, _, _, _| {
///   println!("total hits: {}", counters.iter().map(|c| c.hits).sum::<usize>());
/// }, Duration::from_secs(1));
/// ```
pub fn run_seq<F>(
  tries: usize,
  times: usize,
  regex: Regex,
  progress_handler: F,
  interval: Duration,
) -> anyhow::Result<WakuchinResult, NormalError>
where
  F: Fn(&[Progress], &[HitCounter], Duration, usize, bool),
{
  if tries == 0 {
    return Ok(WakuchinResult {
      tries: 0,
      hits_n: 0,
      hits: Vec::new(),
    });
  }

  if times == 0 {
    return Err(NormalError::TimesIsZero);
  }

  let mut render = Render::new(progress_handler);

  render.render_progress(interval, Progress(ProgressKind::Idle(0, 1)), false);

  let hits = (0..tries)
    .map(|_| gen(times))
    .enumerate()
    .map(|(i, wakuchin)| {
      render.render_progress(
        interval,
        Progress(ProgressKind::Processing(ProcessingDetail::new(
          0, &wakuchin, i, tries, 1,
        ))),
        false,
      );

      if check(&wakuchin, &regex) {
        let hit = Hit::new(i, &wakuchin);

        render.handle_hit(&hit);

        Some(hit)
      } else {
        None
      }
    })
    .filter(|hit| hit.is_some())
    .collect::<Option<Vec<_>>>()
    .expect("hits filtering failed");

  render.render_progress(
    Duration::ZERO,
    Progress(ProgressKind::Done(DoneDetail {
      id: 0,
      total: tries,
      total_workers: 1,
    })),
    true,
  );

  Ok(WakuchinResult {
    tries,
    hits_n: hits.len(),
    hits,
  })
}
