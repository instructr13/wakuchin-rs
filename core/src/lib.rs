//! Core functions of wakuchin tools

pub mod builder;
pub mod convert;
pub mod error;
pub mod handlers;
pub mod progress;
pub mod result;
pub mod symbol;
pub mod worker;

mod gate;
mod hit;
mod render;
mod slot;
mod utils;

use std::borrow::Cow;
use std::str;

use regex::bytes::Regex;

use crate::symbol::WAKUCHIN_BYTES;

/// Generates random wakuchin candidates into a buffer it owns.
#[derive(Clone, Debug)]
pub struct Generator {
  buf: Vec<u8>,
  rng: fastrand::Rng,
}

impl Generator {
  /// # Arguments
  ///
  /// * `times` - wakuchin times n
  ///
  /// # Examples
  ///
  /// ```rust
  /// use wakuchin::Generator;
  ///
  /// let mut generator = Generator::new(2);
  ///
  /// generator.shuffle();
  ///
  /// assert_eq!(generator.bytes().len(), 8);
  /// ```
  #[must_use]
  pub fn new(times: usize) -> Self {
    Self {
      buf: WAKUCHIN_BYTES.repeat(times),
      // Owned rather than the `fastrand` free functions, which reach through a
      // thread local on every call.
      rng: fastrand::Rng::new(),
    }
  }

  /// Replaces the current candidate with a fresh one. Allocates nothing.
  #[inline]
  pub fn shuffle(&mut self) {
    self.rng.shuffle(&mut self.buf);
  }

  /// The current candidate's bytes.
  #[inline]
  #[must_use]
  pub fn bytes(&self) -> &[u8] {
    &self.buf
  }

  /// The current candidate.
  ///
  /// # Panics
  ///
  /// Never in practice: the buffer only ever holds a permutation of
  /// [`WAKUCHIN_BYTES`], which is ASCII.
  #[inline]
  #[must_use]
  pub fn as_str(&self) -> &str {
    str::from_utf8(&self.buf).expect("wakuchin candidates are ASCII")
  }
}

/// Generate a randomized wakuchin string.
///
/// # Arguments
///
/// * `times` - wakuchin times n
///
/// # Returns
///
/// * `String` - randomized wakuchin string
///
/// # Examples
///
/// ```rust
/// use wakuchin::gen;
/// use wakuchin::symbol;
///
/// let wakuchin = gen(3);
///
/// assert_eq!(wakuchin.len(), 12);
///
/// let mut wakuchin_w_count: u32 = 0;
/// let mut wakuchin_k_count: u32 = 0;
/// let mut wakuchin_c_count: u32 = 0;
/// let mut wakuchin_n_count: u32 = 0;
///
/// for c in wakuchin.chars() {
///   match c {
///     symbol::WAKUCHIN_W => wakuchin_w_count += 1,
///     symbol::WAKUCHIN_K => wakuchin_k_count += 1,
///     symbol::WAKUCHIN_C => wakuchin_c_count += 1,
///     symbol::WAKUCHIN_N => wakuchin_n_count += 1,
///     _ => panic!("Unexpected character: {}", c),
///   }
/// }
///
/// assert_eq!(wakuchin_w_count, 3);
/// assert_eq!(wakuchin_k_count, 3);
/// assert_eq!(wakuchin_c_count, 3);
/// assert_eq!(wakuchin_n_count, 3);
/// ```
#[inline]
pub fn gen(times: usize) -> Cow<'static, str> {
  let mut wakuchin = WAKUCHIN_BYTES.repeat(times);

  fastrand::shuffle(&mut wakuchin);

  String::from_utf8(wakuchin)
    .expect("wakuchin candidates are ASCII")
    .into()
}

/// Generate a vector of randomized wakuchin string.
/// This function is useful when you want to generate multiple wakuchin strings.
///
/// # Arguments
///
/// * `len` - length of vector to generate
/// * `times` - wakuchin times n
///
/// # Returns
///
/// * `Vec<String>` - vector of randomized wakuchin string
///
/// # Examples
///
/// ```rust
/// use wakuchin::gen_vec;
/// use wakuchin::symbol;
///
/// let wakuchin_vec = gen_vec(3, 3);
///
/// assert_eq!(wakuchin_vec.len(), 3);
///
/// let mut wakuchin_w_count: u32 = 0;
/// let mut wakuchin_k_count: u32 = 0;
/// let mut wakuchin_c_count: u32 = 0;
/// let mut wakuchin_n_count: u32 = 0;
///
/// for wakuchin in wakuchin_vec {
///   assert_eq!(wakuchin.len(), 12);
///
///   for c in wakuchin.chars() {
///     match c {
///       symbol::WAKUCHIN_W => wakuchin_w_count += 1,
///       symbol::WAKUCHIN_K => wakuchin_k_count += 1,
///       symbol::WAKUCHIN_C => wakuchin_c_count += 1,
///       symbol::WAKUCHIN_N => wakuchin_n_count += 1,
///       _ => panic!("Unexpected character: {}", c),
///     }
///   }
/// }
///
/// assert_eq!(wakuchin_w_count, 9);
/// assert_eq!(wakuchin_k_count, 9);
/// assert_eq!(wakuchin_c_count, 9);
/// assert_eq!(wakuchin_n_count, 9);
/// ```
#[inline]
pub fn gen_vec(len: usize, times: usize) -> Vec<Cow<'static, str>> {
  (0..len).map(|_| gen(times)).collect()
}

/// Check if a string is a internally used wakuchin string.
///
/// # Arguments
///
/// * `wakuchin` - internal wakuchin string to check
///
/// # Returns
///
/// * `bool` - true if internal wakuchin string is valid
///
/// # Examples
///
/// ```rust
/// use wakuchin::validate;
///
/// assert!(validate("WKCN"));
/// assert!(!validate("わくちん"));
/// assert!(!validate("WKCNX"));
/// ```
pub fn validate(wakuchin: &str) -> bool {
  wakuchin.chars().all(|c| symbol::WAKUCHIN.contains(&c))
}

/// Check whether the string is a valid wakuchin characters.
///
/// # Arguments
///
/// * `wakuchin` - wakuchin string to check
///
/// # Returns
///
/// * `bool` - true if wakuchin string is valid
///
/// # Examples
///
/// ```rust
/// use wakuchin::validate_external;
///
/// assert!(validate_external("わくちん"));
/// assert!(!validate_external("WKCN"));
/// assert!(!validate_external("わくうちん"));
/// assert!(!validate_external("WKCNX"));
/// ```
pub fn validate_external(wakuchin: &str) -> bool {
  wakuchin
    .chars()
    .all(|c| symbol::WAKUCHIN_EXTERNAL.contains(&c))
}

/// Check wakuchin candidate bytes with specified regular expression.
/// This function is a wrapper of `regex::bytes::Regex::is_match`.
///
/// # Arguments
///
/// * `chars` - wakuchin candidate bytes to check
/// * `regex` - regular expression to use
///
/// # Returns
///
/// * `bool` - true if wakuchin string is valid
///
/// # Examples
///
/// ```rust
/// use regex::bytes::Regex;
///
/// use wakuchin::check;
///
/// assert!(check(b"WKCN", &Regex::new(r"^[WKCN]+$").unwrap()));
/// assert!(!check("わくちん".as_bytes(), &Regex::new(r"^[WKCN]+$").unwrap()));
/// assert!(!check(b"WKCNX", &Regex::new(r"^[WKCN]+$").unwrap()));
/// ```
#[inline(always)]
pub fn check(chars: &[u8], regex: &Regex) -> bool {
  regex.is_match(chars)
}

#[cfg(test)]
mod test {
  use std::error::Error;

  use regex::bytes::Regex;

  use crate::{
    check, gen, gen_vec, symbol, validate, validate_external, Generator,
  };

  #[test]
  fn test_gen() {
    let wakuchin = gen(3);

    assert_eq!(wakuchin.len(), 12);

    let mut wakuchin_w_count: u32 = 0;
    let mut wakuchin_k_count: u32 = 0;
    let mut wakuchin_c_count: u32 = 0;
    let mut wakuchin_n_count: u32 = 0;

    for c in wakuchin.chars() {
      match c {
        symbol::WAKUCHIN_W => wakuchin_w_count += 1,
        symbol::WAKUCHIN_K => wakuchin_k_count += 1,
        symbol::WAKUCHIN_C => wakuchin_c_count += 1,
        symbol::WAKUCHIN_N => wakuchin_n_count += 1,
        _ => panic!("Unexpected character: {}", c),
      }
    }

    assert_eq!(wakuchin_w_count, 3);
    assert_eq!(wakuchin_k_count, 3);
    assert_eq!(wakuchin_c_count, 3);
    assert_eq!(wakuchin_n_count, 3);
  }

  #[test]
  fn test_gen_vec() {
    let wakuchin_vec = gen_vec(3, 3);

    assert_eq!(wakuchin_vec.len(), 3);

    let mut wakuchin_w_count: u32 = 0;
    let mut wakuchin_k_count: u32 = 0;
    let mut wakuchin_c_count: u32 = 0;
    let mut wakuchin_n_count: u32 = 0;

    for wakuchin in wakuchin_vec {
      assert_eq!(wakuchin.len(), 12);

      for c in wakuchin.chars() {
        match c {
          symbol::WAKUCHIN_W => wakuchin_w_count += 1,
          symbol::WAKUCHIN_K => wakuchin_k_count += 1,
          symbol::WAKUCHIN_C => wakuchin_c_count += 1,
          symbol::WAKUCHIN_N => wakuchin_n_count += 1,
          _ => panic!("Unexpected character: {}", c),
        }
      }
    }

    assert_eq!(wakuchin_w_count, 9);
    assert_eq!(wakuchin_k_count, 9);
    assert_eq!(wakuchin_c_count, 9);
    assert_eq!(wakuchin_n_count, 9);
  }

  #[test]
  fn test_validate() {
    assert!(validate("WKCN"));
    assert!(!validate("わくちん"));
    assert!(!validate("WKCNX"));
  }

  #[test]
  fn test_validate_external() {
    assert!(validate_external("わくちん"));
    assert!(!validate_external("WKCN"));
    assert!(!validate_external("わくうちん"));
    assert!(!validate_external("WKCNX"));
  }

  #[test]
  fn test_check() -> Result<(), Box<dyn Error>> {
    assert!(check(b"WKCN", &Regex::new(r"^[WKCN]+$")?));
    assert!(!check("わくちん".as_bytes(), &Regex::new(r"^[WKCN]+$")?));
    assert!(!check(b"WKCNX", &Regex::new(r"^[WKCN]+$")?));

    Ok(())
  }

  #[test]
  fn test_generator() {
    let mut generator = Generator::new(3);

    for _ in 0..16 {
      generator.shuffle();

      assert_eq!(generator.bytes().len(), 12);
      assert_eq!(generator.as_str().len(), 12);

      for symbol in symbol::WAKUCHIN_BYTES {
        assert_eq!(
          generator.bytes().iter().filter(|&&b| b == symbol).count(),
          3
        );
      }
    }
  }

  #[test]
  fn test_generator_reuse_stays_uniform() {
    const SAMPLES: usize = 240_000;

    let mut generator = Generator::new(1);
    let mut counts = std::collections::HashMap::new();

    for _ in 0..SAMPLES {
      generator.shuffle();

      *counts
        .entry(generator.as_str().to_owned())
        .or_insert(0_usize) += 1;
    }

    assert_eq!(counts.len(), 24, "not every arrangement was produced");

    let expected = SAMPLES as f64 / 24.0;
    let chi_square: f64 = counts
      .values()
      .map(|&observed| {
        let deviation = observed as f64 - expected;

        deviation * deviation / expected
      })
      .sum();

    assert!(
      chi_square < 49.7,
      "chi-square {chi_square} suggests a non-uniform distribution"
    );
  }
}
