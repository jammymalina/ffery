# ffery 🦀

**ffery** (short for *file f✨ery*, use with caution!) is a small command-line utility written in Rust designed for performing bulk operations on files within a directory.

⚠️ **Warning:** This tool modifies files directly on your filesystem based on the commands given. Operations might be irreversible. **Always back up your data before using `ffery` or test it in a safe, non-critical directory first.**

## Features

Currently, `ffery` supports the following command:

*   **`remove-prefix`**: Bulk renames files in a directory by removing a specified prefix, filtered by extension. Useful for cleaning up downloads or recordings (e.g., removing "AUDIO_").
*   **`analyze-music`**: Recursively scans a source directory for music files, extracts metadata (tags), and saves the analysis to a specified file (JSON format). Useful for inspecting your library's tags.
*   **`copy-music`** Recursively copies music files from a source to a destination directory. This command is specifically designed for older/simpler music players (like some car stereos or basic MP3 players) that play files in the order they were written to the filesystem, rather than using tag information or alphabetical order. It sorts files based on metadata (album, disc number, track number) before copying and allows custom filename formatting using tags and a mustache template.


## Installation

1.  **Prerequisites:** Ensure you have the Rust toolchain installed (see [rustup.rs](https://rustup.rs/)).

### Option 1: From Crates.io

```bash
cargo install ffery
```

### Option 2: From Source

2.  **Clone the repository:**
    ```bash
    git clone https://github.com/jammymalina/ffery
    cd ffery
    ```
3.  **Build and Install:**
    ```bash
    cargo install --path .
    ```
    This will build the `ffery` binary and place it in your Cargo binary path (`~/.cargo/bin/` by default), making it available system-wide.

## Usage

`ffery` is run from the command line.

### General Help

To see the available commands and general options:
```bash
ffery --help
```

### Command-Specific Help

For detailed help on a specific command and its options:
```bash
ffery <COMMAND> --help
# Example:
ffery remove-prefix --help
ffery copy-music --help
```

## Commands

### remove-prefix

Removes a specified prefix from filenames matching a given extension within a target directory.

```bash
ffery remove-prefix --prefix <PREFIX_TO_REMOVE> --ext <FILE_EXTENSION> <TARGET_DIRECTORY>
```

**Arguments:**
- --prefix &lt;STRING&gt;: The exact prefix string to remove from the beginning of filenames.
- --ext &lt;STRING&gt;: The file extension (without the dot) to target (e.g., mp3, flac). Only files with this extension will be considered.
- &lt;PATH&gt;: The path to the directory containing the files to process.

### analyze-music

Scans a source directory for music files, extracts metadata, and saves the analysis results. Only flac files are supported.

```bash
ffery analyze-music --src <SOURCE_DIRECTORY> --result <OUTPUT_FILE_PATH>
```

**Arguments:**
- --src &lt;PATH&gt;: The path to the source directory containing music files to analyze. The scan is recursive.
- --result &lt;PATH&gt;: The path where the analysis results will be saved (e.g., analysis.json).

### copy-music

Copies music files from a source to a destination, sorting them by metadata (album, disc, track) before copying to ensure playback order on simple devices. Allows filename customization using metadata tags. Only flac files are supported.

**Arguments:**
- --src &lt;PATH&gt;: The path to the source directory containing music files. Recursively scans for files.
- --dest &lt;PATH&gt;: The path to the destination directory where files will be copied. The directory structure from the source is generally preserved.
- --delay-ms &lt;MILLISECONDS&gt;: (Optional) A small delay introduced between file copy operations. This can sometimes help ensure the filesystem registers the intended write order. Default: 30.
- --override-files: (Optional) If present, existing files in the destination directory with the same name will be overwritten. Use with caution! Default: Off (files are skipped if they exist).
- --filename-template &lt;TEMPLATE&gt;: (Optional) A mustache template string to format the output filenames. Default: `{{#disc_number}}{{disc_number}}-{{/disc_number}}{{track_number}} {{title}}`. Extension is automatically added at the end.
- --pad-width &lt;NUMBER&gt;: (Optional) The width to pad track and disc numbers with leading zeros in the filename template. Default: 2.

**Filename Template (--filename-template):**

This uses mustache syntax. The default template `{{#disc_number}}{{disc_number}}-{{/disc_number}}{{track_number}} {{title}}` means:
- If a disc_number tag exists, output &lt;disc_number&gt;-.
- Output the track_number.
- Output a space, then the title.
- Track and disc numbers are padded according to --pad-width.

Available template variables include:
- artist
- title
- album
- track_number
- disc_number

**Any of the template variables can be null/empty if the audio file metadata does not contain them!**

*Example 1: Basic copy for a simple DAP*

Copy music from ~/Music/Albums to a USB drive mounted at /mnt/usb, using default naming and padding:
```bash
ffery copy-music --src ~/Music/Albums --dest /mnt/usb
# Example output filename: 01-05 Song Title.flac (Disc 1, Track 5)
# Example output filename: 08 Another Song.flac (No Disc number, Track 8)
```

*Example 2: Custom filename format, wider padding, and overwriting*

Copy music, formatting filenames as Artist - Album - Track Title, padding numbers to 3 digits, and overwriting any existing files on the destination:
```bash
ffery copy-music \
    --src ~/Music/RippedCDs \
    --dest /media/player/Music \
    --filename-template "{{artist}} - {{album}} - {{track_number}} {{title}}" \
    --pad-width 3 \
    --override-files
# Example output filename: Some Artist - Great Album - 005 Song Title.flac
```

