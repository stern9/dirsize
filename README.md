# dirsize

[![CI](https://github.com/stern9/dirsize/actions/workflows/ci.yml/badge.svg)](https://github.com/stern9/dirsize/actions/workflows/ci.yml)

A small, fast command-line tool that shows which directories and files are
using your disk space, as a sorted tree.

```console
$ dirsize . -n 4
 333.1 MiB ██████████ 100% ./
 333.0 MiB ██████████ 100% ├── target/
 285.7 MiB █████████░  86% │   ├── debug/
  47.3 MiB █░░░░░░░░░  14% │   ├── release/
   1.9 KiB ░░░░░░░░░░   0% │   ├── .rustc_info.json
     177 B ░░░░░░░░░░   0% │   ├── CACHEDIR.TAG
       0 B ░░░░░░░░░░   0% │   └── (1 more)
  26.6 KiB ░░░░░░░░░░   0% ├── .git/
  26.2 KiB ░░░░░░░░░░   0% │   ├── hooks/
     240 B ░░░░░░░░░░   0% │   ├── info/
     137 B ░░░░░░░░░░   0% │   ├── config
      73 B ░░░░░░░░░░   0% │   ├── description
      21 B ░░░░░░░░░░   0% │   └── (3 more)
  13.0 KiB ░░░░░░░░░░   0% ├── Cargo.lock
  12.4 KiB ░░░░░░░░░░   0% ├── src/
  10.9 KiB ░░░░░░░░░░   0% │   ├── lib.rs
   1.5 KiB ░░░░░░░░░░   0% │   └── main.rs
  18.2 KiB ░░░░░░░░░░   0% └── (7 more)
```

Every entry is sorted largest first within its directory. Bars and percentages
are relative to the total of the directory you asked about.

## Install

You need a [Rust toolchain](https://rustup.rs). Then:

```console
$ git clone https://github.com/stern9/dirsize
$ cd dirsize
$ cargo install --path .
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

Some examples:

```console
$ dirsize ~/projects -d 1 -D     # one level of directories only
$ dirsize . -n 3                 # the three largest entries per directory
$ dirsize /var/log --bytes       # exact byte counts
```

Entries beyond `--top` are collapsed into a single `(N more)` line that carries
their combined size, so the numbers in each directory always add up.

## Behaviour worth knowing

- **Sizes are apparent sizes** (file lengths), not blocks allocated on disk, so
  results can differ from `du` for sparse files and very small files.
- **Symlinks are not followed.** A link counts as the few bytes it occupies,
  which avoids double counting and cycles. The path you pass on the command
  line is the one exception.
- **Hard links are counted once per name.**
- **Unreadable files and directories are skipped**, not fatal. Each one is
  reported on stderr as `dirsize: skipped <path>: <reason>` and the scan
  continues. Only a missing or unreadable starting path is an error.

## How it works

The crate is a small library (`src/lib.rs`) with a thin binary on top
(`src/main.rs`):

- `scan` walks the tree recursively, reading the children of each directory in
  parallel with [rayon](https://crates.io/crates/rayon), and returns a tree of
  entries plus the list of paths it had to skip.
- `render` turns that tree into text. It is a pure function, which is what
  makes the output easy to unit test.

## Development

```console
$ cargo fmt --check
$ cargo clippy --all-targets -- -D warnings
$ cargo test
```

CI runs the same three commands on Linux and macOS for every push.

## Licence

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT licence](LICENSE-MIT), at your option.
