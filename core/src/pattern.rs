//! Compiled hit patterns.

use std::fmt::{self, Debug, Display, Formatter};
use std::str::FromStr;

use regex_automata::meta::{self, Regex};
use regex_automata::util::syntax;
use regex_automata::{Input, MatchKind};
use serde::de::{self, Deserialize, Deserializer};

use crate::error::WakuchinError;

/// A compiled pattern for detecting hits.
///
/// Shared across workers: it is `Sync` and holds no per-search scratch space.
/// Each worker makes its own [`Matcher`], which owns that scratch space.
///
/// # Why not `regex::Regex`
///
/// `regex`'s `is_match` keeps its scratch space in an internal pool and checks
/// it out on every call. The pool has a fast path for the thread that owns it,
/// and a mutex for every other thread.
///
/// Taking a pattern *string* rather than an already-built regex is deliberate.
/// Rebuilding a `regex::Regex` from its `as_str()` would silently discard any
/// `RegexBuilder` configuration the caller had applied, with no error. Every
/// setting that changes what a pattern *means* can be written inline in the
/// pattern instead -- `(?i)`, `(?s)`, `(?m)`, `(?x)`, `(?-u)`.
#[derive(Clone)]
pub struct Pattern {
  regex: Regex,

  /// `meta::Regex` does not keep the pattern it was built from.
  source: Box<str>,
}

impl Pattern {
  /// Compiles `pattern`.
  ///
  /// Matching semantics follow `regex::bytes::Regex`: leftmost-first, empty
  /// matches allowed to land between UTF-8 code units, and the same size
  /// limits.
  ///
  /// # Errors
  ///
  /// [`WakuchinError::Pattern`] if `pattern` is not a valid regular
  /// expression, or needs more memory than the limits allow.
  pub fn new(pattern: &str) -> Result<Self, WakuchinError> {
    // Mirrors what `regex::bytes::RegexBuilder` configures, so a pattern
    // behaves the same here as it would through the `regex` crate.
    let config = meta::Config::new()
      .nfa_size_limit(Some(10 * (1 << 20)))
      .hybrid_cache_capacity(2 * (1 << 20))
      .match_kind(MatchKind::LeftmostFirst)
      .utf8_empty(false);

    Regex::builder()
      .configure(config)
      .syntax(syntax::Config::new().utf8(false))
      .build(pattern)
      .map(|regex| Self {
        regex,
        source: pattern.into(),
      })
      .map_err(|err| {
        // `BuildError`'s own `Display` is just "error parsing pattern 0"; the
        // part a user can act on is the syntax error underneath it.
        WakuchinError::Pattern(
          err
            .syntax_error()
            .map_or_else(|| err.to_string(), ToString::to_string),
        )
      })
  }

  /// The pattern this was compiled from.
  #[must_use]
  pub fn as_str(&self) -> &str {
    &self.source
  }

  /// A matcher with its own scratch space. Make one per thread.
  #[must_use]
  pub fn matcher(&self) -> Matcher<'_> {
    Matcher {
      pattern: self,
      cache: self.regex.create_cache(),
    }
  }

  /// Whether `candidate` matches.
  ///
  /// Allocates scratch space on every call. Fine for a one-off check; for a
  /// loop, keep a [`Matcher`].
  #[must_use]
  pub fn is_match(&self, candidate: &[u8]) -> bool {
    self.matcher().is_match(candidate)
  }
}

impl Debug for Pattern {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    Debug::fmt(&self.source, f)
  }
}

impl Display for Pattern {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    Display::fmt(&self.source, f)
  }
}

impl PartialEq for Pattern {
  fn eq(&self, other: &Self) -> bool {
    self.source == other.source
  }
}

impl Eq for Pattern {}

impl FromStr for Pattern {
  type Err = WakuchinError;

  fn from_str(pattern: &str) -> Result<Self, Self::Err> {
    Self::new(pattern)
  }
}

impl<'de> Deserialize<'de> for Pattern {
  fn deserialize<D: Deserializer<'de>>(
    deserializer: D,
  ) -> Result<Self, D::Error> {
    let pattern = <&str>::deserialize(deserializer)?;

    Self::new(pattern).map_err(de::Error::custom)
  }
}

/// A [`Pattern`] plus the scratch space one search needs.
///
/// Make one per worker and keep it for the whole loop. `is_match` takes
/// `&mut self`, so two threads cannot use one matcher at the same time; that
/// is the point -- the alternative is what `regex` does, which is to reacquire
/// the scratch space on every single call.
#[derive(Clone, Debug)]
pub struct Matcher<'pattern> {
  pattern: &'pattern Pattern,
  cache: meta::Cache,
}

impl Matcher<'_> {
  /// Whether `candidate` matches.
  #[inline(always)]
  pub fn is_match(&mut self, candidate: &[u8]) -> bool {
    // `earliest(true)` stops at the first match instead of finding the
    // leftmost-first one, which is all a boolean answer needs.
    let input = Input::new(candidate).earliest(true);

    self
      .pattern
      .regex
      .search_half_with(&mut self.cache, &input)
      .is_some()
  }

  #[must_use]
  pub fn pattern(&self) -> &Pattern {
    self.pattern
  }
}

#[cfg(test)]
mod test {
  use std::error::Error;

  use crate::Generator;

  use super::{Matcher, Pattern};

  #[test]
  fn matches_like_the_regex_crate() -> Result<(), Box<dyn Error>> {
    let cases: &[(&str, &[(&str, bool)])] = &[
      (
        "^[WKCN]+$",
        &[("WKCN", true), ("WKCNX", false), ("わくちん", false)],
      ),
      ("^WKNCWKNC$", &[("WKNCWKNC", true), ("WKNCWKNCW", false)]),
      ("WKCN", &[("NNWKCNCC", true), ("NNWKNCCC", false)]),
      ("(?i)wkcn", &[("WKCN", true)]),
      (
        "(WKNCWKNC|WCKNWCKN)",
        &[("WCKNWCKN", true), ("WKCNWKCN", false)],
      ),
    ];

    for (pattern, expectations) in cases {
      let compiled = Pattern::new(pattern)?;
      let mut matcher = compiled.matcher();

      assert_eq!(compiled.as_str(), *pattern);

      for (candidate, expected) in *expectations {
        assert_eq!(
          matcher.is_match(candidate.as_bytes()),
          *expected,
          "/{pattern}/ against {candidate:?}"
        );
        assert_eq!(compiled.is_match(candidate.as_bytes()), *expected);
      }
    }

    Ok(())
  }

  #[test]
  fn a_pattern_is_shareable_and_a_matcher_is_owned() {
    const fn assert_send_sync<T: Send + Sync>() {}

    // The pattern is shared by every worker; each moves its own matcher into
    // its thread. Exclusive use of a matcher is enforced by `is_match` taking
    // `&mut self`, not by the matcher being `!Sync` -- the cache it wraps
    // happens to be `Sync`.
    assert_send_sync::<Pattern>();
    assert_send_sync::<Matcher<'_>>();
  }

  #[test]
  fn reports_a_usable_message_for_a_bad_pattern() {
    let err = Pattern::new("(unclosed").unwrap_err().to_string();

    assert!(
      err.contains("regex parse error"),
      "unhelpful error message: {err}"
    );
  }

  /// A matcher reused across candidates has to keep answering correctly -- the
  /// cache it carries is exactly the state `regex` would have thrown away
  /// after every call.
  #[test]
  fn a_reused_matcher_stays_correct() -> Result<(), Box<dyn Error>> {
    let pattern = Pattern::new("^WK")?;
    let mut matcher = pattern.matcher();
    let mut generator = Generator::new(2);

    for _ in 0..10_000 {
      generator.shuffle();

      assert_eq!(
        matcher.is_match(generator.bytes()),
        generator.as_str().starts_with("WK")
      );
    }

    Ok(())
  }
}
