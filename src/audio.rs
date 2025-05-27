use anyhow::{Context, anyhow};
use indicatif::ProgressBar;
use serde::Serialize;
use std::{
    collections::{BTreeSet, HashMap},
    fs, io,
    path::{Path, PathBuf},
    thread::sleep,
    time::Duration,
};

use crate::file_utils;

static SUPPORTED_AUDIO_EXTENSIONS: &[&str] = &[
    "flac", // Free Lossless Audio Codec
];

#[derive(Serialize)]
struct SongsAnalysis {
    artists: Vec<String>,
    albums: Vec<String>,
    missing_song_info: MissingSongInfo,
    misc: MiscSongInfo,
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

        let albums = song_metadata
            .iter()
            .filter(|metadata| metadata.album.is_some())
            .map(|metadata| {
                let album = metadata.album.clone().unwrap();
                let artist = metadata
                    .artist
                    .clone()
                    .unwrap_or_else(|| "Unknown artist".to_string());
                (artist, album)
            })
            .collect::<BTreeSet<(String, String)>>()
            .into_iter()
            .map(|(artist, album)| format!("{album} ({artist})"))
            .collect();

        Self {
            artists: song_metadata
                .iter()
                .filter_map(|val| val.artist.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            albums,
            missing_song_info: MissingSongInfo {
                artist,
                title,
                album,
                disc_number,
                track_number,
            },
            misc: MiscSongInfo {
                most_digits_in_track_number: song_metadata
                    .iter()
                    .map(|metadata| {
                        metadata
                            .track_number
                            .map_or(0, |val| val.checked_ilog10().unwrap_or(0) + 1)
                    })
                    .max()
                    .unwrap_or(0),
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
struct MiscSongInfo {
    most_digits_in_track_number: u32,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Clone)]
struct SongMetadata {
    artist: Option<String>,
    title: Option<String>,
    album: Option<String>,
    disc_number: Option<u32>,
    track_number: Option<u32>,
    filepath: PathBuf,
}

impl SongMetadata {
    fn from_file(filepath: &Path) -> anyhow::Result<Self> {
        let tag = metaflac::Tag::read_from_path(filepath).with_context(|| {
            anyhow!(
                "Unable to read '{}' metadata",
                filepath.to_str().unwrap_or("unknown")
            )
        })?;

        let get_entry_from_tag = |key: &str| -> Option<String> {
            tag.get_vorbis(key)
                .and_then(|mut entries| entries.next())
                .map(str::to_string)
        };

        Ok(Self {
            filepath: filepath.to_path_buf(),
            artist: get_entry_from_tag("ALBUMARTIST"),
            title: get_entry_from_tag("TITLE"),
            album: get_entry_from_tag("ALBUM"),
            disc_number: get_entry_from_tag("DISCNUMBER").and_then(|val| val.parse::<u32>().ok()),
            track_number: get_entry_from_tag("TRACKNUMBER")
                .and_then(|val| Self::parse_track_number(&val)),
        })
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

type AllSongMetadata = HashMap<String, Vec<String>>;

fn get_all_song_metadata_from_file(filepath: &Path) -> anyhow::Result<AllSongMetadata> {
    let tag = metaflac::Tag::read_from_path(filepath).with_context(|| {
        anyhow!(
            "Unable to read '{}' metadata",
            filepath.to_str().unwrap_or("unknown")
        )
    })?;

    Ok(tag
        .vorbis_comments()
        .map(|c| c.comments.clone())
        .unwrap_or_default())
}

pub fn start_analyze_music(src: &Path, output: &Path) -> anyhow::Result<()> {
    if src.is_file() {
        let song_metadata = SongMetadata::from_file(src)?;
        store_song_metadata(vec![song_metadata], output)?;
        return Ok(());
    }

    let file_count = file_utils::count_files_by_extension(src, SUPPORTED_AUDIO_EXTENSIONS)?;
    let bar: ProgressBar = ProgressBar::new(file_count);

    let results: Vec<_> = analyze_music(src, &bar)?;
    bar.finish();
    store_song_metadata(results, output)?;

    Ok(())
}

fn analyze_music(dir: &Path, bar: &ProgressBar) -> anyhow::Result<Vec<SongMetadata>> {
    let mut results = vec![];

    let (audio_files, dirs) = file_utils::walk_directory(dir, SUPPORTED_AUDIO_EXTENSIONS)?;

    for f in audio_files {
        let song_metadata = SongMetadata::from_file(&f)?;
        results.push(song_metadata);
        bar.inc(1);
    }

    for d in &dirs {
        let dir_results = analyze_music(d, bar)?;
        results.extend(dir_results);
    }

    Ok(results)
}

fn store_song_metadata(results: Vec<SongMetadata>, output: &Path) -> anyhow::Result<()> {
    let analysis: SongsAnalysis = SongsAnalysis::from_song_metadata(results);
    let json_data = serde_json::to_string(&analysis)?;

    file_utils::store_data(output, &json_data)
}

pub fn start_get_all_metadata(src: &Path, output: &Path) -> anyhow::Result<()> {
    if src.is_file() {
        let song_metadata: HashMap<String, Vec<String>> = get_all_song_metadata_from_file(src)?;
        store_all_song_metadata(&[song_metadata], output)?;
        return Ok(());
    }

    let file_count = file_utils::count_files_by_extension(src, SUPPORTED_AUDIO_EXTENSIONS)?;
    let bar: ProgressBar = ProgressBar::new(file_count);

    let results: Vec<_> = get_all_metadata(src, &bar)?;
    bar.finish();
    store_all_song_metadata(&results, output)?;

    Ok(())
}

fn get_all_metadata(dir: &Path, bar: &ProgressBar) -> anyhow::Result<Vec<AllSongMetadata>> {
    let mut results = vec![];

    let (audio_files, dirs) = file_utils::walk_directory(dir, SUPPORTED_AUDIO_EXTENSIONS)?;

    for f in audio_files {
        let song_metadata = get_all_song_metadata_from_file(&f)?;
        results.push(song_metadata);
        bar.inc(1);
    }

    for d in &dirs {
        let dir_results = get_all_metadata(d, bar)?;
        results.extend(dir_results);
    }

    Ok(results)
}

fn store_all_song_metadata(results: &[AllSongMetadata], output: &Path) -> anyhow::Result<()> {
    let json_data = serde_json::to_string(&results)?;

    file_utils::store_data(output, &json_data)
}

pub struct CopyFileOptions<'a> {
    pub filename_template: &'a str,
    pub delay_ms: u64,
    pub override_files: bool,
    pub pad_width: usize,
    pub fat_32: bool,
}

#[derive(clap::ValueEnum, Copy, Clone, PartialEq, Eq)]
pub enum TrackNumberModification {
    None,
    Number,
    PaddedNumber,
    IncludeDiscNumber,
}

pub struct CopyMetadataOptions {
    pub track_number_modification: TrackNumberModification,
}

pub fn start_copy_music(
    src: &Path,
    dest: &Path,
    file_options: &CopyFileOptions,
    metadata_options: &CopyMetadataOptions,
) -> anyhow::Result<()> {
    file_utils::validate_dir(src)?;

    let file_count = file_utils::count_files(src)?;
    let bar = ProgressBar::new(file_count);

    let template = mustache::compile_str(file_options.filename_template)?;

    let result = copy_music(src, dest, &template, file_options, metadata_options, &bar);
    bar.finish();

    result
}

fn copy_music(
    src: &Path,
    dest: &Path,
    filename_template: &mustache::Template,
    file_options: &CopyFileOptions,
    metadata_options: &CopyMetadataOptions,
    bar: &ProgressBar,
) -> anyhow::Result<()> {
    file_utils::validate_dir(src)?;

    let src_content: Vec<_> = fs::read_dir(src)?
        .map(|entry_result| entry_result.map(|entry| entry.path()))
        .collect::<Result<Vec<PathBuf>, io::Error>>()?;
    let files = src_content
        .clone()
        .into_iter()
        .filter(|entry| entry.is_file());
    let mut dirs: Vec<_> = src_content
        .into_iter()
        .filter(|entry| entry.is_dir())
        .collect();
    dirs.sort();

    let (audio_files, other_files): (Vec<PathBuf>, Vec<PathBuf>) =
        files.partition(|f| file_utils::file_has_extension(f, SUPPORTED_AUDIO_EXTENSIONS));

    let mut songs = audio_files
        .into_iter()
        .map(|path| SongMetadata::from_file(&path))
        .collect::<anyhow::Result<Vec<_>>>()?;

    songs.sort_by(|a, b| {
        a.disc_number
            .cmp(&b.disc_number)
            .then(a.track_number.cmp(&b.track_number))
    });

    let pad_width = file_options.pad_width;

    for song in &songs {
        let data = mustache::MapBuilder::new()
            .insert("artist", &song.artist)?
            .insert("title", &song.title)?
            .insert("album", &song.album)?
            .insert(
                "disc_number",
                &song.disc_number.map(|val| format!("{val:0>pad_width$}")),
            )?
            .insert(
                "track_number",
                &song.track_number.map(|val| format!("{val:0>pad_width$}")),
            )?
            .build();
        let filename = filename_template.render_data_to_string(&data)?;

        let extension = song
            .filepath
            .extension()
            .ok_or_else(|| anyhow!("Unexpected error - no extension found"))?;
        let mut dest: PathBuf = dest.to_path_buf();
        dest.push(filename);
        dest.set_extension(extension);

        let dest = file_utils::copy_file(
            &song.filepath,
            &dest,
            file_options.override_files,
            file_options.fat_32,
        )?;
        if let Some(dest) = dest {
            modify_file_metadata(&dest, song, metadata_options)?;
            sleep(Duration::from_millis(file_options.delay_ms));
        }

        bar.inc(1);
    }

    for f in &other_files {
        let os_filename: &std::ffi::OsStr = f
            .file_name()
            .ok_or_else(|| anyhow!("Unexpected error - expected filename but none found"))?;
        let mut dest = dest.to_path_buf();
        dest.push(os_filename);

        file_utils::copy_file(f, &dest, file_options.override_files, file_options.fat_32)?;

        bar.inc(1);
    }

    for d in &dirs {
        let last_dir = d.file_name().ok_or_else(|| {
            anyhow!("Unexpected error - expected parent directory but none found")
        })?;
        let mut dest = dest.to_path_buf();
        dest.push(last_dir);
        copy_music(
            d,
            &dest,
            filename_template,
            file_options,
            metadata_options,
            bar,
        )?;
    }

    Ok(())
}

fn modify_file_metadata(
    dest: &Path,
    song_metadata: &SongMetadata,
    metadata_options: &CopyMetadataOptions,
) -> anyhow::Result<()> {
    if metadata_options.track_number_modification == TrackNumberModification::None
        || song_metadata.track_number.is_none()
    {
        return Ok(());
    }

    let mut tag = metaflac::Tag::read_from_path(dest).with_context(|| {
        anyhow!(
            "Unable to read '{}' metadata",
            dest.to_str().unwrap_or("unknown")
        )
    })?;

    let track_number = song_metadata.track_number.unwrap();

    let new_disc_number = match metadata_options.track_number_modification {
        TrackNumberModification::None => None,
        TrackNumberModification::Number => Some(track_number.to_string()),
        TrackNumberModification::PaddedNumber => Some(format!("{track_number:0>2}")),
        TrackNumberModification::IncludeDiscNumber => {
            let padded_number = format!("{track_number:0>2}");
            let disc_number = song_metadata.disc_number.unwrap_or(0).to_string();
            Some(format!("{disc_number}{padded_number}"))
        }
    }
    .unwrap();

    tag.set_vorbis("TRACKNUMBER", vec![new_disc_number]);
    tag.write_to_path(dest)?;

    Ok(())
}
