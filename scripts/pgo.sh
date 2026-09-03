#!/usr/bin/env bash

# Build the CLI with profile-guided optimization.
#
# Usage:
#   scripts/pgo.sh              # instrument, train, optimize
#   scripts/pgo.sh --compare    # also A/B the result against a plain release

set -euo pipefail

cd "$(dirname "$0")/.."

profraw="$PWD/target/pgo"
profdata="$PWD/target/wakuchin.profdata"

sysroot="$(rustc --print sysroot)"
merger="$(find "$sysroot/lib/rustlib" -name llvm-profdata -type f 2>/dev/null | head -1)"

if [ -z "$merger" ]; then
  echo "llvm-profdata is missing from the toolchain. Install it with:" >&2
  echo "  rustup component add llvm-tools" >&2
  exit 1
fi

echo "==> instrumenting"
rm -rf "$profraw" "$profdata"
RUSTFLAGS="-Cprofile-generate=$profraw" \
  cargo build --release --package wakuchin_cli

echo "==> training on the shape of a real workload"

./target/release/wakuchin -i 40000000 -t 2 -r '(WKNCWKNC|WCKNWCKN)' --no-progress true >/dev/null
./target/release/wakuchin -i 4000000  -t 2 -r '^W' --no-progress true >/dev/null
./target/release/wakuchin -i 2000000  -t 3 -r 'WKCN' --no-progress true >/dev/null

echo "==> merging $(ls "$profraw"/*.profraw | wc -l | tr -d ' ') raw profiles"
"$merger" merge -o "$profdata" "$profraw"/*.profraw

echo "==> optimizing"
RUSTFLAGS="-Cprofile-use=$profdata" \
  cargo build --release --package wakuchin_cli

echo "==> done: target/release/wakuchin"

if [ "${1:-}" = "--compare" ]; then
  if ! command -v hyperfine >/dev/null; then
    echo "hyperfine is not installed; skipping the comparison" >&2
    exit 0
  fi

  cp target/release/wakuchin target/wakuchin-pgo
  cargo build --release --package wakuchin_cli
  cp target/release/wakuchin target/wakuchin-plain

  echo "==> comparing (single worker: multi-worker noise swamps a few percent)"
  args="-i 8000000 -t 2 -r (WKNCWKNC|WCKNWCKN) -w 1 --no-progress true"
  # shellcheck disable=SC2086
  hyperfine --warmup 1 --runs 6 \
    -n plain "target/wakuchin-plain $args" \
    -n pgo   "target/wakuchin-pgo $args"
fi
