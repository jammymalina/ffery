use anyhow::{Context, anyhow};
use clap::{Parser, Subcommand};
use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    RemovePrefix {
        #[arg(long)]
        prefix: String,
        #[arg(long)]
        ext: String,
        dir: PathBuf,
    },
}

fn validate_dir(dir: &Path) -> anyhow::Result<()> {
    let d = dir.to_str().unwrap_or("unknown");
    if !dir.exists() {
        return Err(anyhow!("Path '{d}' does not exist"));
    }

    if !dir.is_dir() {
        return Err(anyhow!("Path '{d}' must be a directory"));
    }

    Ok(())
}

fn remove_prefix(prefix: &str, ext: &str, dir: &PathBuf) -> anyhow::Result<()> {
    validate_dir(dir)?;

    let target_ext = OsStr::new(ext);

    let collected_results: Result<Vec<_>, _> = fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path: &PathBuf| path.is_file())
        .filter(|path| path.extension() == Some(target_ext))
        .filter_map(|path| {
            path.file_name()
                .and_then(|os_str| os_str.to_str())
                .map(str::to_owned)
        })
        .map(|filename| {
            (
                filename.clone(),
                filename
                    .strip_prefix(prefix)
                    .unwrap_or(&filename)
                    .trim()
                    .to_string(),
            )
        })
        .filter(|(_, target_filename)| !target_filename.is_empty())
        .map(|(src_filename, target_filename)| {
            fs::rename(dir.join(&src_filename), dir.join(&target_filename)).with_context(|| {
                format!("Failed to rename file '{src_filename}' to '{target_filename}'")
            })
        })
        .collect();

    collected_results.map(|_| ())
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::RemovePrefix { prefix, ext, dir } => remove_prefix(prefix, ext, dir),
    }
}
