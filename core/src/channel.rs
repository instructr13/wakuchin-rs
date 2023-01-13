use flume::unbounded;
use tokio::sync::watch;

pub fn channel<T>() -> (flume::Sender<T>, flume::Receiver<T>) {
  let (tx, rx) = unbounded();

  (tx, rx)
}

pub fn watch<T>(initial_value: T) -> (watch::Sender<T>, watch::Receiver<T>) {
  let (tx, rx) = watch::channel(initial_value);

  (tx, rx)
}
