//! Micro-benchmarks for the two primitives the worker loop is built from.
//!
//! These establish the *ceiling*: `try_loop` is what a worker could do if the
//! surrounding machinery (progress, hit aggregation, cancellation) were free.
//! Everything in `worker.rs` is measured against it.

use std::hint::black_box;
use std::thread::scope;
use std::time::Duration;

use criterion::{criterion_group, BenchmarkId, Criterion, Throughput};
use regex::Regex;

use wakuchin::{check, gen};

/// Candidate lengths to sweep. `times = 2` is the canonical workload.
const TIMES: &[usize] = &[1, 2, 3, 4];

/// Big enough that thread spawn is amortized in the pool-contention bench.
const CORPUS: usize = 200_000;

fn corpus(times: usize) -> Vec<String> {
  (0..CORPUS).map(|_| gen(times).into_owned()).collect()
}

fn bench_gen(c: &mut Criterion) {
  let mut g = c.benchmark_group("primitives/gen");

  g.throughput(Throughput::Elements(1));

  for &times in TIMES {
    g.bench_with_input(BenchmarkId::from_parameter(times), &times, |b, &times| {
      b.iter(|| gen(black_box(times)));
    });
  }

  g.finish();
}

fn bench_check(c: &mut Criterion) {
  let regex = Regex::new(r"^WKNCWKNC$").unwrap();
  let corpus = corpus(2);

  let mut g = c.benchmark_group("primitives/check");

  g.throughput(Throughput::Elements(CORPUS as u64));

  g.bench_function("single_thread", |b| {
    b.iter(|| {
      let mut hits = 0_u64;

      for wakuchin in &corpus {
        hits += u64::from(check(black_box(wakuchin), &regex));
      }

      black_box(hits)
    });
  });

  g.finish();
}

/// Guards the `regex.clone()`-per-worker property.
///
/// `regex_automata`'s `impl Clone for Regex` builds a fresh `CachePool` so that
/// each thread wins its pool's "thread owner" fast path. Sharing one `&Regex`
/// pushes every thread but the owner onto a mutex. `cloned` should beat
/// `shared` here; if it ever stops doing so, the property was lost.
fn bench_check_pool(c: &mut Criterion) {
  const THREADS: usize = 4;

  let regex = Regex::new(r"^WKNCWKNC$").unwrap();
  let corpus = corpus(2);

  let mut g = c.benchmark_group("primitives/check_pool");

  g.throughput(Throughput::Elements((CORPUS * THREADS) as u64));
  g.sample_size(30);

  g.bench_function("cloned_per_thread", |b| {
    b.iter(|| {
      scope(|s| {
        for _ in 0..THREADS {
          let regex = regex.clone();
          let corpus = &corpus;

          s.spawn(move || {
            let mut hits = 0_u64;

            for wakuchin in corpus {
              hits += u64::from(check(wakuchin, &regex));
            }

            black_box(hits)
          });
        }
      });
    });
  });

  g.bench_function("shared", |b| {
    b.iter(|| {
      scope(|s| {
        for _ in 0..THREADS {
          let regex = &regex;
          let corpus = &corpus;

          s.spawn(move || {
            let mut hits = 0_u64;

            for wakuchin in corpus {
              hits += u64::from(check(wakuchin, regex));
            }

            black_box(hits)
          });
        }
      });
    });
  });

  g.finish();
}

/// The ceiling: generate and check with nothing else in the loop.
fn bench_try_loop(c: &mut Criterion) {
  const TRIES: usize = 200_000;

  let regex = Regex::new(r"^WKNCWKNC$").unwrap();

  let mut g = c.benchmark_group("primitives/try_loop");

  g.throughput(Throughput::Elements(TRIES as u64));

  for &times in TIMES {
    g.bench_with_input(BenchmarkId::from_parameter(times), &times, |b, &times| {
      b.iter(|| {
        let mut hits = 0_u64;

        for _ in 0..TRIES {
          hits += u64::from(check(&gen(black_box(times)), &regex));
        }

        black_box(hits)
      });
    });
  }

  g.finish();
}

criterion_group! {
  name = primitives;
  config = Criterion::default().warm_up_time(Duration::from_secs(1));
  targets = bench_gen, bench_check, bench_check_pool, bench_try_loop
}
