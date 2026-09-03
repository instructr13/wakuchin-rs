//! Allocation budget for the search hot loop.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use wakuchin::pattern::Pattern;

use wakuchin::handlers::empty::EmptyProgressHandler;
use wakuchin::handlers::ProgressHandler;
use wakuchin::worker::{run_par, run_seq};

struct Counting;

static ALLOCS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
  unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
    ALLOCS.fetch_add(1, Ordering::Relaxed);

    unsafe { System.alloc(layout) }
  }

  unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
    unsafe { System.dealloc(ptr, layout) }
  }

  unsafe fn realloc(
    &self,
    ptr: *mut u8,
    layout: Layout,
    new_size: usize,
  ) -> *mut u8 {
    ALLOCS.fetch_add(1, Ordering::Relaxed);

    unsafe { System.realloc(ptr, layout, new_size) }
  }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

/// Allocations the hot loop is allowed per try.
const MAX_ALLOCS_PER_TRY: f64 = 0.01;

/// Small enough to stay quick, large enough that the slope dominates noise.
const N: usize = 200_000;

fn empty() -> Box<dyn ProgressHandler> {
  Box::new(EmptyProgressHandler::new())
}

static MEASURING: Mutex<()> = Mutex::new(());

fn count(f: impl FnOnce()) -> usize {
  let _exclusive = MEASURING
    .lock()
    .unwrap_or_else(|poisoned| poisoned.into_inner());

  let before = ALLOCS.load(Ordering::Relaxed);

  f();

  ALLOCS.load(Ordering::Relaxed) - before
}

/// Returns allocations per try, with fixed setup cost removed.
fn slope(mut run: impl FnMut(usize)) -> f64 {
  // Warm up so lazy statics and one-time buffers are not counted as slope.
  run(N);

  let single = count(|| run(N));
  let double = count(|| run(2 * N));

  (double as f64 - single as f64) / N as f64
}

#[test]
fn run_seq_allocation_slope() {
  let regex = Pattern::new(r"^WKNCWKNC$").unwrap();

  let per_try = slope(|tries| {
    run_seq(tries, 2, &regex, empty(), Duration::from_secs(3600)).unwrap();
  });

  println!("run_seq: {per_try:.3} allocations/try");

  assert!(
    per_try <= MAX_ALLOCS_PER_TRY,
    "run_seq allocates {per_try:.3}/try, budget is {MAX_ALLOCS_PER_TRY}"
  );
}

#[test]
fn run_par_allocation_slope() {
  let regex = Pattern::new(r"^WKNCWKNC$").unwrap();

  let per_try = slope(|tries| {
    run_par(tries, 2, &regex, empty(), Duration::from_secs(3600), 4).unwrap();
  });

  println!("run_par: {per_try:.3} allocations/try");

  assert!(
    per_try <= MAX_ALLOCS_PER_TRY,
    "run_par allocates {per_try:.3}/try, budget is {MAX_ALLOCS_PER_TRY}"
  );
}

#[test]
fn a_run_does_not_allocate_per_candidate() {
  /// Above the precompute gate at every `times` measured, so each run really
  /// builds the tables.
  const TRIES: usize = 500_000;

  /// The precompute is allowed a handful of allocations -- a bitset, a packed
  /// table, a buffer, the regex cache. Anything proportional to the space
  /// would show up as thousands.
  const SLACK: usize = 64;

  let regex = Pattern::new(r"^WKNCWKNC$").unwrap();

  let run = |times| {
    run_seq(TRIES, times, &regex, empty(), Duration::from_secs(3600)).unwrap();
  };

  // Warm up: the first run pays for lazy statics and the allocator's arenas.
  run(2);
  run(3);

  // 2,520 candidates against 369,600 -- a 147x larger space.
  let small = count(|| run(2));
  let large = count(|| run(3));

  println!(
    "run_seq({TRIES} tries): {small} allocations at times=2 (2,520 \
     candidates), {large} at times=3 (369,600)"
  );

  assert!(
    large <= small + SLACK,
    "growing the candidate space 147x took the allocation count from {small} \
     to {large}; something scales with the number of candidates"
  );
}
