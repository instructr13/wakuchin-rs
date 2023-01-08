use flume::{bounded, unbounded};
use tokio::sync::watch;

pub(crate) fn channel<T>() -> (flume::Sender<T>, flume::Receiver<T>) {
  let (tx, rx) = unbounded();

  (tx, rx)
}

pub(crate) fn oneshot<T>() -> (flume::Sender<T>, flume::Receiver<T>) {
  let (tx, rx) = bounded(1);

  (tx, rx)
}

pub(crate) fn watch<T>(
  initial_value: T,
) -> (watch::Sender<T>, watch::Receiver<T>) {
  let (tx, rx) = watch::channel(initial_value);

  (tx, rx)
}