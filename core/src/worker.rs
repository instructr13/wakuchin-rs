//! Wakuchin researcher main functions

use std::cell::RefCell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::available_parallelism;
use std::time::Duration;

use anyhow::anyhow;
use divide_range::RangeDivisions;
use itertools::Itertools;
use parking_lot::Mutex;
use regex::Regex;
use tokio::runtime::{Builder, Runtime};
use tokio::signal;
use tokio::task::JoinSet;

use crate::channel::{channel, oneshot, watch};
use crate::error::WakuchinError;
use crate::handlers::ProgressHandler;
use crate::hit::counter::ThreadHitCounter;
use crate::progress::{
  DoneDetail, IdleDetail, ProcessingDetail, Progress, ProgressKind,
};
use crate::render::{Render, ThreadRender};
use crate::result::{Hit, WakuchinResult};
use crate::{check, gen};

type Result<T> = std::result::Result<T, WakuchinError>;

fn get_total_workers(workers: usize) -> Result<usize> {
  if workers != 0 {
    return Ok(workers);
  }

  available_parallelism()
    .map(|num| num.into())
    .map_err(|e| e.into())
}

fn create_runtime(workers: usize) -> Result<Runtime> {
  Builder::new_multi_thread()
    .worker_threads(workers + 2)
    .thread_name_fn(|| {
      static ATOMIC_ID: AtomicUsize = AtomicUsize::new(0);

      let id = ATOMIC_ID.fetch_add(1, Ordering::SeqCst);

      format!("wakuchin-worker-{}", id)
    })
    .thread_stack_size(4 * 1024 * 1024)
    .enable_time()
    .build()
    .map_err(|e| e.into())
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
///   use std::sync::{Arc, Mutex};
///   use std::time::Duration;
///
///   use regex::Regex;
///
///   use wakuchin::handlers::ProgressHandler;
///   use wakuchin::handlers::empty::EmptyProgressHandler;
///   use wakuchin::worker::run_par;
///
///   # async fn try_main_async() -> Result<(), Box<dyn std::error::Error>> {
///   let handler: Arc<Mutex<Box<dyn ProgressHandler>>> = Arc::new(Mutex::new(Box::new(EmptyProgressHandler::new())));
///   let result = run_par(10, 0, Regex::new(r"WKCN")?, handler, Duration::from_secs(1), 0).await;
///
///   assert!(result.is_err());
///   assert_eq!(result.err().unwrap().to_string(), "times cannot be zero");
///   #
///   #   Ok(())
///   # }
///   #
///   # #[tokio::main]
///   # async fn main() {
///   #   try_main_async().await.unwrap();
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
/// # async fn try_main_async() -> Result<(), Box<dyn std::error::Error>> {
/// let tries = 10;
/// let handler: Arc<Mutex<Box<dyn ProgressHandler>>>
///   = Arc::new(Mutex::new(Box::new(MsgpackProgressHandler::new(tries, Arc::new(Mutex::new(stdout()))))));
/// let result = run_par(tries, 1, Regex::new(r"WKCN")?, handler, Duration::from_secs(1), 4).await?;
///
/// println!("{}", result.out(ResultOutputFormat::Text)?);
/// #
/// #   Ok(())
/// # }
/// #
/// # #[tokio::main]
/// # async fn main() {
/// #   try_main_async().await.unwrap();
/// # }
/// ```
pub async fn run_par(
  tries: usize,
  times: usize,
  regex: Regex,
  progress_handler: Mutex<Box<dyn ProgressHandler>>,
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

  let (accidential_stop_tx, accidential_stop_rx) = watch(false);
  let (hit_tx, hit_rx) = channel();
  let counter = ThreadHitCounter::new(hit_rx);

  let (progress_tx_vec, progress_rx_vec): (Vec<_>, Vec<_>) = (0..total_workers)
    .map(|id| {
      watch(Progress(ProgressKind::Idle(IdleDetail {
        id: id + 1,
        total_workers,
      })))
    })
    .unzip();

  let render = ThreadRender::new(
    accidential_stop_rx.clone(),
    counter.clone(),
    progress_rx_vec,
    progress_handler,
    tries,
    total_workers,
  );

  render.invoke_before_start()?;

  let runtime = create_runtime(total_workers)?;

  let runtime_handle = runtime.handle();

  let mut handles_to_abort = Vec::with_capacity(workers + 2);

  let mut hit_join_set = JoinSet::new();
  let mut ui_join_set: JoinSet<Result<()>> = JoinSet::new();

  // hit handler
  handles_to_abort.push(counter.run(&mut hit_join_set, runtime_handle));

  // progress reporter
  handles_to_abort.push(ui_join_set.spawn_on(
    {
      async move {
        render.run(progress_interval).await?;

        render.invoke_after_finish()?;

        Ok(())
      }
    },
    runtime_handle,
  ));

  // set SIGINT/SIGTERM handler
  #[cfg(not(target_arch = "wasm32"))]
  let mut signal_join_set = JoinSet::new();

  #[cfg(not(target_arch = "wasm32"))]
  signal_join_set.spawn(async move {
    signal::ctrl_c().await.unwrap();

    accidential_stop_tx.send(true).unwrap();
  });

  let mut worker_join_set = JoinSet::new();

  (0..tries)
    .divide_evenly_into(total_workers)
    .zip(progress_tx_vec.into_iter())
    .enumerate()
    .for_each(|(id, (wakuchins, progress_tx))| {
      let accidential_stop_rx = accidential_stop_rx.clone();
      let hit_tx = hit_tx.clone();
      let regex = regex.clone();
      let total = wakuchins.len();

      handles_to_abort.push(worker_join_set.spawn_on(
        async move {
          let mut hits = Vec::new();

          for (i, wakuchin) in wakuchins.map(|_| gen(times)).enumerate() {
            if check(&wakuchin, &regex) {
              let hit = Hit::new(i, &wakuchin);

              hit_tx
                .send_async(hit.clone())
                .await
                .expect("hit channel is unavailable");

              hits.push(hit);
            }

            let progress_tx_result = progress_tx.send(Progress(
              ProgressKind::Processing(ProcessingDetail::new(
                id + 1,
                wakuchin,
                i,
                total,
                total_workers,
              )),
            ));

            if *accidential_stop_rx.borrow() && progress_tx_result.is_err() {
              break;
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

          hits
        },
        runtime_handle,
      ));
    });

  drop(hit_tx);

  signal_join_set.spawn({
    let mut accidential_stop_rx = accidential_stop_rx.clone();

    async move {
      accidential_stop_rx.changed().await.unwrap();

      for handle in handles_to_abort {
        handle.abort();
      }
    }
  });

  let mut hits_detail = Vec::new();

  while let Some(hits) = worker_join_set.join_next().await {
    match hits {
      Ok(hits) => {
        for hit in hits.into_iter() {
          hits_detail.push(hit);
        }
      }
      Err(e) => {
        runtime.shutdown_background();

        if e.is_cancelled() {
          return Err(WakuchinError::Cancelled);
        }

        return Err(e.into());
      }
    }
  }

  while let Some(result) = hit_join_set.join_next().await {
    if let Err(e) = result {
      runtime.shutdown_background();

      if e.is_cancelled() {
        return Err(WakuchinError::Cancelled);
      }

      return Err(e.into());
    }
  }

  // after all workers have finished, wait for ui threads to finish
  while let Some(result) = ui_join_set.join_next().await {
    if let Err(e) = result {
      runtime.shutdown_background();

      if e.is_cancelled() {
        return Err(WakuchinError::Cancelled);
      }

      return Err(e.into());
    }

    result??; // JoinError<Error<T>>
  }

  signal_join_set.abort_all();

  while signal_join_set.join_next().await.is_some() {}

  // cleanup
  runtime.shutdown_background();

  let hits = counter.get_all().into_hit_counts();
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
///   use std::cell::RefCell;
///   use std::rc::Rc;
///   use std::time::Duration;
///
///   use regex::Regex;
///
///   use wakuchin::handlers::ProgressHandler;
///   use wakuchin::handlers::empty::EmptyProgressHandler;
///   use wakuchin::worker::run_seq;
///
///   let handler: Rc<RefCell<dyn ProgressHandler>> = Rc::new(RefCell::new(EmptyProgressHandler::new()));
///   let result = run_seq(10, 0, Regex::new(r"WKCN")?, handler, Duration::from_secs(1));
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
/// use std::cell::RefCell;
/// use std::io::stdout;
/// use std::rc::Rc;
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
/// let handler: Rc<RefCell<dyn ProgressHandler>>
///   = Rc::new(RefCell::new(MsgpackProgressHandler::new(tries, Arc::new(Mutex::new(stdout())))));
///
/// let result = run_seq(tries, 1, Regex::new(r"WKCN")?, handler, Duration::from_secs(1))?;
///
/// println!("{}", result.out(ResultOutputFormat::Text)?);
/// #
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn run_seq(
  tries: usize,
  times: usize,
  regex: Regex,
  progress_handler: RefCell<Box<dyn ProgressHandler>>,
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

  let (accidential_stop_tx, accidential_stop_rx) = oneshot::<()>();

  #[cfg(not(target_arch = "wasm32"))]
  ctrlc::set_handler(move || {
    accidential_stop_tx.send(()).unwrap();
  })
  .map_err(|e| anyhow!(e))?;

  render.render_progress(
    progress_interval,
    Progress(ProgressKind::Idle(IdleDetail {
      id: 0,
      total_workers: 1,
    })),
    false,
  )?;

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

      if accidential_stop_rx.try_recv().is_ok() {
        render.invoke_on_accidential_stop()?;

        return Err(WakuchinError::Cancelled);
      }

      if check(&wakuchin, &regex) {
        let hit = Hit::new(i, &wakuchin);

        render.handle_hit(wakuchin);

        Ok(Some(hit))
      } else {
        Ok(None)
      }
    })
    .filter_map_ok(|hit| hit)
    .collect::<Result<Vec<_>>>()?;

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
