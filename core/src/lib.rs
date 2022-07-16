mod symbol;

pub mod convert;

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

    wakuchin.chars().for_each(|c| match c {
      symbol::WAKUCHIN_W => wakuchin_w_count += 1,
      symbol::WAKUCHIN_K => wakuchin_k_count += 1,
      symbol::WAKUCHIN_C => wakuchin_c_count += 1,
      symbol::WAKUCHIN_N => wakuchin_n_count += 1,
      _ => panic!("Unexpected char: {}", c),
    });

    assert_eq!(wakuchin_w_count, 3);
    assert_eq!(wakuchin_k_count, 3);
    assert_eq!(wakuchin_c_count, 3);
    assert_eq!(wakuchin_n_count, 3);
  }
}
