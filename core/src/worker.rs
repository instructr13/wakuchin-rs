use regex::Regex;

use crate::{check, gen_vec};

use rayon::prelude::*;

#[derive(Debug)]
pub struct Hit {
  pub hit_on: usize,
  pub chars: String,
}

#[derive(Debug)]
pub struct Result {
  pub tries: usize,
  pub hits_n: usize,
  pub hits: Vec<Hit>,
}

pub async fn run_par(tries: usize, times: usize, regex: Regex) -> Result {
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

  Result {
    tries,
    hits_n: hits.len(),
    hits,
  }
}

pub async fn run_seq(tries: usize, times: usize, regex: Regex) -> Result {
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

  Result {
    tries,
    hits_n: hits.len(),
    hits,
  }
}
