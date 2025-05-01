use audio::start_analyze_music;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

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
        #[arg(
            long,
            default_value_t = String::from("{{#disc_number}}{{disc_number}}-{{/disc_number}}{{track_number}} {{title}}")
        )]
        filename_template: String,
        #[arg(long, default_value_t = 2)]
        pad_width: usize,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::RemovePrefix { prefix, ext, dir } => {
            file_utils::remove_prefix_from_files(prefix, ext, dir)
        }
        Commands::AnalyzeMusic { result, src } => start_analyze_music(src, result),
        Commands::CopyMusic {
            src,
            dest,
            delay_ms,
            override_files,
            filename_template,
            pad_width,
        } => audio::start_copying_music(
            src,
            dest,
            *delay_ms,
            *override_files,
            filename_template,
            *pad_width,
        ),
    }
}
