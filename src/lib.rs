//! Core logic for `dirsize`: scan a directory tree in parallel and render the
//! largest entries as a sorted tree.

use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use rayon::prelude::*;

/// A file or directory with its total size in bytes.
///
/// Sizes are apparent sizes (file lengths), and a directory's size is the sum
/// of everything beneath it. Children are sorted largest first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub size: u64,
    pub is_dir: bool,
    pub children: Vec<Entry>,
}

/// A path that could not be read during a scan.
#[derive(Debug)]
pub struct ScanError {
    pub path: PathBuf,
    pub source: io::Error,
}

/// The result of scanning a tree: what was measured and what was skipped.
#[derive(Debug)]
pub struct Scan {
    pub root: Entry,
    pub errors: Vec<ScanError>,
}

/// Scans `root` recursively.
///
/// Fails only if `root` itself cannot be read. Anything unreadable below it is
/// skipped and reported in [`Scan::errors`]. Symlinks are not followed, except
/// for `root` itself.
pub fn scan(root: &Path) -> io::Result<Scan> {
    let meta = fs::metadata(root)?;
    let (root, errors) = scan_entry(root, root.display().to_string(), &meta);
    Ok(Scan { root, errors })
}

fn scan_entry(path: &Path, name: String, meta: &fs::Metadata) -> (Entry, Vec<ScanError>) {
    if !meta.is_dir() {
        let entry = Entry {
            name,
            size: meta.len(),
            is_dir: false,
            children: Vec::new(),
        };
        return (entry, Vec::new());
    }

    let mut errors = Vec::new();
    let mut listing: Vec<(PathBuf, OsString)> = Vec::new();
    match fs::read_dir(path) {
        Ok(read_dir) => {
            for item in read_dir {
                match item {
                    Ok(item) => listing.push((item.path(), item.file_name())),
                    Err(source) => errors.push(ScanError {
                        path: path.to_path_buf(),
                        source,
                    }),
                }
            }
        }
        Err(source) => errors.push(ScanError {
            path: path.to_path_buf(),
            source,
        }),
    }

    let results: Vec<(Option<Entry>, Vec<ScanError>)> = listing
        .into_par_iter()
        .map(|(child_path, child_name)| {
            let child_name = child_name.to_string_lossy().into_owned();
            match fs::symlink_metadata(&child_path) {
                Ok(meta) => {
                    let (entry, errors) = scan_entry(&child_path, child_name, &meta);
                    (Some(entry), errors)
                }
                Err(source) => {
                    let error = ScanError {
                        path: child_path,
                        source,
                    };
                    (None, vec![error])
                }
            }
        })
        .collect();

    let mut children = Vec::with_capacity(results.len());
    for (entry, child_errors) in results {
        children.extend(entry);
        errors.extend(child_errors);
    }
    sort_largest_first(&mut children);

    let entry = Entry {
        name,
        size: children.iter().map(|child| child.size).sum(),
        is_dir: true,
        children,
    };
    (entry, errors)
}

/// Sorts by size descending, breaking ties by name so output is deterministic.
fn sort_largest_first(entries: &mut [Entry]) {
    entries.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.name.cmp(&b.name)));
}

/// Controls how much of the tree [`render`] prints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderOptions {
    /// Levels below the root to show.
    pub depth: usize,
    /// Maximum entries to show per directory.
    pub top: usize,
    /// Hide files and show directories only.
    pub dirs_only: bool,
    /// Print exact byte counts instead of human-readable sizes.
    pub bytes: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            depth: 2,
            top: 10,
            dirs_only: false,
            bytes: false,
        }
    }
}

const BAR_WIDTH: usize = 10;

/// Renders `root` as a tree, one line per entry, largest first.
///
/// Bars and percentages are relative to the root's total size.
pub fn render(root: &Entry, opts: &RenderOptions) -> String {
    let mut out = String::new();
    write_line(&mut out, root.size, root.size, opts, &label(root));
    render_children(&mut out, root, root.size, "", opts.depth, opts);
    out
}

fn render_children(
    out: &mut String,
    entry: &Entry,
    total: u64,
    prefix: &str,
    depth_left: usize,
    opts: &RenderOptions,
) {
    if depth_left == 0 {
        return;
    }

    let visible: Vec<&Entry> = entry
        .children
        .iter()
        .filter(|child| child.is_dir || !opts.dirs_only)
        .collect();
    let (shown, hidden) = visible.split_at(visible.len().min(opts.top));

    for (i, child) in shown.iter().enumerate() {
        let last = i + 1 == shown.len() && hidden.is_empty();
        let (branch, indent) = if last {
            ("└── ", "    ")
        } else {
            ("├── ", "│   ")
        };
        let text = format!("{prefix}{branch}{}", label(child));
        write_line(out, child.size, total, opts, &text);
        let child_prefix = format!("{prefix}{indent}");
        render_children(out, child, total, &child_prefix, depth_left - 1, opts);
    }

    if !hidden.is_empty() {
        let size = hidden.iter().map(|child| child.size).sum();
        let text = format!("{prefix}└── ({} more)", hidden.len());
        write_line(out, size, total, opts, &text);
    }
}

fn label(entry: &Entry) -> String {
    if entry.is_dir && !entry.name.ends_with('/') {
        format!("{}/", entry.name)
    } else {
        entry.name.clone()
    }
}

fn write_line(out: &mut String, size: u64, total: u64, opts: &RenderOptions, text: &str) {
    let size_text = if opts.bytes {
        size.to_string()
    } else {
        human_size(size)
    };
    let fraction = if total == 0 {
        0.0
    } else {
        size as f64 / total as f64
    };
    let filled = ((fraction * BAR_WIDTH as f64).round() as usize).min(BAR_WIDTH);
    let bar = format!("{}{}", "█".repeat(filled), "░".repeat(BAR_WIDTH - filled));
    let percent = (fraction * 100.0).round() as u64;
    // Writing to a String cannot fail.
    let _ = writeln!(out, "{size_text:>10} {bar} {percent:>3}% {text}");
}

/// Formats a byte count using binary units, e.g. `1.5 MiB`.
pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["KiB", "MiB", "GiB", "TiB", "PiB", "EiB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64 / 1024.0;
    let mut unit = 0;
    // 1023.95 is where one decimal place would round up to "1024.0".
    while value >= 1023.95 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &str, size: u64) -> Entry {
        Entry {
            name: name.to_string(),
            size,
            is_dir: false,
            children: Vec::new(),
        }
    }

    fn dir(name: &str, mut children: Vec<Entry>) -> Entry {
        sort_largest_first(&mut children);
        Entry {
            name: name.to_string(),
            size: children.iter().map(|child| child.size).sum(),
            is_dir: true,
            children,
        }
    }

    fn sample() -> Entry {
        dir(
            "root",
            vec![
                file("small.txt", 100),
                dir("big", vec![file("a.bin", 600), file("b.bin", 200)]),
                file("medium.txt", 100),
            ],
        )
    }

    fn lines(lines: &[&str]) -> String {
        lines.iter().map(|line| format!("{line}\n")).collect()
    }

    fn bytes_opts() -> RenderOptions {
        RenderOptions {
            bytes: true,
            ..RenderOptions::default()
        }
    }

    #[test]
    fn human_size_picks_the_right_unit() {
        assert_eq!(human_size(0), "0 B");
        assert_eq!(human_size(1023), "1023 B");
        assert_eq!(human_size(1024), "1.0 KiB");
        assert_eq!(human_size(1536), "1.5 KiB");
        assert_eq!(human_size(5 * 1024 * 1024), "5.0 MiB");
        assert_eq!(human_size(3 * 1024 * 1024 * 1024), "3.0 GiB");
    }

    #[test]
    fn human_size_never_shows_1024_of_a_unit() {
        assert_eq!(human_size(1024 * 1024 - 1), "1.0 MiB");
        assert_eq!(human_size(u64::MAX), "16.0 EiB");
    }

    #[test]
    fn sort_is_by_size_then_name() {
        let root = sample();
        let names: Vec<&str> = root.children.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["big", "medium.txt", "small.txt"]);
    }

    #[test]
    fn render_draws_a_sorted_tree() {
        let expected = lines(&[
            "      1000 ██████████ 100% root/",
            "       800 ████████░░  80% ├── big/",
            "       600 ██████░░░░  60% │   ├── a.bin",
            "       200 ██░░░░░░░░  20% │   └── b.bin",
            "       100 █░░░░░░░░░  10% ├── medium.txt",
            "       100 █░░░░░░░░░  10% └── small.txt",
        ]);
        assert_eq!(render(&sample(), &bytes_opts()), expected);
    }

    #[test]
    fn render_respects_depth() {
        let opts = RenderOptions {
            depth: 1,
            ..bytes_opts()
        };
        let out = render(&sample(), &opts);
        assert!(out.contains("big/"));
        assert!(!out.contains("a.bin"));

        let opts = RenderOptions {
            depth: 0,
            ..bytes_opts()
        };
        assert_eq!(render(&sample(), &opts).lines().count(), 1);
    }

    #[test]
    fn render_collapses_entries_beyond_top() {
        let opts = RenderOptions {
            top: 1,
            depth: 1,
            ..bytes_opts()
        };
        let expected = lines(&[
            "      1000 ██████████ 100% root/",
            "       800 ████████░░  80% ├── big/",
            "       200 ██░░░░░░░░  20% └── (2 more)",
        ]);
        assert_eq!(render(&sample(), &opts), expected);
    }

    #[test]
    fn render_dirs_only_hides_files() {
        let opts = RenderOptions {
            dirs_only: true,
            ..bytes_opts()
        };
        let expected = lines(&[
            "      1000 ██████████ 100% root/",
            "       800 ████████░░  80% └── big/",
        ]);
        assert_eq!(render(&sample(), &opts), expected);
    }

    #[test]
    fn render_handles_an_empty_tree() {
        let out = render(&dir("empty", Vec::new()), &RenderOptions::default());
        assert_eq!(out, "       0 B ░░░░░░░░░░   0% empty/\n");
    }
}
