use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;

use crate::progress::{DoneDetail, ProcessingDetail, Progress, ProgressKind};
use crate::result::HitCounter;

use super::ProgressHandler;

#[derive(Clone, Debug, Serialize)]
struct MsgpackProgress<'a> {
  progresses: &'a [Progress],
  hit_counters: &'a [HitCounter],
  current_rate: f32,
  remaining_time: f32,
  tries: usize,
  all_done: bool,
}

pub struct MsgpackProgressHandler {
  tries: usize,
  writer: Arc<Mutex<dyn Write + Send>>,
}

impl MsgpackProgressHandler {
  pub fn new(
    tries: usize,
    writer: Arc<Mutex<dyn Write + Send + 'static>>,
  ) -> Self {
    Self { tries, writer }
  }
}

impl ProgressHandler for MsgpackProgressHandler {
  fn handle(
    &mut self,
    progresses: &[Progress],
    counters: &[HitCounter],
    elapsed_time: Duration,
    current_diff: usize,
    all_done: bool,
  ) -> anyhow::Result<()> {
    let mut buf = Vec::new();
    let mut serializer = rmp_serde::Serializer::new(&mut buf);

    let mut current_total = 0;

    for progress in progresses {
      match progress {
        Progress(ProgressKind::Processing(ProcessingDetail {
          current,
          ..
        })) => {
          current_total += current;
        }
        Progress(ProgressKind::Done(DoneDetail { total, .. })) => {
          current_total += total;
        }
        _ => {}
      }
    }

    if current_total > self.tries {
      current_total = self.tries;
    }

    let elapsed_time = elapsed_time.as_secs_f32();
    let current_rate = current_diff as f32 / elapsed_time;
    let remaining_time = (self.tries - current_total) as f32 / current_rate;

    let progress = MsgpackProgress {
      progresses,
      hit_counters: counters,
      current_rate,
      remaining_time,
      tries: self.tries,
      all_done,
    };

    progress.serialize(&mut serializer)?;

    self.writer.lock().unwrap().write_all(&buf)?;

    Ok(())
  }
}

#[cfg(test)]
mod test {
  use std::io::{Cursor, Read, Seek, SeekFrom};
  use std::sync::{Arc, Mutex};
  use std::time::Duration;

  use anyhow::Result;

  use crate::handlers::ProgressHandler;
  use crate::progress::{ProcessingDetail, Progress, ProgressKind};
  use crate::result::HitCounter;

  use super::MsgpackProgressHandler;

  #[test]
  fn test_progress() -> Result<()> {
    let tries = 100;
    let cursor = Arc::new(Mutex::new(Cursor::new(Vec::new())));
    let mut handler = MsgpackProgressHandler::new(tries, cursor.clone());
    let mut progresses = Vec::new();
    let mut counters = Vec::new();

    let progress_item = Progress(ProgressKind::Processing(ProcessingDetail {
      current: 0,
      total: 100,
      id: 0,
      wakuchin: "WKNCWKNC".into(),
      total_workers: 1,
    }));

    progresses.push(progress_item);

    let counter = HitCounter {
      chars: "あ".to_string(),
      hits: 0,
    };

    counters.push(counter);

    let elapsed_time = Duration::from_secs(1);
    let current_diff = 1;
    let all_done = false;

    handler.handle(
      &progresses,
      &counters,
      elapsed_time,
      current_diff,
      all_done,
    )?;

    let mut result_vec = Vec::new();

    {
      let mut cursor = cursor.lock().unwrap();

      cursor.seek(SeekFrom::Start(0))?;
      cursor.read_to_end(&mut result_vec)?;
    }

    let result = base64::encode(result_vec);

    assert_eq!(
      "lpGBqlByb2Nlc3NpbmeVAKhXS05DV0tOQwBkAZGSo+OBggDKP4AAAMpCyAAAZMI=",
      result
    );

    Ok(())
  }
}
