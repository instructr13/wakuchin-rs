use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::Duration;

/// A one-shot "the work is finished" gate that a waiter can block on.
#[derive(Clone, Default)]
pub struct DoneGate {
  inner: Arc<(Mutex<bool>, Condvar)>,
}

impl DoneGate {
  pub fn is_done(&self) -> bool {
    *self.lock()
  }

  pub fn signal(&self) {
    // Held across the store so a waiter cannot read the flag, find it unset,
    // and begin waiting after the notification has already gone out.
    let mut done = self.lock();

    *done = true;

    self.inner.1.notify_all();
  }

  /// Blocks until signalled or `timeout` elapses. Returns whether it is done.
  pub fn wait(&self, timeout: Duration) -> bool {
    let done = self.lock();

    if *done {
      return true;
    }

    let (done, _timed_out) = self
      .inner
      .1
      .wait_timeout(done, timeout)
      .unwrap_or_else(|poisoned| poisoned.into_inner());

    *done
  }

  /// The guarded `bool` is meaningful even if a thread panicked while holding
  /// the lock, so recover from poisoning rather than propagating the panic into
  /// the renderer.
  fn lock(&self) -> MutexGuard<'_, bool> {
    self
      .inner
      .0
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner())
  }
}

#[cfg(test)]
mod test {
  use std::thread::{scope, sleep};
  use std::time::{Duration, Instant};

  use super::DoneGate;

  #[test]
  fn wait_times_out_when_never_signalled() {
    let gate = DoneGate::default();
    let started = Instant::now();

    assert!(!gate.wait(Duration::from_millis(20)));
    assert!(started.elapsed() >= Duration::from_millis(20));
    assert!(!gate.is_done());
  }

  #[test]
  fn wait_returns_immediately_once_signalled() {
    let gate = DoneGate::default();

    gate.signal();

    let started = Instant::now();

    assert!(gate.wait(Duration::from_secs(30)));
    assert!(started.elapsed() < Duration::from_secs(1));
  }

  #[test]
  fn waiter_wakes_on_signal_rather_than_on_timeout() {
    let gate = DoneGate::default();

    scope(|s| {
      s.spawn(|| {
        sleep(Duration::from_millis(10));

        gate.signal();
      });

      let started = Instant::now();

      assert!(gate.wait(Duration::from_secs(30)));
      assert!(started.elapsed() < Duration::from_secs(5));
    });
  }

  #[test]
  fn a_signal_before_the_wait_is_not_lost() {
    let gate = DoneGate::default();
    let signaller = gate.clone();

    signaller.signal();

    let started = Instant::now();

    assert!(gate.wait(Duration::from_secs(30)));
    assert!(started.elapsed() < Duration::from_secs(1));
  }
}
