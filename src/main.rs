use anyhow::{Context, anyhow};
use clap::{Parser, Subcommand};
use indicatif::ProgressBar;
use std::{
    cmp::Ordering,
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
    thread::sleep,
    time::Duration,
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

fn start_copying_music(
    src: &PathBuf,
    dest: &Path,
    delay_ms: u64,
    override_files: bool,
) -> anyhow::Result<()> {
    validate_dir(src)?;

    let file_count = count_files(src)?;
    let bar = ProgressBar::new(file_count);

    let result = copy_music(src, dest, delay_ms, override_files, &bar);
    bar.finish();

    result
}

fn copy_music(
    src: &PathBuf,
    dest: &Path,
    delay_ms: u64,
    override_files: bool,
    bar: &ProgressBar,
) -> anyhow::Result<()> {
    validate_dir(src)?;

    let src_content: Vec<_> = fs::read_dir(src)?
        .map(|entry_result| entry_result.map(|entry| entry.path()))
        .collect::<Result<Vec<PathBuf>, io::Error>>()?;
    let mut files: Vec<_> = src_content
        .clone()
        .into_iter()
        .filter(|entry| entry.is_file())
        .collect();
    let mut dirs: Vec<_> = src_content
        .into_iter()
        .filter(|entry| entry.is_dir())
        .collect();
    dirs.sort();

    files.sort_by(|a, b| {
        let num_a = extract_leading_number(a);
        let num_b = extract_leading_number(b);

        match (num_a, num_b) {
            // Both paths have leading numbers: compare the numbers if the are different
            (Some(na), Some(nb)) => {
                if na == nb {
                    return a.cmp(b);
                }
                na.cmp(&nb)
            }
            // Only path 'a' has a leading number: 'a' comes first
            (Some(_), None) => Ordering::Less,
            // Only path 'b' has a leading number: 'b' comes first
            (None, Some(_)) => Ordering::Greater,
            // Neither path has a leading number: sort them alphabetically
            // PathBuf implements Ord, which performs a lexicographical comparison.
            (None, None) => a.cmp(b),
        }
    });

    for f in &files {
        let os_filename = f
            .file_name()
            .ok_or_else(|| anyhow!("Unexpected error - expected filename but none found"))?;
        let mut dest = dest.to_path_buf();
        dest.push(os_filename);

        if dest.exists() && !override_files {
            bar.inc(1);
            continue;
        }

        if let Some(parent_dir) = dest.parent() {
            fs::create_dir_all(parent_dir)?;
        }

        fs::copy(f, &dest).with_context(|| {
            format!(
                "Failed to copy file '{}' to '{}'",
                f.to_str().unwrap_or("unknown"),
                dest.to_str().unwrap_or("unknown"),
            )
        })?;

        sleep(Duration::from_millis(delay_ms));
        bar.inc(1);
    }

    for d in &dirs {
        let last_dir = d.file_name().ok_or_else(|| {
            anyhow!("Unexpected error - expected parent directory but none found")
        })?;
        let mut dest = dest.to_path_buf();
        dest.push(last_dir);
        copy_music(d, &dest, delay_ms, override_files, bar)?;
    }

    Ok(())
}

fn extract_leading_number(path: &Path) -> Option<u64> {
    let os_filename = path.file_name()?;
    let filename = os_filename.to_str()?;

    let num_part_len = filename.chars().take_while(char::is_ascii_digit).count();
    if num_part_len == 0 {
        return None;
    }

    let num_str = &filename[0..num_part_len];
    num_str.parse().ok()
}

fn count_files(dir: &PathBuf) -> anyhow::Result<u64> {
    let mut count = 0;

    validate_dir(dir)?;

    for entry_result in fs::read_dir(dir)? {
        let entry = entry_result?; // Handle potential error reading a specific entry
        let path = entry.path();

        if path.is_file() {
            count += 1;
        } else if path.is_dir() {
            count += count_files(&path)?;
        }
    }

    Ok(count)
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::RemovePrefix { prefix, ext, dir } => remove_prefix(prefix, ext, dir),
        Commands::CopyMusic {
            src,
            dest,
            delay_ms,
            override_files,
        } => start_copying_music(src, dest, *delay_ms, *override_files),
    }
}
