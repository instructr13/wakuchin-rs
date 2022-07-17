use regex::Regex;

use crate::{
  check, gen_vec,
  result::{Hit, WakuchinResult},
};

use rayon::prelude::*;

type HitHandler = fn(&Hit);

pub async fn run_par(
  tries: usize,
  times: usize,
  regex: Regex,
  handler: HitHandler,
) -> WakuchinResult {
  let hits = gen_vec(tries, times)
    .par_iter()
    .enumerate()
    .map(|(i, wakuchin)| {
      let wakuchin = wakuchin.clone();
      let regex = regex.clone();
      let handler = handler.clone();

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

pub async fn run_seq(
  tries: usize,
  times: usize,
  regex: Regex,
  handler: HitHandler,
) -> WakuchinResult {
  let hits = gen_vec(tries, times)
    .iter()
    .enumerate()
    .map(|(i, wakuchin)| {
      let wakuchin = wakuchin.clone();
      let regex = regex.clone();
      let handler = handler.clone();

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
