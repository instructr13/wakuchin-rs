//! Functions to manipulate the result of a research

use std::fmt;

use serde::Serialize;

/// The output format of the result
#[derive(Clone)]
pub enum ResultOutputFormat {
  /// Text output
  ///
  /// # Example
  ///
  /// The expected output would be:
  /// ```txt
  /// --- Result ---
  /// Tries: 10
  /// Hits: 2
  /// Hits%: 20%
  /// ```
  Text,
  /// JSON output
  ///
  /// # Example
  ///
  /// The expected output would be:
  /// ```json
  /// {
  ///   "tries": 10,
  ///   "hits_n": 2,
  ///   "hits": [
  ///     {
  ///       "hit_on": 0,
  ///       "chars": "WKCN"
  ///     },
  ///     {
  ///       "hit_on": 5,
  ///       "chars": "WKCN"
  ///     }
  ///   ]
  /// }
  /// ```
  Json,
}

/// Used when the researcher detects a hit
#[derive(Debug, Serialize)]
pub struct Hit {
  /// The index of the hit
  pub hit_on: usize,
  /// Wakuchin characters that were hit
  pub chars: String,
}

/// The result of a research
#[derive(Debug, Serialize)]
pub struct WakuchinResult {
  /// The number of tries
  pub tries: usize,
  /// The number of hits
  pub hits_n: usize,
  /// A vector of `Hit`
  pub hits: Vec<Hit>,
}

/// Return string of the result with specific output format.
///
/// # Arguments
///
/// * `format` - the output format of the result
/// * `result` - the result to format
///
/// # Returns
///
/// * `String` - t0e formatted result
///
/// # Examples
///
/// ```rust
/// use wakuchin_core::result::{out, Hit, ResultOutputFormat, WakuchinResult};
///
/// let result = WakuchinResult {
///   tries: 10,
///   hits_n: 3,
///   hits: vec![
///     Hit {
///       hit_on: 0,
///       chars: "WKCN".to_string(),
///     },
///     Hit {
///       hit_on: 1,
///       chars: "WKNC".to_string(),
///     },
///     Hit {
///       hit_on: 2,
///       chars: "WKCN".to_string(),
///     },
///   ],
/// };
///
/// assert_eq!(
///   out(ResultOutputFormat::Text, &result),
///   "--- Result ---
/// Tries: 10
/// Hits: 3
/// Hits%: 30%"
/// );
///
/// assert_eq!(
///   out(ResultOutputFormat::Json, &result),
///   r#"{"tries":10,"hits_n":3,"hits":[{"hit_on":0,"chars":"WKCN"},{"hit_on":1,"chars":"WKNC"},{"hit_on":2,"chars":"WKCN"}]}"#
/// );
/// ```
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
    ResultOutputFormat::Json => serde_json::to_string(result)
      .unwrap_or_else(|e| panic!("error when serializing result: {}", e)),
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
