<div align="center">

# wakuchin-rs &middot; [![GitHub license](https://img.shields.io/github/license/P2P-Develop/wakuchin-rs?color=blue&style=flat-square)](https://github.com/P2P-Develop/wakuchin-rs/blob/main/LICENSE) [![wakuchin-rs at crates.io](https://img.shields.io/crates/v/wakuchin.svg?style=flat-square)](https://crates.io/crates/wakuchin) ![GitHub branch checks state at main](https://img.shields.io/github/actions/workflow/status/P2P-Develop/wakuchin-rs/rust.yml?branch=main&style=flat-square)

</div>
<p align="center">A new generation wakuchin researcher software written in Rust</p>

## Features

- 🚀 **Fast**
  - Research wakuchin with parallelism, zero allocations per try
  - **179 Mtries/s** on an Apple M4 \(10 cores\) — 300,000,000 tries of `WKCN * 2` in **1.67 s**

    | workers | time | ns/try | speedup |
    | ------: | ---: | -----: | ------: |
    | 1 | 10.46 s | 34.9 | 1.00× |
    | 2 | 5.28 s | 17.6 | 1.98× |
    | 4 | 2.85 s | 9.5 | 3.67× |
    | 8 | 1.89 s | 6.3 | 5.52× |
    | 10 \(default\) | 1.67 s | 5.6 | 6.25× |
    | 16 | 1.68 s | 5.6 | 6.21× |

    Measured with [`wakuchin_cli`](cli) `--release`, `--no-progress`, regex
    `(WKNCWKNC|WCKNWCKN)` \(hit rate 0.04%\), best of several runs per point.
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

## Testing

```bash
$ cargo test --workspace --all-features
```

`core/tests/alloc_budget.rs` asserts the search loop allocates nothing per try.
It measures the slope between two run sizes, so setup cost cancels out.
