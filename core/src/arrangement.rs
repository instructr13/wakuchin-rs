use fixedbitset::FixedBitSet;

use crate::pattern::Pattern;
use crate::result::Hit;
use crate::symbol::WAKUCHIN_BYTES;
use crate::MAX_TIMES;

/// Largest hot bitset we are willing to build, in bytes.
const BITSET_BYTES_MAX: usize = 64 * 1024;

/// Every candidate a run can produce, with the pattern's verdict on each one
/// already computed.
#[derive(Debug)]
pub struct Arrangements {
  /// One bit per arrangement, indexed by lexicographic rank. Hot: probed once
  /// per try.
  matches: FixedBitSet,

  /// Each arrangement in [`Hit`]'s packed form, same order.
  /// Cold: read only when a draw hits.
  packed: Box<[u64]>,

  /// Characters per candidate, i.e. `4 * times`.
  chars: u8,
}

impl Arrangements {
  /// Precomputes the verdict for every candidate, or returns `None` when that
  /// is not worth doing.
  #[must_use]
  pub fn precompute(
    pattern: &Pattern,
    times: usize,
    tries: usize,
  ) -> Option<Self> {
    if tries < count(times)? {
      return None;
    }

    Self::enumerate(pattern, times)
  }

  /// Evaluates the pattern for every candidate, regardless of how many tries
  /// the run will make.
  ///
  /// Used by the exact answer, which examines the space rather than sampling
  /// it, so there is nothing to amortize. `None` only when the space is too
  /// large to enumerate.
  #[must_use]
  pub fn enumerate(pattern: &Pattern, times: usize) -> Option<Self> {
    let arrangements = count(times)?;

    if arrangements.div_ceil(8) > BITSET_BYTES_MAX {
      return None;
    }

    let chars = u8::try_from(times * WAKUCHIN_BYTES.len()).ok()?;

    let mut candidate = sorted(times);
    let mut matches = FixedBitSet::with_capacity(arrangements);
    let mut packed = Vec::with_capacity(arrangements);
    let mut matcher = pattern.matcher();
    let mut rank = 0;

    // One walk fills both structures, so a rank means the same thing to the
    // bitset and to the packed table by construction -- there is no second
    // ordering to get wrong.
    loop {
      if matcher.is_match(&candidate) {
        matches.insert(rank);
      }

      packed.push(pack(&candidate));

      rank += 1;

      if !next_permutation(&mut candidate) {
        break;
      }
    }

    debug_assert_eq!(
      rank, arrangements,
      "enumeration disagrees with the count"
    );

    Some(Self {
      matches,
      packed: packed.into_boxed_slice(),
      chars,
    })
  }

  /// Number of distinct candidates.
  #[must_use]
  pub fn len(&self) -> usize {
    self.packed.len()
  }

  /// Number of candidates the pattern matches.
  ///
  /// `hits() / len()` is the exact probability that one try hits -- no
  /// sampling error.
  #[must_use]
  pub fn hits(&self) -> usize {
    self.matches.count_ones(..)
  }

  #[inline]
  #[must_use]
  pub fn is_match(&self, rank: usize) -> bool {
    self.matches.contains(rank)
  }

  /// Writes candidate `rank` into `buf`, which must hold `4 * times` bytes.
  ///
  /// Only called when a draw hits, and once per chunk for the progress
  /// display, so it reads the cold table rather than the hot bitset.
  pub fn write(&self, rank: usize, buf: &mut [u8]) {
    let bits = self.packed[rank];

    for (index, slot) in buf.iter_mut().enumerate() {
      *slot = WAKUCHIN_BYTES[(bits >> (index * 2)) as usize & 0b11];
    }
  }

  /// Records a hit at `rank` without materialising its characters.
  pub fn hit(&self, hit_on: usize, rank: usize) -> Hit {
    Hit::from_packed(hit_on, self.packed[rank], self.chars)
  }

  /// Every candidate the pattern matches, as a hit recorded at its own rank.
  pub fn matching(&self) -> impl Iterator<Item = Hit> + '_ {
    self.matches.ones().map(|rank| self.hit(rank, rank))
  }
}

/// `(4n)!/(n!)^4`, or `None` if it does not fit in a `usize`.
///
/// Computed as a product of binomial coefficients so the intermediate values
/// stay as small as possible: `(4n choose n) * (3n choose n) * (2n choose n)`.
/// A plain factorial would overflow at `n = 5` while the answer still fits.
fn count(times: usize) -> Option<usize> {
  if times == 0 || times > MAX_TIMES {
    return None;
  }

  let mut total = 1_usize;

  for group in 1..WAKUCHIN_BYTES.len() {
    total = total.checked_mul(binomial(group * times + times, times)?)?;
  }

  Some(total)
}

/// `n choose k`, or `None` on overflow.
fn binomial(n: usize, k: usize) -> Option<usize> {
  let mut result = 1_usize;

  for step in 1..=k {
    // Exact at every step: `result` is always `(n choose step-1)` times the
    // partial numerator, and the division is guaranteed to be exact.
    result = result.checked_mul(n - k + step)? / step;
  }

  Some(result)
}

/// The lexicographically first candidate: the alphabet, sorted, each symbol
/// repeated `times` times.
fn sorted(times: usize) -> Vec<u8> {
  let mut candidate = WAKUCHIN_BYTES.repeat(times);

  candidate.sort_unstable();
  candidate
}

/// Replaces `candidate` with its lexicographic successor, or returns `false`
/// when it is already the last one.
///
/// Walking permutations this way visits each *distinct* arrangement of a
/// multiset exactly once, in rank order, and allocates nothing. Building the
/// tables from one such walk is what keeps the bitset and the packed table in
/// agreement.
fn next_permutation(candidate: &mut [u8]) -> bool {
  let Some(pivot) = candidate.windows(2).rposition(|pair| pair[0] < pair[1])
  else {
    return false;
  };

  let successor = candidate
    .iter()
    .rposition(|byte| *byte > candidate[pivot])
    .expect("a pivot implies a larger byte to its right");

  candidate.swap(pivot, successor);
  candidate[pivot + 1..].reverse();

  true
}

/// Two bits per character, least significant first, matching
/// [`crate::result::Hit`]'s packed form.
fn pack(candidate: &[u8]) -> u64 {
  candidate.iter().enumerate().fold(0, |bits, (index, byte)| {
    let symbol = WAKUCHIN_BYTES
      .iter()
      .position(|slot| slot == byte)
      .expect("candidates are made of wakuchin bytes");

    bits | (symbol as u64) << (index * 2)
  })
}

#[cfg(test)]
mod test {
  use std::collections::HashSet;

  use crate::pattern::Pattern;
  use crate::symbol::WAKUCHIN_BYTES;

  use super::{count, next_permutation, sorted, Arrangements};

  /// Every candidate this crate can generate, in lexicographic order.
  fn walk(times: usize) -> Vec<Vec<u8>> {
    let mut candidate = sorted(times);
    let mut all = Vec::new();

    loop {
      all.push(candidate.clone());

      if !next_permutation(&mut candidate) {
        break;
      }
    }

    all
  }

  #[test]
  fn counts_the_arrangement_space() {
    assert_eq!(count(0), None);
    assert_eq!(count(1), Some(24));
    assert_eq!(count(2), Some(2_520));
    assert_eq!(count(3), Some(369_600));
    assert_eq!(count(4), Some(63_063_000));
    assert_eq!(count(8), Some(99_561_092_450_391_000));

    // Past `MAX_TIMES` the answer stops fitting in a u64, so it must refuse
    // rather than wrap.
    assert_eq!(count(9), None);
  }

  /// The walk has to visit each distinct arrangement exactly once. If it
  /// visited one twice, or skipped one, the bitset would describe a different
  /// space than the sampler draws from.
  #[test]
  fn the_walk_is_a_bijection_onto_the_space() {
    for times in 1..=3 {
      let all = walk(times);
      let unique: HashSet<_> = all.iter().collect();

      assert_eq!(all.len(), count(times).unwrap(), "times = {times}");
      assert_eq!(unique.len(), all.len(), "times = {times}: duplicates");

      for candidate in &all {
        assert_eq!(candidate.len(), times * 4);

        for symbol in WAKUCHIN_BYTES {
          let seen = candidate.iter().filter(|byte| **byte == symbol).count();

          assert_eq!(seen, times, "times = {times}: wrong symbol count");
        }
      }

      assert!(all.windows(2).all(|pair| pair[0] < pair[1]), "not sorted");
    }
  }

  /// The exhaustive proof that memoisation did not change the predicate: for
  /// every candidate the run could ever produce, the bit agrees with the regex.
  #[test]
  fn every_bit_agrees_with_the_pattern() {
    let patterns = [
      r"^WKNCWKNC$",
      r"WKNCWKNC",
      r"(WKNCWKNC|WCKNWCKN)",
      r"^W",
      r"W",
      r"(?i)wkcn",
      r"^[WKCN]+$",
      r"N$",
    ];

    for times in 1..=3 {
      let all = walk(times);

      for source in patterns {
        let pattern = Pattern::new(source).unwrap();
        let mut matcher = pattern.matcher();

        // `tries` large enough that the gate always opens.
        let arrangements =
          Arrangements::precompute(&pattern, times, usize::MAX).unwrap();

        assert_eq!(arrangements.len(), all.len());

        let mut expected_hits = 0;

        for (rank, candidate) in all.iter().enumerate() {
          let wanted = matcher.is_match(candidate);

          assert_eq!(
            arrangements.is_match(rank),
            wanted,
            "/{source}/ times={times} rank={rank}"
          );

          expected_hits += usize::from(wanted);
        }

        assert_eq!(arrangements.hits(), expected_hits, "/{source}/");
      }
    }
  }

  /// `write` and `hit` have to be the inverse of the enumeration, or the run
  /// would report the wrong candidate for the right index.
  #[test]
  fn materialising_inverts_the_enumeration() {
    let pattern = Pattern::new(r"^W").unwrap();

    for times in 1..=3 {
      let all = walk(times);
      let arrangements =
        Arrangements::precompute(&pattern, times, usize::MAX).unwrap();
      let mut buf = vec![0; times * 4];

      for (rank, candidate) in all.iter().enumerate() {
        arrangements.write(rank, &mut buf);

        assert_eq!(&buf, candidate, "times={times} rank={rank}");
        assert_eq!(
          arrangements.hit(7, rank).chars(),
          std::str::from_utf8(candidate).unwrap()
        );
      }
    }
  }

  #[test]
  fn the_gate_refuses_what_is_not_worth_precomputing() {
    let pattern = Pattern::new(r"^W").unwrap();

    // Too few tries to amortise the build.
    assert!(Arrangements::precompute(&pattern, 2, 2_519).is_none());
    assert!(Arrangements::precompute(&pattern, 2, 2_520).is_some());

    // times = 4 needs a 7.9 MiB bitset, which is past the cache budget.
    assert!(Arrangements::precompute(&pattern, 4, usize::MAX).is_none());

    // Out of range entirely.
    assert!(Arrangements::precompute(&pattern, 0, usize::MAX).is_none());
    assert!(Arrangements::precompute(
      &pattern,
      super::MAX_TIMES + 1,
      usize::MAX
    )
    .is_none());
  }

  /// The precompute knows the hit probability exactly.
  #[test]
  fn hit_counts_are_exact() {
    for (source, times, hits, total) in [
      (r"^WKNCWKNC$", 2, 1, 2_520),
      (r"(WKNCWKNC|WCKNWCKN)", 2, 2, 2_520),
      (r"^W", 2, 630, 2_520),
      (r"W", 2, 2_520, 2_520),
      (r"^W", 1, 6, 24),
    ] {
      let pattern = Pattern::new(source).unwrap();
      let arrangements =
        Arrangements::precompute(&pattern, times, usize::MAX).unwrap();

      assert_eq!(arrangements.len(), total, "/{source}/");
      assert_eq!(arrangements.hits(), hits, "/{source}/");
    }
  }
}
