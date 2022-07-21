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
Hits%: {}%",
      result.tries,
      result.hits_n,
      result.hits_n as f64 / result.tries as f64 * 100.0,
    )),
    ResultOutputFormat::Json => serde_json::to_string(result).unwrap(),
  }
}

#[cfg(test)]
mod test {
  use crate::result::{out, Hit, ResultOutputFormat, WakuchinResult};

  #[test]
  fn test_out() {
    let result = WakuchinResult {
      tries: 10,
      hits_n: 3,
      hits: vec![
        Hit {
          hit_on: 0,
          chars: "a".to_string(),
        },
        Hit {
          hit_on: 1,
          chars: "b".to_string(),
        },
        Hit {
          hit_on: 2,
          chars: "c".to_string(),
        },
      ],
    };

    assert_eq!(
      out(ResultOutputFormat::Text, &result),
      "--- Result ---
Tries: 10
Hits: 3
Hits%: 30%"
    );

    assert_eq!(
      out(ResultOutputFormat::Json, &result),
      r#"{"tries":10,"hits_n":3,"hits":[{"hit_on":0,"chars":"a"},{"hit_on":1,"chars":"b"},{"hit_on":2,"chars":"c"}]}"#
    );
  }
}
