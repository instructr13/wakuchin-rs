<div align="center">

# wakuchin-rs &middot; [![GitHub license](https://img.shields.io/github/license/P2P-Develop/wakuchin-rs?color=blue&style=flat-square)](https://github.com/P2P-Develop/wakuchin-rs/blob/main/LICENSE) [![wakuchin-rs at crates.io](https://img.shields.io/crates/v/wakuchin.svg?style=flat-square)](https://crates.io/crates/wakuchin) ![GitHub branch checks state at main](https://img.shields.io/github/actions/workflow/status/P2P-Develop/wakuchin-rs/rust.yml?branch=main&style=flat-square)

</div>
<p align="center">A new generation wakuchin researcher software written in Rust</p>

## Features

- 🚀 **Fast**
  - Research wakuchin with parallelism, zero allocations per try
  - **167 Mtries/s** on an Apple M4 \(10 cores\) — 300,000,000 tries of `WKCN * 2` in **1.79 s**

    | workers | time | ns/try | speedup |
    | ------: | ---: | -----: | ------: |
    | 1 | 10.53 s | 35.1 | 1.00× |
    | 4 | 3.04 s | 10.1 | 3.47× |
    | 8 | 2.00 s | 6.7 | 5.27× |
    | 10 \(default\) | 1.79 s | 6.0 | 5.89× |

    Measured with [`wakuchin_cli`](cli) `--release`, `--no-progress`, regex
    `(WKNCWKNC|WCKNWCKN)` \(hit rate 0.04%\), best of one run per point. Cost
    rises with the hit rate: the same machine does 231 Mtries/s at 0.04% hits
    and 50 Mtries/s when every try hits.
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
