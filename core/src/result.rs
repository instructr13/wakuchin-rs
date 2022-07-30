//! Functions to manipulate the result of a research

use itertools::Itertools;
use serde::{Deserialize, Serialize};
use smooth::Smooth;

/// The output format of the result
#[derive(Clone, Debug, Deserialize)]
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
  #[serde(rename = "text")]
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
  ///   "hits_detail": [
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
  #[serde(rename = "json")]
  Json,
}

/// Used when the researcher detects a hit
#[derive(Clone, Debug, Serialize)]
pub struct Hit {
  /// The index of the hit
  pub hit_on: usize,

  /// Wakuchin characters that were hit
  pub chars: String,
}

impl Hit {
  pub fn new(hit_on: usize, chars: impl Into<String>) -> Self {
    Self {
      hit_on,
      chars: chars.into(),
    }
  }
}

/// The count of hits that you will use in progress_handler.
#[derive(Clone, Debug, Serialize)]
pub struct HitCounter {
  /// Wakuchin chars that were hit.
  pub chars: String,
  /// The count of hits.
  pub hits: usize,
}

impl HitCounter {
  /// Create new hit counter.
  ///
  /// # Arguments
  ///
  /// * `chars` - Wakuchin chars that were hit.
  /// * `hits` - The count of hits.
  ///
  /// # Returns
  ///
  /// * `HitCounter` - New hit counter.
  pub fn new(chars: impl Into<String>, hits: usize) -> Self {
    Self {
      chars: chars.into(),
      hits,
    }
  }
}

/// The result of a research
#[derive(Debug, Serialize)]
pub struct WakuchinResult {
  /// The number of tries
  pub tries: usize,

  /// Total number of hits
  pub hits_total: usize,

  /// The count of each hits
  pub hits: Vec<HitCounter>,

  /// A vector of `Hit`
  pub hits_detail: Vec<Hit>,
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
/// use wakuchin::result::{out, Hit, HitCounter, ResultOutputFormat, WakuchinResult};
///
/// let result = WakuchinResult {
///   tries: 10,
///   hits_total: 3,
///   hits: vec![
///     HitCounter::new("WKCN", 2),
///     HitCounter::new("WKNC", 1),
///   ],
///   hits_detail: vec![
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
/// WKCN hits: 2 (20%)
/// WKNC hits: 1 (10%)
/// Total hits: 3 (30%)"
/// );
///
/// assert_eq!(
///   out(ResultOutputFormat::Json, &result),
///   r#"{"tries":10,"hits_total":3,"hits":[{"chars":"WKCN","hits":2},{"chars":"WKNC","hits":1}],"hits_detail":[{"hit_on":0,"chars":"WKCN"},{"hit_on":1,"chars":"WKNC"},{"hit_on":2,"chars":"WKCN"}]}"#
/// );
/// ```
pub fn out(format: ResultOutputFormat, result: &WakuchinResult) -> String {
  match format {
    ResultOutputFormat::Text => {
      format!(
        "--- Result ---
Tries: {}
{}
Total hits: {} ({}%)",
        result.tries,
        (&result.hits)
          .iter()
          .map(|h| format!(
            "{} hits: {} ({}%)",
            h.chars,
            h.hits,
            (h.hits as f64 / result.tries as f64 * 100.0).smooth_str()
          ))
          .join("\n"),
        result.hits_total,
        (result.hits_total as f64 / result.tries as f64 * 100.0).smooth_str()
      )
    }
    ResultOutputFormat::Json => serde_json::to_string(result)
      .unwrap_or_else(|e| panic!("error when serializing result: {}", e)),
  }
}

#[cfg(test)]
mod test {
  use crate::result::{
    out, Hit, HitCounter, ResultOutputFormat, WakuchinResult,
  };

  #[test]
  fn test_out() {
    let result = WakuchinResult {
      tries: 10,
      hits_total: 3,
      hits: vec![
        HitCounter::new("a".to_string(), 1),
        HitCounter::new("b".to_string(), 1),
        HitCounter::new("c".to_string(), 1),
      ],
      hits_detail: vec![
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
a hits: 1 (10%)
b hits: 1 (10%)
c hits: 1 (10%)
Total hits: 3 (30%)"
    );

    assert_eq!(
      out(ResultOutputFormat::Json, &result),
      r#"{"tries":10,"hits_total":3,"hits":[{"chars":"a","hits":1},{"chars":"b","hits":1},{"chars":"c","hits":1}],"hits_detail":[{"hit_on":0,"chars":"a"},{"hit_on":1,"chars":"b"},{"hit_on":2,"chars":"c"}]}"#
    );
  }
}
