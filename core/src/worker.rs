use std::sync::Arc;

use regex::Regex;

use crate::{
  check, gen_vec,
  result::{Hit, WakuchinResult},
};

use rayon::prelude::*;

pub async fn run_par<F>(
  tries: usize,
  times: usize,
  regex: Regex,
  handler: F,
) -> WakuchinResult
where
  F: Fn(&Hit) -> () + Send + Sync,
{
  let handler_thread_safe = Arc::new(handler);
  let hits = gen_vec(tries, times)
    .par_iter()
    .enumerate()
    .map(|(i, wakuchin)| {
      let wakuchin = wakuchin.clone();
      let regex = regex.clone();
      let handler = handler_thread_safe.clone();

      if check(&wakuchin, regex) {
        let hit = Hit {
          hit_on: i,
          chars: wakuchin,
        };

        handler(&hit);

        Some(hit)
      } else {
        None
      }
    })
    .filter(|hit| hit.is_some())
    .collect::<Option<Vec<_>>>()
    .unwrap();

  WakuchinResult {
    tries,
    hits_n: hits.len(),
    hits,
  }
}

pub async fn run_seq<F>(
  tries: usize,
  times: usize,
  regex: Regex,
  handler: F,
) -> WakuchinResult
where
  F: Fn(&Hit),
{
  let hits = gen_vec(tries, times)
    .iter()
    .enumerate()
    .map(|(i, wakuchin)| {
      let wakuchin = wakuchin.clone();
      let regex = regex.clone();

      if check(&wakuchin, regex) {
        let hit = Hit {
          hit_on: i,
          chars: wakuchin,
        };

        handler(&hit);

        Some(hit)
      } else {
        None
      }
    })
    .filter(|hit| hit.is_some())
    .collect::<Option<Vec<_>>>()
    .unwrap();

  WakuchinResult {
    tries,
    hits_n: hits.len(),
    hits,
  }
}
