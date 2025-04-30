use anyhow::Context;
use audio::start_analyze_music;
use clap::{Parser, Subcommand};
use std::{ffi::OsStr, fs, path::PathBuf};

mod audio;
mod file_utils;

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
    AnalyzeMusic {
        #[arg(long)]
        result: PathBuf,
        src: PathBuf,
    },
    CopyMusic {
        #[arg(long)]
        src: PathBuf,
        #[arg(long)]
        dest: PathBuf,
        #[arg(long, default_value_t = 30)]
        delay_ms: u64,
        #[arg(long, action)]
        override_files: bool,
    },
}

fn remove_prefix(prefix: &str, ext: &str, dir: &PathBuf) -> anyhow::Result<()> {
    file_utils::validate_dir(dir)?;

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
        Commands::AnalyzeMusic { result, src } => start_analyze_music(src, result),
        Commands::CopyMusic {
            src,
            dest,
            delay_ms,
            override_files,
        } => audio::start_copying_music(src, dest, *delay_ms, *override_files),
    }
}
