# dirsize

[![CI](https://github.com/stern9/dirsize/actions/workflows/ci.yml/badge.svg)](https://github.com/stern9/dirsize/actions/workflows/ci.yml)

A fast command-line tool that shows what is using your disk space, as a tree
sorted largest first.

```console
$ dirsize . -n 3
 333.1 MiB ██████████ 100% ./
 333.0 MiB ██████████ 100% ├── target/
 285.7 MiB █████████░  86% │   ├── debug/
  47.3 MiB █░░░░░░░░░  14% │   ├── release/
   1.9 KiB ░░░░░░░░░░   0% │   ├── .rustc_info.json
     177 B ░░░░░░░░░░   0% │   └── (2 more)
  53.1 KiB ░░░░░░░░░░   0% ├── .git/
  26.2 KiB ░░░░░░░░░░   0% │   ├── hooks/
  22.6 KiB ░░░░░░░░░░   0% │   ├── objects/
   2.5 KiB ░░░░░░░░░░   0% │   ├── logs/
   1.8 KiB ░░░░░░░░░░   0% │   └── (7 more)
  13.0 KiB ░░░░░░░░░░   0% ├── Cargo.lock
  33.0 KiB ░░░░░░░░░░   0% └── (9 more)
```

Bars and percentages are relative to the directory you asked about.

## Install

With a [Rust toolchain](https://rustup.rs) installed:

```sh
git clone https://github.com/stern9/dirsize
cd dirsize
cargo install --path .
```

## Usage

```text
Usage: dirsize [OPTIONS] [PATH]

Arguments:
  [PATH]  Directory to analyze [default: .]

Options:
  -d, --depth <DEPTH>  Levels below the root to show [default: 2]
  -n, --top <TOP>      Maximum entries to show per directory [default: 10]
  -D, --dirs-only      Show directories only
  -b, --bytes          Print exact byte counts instead of human-readable sizes
  -h, --help           Print help
  -V, --version        Print version
```

Entries beyond `--top` collapse into one `(N more)` line carrying their
combined size, so each directory still adds up.

## Good to know

- Sizes are file lengths, not blocks on disk, so they can differ from `du`.
- Symlinks are not followed, except the path given on the command line.
- Unreadable entries are skipped with a warning on stderr; the scan continues.
- Directories are read in parallel with [rayon](https://crates.io/crates/rayon).

## Development

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## License

Licensed under either of [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT),
at your option.
