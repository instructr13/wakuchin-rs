use std::cell::RefCell;
use std::sync::Mutex;
use std::time::Duration;

use criterion::{criterion_group, Criterion};

use regex::Regex;

use wakuchin::handlers::empty::EmptyProgressHandler;
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
        Mutex::new(Box::new(EmptyProgressHandler::new())),
        Duration::from_millis(20),
        0,
      )
      .await
    });
  });
}

fn speed_seq(c: &mut Criterion) {
  c.bench_function("sequential processing speed", |b| {
    b.iter(|| {
      run_seq(
        30000,
        2,
        Regex::new(r"^WKNCWKNC$").unwrap(),
        RefCell::new(Box::new(EmptyProgressHandler::new())),
        Duration::from_millis(20),
      )
    });
  });
}

criterion_group!(runs, speed_par, speed_seq);
