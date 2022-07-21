//! Wakuchin symbol definitions

/// Internally used wakuchin chars
pub const WAKUCHIN: [char; 4] =
  [WAKUCHIN_W, WAKUCHIN_K, WAKUCHIN_C, WAKUCHIN_N];

/// Externally used wakuchin chars
pub const WAKUCHIN_EXTERNAL: [char; 4] = [
  WAKUCHIN_EXTERNAL_W,
  WAKUCHIN_EXTERNAL_K,
  WAKUCHIN_EXTERNAL_C,
  WAKUCHIN_EXTERNAL_N,
];

/// Internal wakuchin W
pub const WAKUCHIN_W: char = 'W';

/// Internal wakuchin K
pub const WAKUCHIN_K: char = 'K';

/// Internal wakuchin C
pub const WAKUCHIN_C: char = 'C';

/// Internal wakuchin N
pub const WAKUCHIN_N: char = 'N';

/// External wakuchin W
pub const WAKUCHIN_EXTERNAL_W: char = 'わ';

/// External wakuchin K
pub const WAKUCHIN_EXTERNAL_K: char = 'く';

/// External wakuchin C
pub const WAKUCHIN_EXTERNAL_C: char = 'ち';

/// External wakuchin N
pub const WAKUCHIN_EXTERNAL_N: char = 'ん';
