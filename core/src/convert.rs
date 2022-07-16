use super::symbol;

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

pub fn chars_to_wakuchin(chars: &str) -> String {
  String::from_iter(chars.chars().map(self::char_to_wakuchin))
}

pub fn wakuchin_to_chars(chars: &str) -> String {
  String::from_iter(chars.chars().map(self::wakuchin_to_char))
}

#[cfg(test)]
mod test {
  use crate::{convert, symbol};

  #[test]
  fn char_to_wakuchin_works() {
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
  fn wakuchin_to_char_works() {
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
  fn chars_to_wakuchin_works() {
    assert_eq!(convert::chars_to_wakuchin("WKNCWKNC"), "わくんちわくんち");
  }

  #[test]
  fn wakuchin_to_chars_works() {
    assert_eq!(convert::wakuchin_to_chars("わくんちわくんち"), "WKNCWKNC");
  }
}
