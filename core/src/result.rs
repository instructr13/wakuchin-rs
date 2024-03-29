//! Functions to manipulate the result of a research

use std::{borrow::Cow, str::FromStr};

use itertools::Itertools;
use serde::{Deserialize, Serialize};
use smooth::Smooth;

use crate::error::WakuchinError;

/// The output format of the result
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
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

impl Default for ResultOutputFormat {
  fn default() -> Self {
    Self::Text
  }
}

impl FromStr for ResultOutputFormat {
  type Err = WakuchinError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "text" => Ok(Self::Text),
      "json" => Ok(Self::Json),
      _ => Err(WakuchinError::UnknownResultOutputFormat(s.into())),
    }
  }
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

/// The count of hits you will use in `progress_handler`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HitCount {
  /// Wakuchin chars that were hit.
  pub chars: Cow<'static, str>,
  /// The count of hits.
  pub hits: usize,
}

impl HitCount {
  /// Create new hit counter.
  ///
  /// # Arguments
  ///
  /// * `chars` - Wakuchin chars that were hit.
  /// * `hits` - The count of hits.
  ///
  /// # Returns
  ///
  /// * `HitCount` - New hit counter.
  pub fn new(chars: impl Into<Cow<'static, str>>, hits: usize) -> Self {
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
  pub hits: Vec<HitCount>,

  /// A vector of `Hit`
  pub hits_detail: Vec<Hit>,
}

impl WakuchinResult {
  /// Return string of the result with specific output format.
  /// This function is a wrapper of `out`.
  #[inline]
  pub fn out(
    &self,
    format: ResultOutputFormat,
  ) -> Result<String, WakuchinError> {
    out(format, self)
  }
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
/// * `String` - the formatted result
///
/// # Examples
///
/// ```rust
/// use wakuchin::result::{out, Hit, HitCount, ResultOutputFormat, WakuchinResult};
///
/// let result = WakuchinResult {
///   tries: 10,
///   hits_total: 3,
///   hits: vec![
///     HitCount::new("WKCN", 2),
///     HitCount::new("WKNC", 1),
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
///   out(ResultOutputFormat::Text, &result)?,
///   "--- Result ---
/// Tries: 10
/// WKCN hits: 2 (20%)
/// WKNC hits: 1 (10%)
/// Total hits: 3 (30%)"
/// );
///
/// assert_eq!(
///   out(ResultOutputFormat::Json, &result)?,
///   r#"{"tries":10,"hits_total":3,"hits":[{"chars":"WKCN","hits":2},{"chars":"WKNC","hits":1}],"hits_detail":[{"hit_on":0,"chars":"WKCN"},{"hit_on":1,"chars":"WKNC"},{"hit_on":2,"chars":"WKCN"}]}"#
/// );
/// #
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn out(
  format: ResultOutputFormat,
  result: &WakuchinResult,
) -> Result<String, WakuchinError> {
  let mut itoa_buf = itoa::Buffer::new();

  match format {
    ResultOutputFormat::Text => Ok(format!(
      "--- Result ---
Tries: {}
{}
Total hits: {} ({}%)",
      result.tries,
      result
        .hits
        .iter()
        .map(|h| format!(
          "{} hits: {} ({}%)",
          h.chars,
          itoa_buf.format(h.hits),
          (h.hits as f64 / result.tries as f64 * 100.0).smooth_str()
        ))
        .join("\n"),
      itoa_buf.format(result.hits_total),
      (result.hits_total as f64 / result.tries as f64 * 100.0).smooth_str()
    )),
    ResultOutputFormat::Json => Ok(
      serde_json::to_string(result)
        .map_err(|e| WakuchinError::SerializeError(e.into()))?,
    ),
  }
}

#[cfg(test)]
mod test {
  use std::error::Error;

  use crate::result::{out, Hit, HitCount, ResultOutputFormat, WakuchinResult};

  #[test]
  fn test_out() -> Result<(), Box<dyn Error>> {
    let result = WakuchinResult {
      tries: 10,
      hits_total: 3,
      hits: vec![
        HitCount::new("a".to_string(), 1),
        HitCount::new("b".to_string(), 1),
        HitCount::new("c".to_string(), 1),
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
      out(ResultOutputFormat::Text, &result)?,
      "--- Result ---
Tries: 10
a hits: 1 (10%)
b hits: 1 (10%)
c hits: 1 (10%)
Total hits: 3 (30%)"
    );

    assert_eq!(
      out(ResultOutputFormat::Json, &result)?,
      r#"{"tries":10,"hits_total":3,"hits":[{"chars":"a","hits":1},{"chars":"b","hits":1},{"chars":"c","hits":1}],"hits_detail":[{"hit_on":0,"chars":"a"},{"hit_on":1,"chars":"b"},{"hit_on":2,"chars":"c"}]}"#
    );

    Ok(())
  }
}
