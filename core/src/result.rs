//! Functions to manipulate the result of a research

use std::{borrow::Cow, str::FromStr};

use itertools::Itertools;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};
use smooth::Smooth;

use crate::error::WakuchinError;
use crate::symbol::WAKUCHIN_BYTES;

/// The output format of the result
#[derive(
  Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Deserialize,
)]
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
  #[default]
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

const NOT_A_SYMBOL: u8 = u8::MAX;

/// Byte to two-bit symbol index, derived from [`WAKUCHIN_BYTES`] so the two
/// cannot drift apart. A table rather than a scan of the alphabet because
/// packing runs once per character of every hit.
const SYMBOL_OF_BYTE: [u8; 256] = {
  let mut table = [NOT_A_SYMBOL; 256];
  let mut symbol = 0;

  while symbol < WAKUCHIN_BYTES.len() {
    table[WAKUCHIN_BYTES[symbol] as usize] = symbol as u8;
    symbol += 1;
  }

  table
};

/// Longest candidate, in characters, that fits in a packed `u64`.
const PACKED_CHARS_MAX: usize = 32;

/// Used when the researcher detects a hit
///
/// The characters are stored packed rather than as a `String`. Every candidate
/// is a permutation of [`WAKUCHIN_BYTES`] repeated `times`
/// times, so two bits per character describe it exactly, and up to `times = 8`
/// the whole arrangement fits in a `u64`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
  /// The index of the hit
  pub hit_on: usize,

  chars: HitChars,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum HitChars {
  /// Two bits per character, the least significant first.
  Packed { bits: u64, chars: u8 },

  /// Anything the packing cannot describe.
  Spilled(Box<str>),
}

impl Hit {
  pub fn new(hit_on: usize, chars: &str) -> Self {
    Self {
      hit_on,
      chars: HitChars::new(chars),
    }
  }

  /// Records a hit whose characters are already in the packed form.
  ///
  /// # Panics
  ///
  /// If `chars` exceeds [`PACKED_CHARS_MAX`], or if `bits` has any set bit
  /// above the `chars` characters it claims to describe.
  #[must_use]
  pub(crate) fn from_packed(hit_on: usize, bits: u64, chars: u8) -> Self {
    assert!(
      usize::from(chars) <= PACKED_CHARS_MAX,
      "{chars} characters cannot be packed into a u64"
    );

    let used = usize::from(chars) * 2;

    assert!(
      used == u64::BITS as usize || bits >> used == 0,
      "packed bits describe more than {chars} characters"
    );

    Self {
      hit_on,
      chars: HitChars::Packed { bits, chars },
    }
  }

  /// Wakuchin characters that were hit.
  #[must_use]
  pub fn chars(&self) -> Cow<'_, str> {
    match &self.chars {
      HitChars::Packed { bits, chars } => Cow::Owned(unpack(*bits, *chars)),
      HitChars::Spilled(chars) => Cow::Borrowed(chars),
    }
  }
}

impl HitChars {
  fn new(chars: &str) -> Self {
    match pack(chars) {
      Some((bits, chars)) => Self::Packed { bits, chars },
      None => Self::Spilled(chars.into()),
    }
  }
}

impl Serialize for Hit {
  fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    let mut hit = serializer.serialize_struct("Hit", 2)?;

    hit.serialize_field("hit_on", &self.hit_on)?;
    hit.serialize_field("chars", &*self.chars())?;

    hit.end()
  }
}

/// Returns the packed arrangement and its length, or `None` if `chars` is too
/// long or is not made of wakuchin characters.
fn pack(chars: &str) -> Option<(u64, u8)> {
  let bytes = chars.as_bytes();

  if bytes.len() > PACKED_CHARS_MAX {
    return None;
  }

  let mut bits = 0;

  for (index, byte) in bytes.iter().enumerate() {
    let symbol = SYMBOL_OF_BYTE[usize::from(*byte)];

    if symbol == NOT_A_SYMBOL {
      return None;
    }

    bits |= u64::from(symbol) << (index * 2);
  }

  Some((bits, bytes.len() as u8))
}

fn unpack(bits: u64, chars: u8) -> String {
  (0..usize::from(chars))
    .map(|index| {
      char::from(WAKUCHIN_BYTES[(bits >> (index * 2)) as usize & 0b11])
    })
    .collect()
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
///     Hit::new(0, "WKCN"),
///     Hit::new(1, "WKNC"),
///     Hit::new(2, "WKCN"),
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
      hits_detail: vec![Hit::new(0, "a"), Hit::new(1, "b"), Hit::new(2, "c")],
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
