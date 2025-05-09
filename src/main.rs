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
        #[arg(short = 'p', long)]
        prefix: String,
        #[arg(short = 'e', long)]
        ext: String,
        dir: PathBuf,
    },
    AnalyzeMusic {
        #[arg(short = 'r', long)]
        result: PathBuf,
        src: PathBuf,
    },
    CopyMusic {
        #[arg(short = 's', long)]
        src: PathBuf,
        #[arg(short = 'd', long)]
        dest: PathBuf,
        #[arg(long, default_value_t = 30)]
        delay_ms: u64,
        #[arg(short = 'o', long, action)]
        override_files: bool,
        #[arg(long, action)]
        fat_32: bool,
        #[arg(
            short = 't',
            long,
            default_value_t = String::from("{{#disc_number}}{{{disc_number}}}-{{/disc_number}}{{{track_number}}} {{{title}}}")
        )]
        filename_template: String,
        #[arg(long, default_value_t = 2)]
        pad_width: usize,
        #[arg(short = 'm', long, value_enum, default_value_t = audio::TrackNumberModification::None)]
        metadata_track_number_modification: audio::TrackNumberModification,
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
            metadata_track_number_modification,
            fat_32,
        } => audio::start_copying_music(
            src,
            dest,
            &audio::CopyFileOptions {
                filename_template,
                delay_ms: *delay_ms,
                override_files: *override_files,
                pad_width: *pad_width,
                fat_32: *fat_32,
            },
            &audio::CopyMetadataOptions {
                track_number_modification: *metadata_track_number_modification,
            },
        ),
    }
}
