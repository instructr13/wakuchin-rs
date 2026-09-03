use criterion::criterion_main;

mod benchmarks;

criterion_main! {
  benchmarks::primitives::primitives,
  benchmarks::worker::overhead,
  benchmarks::worker::runs,
  benchmarks::worker::hit_path,
  benchmarks::worker::render,
}
