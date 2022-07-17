mod symbol;

pub mod convert;
pub mod result;
pub mod worker;

use regex::Regex;

use rand::prelude::SliceRandom;

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

pub fn gen_vec(len: usize, times: usize) -> Vec<String> {
  (0..len).map(|_| gen(times)).collect()
}

pub fn validate(wakuchin: &str) -> bool {
  wakuchin.chars().all(|c| symbol::WAKUCHIN.contains(&c))
}

pub fn check(chars: &str, regex: Regex) -> bool {
  regex.is_match(chars)
}

#[cfg(test)]
mod test {
  use crate::{gen, symbol};

  #[test]
  fn gen_works() {
    let wakuchin = gen(3);
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
}
