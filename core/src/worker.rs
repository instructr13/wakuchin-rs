//! Wakuchin researcher main functions

use std::sync::Arc;

use regex::Regex;

use crate::{
  check, gen_vec,
  result::{Hit, WakuchinResult},
};

use rayon::prelude::*;

/// Research wakuchin with parallelism.
///
/// * `tries` - number of tries
/// * `times` - wakuchin times n
/// * `regex` - regular expression to detect hit
/// * `handler` - handler function to handle hits
///
/// # Returns
///
/// * `WakuchinResult` - the result of the research
///
/// # Examples
///
/// ```rust
/// use regex::Regex;
///
/// use wakuchin::worker::run_par;
///
/// let result = run_par(10, 1, Regex::new(r"WKCN").unwrap(), |hit| {
///   println!("{}", hit.chars);
/// });
/// ```
pub async fn run_par<F>(
  tries: usize,
  times: usize,
  regex: Regex,
  handler: F,
) -> WakuchinResult
where
  F: Fn(&Hit) + Send + Sync,
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
    .expect("hits filtering failed");

  WakuchinResult {
    tries,
    hits_n: hits.len(),
    hits,
  }
}

/// Research wakuchin with sequential.
/// This function is useful when you don't use multi-core processors.
///
/// # Arguments
///
/// * `tries` - number of tries
/// * `times` - wakuchin times n
/// * `regex` - regular expression to detect hit
/// * `handler` - handler function to handle hits
///
/// # Returns
///
/// * `WakuchinResult` - the result of the research
///
/// # Examples
///
/// ```rust
/// use regex::Regex;
///
/// use wakuchin::worker::run_seq;
///
/// let result = run_seq(10, 1, Regex::new(r"WKCN").unwrap(), |hit| {
///  println!("{}", hit.chars);
/// });
/// ```
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
    .expect("hits filtering failed");

  WakuchinResult {
    tries,
    hits_n: hits.len(),
    hits,
  }
}
