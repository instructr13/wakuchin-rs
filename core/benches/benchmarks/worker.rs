//! End-to-end benchmarks for the worker model.

use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use criterion::{criterion_group, BenchmarkId, Criterion, SamplingMode, Throughput};
use regex::Regex;

use wakuchin::handlers::empty::EmptyProgressHandler;
use wakuchin::handlers::ProgressHandler;
use wakuchin::progress::Progress;
use wakuchin::result::HitCount;
use wakuchin::worker::{run_par, run_seq};

const TIMES: usize = 2;

/// Worker counts to sweep. 0 means `available_parallelism()`.
const WORKERS: &[usize] = &[1, 2, 4, 8, 0];

/// Progress refresh interval used everywhere except `render`.
const IDLE_INTERVAL: Duration = Duration::from_millis(300);

/// Matches roughly 2 in 2520 arrangements at `times = 2`.
const RARE: &str = r"^WKNCWKNC$";

fn label(workers: usize) -> String {
  if workers == 0 {
    "max".to_owned()
  } else {
    workers.to_string()
  }
}

fn empty() -> Box<dyn ProgressHandler> {
  Box::new(EmptyProgressHandler::new())
}

/// `F(W)`: everything `run_par` costs that is not per-try work -- thread
/// spawns, channel and map construction, signal registration, teardown.
/// Subtract it from the other groups to recover the real per-try cost.
fn bench_overhead(c: &mut Criterion) {
  let regex = Regex::new(RARE).unwrap();

  let mut g = c.benchmark_group("worker/overhead");

  for &workers in WORKERS {
    g.bench_with_input(
      BenchmarkId::from_parameter(label(workers)),
      &workers,
      |b, &workers| {
        b.iter(|| {
          run_par(
            black_box(1),
            black_box(TIMES),
            &regex,
            empty(),
            IDLE_INTERVAL,
            black_box(workers),
          )
          .unwrap()
        });
      },
    );
  }

  g.finish();
}

fn bench_run_par(c: &mut Criterion) {
  const TRIES: usize = 2_000_000;

  let regex = Regex::new(RARE).unwrap();

  let mut g = c.benchmark_group("worker/run_par");

  g.throughput(Throughput::Elements(TRIES as u64));
  g.sample_size(20);
  g.sampling_mode(SamplingMode::Flat);

  for &workers in WORKERS {
    g.bench_with_input(
      BenchmarkId::from_parameter(label(workers)),
      &workers,
      |b, &workers| {
        b.iter(|| {
          run_par(
            black_box(TRIES),
            black_box(TIMES),
            &regex,
            empty(),
            IDLE_INTERVAL,
            black_box(workers),
          )
          .unwrap()
        });
      },
    );
  }

  g.finish();
}

fn bench_run_seq(c: &mut Criterion) {
  const TRIES: usize = 1_000_000;

  // Compiled once, outside `b.iter` -- the previous revision recompiled it on
  // every iteration, which made seq and par incomparable.
  let regex = Regex::new(RARE).unwrap();

  let mut g = c.benchmark_group("worker/run_seq");

  g.throughput(Throughput::Elements(TRIES as u64));
  g.sample_size(20);
  g.sampling_mode(SamplingMode::Flat);

  for &times in &[1_usize, 2, 3] {
    g.bench_with_input(BenchmarkId::from_parameter(times), &times, |b, &times| {
      b.iter(|| {
        run_seq(
          black_box(TRIES),
          black_box(times),
          &regex,
          empty(),
          IDLE_INTERVAL,
        )
        .unwrap()
      });
    });
  }

  g.finish();
}

/// The hit path, which the previous bench never exercised at all.
///
/// `hit_rate` shows what one hit costs; `hit_rate_scaling` shows whether the
/// parallel path still scales once hits are common. A permissive regex funnels
/// every worker through a single channel and a single aggregating thread, so
/// this is where the worker model fails hardest.
fn bench_hit_rate(c: &mut Criterion) {
  const TRIES: usize = 1_000_000;

  let patterns: &[(&str, &str)] = &[
    ("0.04pct", RARE),
    ("4pct", r"^WK"),
    ("25pct", r"^W"),
    ("100pct", r"W"),
  ];

  let mut g = c.benchmark_group("worker/hit_rate");

  g.throughput(Throughput::Elements(TRIES as u64));
  g.sample_size(10);
  g.sampling_mode(SamplingMode::Flat);

  for (name, pattern) in patterns {
    let regex = Regex::new(pattern).unwrap();

    g.bench_function(BenchmarkId::from_parameter(name), |b| {
      b.iter(|| {
        run_par(
          black_box(TRIES),
          black_box(TIMES),
          &regex,
          empty(),
          IDLE_INTERVAL,
          black_box(8),
        )
        .unwrap()
      });
    });
  }

  g.finish();
}

fn bench_hit_rate_scaling(c: &mut Criterion) {
  const TRIES: usize = 1_000_000;

  // 25% hit rate: frequent enough to saturate the aggregation path, cheap
  // enough to run in a bench.
  let regex = Regex::new(r"^W").unwrap();

  let mut g = c.benchmark_group("worker/hit_rate_scaling");

  g.throughput(Throughput::Elements(TRIES as u64));
  g.sample_size(10);
  g.sampling_mode(SamplingMode::Flat);

  for &workers in WORKERS {
    g.bench_with_input(
      BenchmarkId::from_parameter(label(workers)),
      &workers,
      |b, &workers| {
        b.iter(|| {
          run_par(
            black_box(TRIES),
            black_box(TIMES),
            &regex,
            empty(),
            IDLE_INTERVAL,
            black_box(workers),
          )
          .unwrap()
        });
      },
    );
  }

  g.finish();
}

/// Counts `handle` calls so the render path is actually exercised. With the
/// default 300 ms interval against a sub-second run, it never fires at all.
#[derive(Clone, Default)]
struct CountingProgressHandler {
  calls: Arc<AtomicUsize>,
}

impl ProgressHandler for CountingProgressHandler {
  fn handle(
    &mut self,
    progresses: &[Progress],
    hit_counts: &[HitCount],
    _: Duration,
    _: usize,
    _: bool,
  ) -> anyhow::Result<()> {
    black_box(progresses.len());
    black_box(hit_counts.len());

    self.calls.fetch_add(1, Ordering::Relaxed);

    Ok(())
  }
}

fn bench_render(c: &mut Criterion) {
  const TRIES: usize = 1_000_000;

  let regex = Regex::new(RARE).unwrap();
  let calls = Arc::new(AtomicUsize::new(0));

  let mut g = c.benchmark_group("worker/render");

  g.throughput(Throughput::Elements(TRIES as u64));
  g.sample_size(20);
  g.sampling_mode(SamplingMode::Flat);

  g.bench_function("interval_1ms", |b| {
    b.iter(|| {
      run_par(
        black_box(TRIES),
        black_box(TIMES),
        &regex,
        Box::new(CountingProgressHandler {
          calls: calls.clone(),
        }),
        Duration::from_millis(1),
        black_box(8),
      )
      .unwrap()
    });
  });

  g.finish();

  assert!(
    calls.load(Ordering::Relaxed) > 0,
    "the render path never ran; this benchmark is measuring nothing"
  );
}

criterion_group! {
  name = overhead;
  config = Criterion::default().warm_up_time(Duration::from_secs(1));
  targets = bench_overhead
}

criterion_group! {
  name = runs;
  config = Criterion::default().warm_up_time(Duration::from_secs(1));
  targets = bench_run_par, bench_run_seq
}

criterion_group! {
  name = hit_path;
  config = Criterion::default().warm_up_time(Duration::from_secs(1));
  targets = bench_hit_rate, bench_hit_rate_scaling
}

criterion_group! {
  name = render;
  config = Criterion::default().warm_up_time(Duration::from_secs(1));
  targets = bench_render
}
