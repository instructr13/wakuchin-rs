#[derive(Clone, Debug)]
pub enum ProgressKind {
  Idle(usize, usize),
  Processing(ProcessingDetail),
  Done(usize, usize),
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct ProcessingDetail {
  pub id: usize,
  pub wakuchin: String,
  pub current: usize,
  pub total: usize,
  pub total_workers: usize,
}

impl ProcessingDetail {
  pub fn new(
    id: usize,
    wakuchin: impl Into<String>,
    current: usize,
    total: usize,
    total_workers: usize,
  ) -> Self {
    Self {
      id,
      wakuchin: wakuchin.into(),
      current,
      total,
      total_workers,
    }
  }
}

#[derive(Clone, Debug)]
pub struct Progress(pub ProgressKind);

#[derive(Clone, Debug)]
pub struct HitCounter {
  pub chars: String,
  pub hits: usize,
}

impl HitCounter {
  pub fn new(chars: impl Into<String>, hits: usize) -> Self {
    Self {
      chars: chars.into(),
      hits,
    }
  }
}
