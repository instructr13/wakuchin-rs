<div align="center">

# wakuchin-rs &middot; [![GitHub license](https://img.shields.io/github/license/P2P-Develop/wakuchin-rs?color=blue&style=flat-square)](https://github.com/P2P-Develop/wakuchin-rs/blob/main/LICENSE) [![wakuchin-rs at crates.io](https://img.shields.io/crates/v/wakuchin.svg?style=flat-square)](https://crates.io/crates/wakuchin) [![GitHub branch checks state at main](https://img.shields.io/github/checks-status/P2P-Develop/wakuchin-rs/main?style=flat-square)](https://github.com/P2P-Develop/wakuchin-rs/actions?query=branch%3Amain)

</div>
<p align="center">A new generation wakuchin researcher software written in Rust</p>

## Features

- 🚀 **Fast**
  - Research wakuchin with parallelism
  - Doing research WKCN \* 2 with 200000 tries time is about **60ms** \(tested with [`wakuchin_cli`](cli)\)
- 🔧 **Extendable**
  - You can use the Rust APIs of [`wakuchin_core`](core)
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
