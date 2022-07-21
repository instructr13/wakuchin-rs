# CLI (`wakuchin_cli`)

## Usage

```
wakuchin_cli 0.1.0
P2P-Develop
A next generation wakuchin researcher software written in Rust

USAGE:
    wakuchin [OPTIONS]

OPTIONS:
    -f, --format <text|json>    Output format [default: text]
    -h, --help                  Print help information
    -i, --tries <N>             Number of tries
    -r, --regex <REGEX>         Regex to detect hits
    -t, --times <N>             Wakuchin times n
    -V, --version               Print version information
```

## Building from source

```bash
$ cargo build --profile release --package wakuchin_cli
$ target/release/wakuchin
```
