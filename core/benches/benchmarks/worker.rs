use criterion::{criterion_group, Criterion};

use regex::Regex;

use wakuchin::worker::run_par;
use wakuchin::worker::run_seq;

fn speed_par(c: &mut Criterion) {
  let rt = tokio::runtime::Runtime::new().unwrap();

  c.bench_function("parallel processing speed", |b| {
    b.to_async(&rt).iter(|| async {
      run_par(
        30000,
        2,
        Regex::new(r"^WKNCWKNC$").unwrap(),
        |_, _, _| {},
        None,
      )
      .await
    });
  });
}

fn speed_seq(c: &mut Criterion) {
  c.bench_function("sequential processing speed", |b| {
    b.iter(|| {
      run_seq(30000, 2, Regex::new(r"^WKNCWKNC$").unwrap(), |_, _, _| {})
    });
  });
}

criterion_group!(runs, speed_par, speed_seq);
