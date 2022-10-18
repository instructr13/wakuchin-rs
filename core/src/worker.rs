//! Wakuchin researcher main functions

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use divide_range::RangeDivisions;
use flume::unbounded as channel;
use itertools::Itertools;
use regex::Regex;
use tokio::runtime::Builder;
use tokio::sync::{watch::channel as progress_channel, RwLock};
use tokio::task::JoinSet;

use crate::error::WakuchinError;
use crate::handlers::ProgressHandler;
use crate::progress::{
  DoneDetail, IdleDetail, ProcessingDetail, Progress, ProgressKind,
};
use crate::render::{Render, ThreadRender};
use crate::result::{Hit, WakuchinResult};
use crate::{check, gen};

type Result<T> = std::result::Result<T, WakuchinError>;

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
/// * [`Error::TimesIsZero`](crate::error::Error::TimesIsZero) - Returns if you passed a zero to `times`
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
/// * `JoinError` - Returns when any worker raised an error
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
  progress_handler: Arc<Mutex<Box<dyn ProgressHandler>>>,
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

  {
    let progress_handler = progress_handler.lock().unwrap();

    progress_handler.before_start()
  }?;

  let total_workers = {
    let total_workers = if workers == 0 {
      num_cpus::get()
    } else {
      workers
    };

    if tries < total_workers {
      tries
    } else {
      total_workers
    }
  };

  let runtime = Builder::new_multi_thread()
    .worker_threads(total_workers + 2)
    .thread_name_fn(|| {
      static ATOMIC_ID: AtomicUsize = AtomicUsize::new(0);

      let id = ATOMIC_ID.fetch_add(1, Ordering::SeqCst);

      format!("wakuchin-worker-{}", id)
    })
    .thread_stack_size(4 * 1024 * 1024)
    .enable_time()
    .build()?;

  let runtime_handle = runtime.handle();

  let (hit_tx, hit_rx) = channel();

  let (progress_tx_vec, progress_rx_vec): (Vec<_>, Vec<_>) = (0..total_workers)
    .map(|id| {
      progress_channel(Progress(ProgressKind::Idle(IdleDetail {
        id: id + 1,
        total_workers,
      })))
    })
    .unzip();

  let render = Arc::new(RwLock::new(ThreadRender::new(
    hit_rx,
    progress_rx_vec,
    progress_handler.clone(),
    tries,
    total_workers,
  )));

  // create temporary lock to get inner
  let render_guard = render.read().await;

  let inner = render_guard.inner.clone();

  drop(render_guard);

  let mut ui_handles = JoinSet::new();

  // hit handler
  ui_handles.spawn_on(
    async move {
      inner.wait_for_hit().await;
    },
    runtime_handle,
  );

  // progress reporter
  ui_handles.spawn_on(
    {
      let render = render.clone();

      async move {
        let mut render = render.write().await;

        render
          .start_render_progress(progress_interval)
          .await
          .unwrap();
      }
    },
    runtime_handle,
  );

  let mut worker_handles = JoinSet::new();

  (0..tries)
    .divide_evenly_into(total_workers)
    .zip(progress_tx_vec.into_iter())
    .enumerate()
    .map(|(id, (wakuchins, progress_tx))| {
      let hit_tx = hit_tx.clone();
      let regex = regex.clone();
      let total = wakuchins.len();

      worker_handles.spawn_on(
        async move {
          let mut hits = Vec::new();

          for (i, wakuchin) in wakuchins.map(|_| gen(times)).enumerate() {
            progress_tx
              .send(Progress(ProgressKind::Processing(ProcessingDetail::new(
                id + 1,
                &wakuchin,
                i,
                total,
                total_workers,
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
              total_workers,
            })))
            .expect("progress channel is unavailable");

          hits
        },
        runtime_handle,
      );
    })
    .collect_vec();

  drop(hit_tx);

  let mut hits_detail = Vec::new();

  while let Some(hits) = worker_handles.join_next().await {
    let hits = hits?;

    for hit in hits.into_iter() {
      hits_detail.push(hit);
    }
  }

  // after all workers have finished, wait for ui threads to finish
  while let Some(result) = ui_handles.join_next().await {
    result?;
  }

  // cleanup
  runtime.shutdown_background();
  {
    let progress_handler = progress_handler.lock().unwrap();

    progress_handler.after_finish()
  }?;

  let hits = render.read().await.hits();
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
///   * `progress` - referenced slice of [`Progress`]
///   * `counters` - referenced slice of [`HitCounter`]
///   * `interval` - interval of progress refresh you were specified, usable for calculation of speed
///   * `current_diff` - `current - previous` of progress, usable for calculation of speed
///   * `all_done` - true if all workers are done, usable for finalization
/// * `progress_interval` - progress refresh interval
///
/// # Returns
///
/// * `Result<WakuchinResult, Box<dyn Error>>` - the result of the research (see [`WakuchinResult`])
///
/// # Errors
///
/// * `wakuchin::error::Error::TimesIsZero` - Returns if you passed a zero to `times`
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
  progress_handler: Rc<RefCell<dyn ProgressHandler>>,
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

  {
    let progress_handler = progress_handler.borrow();

    progress_handler.before_start()
  }?;

  let mut render = Render::new(progress_handler.clone());

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
    .map(|(i, wakuchin)| -> Result<Option<Hit>> {
      render.render_progress(
        progress_interval,
        Progress(ProgressKind::Processing(ProcessingDetail::new(
          0, &wakuchin, i, tries, 1,
        ))),
        false,
      )?;

      if check(&wakuchin, &regex) {
        let hit = Hit::new(i, &wakuchin);

        render.handle_hit(&hit);

        Ok(Some(hit))
      } else {
        Ok(None)
      }
    })
    .collect::<Result<Vec<Option<_>>>>()?
    .into_iter()
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
  )?;

  {
    let progress_handler = progress_handler.borrow();

    progress_handler.after_finish()
  }?;

  let hits = render.hits();
  let hits_total = hits.iter().map(|c| c.hits).sum::<usize>();

  Ok(WakuchinResult {
    tries,
    hits_total,
    hits,
    hits_detail,
  })
}
