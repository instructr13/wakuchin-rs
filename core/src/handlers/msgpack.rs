use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::progress::{DoneDetail, ProcessingDetail, Progress, ProgressKind};
use crate::result::HitCounter;

#[derive(Debug, Serialize, Deserialize)]
pub struct BuiltinProgressHandler {
  pub progresses: Vec<Progress>,
  pub hit_counters: Vec<HitCounter>,
  pub current_rate: f32,
  pub remaining_time: Duration,
  pub tries: usize,
  pub all_done: bool,
}

pub fn progress(
  tries: usize,
) -> impl Fn(&[Progress], &[HitCounter], Duration, usize, bool) {
  move |progresses, counters, elapsed_time, current_diff, all_done| {
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

    let elapsed_time = elapsed_time.as_secs_f32();
    let current_rate = current_diff as f32 / elapsed_time;
    let remaining_time =
      Duration::from_secs_f32((tries - current_total) as f32 / current_rate);

    let handler = BuiltinProgressHandler {
      progresses: progresses.to_vec(),
      hit_counters: counters.to_vec(),
      current_rate,
      remaining_time,
      tries,
      all_done,
    };

    handler.serialize(&mut serializer).unwrap();

    println!("{}", base64::encode(&buf));
  }
}
