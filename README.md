<div align="center">

# wakuchin-rs &middot; [![GitHub license](https://img.shields.io/github/license/P2P-Develop/wakuchin-rs?color=blue&style=flat-square)](https://github.com/P2P-Develop/wakuchin-rs/blob/main/LICENSE) [![wakuchin-rs at crates.io](https://img.shields.io/crates/v/wakuchin.svg?style=flat-square)](https://crates.io/crates/wakuchin) ![GitHub branch checks state at main](https://img.shields.io/github/actions/workflow/status/P2P-Develop/wakuchin-rs/rust.yml?branch=main&style=flat-square)

</div>
<p align="center">A new generation wakuchin researcher software written in Rust</p>

## Features

- 🚀 **Fast**
  - Research wakuchin with parallelism
  - Doing research WKCN \* 2 with 200000 tries time is about **13ms** \(tested with [`wakuchin_cli`](cli) on Intel Core i5-12600K\)
- 🔧 **Extendable**
  - You can use the Rust APIs of [`wakuchin`](core)
- 📰 **Researcher-friendly**
  - The result output can be: **plain text** or **json**

## Benchmarking

```bash
$ cargo bench
```

## Testing

```bash
$ cargo test
```
