<div align="center">

# wakuchin-rs &middot; [![GitHub license](https://img.shields.io/github/license/P2P-Develop/wakuchin-rs?color=blue&style=flat-square)](https://github.com/P2P-Develop/wakuchin-rs/blob/main/LICENSE) [![wakuchin-rs at crates.io](https://img.shields.io/crates/v/wakuchin.svg?style=flat-square)](https://crates.io/crates/wakuchin) ![GitHub branch checks state at main](https://img.shields.io/github/actions/workflow/status/P2P-Develop/wakuchin-rs/rust.yml?branch=main&style=flat-square)

</div>
<p align="center">A new generation wakuchin researcher software written in Rust</p>

## Features

- 🚀 **Fast**
  - Research wakuchin with parallelism, zero allocations per try
  - **3.5 Gtries/s** on an Apple M4 \(10 cores\) — 300,000,000 tries of
    `WKCN * 2` in **84 ms**

    | workers | time | ns/try | speedup |
    | ------: | ---: | -----: | ------: |
    | 1 | 407 ms | 1.36 | 1.00× |
    | 2 | 213 ms | 0.71 | 1.92× |
    | 4 | 114 ms | 0.38 | 3.57× |
    | 8 | 84 ms | 0.28 | 4.83× |
    | 10 \(default\) | 133 ms | 0.44 | 3.06× |

    Measured with [`wakuchin_cli`](cli) `--release`, `--no-progress`, regex
    `(WKNCWKNC|WCKNWCKN)`, best of three runs per point.
- 🎯 **Exact when it can be**
  - `--exact` skips sampling entirely and reports the answer with **no statistical error**

    ```console
    $ wakuchin --times 2 --regex '(WKNCWKNC|WCKNWCKN)' --exact true
    --- Result ---
    Tries: 2520
    WCKNWCKN hits: 1 (0.04%)
    WKNCWKNC hits: 1 (0.04%)
    Total hits: 2 (0.08%)
    ```

    A candidate is a permutation of `WKCN` repeated `times` times, so there are
    only `(4n)!/(n!)^4 = 2520` of them at `times = 2`. Checking every one is
    cheaper than a few thousand random tries, and answers the question outright
    instead of estimating it. Available while that space is small enough to
    enumerate; past that it says so rather than quietly returning an estimate.
- 🔧 **Extendable**
  - You can use the Rust APIs of [`wakuchin`](core)
- 📰 **Researcher-friendly**
  - The result output can be: **plain text** or **json**

## Benchmarking

```bash
# Micro-benchmarks and the worker model, via criterion.
$ cargo bench --package wakuchin --bench bench_core

# Record a baseline, then compare later runs against it.
$ cargo bench --package wakuchin --bench bench_core -- --save-baseline main
$ cargo bench --package wakuchin --bench bench_core -- --baseline main
```

## Profile-guided optimization

`scripts/pgo.sh` builds the CLI with PGO. It measured **-4.1%** wall time
(single worker) and an 11% smaller binary against the plain release profile.

Two things worth knowing before reaching for it:

- Most of that win was on the regex engine, and the regex has since left the hot
  loop for small `times`. Re-measure rather than assuming the 4% still applies.

## Testing

```bash
$ cargo test --workspace --all-features
```
