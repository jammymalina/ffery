use anyhow::{Context, anyhow};
use indicatif::ProgressBar;
use serde::Serialize;
use std::{
    cmp::Ordering,
    fs, io,
    path::{Path, PathBuf},
    thread::sleep,
    time::Duration,
};

use crate::file_utils::{self, file_has_extension};

static SUPPORTED_AUDIO_EXTENSIONS: &[&str] = &[
    "flac", // Free Lossless Audio Codec
];

#[derive(Serialize)]
struct SongsAnalysis {
    missing_song_info: MissingSongInfo,
    song_metadata: Vec<SongMetadata>,
}

impl SongsAnalysis {
    fn from_song_metadata(song_metadata: Vec<SongMetadata>) -> Self {
        let (artist, title, album, disc_number, track_number) =
            song_metadata.iter().fold((0, 0, 0, 0, 0), |acc, metadata| {
                (
                    acc.0 + u32::from(metadata.artist.is_none()),
                    acc.1 + u32::from(metadata.title.is_none()),
                    acc.2 + u32::from(metadata.album.is_none()),
                    acc.3 + u32::from(metadata.disc_number.is_none()),
                    acc.4 + u32::from(metadata.track_number.is_none()),
                )
            });

        Self {
            missing_song_info: MissingSongInfo {
                artist,
                title,
                album,
                disc_number,
                track_number,
            },
            song_metadata,
        }
    }
}

#[derive(Serialize)]
struct MissingSongInfo {
    artist: u32,
    title: u32,
    album: u32,
    disc_number: u32,
    track_number: u32,
}

#[derive(Serialize)]
struct SongMetadata {
    artist: Option<String>,
    title: Option<String>,
    album: Option<String>,
    disc_number: Option<u32>,
    track_number: Option<u32>,
}

impl SongMetadata {
    fn from_tag(tag: &metaflac::Tag) -> Self {
        let get_entry_from_tag = |key: &str| -> Option<String> {
            tag.get_vorbis(key)
                .and_then(|mut entries| entries.next())
                .map(str::to_string)
        };

        Self {
            artist: get_entry_from_tag("ARTIST"),
            title: get_entry_from_tag("TITLE"),
            album: get_entry_from_tag("ALBUM"),
            disc_number: get_entry_from_tag("DISCNUMBER").and_then(|val| val.parse::<u32>().ok()),
            track_number: get_entry_from_tag("TRACKNUMBER")
                .and_then(|val| Self::parse_track_number(&val)),
        }
    }

    fn parse_track_number(val: &str) -> Option<u32> {
        let num_part_len = val.chars().take_while(char::is_ascii_digit).count();
        if num_part_len == 0 {
            return None;
        }

        let num_str = &val[0..num_part_len];
        num_str.parse().ok()
    }
}

pub fn start_analyze_music(src: &PathBuf, output: &PathBuf) -> anyhow::Result<()> {
    if src.is_file() {
        let song_metadata = get_song_metadata(src)?;
        store_song_metadata(vec![song_metadata], output)?;
        return Ok(());
    }

    let file_count = file_utils::count_files_by_extension(src, SUPPORTED_AUDIO_EXTENSIONS)?;
    let bar = ProgressBar::new(file_count);

    let results: Vec<_> = analyze_music(src, &bar)?;
    bar.finish();
    store_song_metadata(results, output)?;

    Ok(())
}

fn analyze_music(dir: &PathBuf, bar: &ProgressBar) -> anyhow::Result<Vec<SongMetadata>> {
    file_utils::validate_dir(dir)?;

    let mut results = vec![];

    let src_content: Vec<_> = fs::read_dir(dir)?
        .map(|entry_result| entry_result.map(|entry| entry.path()))
        .collect::<Result<Vec<PathBuf>, io::Error>>()?;
    let mut audio_files: Vec<_> = src_content
        .clone()
        .into_iter()
        .filter(|entry| entry.is_file() && file_has_extension(entry, SUPPORTED_AUDIO_EXTENSIONS))
        .collect();
    let mut dirs: Vec<_> = src_content
        .into_iter()
        .filter(|entry| entry.is_dir())
        .collect();
    dirs.sort();

    audio_files.sort();

    for f in &audio_files {
        let song_metadata = get_song_metadata(f)?;
        results.push(song_metadata);
        bar.inc(1);
    }

    for d in &dirs {
        let dir_results = analyze_music(d, bar)?;
        results.extend(dir_results);
    }

    Ok(results)
}

fn get_song_metadata(f: &Path) -> anyhow::Result<SongMetadata> {
    let tag = metaflac::Tag::read_from_path(f).with_context(|| {
        anyhow!(
            "Unable to read '{}' metadata",
            f.to_str().unwrap_or("unknown")
        )
    })?;
    Ok(SongMetadata::from_tag(&tag))
}

fn store_song_metadata(results: Vec<SongMetadata>, output: &PathBuf) -> anyhow::Result<()> {
    if let Some(parent_dir) = output.parent() {
        fs::create_dir_all(parent_dir)?;
    }

    let analysis = SongsAnalysis::from_song_metadata(results);

    let json_data = serde_json::to_string(&analysis)?;
    fs::write(output, json_data)?;

    Ok(())
}

pub fn start_copying_music(
    src: &PathBuf,
    dest: &Path,
    delay_ms: u64,
    override_files: bool,
) -> anyhow::Result<()> {
    file_utils::validate_dir(src)?;

    let file_count = file_utils::count_files(src)?;
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
    file_utils::validate_dir(src)?;

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
