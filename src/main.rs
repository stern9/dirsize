use std::io::{self, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use dirsize::{RenderOptions, render, scan};

/// Show which directories and files are using your disk space.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Directory to analyze
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Levels below the root to show
    #[arg(short, long, default_value_t = 2)]
    depth: usize,

    /// Maximum entries to show per directory
    #[arg(short = 'n', long, default_value_t = 10)]
    top: usize,

    /// Show directories only
    #[arg(short = 'D', long)]
    dirs_only: bool,

    /// Print exact byte counts instead of human-readable sizes
    #[arg(short, long)]
    bytes: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let result = scan(&cli.path).with_context(|| format!("cannot read {}", cli.path.display()))?;

    let opts = RenderOptions {
        depth: cli.depth,
        top: cli.top,
        dirs_only: cli.dirs_only,
        bytes: cli.bytes,
    };
    let output = render(&result.root, &opts);

    // A closed pipe (e.g. `dirsize | head`) is a normal way to stop reading.
    if let Err(err) = io::stdout().lock().write_all(output.as_bytes())
        && err.kind() != io::ErrorKind::BrokenPipe
    {
        return Err(err).context("cannot write to stdout");
    }

    for error in &result.errors {
        eprintln!(
            "dirsize: skipped {}: {}",
            error.path.display(),
            error.source
        );
    }

    Ok(())
}
