use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::{engine::general_purpose, Engine as _};
use serde::Serialize;

use crate::progress::{DoneDetail, ProcessingDetail, Progress, ProgressKind};
use crate::result::HitCount;

use super::ProgressHandler;

#[derive(Clone, Debug, Serialize)]
struct MsgpackProgress<'a> {
  progresses: &'a [Progress],
  hit_counts: &'a [HitCount],
  current_rate: f64,
  remaining_time: f64,
  tries: usize,
  all_done: bool,
}

pub struct MsgpackBase64ProgressHandler {
  tries: usize,
  writer: Arc<Mutex<dyn Write + Send>>,
}

impl MsgpackBase64ProgressHandler {
  pub fn new(tries: usize, writer: Arc<Mutex<dyn Write + Send>>) -> Self {
    Self { tries, writer }
  }
}

impl ProgressHandler for MsgpackBase64ProgressHandler {
  fn handle(
    &mut self,
    progresses: &[Progress],
    hit_counts: &[HitCount],
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

    let elapsed_time = elapsed_time.as_secs_f64();
    let current_rate = current_diff as f64 / elapsed_time;
    let remaining_time = (self.tries - current_total) as f64 / current_rate;

    let progress = MsgpackProgress {
      progresses,
      hit_counts,
      current_rate,
      remaining_time,
      tries: self.tries,
      all_done,
    };

    progress.serialize(&mut serializer)?;

    let encoded = general_purpose::STANDARD.encode(&mut buf);

    let mut writer = self.writer.lock().unwrap();
    writer.write_all(encoded.as_bytes())?;

    Ok(())
  }
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
    hit_counts: &[HitCount],
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

    let elapsed_time = elapsed_time.as_secs_f64();
    let current_rate = current_diff as f64 / elapsed_time;
    let remaining_time = (self.tries - current_total) as f64 / current_rate;

    let progress = MsgpackProgress {
      progresses,
      hit_counts,
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
  use base64::engine::general_purpose;
  use base64::Engine;

  use crate::handlers::ProgressHandler;
  use crate::progress::{ProcessingDetail, Progress, ProgressKind};
  use crate::result::HitCount;

  use super::MsgpackBase64ProgressHandler;
  use super::MsgpackProgressHandler;

  #[test]
  fn test_msgpack_base64_progress() -> Result<()> {
    let tries = 100;
    let cursor = Arc::new(Mutex::new(Cursor::new(Vec::new())));
    let mut handler = MsgpackBase64ProgressHandler::new(tries, cursor.clone());

    let progresses =
      vec![Progress(ProgressKind::Processing(ProcessingDetail {
        current: 0,
        total: 100,
        id: 0,
        wakuchin: "WKNCWKNC".into(),
      }))];

    let hit_counts = vec![HitCount {
      chars: "あ".into(),
      hits: 0,
    }];

    let elapsed_time = Duration::from_secs(1);
    let current_diff = 1;
    let all_done = false;

    handler.handle(
      &progresses,
      &hit_counts,
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

    let result = String::from_utf8(result_vec)?;

    assert_eq!(
      "lpGBqlByb2Nlc3NpbmeUAKhXS05DV0tOQwBkkZKj44GCAMs/8AAAAAAAAMtAWQAAAAAAAGTC",
      result
    );

    Ok(())
  }

  #[test]
  fn test_msgpack_progress() -> Result<()> {
    let tries = 100;
    let cursor = Arc::new(Mutex::new(Cursor::new(Vec::new())));
    let mut handler = MsgpackProgressHandler::new(tries, cursor.clone());

    let progresses =
      vec![Progress(ProgressKind::Processing(ProcessingDetail {
        current: 0,
        total: 100,
        id: 0,
        wakuchin: "WKNCWKNC".into(),
      }))];

    let hit_counts = vec![HitCount {
      chars: "あ".into(),
      hits: 0,
    }];

    let elapsed_time = Duration::from_secs(1);
    let current_diff = 1;
    let all_done = false;

    handler.handle(
      &progresses,
      &hit_counts,
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

    let result = general_purpose::STANDARD.encode(result_vec);

    assert_eq!(
      "lpGBqlByb2Nlc3NpbmeUAKhXS05DV0tOQwBkkZKj44GCAMs/8AAAAAAAAMtAWQAAAAAAAGTC",
      result
    );

    Ok(())
  }
}
