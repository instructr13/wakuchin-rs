//! Core functions of wakuchin tools

pub mod convert;
pub mod result;
pub mod symbol;
pub mod worker;

use regex::Regex;

use rand::prelude::SliceRandom;

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
/// use wakuchin_core::gen;
/// use wakuchin_core::symbol;
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
pub fn gen(times: usize) -> String {
  let mut wakuchin: Vec<char> = symbol::WAKUCHIN
    .iter()
    .cycle()
    .take(symbol::WAKUCHIN.len() * times)
    .map(|&c| c)
    .collect();

  let mut rng = rand::thread_rng();

  wakuchin.shuffle(&mut rng);

  wakuchin.iter().collect()
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
/// use wakuchin_core::gen_vec;
/// use wakuchin_core::symbol;
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
pub fn gen_vec(len: usize, times: usize) -> Vec<String> {
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
/// use wakuchin_core::validate;
///
/// assert_eq!(validate("WKCN"), true);
/// assert_eq!(validate("わくちん"), false);
/// assert_eq!(validate("WKCNX"), false);
/// ```
pub fn validate(wakuchin: &str) -> bool {
  wakuchin.chars().all(|c| symbol::WAKUCHIN.contains(&c))
}

/// Check if a string is a wakuchin string.
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
/// use wakuchin_core::validate_external;
///
/// assert_eq!(validate_external("わくちん"), true);
/// assert_eq!(validate_external("WKCN"), false);
/// assert_eq!(validate_external("わくうちん"), false);
/// assert_eq!(validate_external("WKCNX"), false);
/// ```
pub fn validate_external(wakuchin: &str) -> bool {
  wakuchin
    .chars()
    .all(|c| symbol::WAKUCHIN_EXTERNAL.contains(&c))
}

/// Check wakuchin string with specified regular expression.
/// This function is a wrapper of `Regex::is_match`.
///
/// # Arguments
///
/// * `chars` - wakuchin string to check
/// * `regex` - regular expression to use
///
/// # Returns
///
/// * `bool` - true if wakuchin string is valid
///
/// # Examples
///
/// ```rust
/// use regex::Regex;
///
/// use wakuchin_core::check;
///
/// assert_eq!(check("WKCN", Regex::new(r"^[WKCN]+$").unwrap()), true);
/// assert_eq!(check("わくちん", Regex::new(r"^[WKCN]+$").unwrap()), false);
/// assert_eq!(check("WKCNX", Regex::new(r"^[WKCN]+$").unwrap()), false);
/// ```
pub fn check(chars: &str, regex: Regex) -> bool {
  regex.is_match(chars)
}

#[cfg(test)]
mod test {
  use regex::Regex;

  use crate::{check, gen, gen_vec, symbol, validate, validate_external};

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
    assert_eq!(validate("WKCN"), true);
    assert_eq!(validate("わくちん"), false);
    assert_eq!(validate("WKCNX"), false);
  }

  #[test]
  fn test_validate_external() {
    assert_eq!(validate_external("わくちん"), true);
    assert_eq!(validate_external("WKCN"), false);
    assert_eq!(validate_external("わくうちん"), false);
    assert_eq!(validate_external("WKCNX"), false);
  }

  #[test]
  fn test_check() {
    assert_eq!(check("WKCN", Regex::new(r"^[WKCN]+$").unwrap()), true);
    assert_eq!(check("わくちん", Regex::new(r"^[WKCN]+$").unwrap()), false);
    assert_eq!(check("WKCNX", Regex::new(r"^[WKCN]+$").unwrap()), false);
  }
}
