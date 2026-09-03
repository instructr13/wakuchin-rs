use std::collections::HashMap;

use crate::result::HitCount;

/// Counts how many times each distinct wakuchin string was hit.
#[derive(Debug, Default)]
pub struct HitTally {
  counts: HashMap<Box<str>, usize>,
}

impl HitTally {
  pub fn add(&mut self, chars: &str) {
    // Only a brand-new string allocates; repeats are one lookup.
    if let Some(hits) = self.counts.get_mut(chars) {
      *hits += 1;
    } else {
      self.counts.insert(chars.into(), 1);
    }
  }

  pub fn merge_from(&mut self, other: &Self) {
    for (chars, hits) in &other.counts {
      *self.counts.entry(chars.clone()).or_default() += hits;
    }
  }

  pub fn clear(&mut self) {
    self.counts.clear();
  }

  /// Fills `out` with the tally, ordered by wakuchin string.
  pub fn write_hit_counts(&self, out: &mut Vec<HitCount>) {
    out.clear();
    out.extend(
      self
        .counts
        .iter()
        .map(|(chars, &hits)| HitCount::new(chars.to_string(), hits)),
    );
    out.sort_unstable_by(|a, b| a.chars.cmp(&b.chars));
  }

  pub fn into_hit_counts(self) -> Vec<HitCount> {
    let mut out = Vec::new();

    self.write_hit_counts(&mut out);

    out
  }
}

#[cfg(test)]
mod test {
  use super::HitTally;

  #[test]
  fn counts_repeats() {
    let mut tally = HitTally::default();

    tally.add("WKCN");
    tally.add("WKNC");
    tally.add("WKCN");

    let counts = tally.into_hit_counts();

    assert_eq!(counts.len(), 2);
    assert_eq!(counts[0].chars, "WKCN");
    assert_eq!(counts[0].hits, 2);
    assert_eq!(counts[1].chars, "WKNC");
    assert_eq!(counts[1].hits, 1);
  }

  #[test]
  fn merging_sums_overlapping_entries() {
    let mut left = HitTally::default();
    let mut right = HitTally::default();

    left.add("WKCN");
    left.add("WKCN");
    right.add("WKCN");
    right.add("NCKW");

    left.merge_from(&right);

    let counts = left.into_hit_counts();

    assert_eq!(counts.len(), 2);
    assert_eq!(counts[0].chars, "NCKW");
    assert_eq!(counts[0].hits, 1);
    assert_eq!(counts[1].chars, "WKCN");
    assert_eq!(counts[1].hits, 3);
  }

  #[test]
  fn output_order_is_stable() {
    let strings = ["WKCN", "NCKW", "CWNK", "KNWC"];

    let counts = |skip: usize| {
      let mut tally = HitTally::default();

      for chars in strings.iter().cycle().skip(skip).take(strings.len()) {
        tally.add(chars);
      }

      tally
        .into_hit_counts()
        .into_iter()
        .map(|count| count.chars.to_string())
        .collect::<Vec<_>>()
    };

    assert_eq!(counts(0), counts(2));
  }
}
