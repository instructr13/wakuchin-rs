//! Wakuchin researcher main functions

use std::panic::resume_unwind;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{available_parallelism, scope};
use std::time::Duration;

use divide_range::RangeDivisions;
use flume::bounded;
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

#[cfg(not(target_arch = "wasm32"))]
use signal_hook::consts::SIGINT;

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
    .map(|id| watch(Progress(ProgressKind::Idle(IdleDetail { id: id + 1 }))))
    .unzip();

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

  // used internally to prevent 'static lifetime issues
  #[cfg(not(target_arch = "wasm32"))]
  let (internal_stop_tx, internal_stop_rx) = bounded(1);

  let hits = scope::<_, Result<Vec<HitCount>>>(|s| {
    // signal handler
    #[cfg(not(target_arch = "wasm32"))]
    let signal_id = unsafe {
      signal_hook_registry::register(SIGINT, move || {
        internal_stop_tx.send(()).unwrap();
      })
    }?;

    let is_stopped_accidentially = is_stopped_accidentially.as_ref();

    #[cfg(not(target_arch = "wasm32"))]
    let signal_handle = s.spawn(|| loop {
      if counter.count_stopped.load(Ordering::Acquire) {
        return;
      }

      if internal_stop_rx.is_full() {
        is_stopped_accidentially.store(true, Ordering::SeqCst);

        return;
      }

      std::hint::spin_loop();
    });

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
        let regex = regex.clone();
        let hit_tx = hit_tx.clone();

        worker_handles.push(s.spawn(move || {
          let total = wakuchins.len();

          let mut hits = Vec::new();

          for (current, (i, wakuchin)) in
            wakuchins.map(|i| (i, gen(times))).enumerate()
          {
            if is_stopped_accidentially.load(Ordering::Relaxed) {
              drop(hit_tx);

              return Err(WakuchinError::Cancelled);
            }

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
                  ProcessingDetail::new(id + 1, wakuchin, current, total),
                )))
                .expect("progress channel is unavailable");
            }
          }

          drop(hit_tx);

          if !progress_tx.is_closed() {
            progress_tx
              .send(Progress(ProgressKind::Done(DoneDetail {
                id: id + 1,
                total,
              })))
              .unwrap();
          }

          Ok(hits)
        }));
      });

    for worker_handle in worker_handles {
      for hit in worker_handle
        .join()
        .unwrap_or_else(|e| resume_unwind(e))?
        .into_iter()
      {
        hits_detail.push(hit);
      }
    }

    // cleanup
    drop(hit_tx);

    // after all workers have finished, wait for ui and hit threads to finish
    hit_handle.join().unwrap_or_else(|e| resume_unwind(e));
    ui_handle.join().unwrap_or_else(|e| resume_unwind(e))?;

    #[cfg(not(target_arch = "wasm32"))]
    {
      signal_handle.join().unwrap_or_else(|e| resume_unwind(e));
      signal_hook_registry::unregister(signal_id);
    }

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

  let is_stopped_accidentially = AtomicBool::new(false);

  // used internally to prevent 'static lifetime issues
  #[cfg(not(target_arch = "wasm32"))]
  let (internal_stop_tx, internal_stop_rx) = bounded(1);

  let (hits_detail, hits) = scope(|s| {
    let is_stopped_accidentially = &is_stopped_accidentially;

    #[cfg(not(target_arch = "wasm32"))]
    let signal_id = unsafe {
      signal_hook_registry::register(SIGINT, move || {
        internal_stop_tx.send(()).unwrap();
      })
    }?;

    #[cfg(not(target_arch = "wasm32"))]
    let signal_handle = s.spawn(|| loop {
      if is_stopped_accidentially.load(Ordering::SeqCst) {
        return;
      }

      if internal_stop_rx.is_full() {
        is_stopped_accidentially.store(true, Ordering::SeqCst);

        return;
      }

      std::hint::spin_loop();
    });

    let mut render = Render::new(progress_handler);

    render.invoke_before_start()?;

    render.render_progress(
      progress_interval,
      Progress(ProgressKind::Idle(IdleDetail { id: 0 })),
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
          ))),
          false,
        )?;

        if is_stopped_accidentially.load(Ordering::SeqCst) {
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

    // cleanup
    is_stopped_accidentially.store(true, Ordering::SeqCst);

    #[cfg(not(target_arch = "wasm32"))]
    {
      signal_handle.join().unwrap_or_else(|e| resume_unwind(e));
      signal_hook_registry::unregister(signal_id);
    }

    render.render_progress(
      Duration::ZERO,
      Progress(ProgressKind::Done(DoneDetail {
        id: 0,
        total: tries,
      })),
      true,
    )?;

    render.invoke_after_finish()?;

    Ok((hits_detail, render.hits()))
  })?;

  let hits_total = hits.iter().map(|c| c.hits).sum::<usize>();

  Ok(WakuchinResult {
    tries,
    hits_total,
    hits,
    hits_detail,
  })
}
