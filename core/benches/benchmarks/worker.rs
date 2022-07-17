use criterion::{criterion_group, Criterion};

use regex::Regex;

use wakuchin_core::worker::run_par;
use wakuchin_core::worker::run_seq;

fn speed_par(c: &mut Criterion) {
  c.bench_function("parallel processing speed", |b| {
    b.iter(|| run_par(30000, 2, Regex::new(r"^WKNCWKNC$").unwrap()))
  });
}
fn speed_seq(c: &mut Criterion) {
  c.bench_function("sequential processing speed", |b| {
    b.iter(|| run_seq(30000, 2, Regex::new(r"^WKNCWKNC$").unwrap()))
  });
}

criterion_group!(runs, speed_par, speed_seq);
