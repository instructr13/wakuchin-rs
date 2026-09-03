//! Wakuchin researcher main functions

use std::cmp::min;
use std::ops::Range;
use std::panic::resume_unwind;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{available_parallelism, scope};
use std::time::Duration;

use divide_range::RangeDivisions;
use regex::Regex;

use crate::error::WakuchinError;
use crate::gate::Countdown;
use crate::handlers::ProgressHandler;
use crate::hit::HitTally;
use crate::progress::{
  DoneDetail, ProcessingDetail, Progress, ProgressKind,
};
use crate::render::{Render, ThreadRender};
use crate::result::{Hit, HitCount, WakuchinResult};
use crate::slot::WorkerProgress;
use crate::{check, gen};

type Result<T> = std::result::Result<T, WakuchinError>;

/// Tries a worker takes on at a time.
///
/// Big enough that publishing progress and polling the cancellation flag are
/// noise per try, small enough that Ctrl-C still feels immediate and the tail
/// of a run stays balanced across workers.
const CHUNK: usize = 4096;

/// Splits a range into consecutive sub-ranges of at most `size`.
fn chunks(range: Range<usize>, size: usize) -> impl Iterator<Item = Range<usize>> {
  let end = range.end;

  range
    .step_by(size)
    .map(move |start| start..min(start + size, end))
}

#[cfg(not(target_arch = "wasm32"))]
use signal_hook::consts::SIGINT;

/// Sets a shared flag when SIGINT arrives, and unregisters itself on drop.
///
/// The handler body must be async-signal-safe, so it is exactly one atomic
/// store. Anything that takes a lock -- a channel send, for instance -- can
/// deadlock the thread the signal happened to land on, and a bounded channel
/// nobody drains will block outright on a second signal.
///
/// Unregistering has to happen on *every* path out of a research run. The
/// registry is process-global, so a leaked handler outlives the run it belongs
/// to and fires again later against state that is gone.
#[cfg(not(target_arch = "wasm32"))]
struct InterruptFlag {
  id: signal_hook::SigId,
}

#[cfg(not(target_arch = "wasm32"))]
impl InterruptFlag {
  fn install(flag: Arc<AtomicBool>) -> Result<Self> {
    signal_hook::flag::register(SIGINT, flag)
      .map(|id| Self { id })
      .map_err(WakuchinError::SignalHandler)
  }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for InterruptFlag {
  fn drop(&mut self) {
    signal_hook::low_level::unregister(self.id);
  }
}

/// Searches one worker's share of the try space.
///
/// Work is done in chunks so that publishing progress and polling the
/// cancellation flag cost effectively nothing per try. One chunk of latency on
/// Ctrl-C is tens of microseconds.
fn search(
  range: Range<usize>,
  times: usize,
  regex: &Regex,
  progress: &WorkerProgress,
  cancelled: &AtomicBool,
) -> Result<Vec<Hit>> {
  progress.claim(range.len());

  let mut hits: Vec<Hit> = Vec::new();
  let mut processed = 0;

  for chunk in chunks(range, CHUNK) {
    let recorded = hits.len();
    let mut candidate = String::new();

    for index in chunk {
      let wakuchin = gen(times);

      if check(&wakuchin, regex) {
        hits.push(Hit::new(index, &*wakuchin));
      }

      processed += 1;
      candidate = wakuchin.into_owned();
    }

    progress.record_hits(&hits[recorded..]);
    progress.publish(processed, &candidate);

    if cancelled.load(Ordering::Relaxed) {
      return Err(WakuchinError::Cancelled);
    }
  }

  progress.finish(processed);

  Ok(hits)
}

/// Merges every worker's tally into the final hit counts.
fn merge_hits(workers: &[WorkerProgress]) -> Vec<HitCount> {
  let mut tally = HitTally::default();

  for worker in workers {
    worker.merge_hits_into(&mut tally);
  }

  tally.into_hit_counts()
}


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
///
/// # Panics
///
/// This function panics when workers raised panic (mostly about disconnected channels).
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

  let cancelled = Arc::new(AtomicBool::new(false));

  let progress: Vec<WorkerProgress> =
    (0..total_workers).map(|_| WorkerProgress::default()).collect();
  let finished = Countdown::new(total_workers);

  let mut render =
    ThreadRender::new(&cancelled, &finished, &progress, progress_handler);

  #[cfg(not(target_arch = "wasm32"))]
  let _interrupt = InterruptFlag::install(cancelled.clone())?;

  let mut hits_detail = Vec::new();

  scope::<_, Result<()>>(|s| {
    let mut worker_handles = Vec::with_capacity(total_workers);

    (0..tries)
      .divide_evenly_into(total_workers)
      .zip(&progress)
      .for_each(|(range, progress)| {
        // Cloned per worker on purpose: `regex_automata`'s `Clone` builds a
        // fresh `CachePool` so each thread wins its pool's thread-owner fast
        // path. Sharing one `&Regex` puts every thread but the owner on a
        // mutex, which measures about 2.6x slower.
        let regex = regex.clone();
        let cancelled = cancelled.as_ref();

        // Dropped on every exit, panics included, so the renderer below can
        // never be left waiting on a worker that will not report.
        let party = finished.party();

        worker_handles.push(s.spawn(move || {
          let _party = party;

          search(range, times, &regex, progress, cancelled)
        }));
      });

    // The main thread renders. It would otherwise just block in `join`, and a
    // dedicated render thread oversubscribes the machine when `workers` is
    // the core count.
    render.run(progress_interval)?;

    for handle in worker_handles {
      hits_detail.extend(handle.join().unwrap_or_else(|e| resume_unwind(e))?);
    }

    Ok(())
  })?;

  let hits = merge_hits(&progress);
  let hits_total = hits.iter().map(|count| count.hits).sum();

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
/// # Panics
///
/// This function panics when it fails to spawn signal handler thread, or to register signal hooks.
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

  let cancelled = Arc::new(AtomicBool::new(false));

  #[cfg(not(target_arch = "wasm32"))]
  let _interrupt = InterruptFlag::install(cancelled.clone())?;

  let mut render = Render::new(progress_handler);

  render.invoke_before_start()?;

  let mut hits_detail = Vec::new();
  let mut processed = 0;

  // Chunked for the same reason as `run_par`: it keeps the clock read and the
  // cancellation poll out of the per-try path.
  for chunk in chunks(0..tries, CHUNK) {
    let mut candidate = String::new();

    for index in chunk {
      let wakuchin = gen(times);

      if check(&wakuchin, regex) {
        render.handle_hit(&wakuchin);
        hits_detail.push(Hit::new(index, &*wakuchin));
      }

      processed += 1;
      candidate = wakuchin.into_owned();
    }

    if cancelled.load(Ordering::Relaxed) {
      render.invoke_on_accidential_stop()?;

      return Err(WakuchinError::Cancelled);
    }

    // Assembled only when it will actually be displayed. The previous shape
    // built -- and immediately discarded -- a `ProcessingDetail` with a cloned
    // candidate on *every* try, and read the clock to decide.
    if render.due(progress_interval) {
      render.render_progress(
        progress_interval,
        Progress(ProgressKind::Processing(ProcessingDetail::new(
          0,
          candidate,
          processed,
          tries,
        ))),
        false,
      )?;
    }
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

  let hits = render.into_hit_counts();
  let hits_total = hits.iter().map(|count| count.hits).sum();

  Ok(WakuchinResult {
    tries,
    hits_total,
    hits,
    hits_detail,
  })
}
