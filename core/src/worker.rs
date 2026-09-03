//! Wakuchin researcher main functions

use std::cmp::min;
use std::ops::Range;
use std::panic::resume_unwind;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::{available_parallelism, scope};
use std::time::Duration;

use itertools::Itertools;

use crate::error::WakuchinError;
use crate::gate::Countdown;
use crate::handlers::ProgressHandler;
use crate::hit::HitTally;
use crate::pattern::Pattern;
use crate::progress::{DoneDetail, ProcessingDetail, Progress, ProgressKind};
use crate::render::{Render, ThreadRender};
use crate::result::{Hit, HitCount, WakuchinResult};
use crate::slot::WorkerProgress;
use crate::Generator;

type Result<T> = std::result::Result<T, WakuchinError>;

/// Smallest chunk a worker will take. Below this, publishing progress and
/// polling the cancellation flag stop being noise per try.
const CHUNK_MIN: usize = 64;

/// Largest chunk a worker will take. Caps how long cancellation can go
/// unnoticed, and how much work a straggler can be sitting on at the end.
const CHUNK_MAX: usize = 65_536;

/// How finely the remaining work is divided each time a chunk is handed out.
/// Higher means smaller chunks: less tail imbalance, more trips to the shared
/// counter.
const GUIDED_DIVISOR: usize = 16;

/// Hands out tries to workers on demand.
struct Cursor {
  next: AtomicUsize,
  tries: usize,
  workers: usize,
}

impl Cursor {
  fn new(tries: usize, workers: usize) -> Self {
    Self {
      next: AtomicUsize::new(0),
      tries,
      workers,
    }
  }

  /// Takes the next chunk, or `None` once the work is gone.
  fn claim(&self) -> Option<Range<usize>> {
    let mut start = self.next.load(Ordering::Relaxed);

    loop {
      if start >= self.tries {
        return None;
      }

      // A compare-exchange rather than a `fetch_add` because the size depends
      // on how much is left, which is the value being read.
      let end = min(start + self.chunk(self.tries - start), self.tries);

      match self.next.compare_exchange_weak(
        start,
        end,
        Ordering::Relaxed,
        Ordering::Relaxed,
      ) {
        Ok(_) => return Some(start..end),
        Err(current) => start = current,
      }
    }
  }

  fn chunk(&self, remaining: usize) -> usize {
    (remaining / (self.workers * GUIDED_DIVISOR)).clamp(CHUNK_MIN, CHUNK_MAX)
  }
}

#[cfg(not(target_arch = "wasm32"))]
use signal_hook::consts::SIGINT;

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

/// Where a search loop reports what it has done.
trait Reporter {
  /// Called when a chunk of `tries` has been taken on.
  fn claim(&mut self, tries: usize);

  /// Called at the end of each chunk. `counts` holds only that chunk's hits.
  fn record(
    &mut self,
    counts: &HitTally,
    processed: usize,
    candidate: &str,
  ) -> Result<()>;

  /// Called once, after the last chunk.
  fn finish(&mut self, processed: usize) -> Result<()>;
}

/// The search loop: take a chunk, search it, report, repeat.
///
/// Progress and the cancellation flag are touched once per chunk rather than
/// once per try, so both cost-effectively nothing. One chunk is the upper bound
/// on how long Ctrl-C can go unnoticed.
fn search(
  cursor: &Cursor,
  times: usize,
  pattern: &Pattern,
  reporter: &mut impl Reporter,
  cancelled: &AtomicBool,
) -> Result<Vec<Hit>> {
  let mut matcher = pattern.matcher();

  let mut generator = Generator::new(times);

  let mut hits: Vec<Hit> = Vec::new();
  let mut processed = 0;

  let mut counts = HitTally::default();

  while let Some(chunk) = cursor.claim() {
    reporter.claim(chunk.len());

    for index in chunk {
      generator.shuffle();

      if matcher.is_match(generator.bytes()) {
        // Validated once, not once per use: `as_str` checks the buffer is UTF-8.
        let candidate = generator.as_str();

        counts.add(candidate);
        hits.push(Hit::new(index, candidate));
      }

      processed += 1;
    }

    reporter.record(&counts, processed, generator.as_str())?;
    counts.clear();

    if cancelled.load(Ordering::Relaxed) {
      return Err(WakuchinError::Cancelled);
    }
  }

  reporter.finish(processed)?;

  Ok(hits)
}

/// Reports into the slot a parallel worker owns.
impl Reporter for &WorkerProgress {
  fn claim(&mut self, tries: usize) {
    WorkerProgress::claim(self, tries);
  }

  fn record(
    &mut self,
    counts: &HitTally,
    processed: usize,
    candidate: &str,
  ) -> Result<()> {
    self.record_hits(counts);
    self.publish(processed, candidate);

    Ok(())
  }

  fn finish(&mut self, processed: usize) -> Result<()> {
    WorkerProgress::finish(self, processed);

    Ok(())
  }
}

/// Reports straight into the renderer, for a sequential run.
struct RenderReporter<'render> {
  render: &'render mut Render,
  interval: Duration,
  tries: usize,
}

impl Reporter for RenderReporter<'_> {
  fn claim(&mut self, _tries: usize) {}

  fn record(
    &mut self,
    counts: &HitTally,
    processed: usize,
    candidate: &str,
  ) -> Result<()> {
    self.render.record_hits(counts);

    // Assembled only when it will actually be displayed.
    if self.render.due(self.interval) {
      self.render.render_progress(
        self.interval,
        Progress(ProgressKind::Processing(ProcessingDetail::new(
          0,
          candidate.to_owned(),
          processed,
          self.tries,
        ))),
        false,
      )?;
    }

    Ok(())
  }

  fn finish(&mut self, _processed: usize) -> Result<()> {
    Ok(())
  }
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
///   use wakuchin::pattern::Pattern;
///
///   use wakuchin::handlers::ProgressHandler;
///   use wakuchin::handlers::empty::EmptyProgressHandler;
///   use wakuchin::worker::run_par;
///
///   # fn main() -> Result<(), Box<dyn std::error::Error>> {
///   let handler: Box<dyn ProgressHandler> = Box::new(EmptyProgressHandler::new());
///   let result = run_par(10, 0, &Pattern::new(r"WKCN")?, handler, Duration::from_secs(1), 0);
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
/// use wakuchin::pattern::Pattern;
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
/// let result = run_par(tries, 1, &Pattern::new(r"WKCN")?, handler, Duration::from_secs(1), 4)?;
///
/// println!("{}", result.out(ResultOutputFormat::Text)?);
/// #
/// #   Ok(())
/// # }
/// ```
pub fn run_par(
  tries: usize,
  times: usize,
  regex: &Pattern,
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

  let progress: Vec<WorkerProgress> = (0..total_workers)
    .map(|_| WorkerProgress::default())
    .collect();
  let finished = Countdown::new(total_workers);
  let cursor = Cursor::new(tries, total_workers);

  let mut render =
    ThreadRender::new(&cancelled, &finished, &progress, progress_handler);

  #[cfg(not(target_arch = "wasm32"))]
  let _interrupt = InterruptFlag::install(cancelled.clone())?;

  let mut hits_detail = Vec::new();

  scope::<_, Result<()>>(|s| {
    let mut worker_handles = Vec::with_capacity(total_workers);

    for progress in &progress {
      let cancelled = cancelled.as_ref();
      let cursor = &cursor;

      // Dropped on every exit, panics included, so the renderer below can never
      // be left waiting on a worker that will not report.
      let party = finished.party();

      worker_handles.push(s.spawn(move || {
        let _party = party;

        let mut reporter = progress;

        search(cursor, times, regex, &mut reporter, cancelled)
      }));
    }

    render.run(progress_interval)?;

    let mut per_worker = Vec::with_capacity(total_workers);

    for handle in worker_handles {
      per_worker.push(handle.join().unwrap_or_else(|e| resume_unwind(e))?);
    }

    // Workers take chunks as they finish rather than owning one contiguous
    // range, so each worker's hits are ordered but the concatenation is not.
    //
    // Merged rather than sorted: every input run is already ascending, so this
    // costs O(n log workers) comparisons instead of O(n log n).
    hits_detail.reserve(per_worker.iter().map(Vec::len).sum());
    hits_detail.extend(
      per_worker
        .into_iter()
        .kmerge_by(|left, right| left.hit_on <= right.hit_on),
    );

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
///   use wakuchin::pattern::Pattern;
///
///   use wakuchin::handlers::ProgressHandler;
///   use wakuchin::handlers::empty::EmptyProgressHandler;
///   use wakuchin::worker::run_seq;
///
///   let handler: Box<dyn ProgressHandler> = Box::new(EmptyProgressHandler::new());
///   let result = run_seq(10, 0, &Pattern::new(r"WKCN")?, handler, Duration::from_secs(1));
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
/// use wakuchin::pattern::Pattern;
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
/// let result = run_seq(tries, 1, &Pattern::new(r"WKCN")?, handler, Duration::from_secs(1))?;
///
/// println!("{}", result.out(ResultOutputFormat::Text)?);
/// #
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn run_seq(
  tries: usize,
  times: usize,
  regex: &Pattern,
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

  // The same loop `run_par`'s workers run, with a single-party cursor. The two
  // shared no code before, and had drifted: the sequential path was building a
  // progress message -- and cloning the candidate into it -- on every try, only
  // for the renderer to discard it unread.
  let cursor = Cursor::new(tries, 1);

  let hits_detail = {
    let mut reporter = RenderReporter {
      render: &mut render,
      interval: progress_interval,
      tries,
    };

    search(&cursor, times, regex, &mut reporter, &cancelled)
  };

  let hits_detail = match hits_detail {
    Ok(hits_detail) => hits_detail,
    Err(err) => {
      if matches!(err, WakuchinError::Cancelled) {
        render.invoke_on_accidental_stop()?;
      }

      return Err(err);
    }
  };

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

#[cfg(test)]
mod test {
  use std::error::Error;
  use std::time::Duration;

  use crate::pattern::Pattern;

  use crate::handlers::empty::EmptyProgressHandler;
  use crate::handlers::ProgressHandler;
  use crate::progress::{Progress, ProgressKind};
  use crate::result::HitCount;

  use super::{run_par, run_seq, Cursor, CHUNK_MAX, CHUNK_MIN};

  fn empty() -> Box<dyn ProgressHandler> {
    Box::new(EmptyProgressHandler::new())
  }

  /// Long enough that the periodic refresh never fires during a test.
  fn interval() -> Duration {
    Duration::from_secs(3600)
  }

  #[test]
  fn cursor_hands_out_every_try_exactly_once() {
    let cursor = Cursor::new(100_000, 4);
    let mut seen = vec![0_u8; 100_000];

    while let Some(chunk) = cursor.claim() {
      assert!(chunk.len() <= CHUNK_MAX);

      for index in chunk {
        seen[index] += 1;
      }
    }

    assert!(seen.iter().all(|&count| count == 1));
  }

  #[test]
  fn cursor_chunks_shrink_towards_the_end() {
    let cursor = Cursor::new(1_000_000, 1);

    let first = cursor.claim().unwrap().len();
    let mut last = first;

    while let Some(chunk) = cursor.claim() {
      last = chunk.len();
    }

    assert!(first >= CHUNK_MIN);
    assert!(
      last <= first,
      "guided scheduling should not grow chunks: {first} then {last}"
    );
  }

  #[test]
  fn run_par_reports_every_try() -> Result<(), Box<dyn Error>> {
    let result = run_par(
      50_000,
      2,
      &Pattern::new(r"^WKNCWKNC$")?,
      empty(),
      interval(),
      4,
    )?;

    assert_eq!(result.tries, 50_000);
    assert_eq!(
      result.hits_total,
      result.hits.iter().map(|count| count.hits).sum::<usize>()
    );
    assert_eq!(result.hits_total, result.hits_detail.len());

    Ok(())
  }

  #[test]
  fn run_par_returns_hits_in_index_order() -> Result<(), Box<dyn Error>> {
    // Permissive enough that every worker contributes, so a missing final sort
    // shows up as interleaved indices.
    let result =
      run_par(50_000, 2, &Pattern::new(r"^W")?, empty(), interval(), 8)?;

    assert!(result.hits_detail.len() > 100);
    assert!(
      result
        .hits_detail
        .windows(2)
        .all(|pair| pair[0].hit_on <= pair[1].hit_on),
      "hits_detail is not sorted by hit_on"
    );

    Ok(())
  }

  /// The aggregate progress bar divides the sum of the workers' reported totals
  /// by `tries`, so the two have to agree exactly once the run is over. Under
  /// dynamic work handout the per-worker totals are no longer `tries / workers`,
  /// which is what makes this worth asserting.
  #[test]
  fn worker_totals_sum_to_tries() -> Result<(), Box<dyn Error>> {
    struct Auditor {
      tries: usize,
      audited: bool,
    }

    impl ProgressHandler for Auditor {
      fn handle(
        &mut self,
        progresses: &[Progress],
        _: &[HitCount],
        _: Duration,
        _: usize,
        all_done: bool,
      ) -> anyhow::Result<()> {
        if !all_done {
          return Ok(());
        }

        let total: usize = progresses
          .iter()
          .map(|progress| match progress {
            Progress(ProgressKind::Done(detail)) => detail.total,
            Progress(ProgressKind::Processing(detail)) => detail.current,
            Progress(ProgressKind::Idle(_)) => 0,
          })
          .sum();

        assert_eq!(total, self.tries);

        self.audited = true;

        Ok(())
      }

      fn after_finish(&mut self) -> anyhow::Result<()> {
        assert!(self.audited, "the final refresh never ran");

        Ok(())
      }
    }

    run_par(
      50_000,
      2,
      &Pattern::new(r"^WKNCWKNC$")?,
      Box::new(Auditor {
        tries: 50_000,
        audited: false,
      }),
      interval(),
      4,
    )?;

    Ok(())
  }

  #[test]
  fn par_and_seq_agree_on_shape() -> Result<(), Box<dyn Error>> {
    let regex = Pattern::new(r"^W")?;

    let par = run_par(20_000, 2, &regex, empty(), interval(), 4)?;
    let seq = run_seq(20_000, 2, &regex, empty(), interval())?;

    assert_eq!(par.tries, seq.tries);
    assert_eq!(par.hits_total, par.hits_detail.len());
    assert_eq!(seq.hits_total, seq.hits_detail.len());

    Ok(())
  }

  #[test]
  fn zero_tries_does_nothing() -> Result<(), Box<dyn Error>> {
    let regex = Pattern::new(r"^W")?;

    for result in [
      run_par(0, 2, &regex, empty(), interval(), 4)?,
      run_seq(0, 2, &regex, empty(), interval())?,
    ] {
      assert_eq!(result.tries, 0);
      assert_eq!(result.hits_total, 0);
      assert!(result.hits.is_empty());
      assert!(result.hits_detail.is_empty());
    }

    Ok(())
  }

  #[test]
  fn times_of_zero_is_rejected() -> Result<(), Box<dyn Error>> {
    let regex = Pattern::new(r"^W")?;

    assert!(run_par(10, 0, &regex, empty(), interval(), 4).is_err());
    assert!(run_seq(10, 0, &regex, empty(), interval()).is_err());

    Ok(())
  }
}
