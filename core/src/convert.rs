//! Wakuchin conversion functions

use crate::symbol;

fn char_to_wakuchin(char: char) -> char {
  match char {
    symbol::WAKUCHIN_W => symbol::WAKUCHIN_EXTERNAL_W,
    symbol::WAKUCHIN_K => symbol::WAKUCHIN_EXTERNAL_K,
    symbol::WAKUCHIN_C => symbol::WAKUCHIN_EXTERNAL_C,
    symbol::WAKUCHIN_N => symbol::WAKUCHIN_EXTERNAL_N,
    _ => '\0',
  }
}

fn wakuchin_to_char(char: char) -> char {
  match char {
    symbol::WAKUCHIN_EXTERNAL_W => symbol::WAKUCHIN_W,
    symbol::WAKUCHIN_EXTERNAL_K => symbol::WAKUCHIN_K,
    symbol::WAKUCHIN_EXTERNAL_C => symbol::WAKUCHIN_C,
    symbol::WAKUCHIN_EXTERNAL_N => symbol::WAKUCHIN_N,
    _ => '\0',
  }
}

/// Convert from internally used wakuchin chars to actual wakuchin chars.
/// This is useful when you want to display the wakuchin chars to the user.
///
/// # Arguments
///
/// * `chars` - internal wakuchin chars to convert
///
/// # Returns
///
/// * `String` - actual wakuchin chars
///
/// # Examples
///
/// ```rust
/// use wakuchin_core::convert::chars_to_wakuchin;
///
/// assert_eq!(chars_to_wakuchin("WKCN"), "わくちん");
/// assert_eq!(chars_to_wakuchin("WKNCWKNC"), "わくんちわくんち");
/// ```
pub fn chars_to_wakuchin(chars: &str) -> String {
  String::from_iter(chars.chars().map(self::char_to_wakuchin))
}

/// Convert from actual wakuchin chars to internally used wakuchin chars.
/// This is the inverse of `chars_to_wakuchin`.
///
/// # Arguments
///
/// * `chars` - actual wakuchin chars to convert
///
/// # Returns
///
/// * `String` - internal wakuchin chars
///
/// # Examples
///
/// ```rust
/// use wakuchin_core::convert::wakuchin_to_chars;
///
/// assert_eq!(wakuchin_to_chars("わくちん"), "WKCN");
/// assert_eq!(wakuchin_to_chars("わくんちわくんち"), "WKNCWKNC");
/// ```
pub fn wakuchin_to_chars(chars: &str) -> String {
  String::from_iter(chars.chars().map(self::wakuchin_to_char))
}

#[cfg(test)]
mod test {
  use crate::{convert, symbol};

  #[test]
  fn test_char_to_wakuchin() {
    assert_eq!(
      convert::char_to_wakuchin(symbol::WAKUCHIN_W),
      symbol::WAKUCHIN_EXTERNAL_W
    );
    assert_eq!(
      convert::char_to_wakuchin(symbol::WAKUCHIN_K),
      symbol::WAKUCHIN_EXTERNAL_K
    );
    assert_eq!(
      convert::char_to_wakuchin(symbol::WAKUCHIN_C),
      symbol::WAKUCHIN_EXTERNAL_C
    );
    assert_eq!(
      convert::char_to_wakuchin(symbol::WAKUCHIN_N),
      symbol::WAKUCHIN_EXTERNAL_N
    );
    assert_eq!(convert::char_to_wakuchin('a'), '\0');
  }

  #[test]
  fn test_wakuchin_to_char() {
    assert_eq!(
      convert::wakuchin_to_char(symbol::WAKUCHIN_EXTERNAL_W),
      symbol::WAKUCHIN_W
    );
    assert_eq!(
      convert::wakuchin_to_char(symbol::WAKUCHIN_EXTERNAL_K),
      symbol::WAKUCHIN_K
    );
    assert_eq!(
      convert::wakuchin_to_char(symbol::WAKUCHIN_EXTERNAL_C),
      symbol::WAKUCHIN_C
    );
    assert_eq!(
      convert::wakuchin_to_char(symbol::WAKUCHIN_EXTERNAL_N),
      symbol::WAKUCHIN_N
    );
    assert_eq!(convert::wakuchin_to_char('a'), '\0');
  }

  #[test]
  fn test_chars_to_wakuchin() {
    assert_eq!(convert::chars_to_wakuchin("WKNCWKNC"), "わくんちわくんち");
  }

  #[test]
  fn test_wakuchin_to_chars() {
    assert_eq!(convert::wakuchin_to_chars("わくんちわくんち"), "WKNCWKNC");
  }
}
