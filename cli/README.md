# CLI (`wakuchin_cli`)

## Usage

```
wakuchin_cli 0.1.0
P2P-Develop
A next generation wakuchin researcher software written in Rust

USAGE:
    wakuchin [OPTIONS] [config]

ARGS:
    <config>    Config file path, can be json, yaml, and toml, detected by extension

OPTIONS:
    -f, --format <text|json>    Output format
    -h, --help                  Print help information
    -i, --tries <N>             Number of tries
    -r, --regex <REGEX>         Regex to detect hits
    -t, --times <N>             Wakuchin times n
    -V, --version               Print version information
```

## Installation

### Download from [GitHub Releases](https://github.com/P2P-Develop/wakuchin-rs/releases)

You can download the latest version of `wakuchin_cli` from [here](https://github.com/P2P-Develop/wakuchin-rs/releases/latest).

### Using `cargo`

```bash
$ cargo install wakuchin_cli
```

## Building from source

```bash
$ cargo build --profile release --package wakuchin_cli
$ target/release/wakuchin
```
