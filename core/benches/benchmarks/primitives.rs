//! Micro-benchmarks for the two primitives the worker loop is built from.
//!
//! These establish the *ceiling*: `try_loop` is what a worker could do if the
//! surrounding machinery (progress, hit aggregation, cancellation) were free.
//! Everything in `worker.rs` is measured against it.

use std::hint::black_box;
use std::thread::scope;
use std::time::Duration;

use criterion::{criterion_group, BenchmarkId, Criterion, Throughput};
use wakuchin::pattern::Pattern;

use wakuchin::{check, gen, Generator};

/// An anchored, fixed-length pattern. Matches the end-to-end benches, and is
/// the historical baseline, so keep it for continuity.
const PATTERN: &str = r"^WKNCWKNC$";

/// What the project's own `examples/tries-300000000.toml` uses.
const ALTERNATION: &str = r"(WKNCWKNC|WCKNWCKN)";

/// Unanchored and shorter than any candidate, so no length early-out can fire
/// and the engine really runs at every `times`.
const SWEEP_PATTERN: &str = r"WKCN";

/// Candidate lengths to sweep. `times = 2` is the canonical workload.
const TIMES: &[usize] = &[1, 2, 3, 4];

/// Big enough that thread spawn is amortized in the pool-contention bench.
const CORPUS: usize = 200_000;

/// One contiguous buffer of fixed-stride candidates.
fn corpus(times: usize) -> (Vec<u8>, usize) {
  let stride = times * 4;
  let mut generator = Generator::new(times);
  let mut buf = Vec::with_capacity(CORPUS * stride);

  for _ in 0..CORPUS {
    generator.shuffle();

    buf.extend_from_slice(generator.bytes());
  }

  (buf, stride)
}

fn bench_gen(c: &mut Criterion) {
  let mut g = c.benchmark_group("primitives/gen");

  g.throughput(Throughput::Elements(1));

  for &times in TIMES {
    g.bench_with_input(
      BenchmarkId::from_parameter(times),
      &times,
      |b, &times| {
        b.iter(|| gen(black_box(times)));
      },
    );
  }

  g.finish();

  let mut g = c.benchmark_group("primitives/generator");

  g.throughput(Throughput::Elements(1));

  for &times in TIMES {
    g.bench_with_input(
      BenchmarkId::from_parameter(times),
      &times,
      |b, &times| {
        let mut generator = Generator::new(black_box(times));

        b.iter(|| {
          generator.shuffle();

          black_box(generator.bytes());
        });
      },
    );
  }

  g.finish();
}

/// What one match costs, per pattern shape.
///
/// The three shapes differ by 7x, which is why every other group in this file
/// names the pattern it used. `anchored` is the historical baseline;
/// `alternation` is the workload the README reports.
fn bench_check(c: &mut Criterion) {
  let (corpus, stride) = corpus(2);

  let mut g = c.benchmark_group("primitives/check");

  g.throughput(Throughput::Elements(CORPUS as u64));

  for (name, source) in [
    ("anchored", PATTERN),
    ("unanchored", r"WKNCWKNC"),
    ("alternation", ALTERNATION),
  ] {
    let pattern = Pattern::new(source).unwrap();

    g.bench_function(name, |b| {
      let mut matcher = pattern.matcher();

      b.iter(|| {
        let mut hits = 0_u64;

        for wakuchin in corpus.chunks_exact(stride) {
          hits += u64::from(check(black_box(wakuchin), &mut matcher));
        }

        black_box(hits)
      });
    });
  }

  g.finish();

  let pattern = Pattern::new(PATTERN).unwrap();

  let mut g = c.benchmark_group("primitives/check_one_off");

  g.throughput(Throughput::Elements(1));

  g.bench_function("cold_cache", |b| {
    let sample = &corpus[..stride];

    b.iter(|| black_box(pattern.is_match(black_box(sample))));
  });

  g.finish();
}

/// Why this crate carries a direct `regex-automata` dependency.
///
/// `regex::bytes::Regex::is_match` keeps the engine's scratch space in an
/// internal pool and checks it out on every call; a `Matcher` holds it
/// directly.
///
/// Single-threaded that is worth only 2-4%, which on its own would not justify
/// the dependency -- `matcher_threaded` below is the case that does. Keep both:
/// once the gap closes in *both*, the dependency has stopped paying for itself.
fn bench_matcher(c: &mut Criterion) {
  let facade = regex::bytes::Regex::new(PATTERN).unwrap();
  let pattern = Pattern::new(PATTERN).unwrap();
  let (corpus, stride) = corpus(2);

  let mut g = c.benchmark_group("primitives/matcher");

  g.throughput(Throughput::Elements(CORPUS as u64));

  g.bench_function("facade_pooled", |b| {
    b.iter(|| {
      let mut hits = 0_u64;

      for wakuchin in corpus.chunks_exact(stride) {
        hits += u64::from(facade.is_match(black_box(wakuchin)));
      }

      black_box(hits)
    });
  });

  g.bench_function("owned_cache", |b| {
    let mut matcher = pattern.matcher();

    b.iter(|| {
      let mut hits = 0_u64;

      for wakuchin in corpus.chunks_exact(stride) {
        hits += u64::from(matcher.is_match(black_box(wakuchin)));
      }

      black_box(hits)
    });
  });

  g.finish();
}

/// The ceiling: generate and check with nothing else in the loop.
fn bench_try_loop(c: &mut Criterion) {
  const TRIES: usize = 200_000;

  let pattern = Pattern::new(SWEEP_PATTERN).unwrap();

  let mut g = c.benchmark_group("primitives/try_loop");

  g.throughput(Throughput::Elements(TRIES as u64));

  for &times in TIMES {
    g.bench_with_input(
      BenchmarkId::from_parameter(times),
      &times,
      |b, &times| {
        let mut generator = Generator::new(black_box(times));
        let mut matcher = pattern.matcher();

        b.iter(|| {
          let mut hits = 0_u64;

          for _ in 0..TRIES {
            generator.shuffle();

            hits += u64::from(matcher.is_match(generator.bytes()));
          }

          black_box(hits)
        });
      },
    );
  }

  g.finish();
}

/// The multithreaded half of the same question, which is where the design
/// actually earns its keep.
fn bench_matcher_threaded(c: &mut Criterion) {
  const THREADS: usize = 4;

  let facade = regex::bytes::Regex::new(PATTERN).unwrap();
  let pattern = Pattern::new(PATTERN).unwrap();
  let (corpus, stride) = corpus(2);

  let mut g = c.benchmark_group("primitives/matcher_threaded");

  g.throughput(Throughput::Elements((CORPUS * THREADS) as u64));
  g.sample_size(30);

  let count = |hits: u64| black_box(hits);

  g.bench_function("facade_shared", |b| {
    b.iter(|| {
      scope(|s| {
        for _ in 0..THREADS {
          let facade = &facade;
          let corpus = &corpus;

          s.spawn(move || {
            let mut hits = 0_u64;

            for wakuchin in corpus.chunks_exact(stride) {
              hits += u64::from(facade.is_match(wakuchin));
            }

            count(hits)
          });
        }
      });
    });
  });

  g.bench_function("facade_cloned", |b| {
    b.iter(|| {
      scope(|s| {
        for _ in 0..THREADS {
          let facade = facade.clone();
          let corpus = &corpus;

          s.spawn(move || {
            let mut hits = 0_u64;

            for wakuchin in corpus.chunks_exact(stride) {
              hits += u64::from(facade.is_match(wakuchin));
            }

            count(hits)
          });
        }
      });
    });
  });

  g.bench_function("shared_pattern", |b| {
    b.iter(|| {
      scope(|s| {
        for _ in 0..THREADS {
          let pattern = &pattern;
          let corpus = &corpus;

          s.spawn(move || {
            let mut matcher = pattern.matcher();
            let mut hits = 0_u64;

            for wakuchin in corpus.chunks_exact(stride) {
              hits += u64::from(matcher.is_match(wakuchin));
            }

            count(hits)
          });
        }
      });
    });
  });

  g.finish();
}

criterion_group! {
  name = primitives;
  config = Criterion::default().warm_up_time(Duration::from_secs(1));
  targets = bench_gen, bench_check, bench_matcher, bench_matcher_threaded,
    bench_try_loop
}
