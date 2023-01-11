//! Wakuchin researcher main functions

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, available_parallelism};
use std::time::Duration;

use divide_range::RangeDivisions;
use regex::Regex;

use crate::channel::{channel, watch};
use crate::error::WakuchinError;
use crate::handlers::ProgressHandler;
use crate::hit::counter::ThreadHitCounter;
use crate::progress::{
  DoneDetail, IdleDetail, ProcessingDetail, Progress, ProgressKind,
};
use crate::render::{Render, ThreadRender};
use crate::result::{Hit, HitCount, WakuchinResult};
use crate::{check, gen};

type Result<T> = std::result::Result<T, WakuchinError>;

fn get_total_workers(workers: usize) -> Result<usize> {
  if workers != 0 {
    return Ok(workers);
  }

  available_parallelism().map(Into::into).map_err(Into::into)
}

/// Research wakuchin with parallelism.
///
/// # Arguments
///
/// * `tries` - number of tries
///   If you passed zero, this function do nothing and return immediately with an empty `WakuchinResult`.
/// * `times` - wakuchin times n, cannot be zero
/// * `regex` - compiled regular expression to detect hit
/// * `progress_handler` - handler function to handle progress
/// * `progress_interval` - progress refresh interval
/// * `workers` - number of workers you want to use, default to number of logical cores
///
/// # Returns
///
/// * `Result<WakuchinResult, WakuchinError>` - the result of the research (see [`WakuchinResult`])
///
/// # Errors
///
/// * [`WakuchinError::TimesIsZero`](crate::error::WakuchinError::TimesIsZero) - Returns if you passed a zero to `times`
///   ```rust
///   use std::sync::Mutex;
///   use std::time::Duration;
///
///   use regex::Regex;
///
///   use wakuchin::handlers::ProgressHandler;
///   use wakuchin::handlers::empty::EmptyProgressHandler;
///   use wakuchin::worker::run_par;
///
///   # fn main() -> Result<(), Box<dyn std::error::Error>> {
///   let handler: Box<dyn ProgressHandler> = Box::new(EmptyProgressHandler::new());
///   let result = run_par(10, 0, &Regex::new(r"WKCN")?, handler, Duration::from_secs(1), 0);
///
///   assert!(result.is_err());
///   assert_eq!(result.err().unwrap().to_string(), "times cannot be zero");
///   #
///   #   Ok(())
///   # }
///   ```
/// * [`WakuchinError::WorkerError`](crate::error::WakuchinError::WorkerError) - Returns when any worker raised an error
///
/// # Examples
///
/// ```rust
/// use std::io::stdout;
/// use std::sync::{Arc, Mutex};
/// use std::time::Duration;
///
/// use regex::Regex;
///
/// use wakuchin::handlers::ProgressHandler;
/// use wakuchin::handlers::msgpack::MsgpackProgressHandler;
/// use wakuchin::result::{out, ResultOutputFormat};
/// use wakuchin::worker::run_par;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let tries = 10;
/// let handler: Box<dyn ProgressHandler>
///   = Box::new(MsgpackProgressHandler::new(tries, Arc::new(Mutex::new(stdout()))));
/// let result = run_par(tries, 1, &Regex::new(r"WKCN")?, handler, Duration::from_secs(1), 4)?;
///
/// println!("{}", result.out(ResultOutputFormat::Text)?);
/// #
/// #   Ok(())
/// # }
/// ```
pub fn run_par(
  tries: usize,
  times: usize,
  regex: &Regex,
  progress_handler: Box<dyn ProgressHandler>,
  progress_interval: Duration,
  workers: usize,
) -> Result<WakuchinResult> {
  if tries == 0 {
    return Ok(WakuchinResult {
      tries: 0,
      hits_total: 0,
      hits: Vec::new(),
      hits_detail: Vec::new(),
    });
  }

  if times == 0 {
    return Err(WakuchinError::TimesIsZero);
  }

  let total_workers = get_total_workers(workers)?;

  let is_stopped_accidentially = Arc::new(AtomicBool::new(false));
  let (hit_tx, hit_rx) = channel();

  let (progress_tx_vec, progress_rx_vec): (Vec<_>, Vec<_>) = (0..total_workers)
    .map(|id| {
      watch(Progress(ProgressKind::Idle(IdleDetail {
        id: id + 1,
        total_workers,
      })))
    })
    .unzip();

  // set SIGINT/SIGTERM handler
  #[cfg(not(target_arch = "wasm32"))]
  ctrlc::set_handler({
    let is_stopped_accidentially = is_stopped_accidentially.clone();

    move || {
      is_stopped_accidentially.store(true, Ordering::SeqCst);
    }
  })
  .unwrap_or(());

  let mut hits_detail = Vec::new();

  let counter = ThreadHitCounter::new(hit_rx);

  let mut render = ThreadRender::new(
    is_stopped_accidentially.clone(),
    counter.clone(),
    progress_rx_vec,
    progress_handler,
    tries,
    total_workers,
  );

  let hits = thread::scope::<_, Result<Vec<HitCount>>>(|s| {
    // hit handler
    let hit_handle = s.spawn(|| counter.run());

    // progress reporter
    let ui_handle = s.spawn::<_, Result<()>>(|| {
      render.run(progress_interval)?;

      Ok(())
    });

    let mut worker_handles = Vec::with_capacity(workers);

    (0..tries)
      .divide_evenly_into(total_workers)
      .zip(progress_tx_vec.into_iter())
      .enumerate()
      .for_each(|(id, (wakuchins, progress_tx))| {
        let is_stopped_accidentially = is_stopped_accidentially.clone();
        let regex = regex.clone();
        let hit_tx = hit_tx.clone();

        worker_handles.push(s.spawn(move || {
          let total = wakuchins.len();

          let mut hits = Vec::new();

          for (i, wakuchin) in wakuchins.map(|_| gen(times)).enumerate() {
            if check(&wakuchin, &regex) {
              let hit = Hit::new(i, &*wakuchin);

              hit_tx
                .send(hit.clone())
                .expect("hit channel is unavailable");

              hits.push(hit);
            }

            if !progress_tx.is_closed() {
              progress_tx
                .send(Progress(ProgressKind::Processing(
                  ProcessingDetail::new(
                    id + 1,
                    wakuchin,
                    i,
                    total,
                    total_workers,
                  ),
                )))
                .expect("progress channel is unavailable");
            }

            if is_stopped_accidentially.load(Ordering::Relaxed) {
              drop(hit_tx);

              return Err(WakuchinError::Cancelled);
            }
          }

          drop(hit_tx);

          if !progress_tx.is_closed() {
            progress_tx
              .send(Progress(ProgressKind::Done(DoneDetail {
                id: id + 1,
                total,
                total_workers,
              })))
              .unwrap();
          }

          Ok(hits)
        }));
      });

    for worker_handle in worker_handles {
      for hit in worker_handle.join().unwrap()?.into_iter() {
        hits_detail.push(hit);
      }
    }

    // cleanup
    drop(hit_tx);

    // after all workers have finished, wait for ui and hit threads to finish
    hit_handle.join().unwrap();
    ui_handle.join().unwrap()?;

    Ok(counter.get_all().into_hit_counts())
  })?;

  let hits_total = hits.iter().map(|c| c.hits).sum::<usize>();

  Ok(WakuchinResult {
    tries,
    hits_total,
    hits,
    hits_detail,
  })
}

/// Research wakuchin with sequential.
/// This function is useful when you don't use multi-core processors.
///
/// # Arguments
///
/// * `tries` - number of tries
///   If you passed zero, this function do nothing and return immediately with an empty `WakuchinResult`.
/// * `times` - wakuchin times n, cannot be zero
/// * `regex` - compiled regular expression to detect hit
/// * `progress_handler` - handler function to handle progress
/// * `progress_interval` - progress refresh interval
///
/// # Returns
///
/// * `Result<WakuchinResult, WakuchinError>` - the result of the research (see [`WakuchinResult`])
///
/// # Errors
///
/// * [`WakuchinError::TimesIsZero`](crate::error::WakuchinError::TimesIsZero) - Returns if you passed a zero to `times`
///   ```rust
///   use std::time::Duration;
///
///   use regex::Regex;
///
///   use wakuchin::handlers::ProgressHandler;
///   use wakuchin::handlers::empty::EmptyProgressHandler;
///   use wakuchin::worker::run_seq;
///
///   let handler: Box<dyn ProgressHandler> = Box::new(EmptyProgressHandler::new());
///   let result = run_seq(10, 0, &Regex::new(r"WKCN")?, handler, Duration::from_secs(1));
///
///   assert!(result.is_err());
///   assert_eq!(result.err().unwrap().to_string(), "times cannot be zero");
///   #
///   # Ok::<(), Box<dyn std::error::Error>>(())
///   ```
///
/// # Examples
///
/// ```rust
/// use std::io::stdout;
/// use std::sync::{Arc, Mutex};
/// use std::time::Duration;
///
/// use regex::Regex;
///
/// use wakuchin::handlers::ProgressHandler;
/// use wakuchin::handlers::msgpack::MsgpackProgressHandler;
/// use wakuchin::result::{out, ResultOutputFormat};
/// use wakuchin::worker::run_seq;
///
/// let tries = 10;
///
/// let handler: Box<dyn ProgressHandler>
///   = Box::new(MsgpackProgressHandler::new(tries, Arc::new(Mutex::new(stdout()))));
///
/// let result = run_seq(tries, 1, &Regex::new(r"WKCN")?, handler, Duration::from_secs(1))?;
///
/// println!("{}", result.out(ResultOutputFormat::Text)?);
/// #
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn run_seq(
  tries: usize,
  times: usize,
  regex: &Regex,
  progress_handler: Box<dyn ProgressHandler>,
  progress_interval: Duration,
) -> Result<WakuchinResult> {
  if tries == 0 {
    return Ok(WakuchinResult {
      tries: 0,
      hits_total: 0,
      hits: Vec::new(),
      hits_detail: Vec::new(),
    });
  }

  if times == 0 {
    return Err(WakuchinError::TimesIsZero);
  }

  let mut render = Render::new(progress_handler);

  render.invoke_before_start()?;

  let is_accidentially_stopped = Arc::new(AtomicBool::new(false));

  #[cfg(not(target_arch = "wasm32"))]
  ctrlc::set_handler({
    let is_accidentially_stopped = is_accidentially_stopped.clone();

    move || {
      is_accidentially_stopped.store(true, Ordering::SeqCst);
    }
  })
  .map_err(|e| anyhow::anyhow!(e))?;

  render.render_progress(
    progress_interval,
    Progress(ProgressKind::Idle(IdleDetail {
      id: 0,
      total_workers: 1,
    })),
    false,
  )?;

  let mut hits_detail_err = Ok(());

  let hits_detail = (0..tries)
    .map(|_| gen(times))
    .enumerate()
    .map(|(i, wakuchin)| {
      render.render_progress(
        progress_interval,
        Progress(ProgressKind::Processing(ProcessingDetail::new(
          0,
          wakuchin.clone(),
          i,
          tries,
          1,
        ))),
        false,
      )?;

      if is_accidentially_stopped.load(Ordering::SeqCst) {
        return Err(WakuchinError::Cancelled);
      }

      if check(&wakuchin, regex) {
        let hit = Hit::new(i, &*wakuchin);

        render.handle_hit(wakuchin);

        Ok(Some(hit))
      } else {
        Ok(None)
      }
    })
    .scan(
      &mut hits_detail_err,
      |hits_detail_err, result| match result {
        Ok(result) => Some(result),
        Err(err) => {
          **hits_detail_err = Err(err);

          None
        }
      },
    )
    .flatten()
    .collect();

  if matches!(hits_detail_err, Err(WakuchinError::Cancelled)) {
    render.invoke_on_accidential_stop()?;

    return Err(WakuchinError::Cancelled);
  }

  render.render_progress(
    Duration::ZERO,
    Progress(ProgressKind::Done(DoneDetail {
      id: 0,
      total: tries,
      total_workers: 1,
    })),
    true,
  )?;

  render.invoke_after_finish()?;

  let hits = render.hits();
  let hits_total = hits.iter().map(|c| c.hits).sum::<usize>();

  Ok(WakuchinResult {
    tries,
    hits_total,
    hits,
    hits_detail,
  })
}
