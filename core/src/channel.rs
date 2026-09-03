use flume::unbounded;

pub fn channel<T>() -> (flume::Sender<T>, flume::Receiver<T>) {
  unbounded()
}
