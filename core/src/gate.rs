use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use parking_lot::{Condvar, Mutex};

/// A one-shot "the work is finished" gate that a waiter can block on.
#[derive(Debug, Default)]
struct DoneGate {
  done: Mutex<bool>,
  opened: Condvar,
}

impl DoneGate {
  fn is_done(&self) -> bool {
    *self.done.lock()
  }

  fn signal(&self) {
    // Held across the store so a waiter cannot read the flag, find it unset,
    // and begin waiting after the notification has already gone out.
    let mut done = self.done.lock();

    *done = true;

    self.opened.notify_all();
  }

  /// Blocks until signalled or `timeout` elapses. Returns whether it is done.
  fn wait(&self, timeout: Duration) -> bool {
    let mut done = self.done.lock();

    if !*done {
      self.opened.wait_for(&mut done, timeout);
    }

    *done
  }
}

/// A gate that opens once every party has finished.
#[derive(Debug)]
pub struct Countdown {
  gate: DoneGate,
  remaining: AtomicUsize,
}

impl Countdown {
  pub fn new(parties: usize) -> Self {
    let gate = DoneGate::default();

    if parties == 0 {
      gate.signal();
    }

    Self {
      gate,
      remaining: AtomicUsize::new(parties),
    }
  }

  /// Registers one party. Dropping the returned guard marks it finished.
  pub fn party(&self) -> Party<'_> {
    Party { countdown: self }
  }

  pub fn is_done(&self) -> bool {
    self.gate.is_done()
  }

  /// Blocks until every party has finished or `timeout` elapses.
  pub fn wait(&self, timeout: Duration) -> bool {
    self.gate.wait(timeout)
  }
}

#[derive(Debug)]
pub struct Party<'countdown> {
  countdown: &'countdown Countdown,
}

impl Drop for Party<'_> {
  fn drop(&mut self) {
    if self.countdown.remaining.fetch_sub(1, Ordering::AcqRel) == 1 {
      self.countdown.gate.signal();
    }
  }
}

#[cfg(test)]
mod test {
  use std::panic::catch_unwind;
  use std::thread::{scope, sleep};
  use std::time::{Duration, Instant};

  use super::Countdown;

  #[test]
  fn zero_parties_is_already_done() {
    assert!(Countdown::new(0).is_done());
  }

  #[test]
  fn opens_only_after_the_last_party_finishes() {
    let countdown = Countdown::new(2);

    let first = countdown.party();
    let second = countdown.party();

    drop(first);
    assert!(!countdown.is_done());

    drop(second);
    assert!(countdown.is_done());
  }

  #[test]
  fn wait_times_out_while_a_party_is_outstanding() {
    let countdown = Countdown::new(1);
    let _party = countdown.party();

    let started = Instant::now();

    assert!(!countdown.wait(Duration::from_millis(20)));
    assert!(started.elapsed() >= Duration::from_millis(20));
  }

  #[test]
  fn waiter_wakes_on_the_last_party_rather_than_on_the_timeout() {
    let countdown = Countdown::new(1);

    scope(|s| {
      s.spawn(|| {
        let _party = countdown.party();

        sleep(Duration::from_millis(10));
      });

      let started = Instant::now();

      assert!(countdown.wait(Duration::from_secs(30)));
      assert!(started.elapsed() < Duration::from_secs(5));
    });
  }

  #[test]
  fn a_party_finishing_before_the_wait_is_not_lost() {
    let countdown = Countdown::new(1);

    drop(countdown.party());

    let started = Instant::now();

    assert!(countdown.wait(Duration::from_secs(30)));
    assert!(started.elapsed() < Duration::from_secs(1));
  }

  #[test]
  fn a_panicking_party_still_opens_the_gate() {
    let countdown = Countdown::new(1);

    scope(|s| {
      let _unused = s
        .spawn(|| {
          let _party = countdown.party();

          catch_unwind(|| panic!("worker exploded")).ok();
        })
        .join();
    });

    assert!(countdown.wait(Duration::from_secs(5)));
  }
}
