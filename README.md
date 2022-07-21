<h1 align="center">wakuchin-rs</h1>
<p align="center">A new generation wakuchin researcher software written in Rust</p>

## Features

- **Fast**
  - Research wakuchin with parallelism
  - Doing research WKCN \* 2 with 200000 tries time is about **60ms** \(tested with [`wakuchin_cli`](cli)\)
- **Extendable**
  - You can use the Rust APIs of [`wakuchin_core`](core)
- **Researcher-friendly**
  - The result output can be: **plain text** or **json**

## Benchmarking

```bash
$ cargo bench
```

## Testing

```bash
$ cargo test
```

