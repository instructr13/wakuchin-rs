use std::fmt;

use serde::Serialize;

#[derive(Clone)]
pub enum ResultOutputFormat {
  Text,
  Json,
}

#[derive(Debug, Serialize)]
pub struct Hit {
  pub hit_on: usize,
  pub chars: String,
}

#[derive(Debug, Serialize)]
pub struct WakuchinResult {
  pub tries: usize,
  pub hits_n: usize,
  pub hits: Vec<Hit>,
}

pub fn out(format: ResultOutputFormat, result: &WakuchinResult) -> String {
  match format {
    ResultOutputFormat::Text => fmt::format(format_args!(
      "--- Result ---
Tries: {}
Hits: {}
Hits%: {}",
      result.tries,
      result.hits_n,
      result.hits_n as f64 / result.tries as f64 * 100.0,
    )),
    ResultOutputFormat::Json => serde_json::to_string(result).unwrap(),
  }
}
