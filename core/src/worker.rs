use regex::Regex;

use crate::{
  check, gen_vec,
  result::{Hit, WakuchinResult},
};

use rayon::prelude::*;

pub async fn run_par(
  tries: usize,
  times: usize,
  regex: Regex,
) -> WakuchinResult {
  let hits = gen_vec(tries, times)
    .par_iter()
    .enumerate()
    .map(|(i, wakuchin)| {
      let wakuchin = wakuchin.clone();
      let regex = regex.clone();

      if check(&wakuchin, regex) {
        Some(Hit {
          hit_on: i,
          chars: wakuchin,
        })
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
) -> WakuchinResult {
  let hits = gen_vec(tries, times)
    .iter()
    .enumerate()
    .map(|(i, wakuchin)| {
      let wakuchin = wakuchin.clone();
      let regex = regex.clone();

      if check(&wakuchin, regex) {
        Some(Hit {
          hit_on: i,
          chars: wakuchin,
        })
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
