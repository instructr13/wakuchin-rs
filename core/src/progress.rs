pub trait Progress {
  fn new(total: u64) -> Self;
  fn message(&mut self, message: &str);
  fn tick(&mut self);
  fn step(&mut self, i: u64) -> u64;
  fn set(&mut self, i: u64) -> u64;
  fn inc(&mut self) -> u64 {
    step(1)
  }
  fn finish(&mut self);
}
