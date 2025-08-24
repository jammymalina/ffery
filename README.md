# ffery 🦀

**ffery** (short for *file f✨ery*, use with caution!) is a small command-line utility written in Rust designed for performing bulk operations on files within a directory.

⚠️ **Warning:** This tool modifies files directly on your filesystem based on the commands given. Operations might be irreversible. **Always back up your data before using `ffery` or test it in a safe, non-critical directory first.**

## Features

Currently, `ffery` supports the following command:

*   **`remove-prefix`**: Bulk renames files in a directory by removing a specified prefix, filtered by extension. Useful for cleaning up downloads or recordings (e.g., removing "AUDIO_").
*   **`analyze-music`**: Recursively scans a source directory for music files, extracts metadata (tags), and saves the analysis to a specified file (JSON format). Useful for inspecting your library's tags.
*   **`copy-music`** Recursively copies music files from a source to a destination directory. This command is specifically designed for older/simpler music players (like some car stereos or basic MP3 players) that play files in the order they were written to the filesystem, rather than using tag information or alphabetical order. It sorts files based on metadata (album, disc number, track number) before copying. It allows custom filename formatting using tags and a mustache template, can sanitize filenames for FAT32 compatibility, and offers options to modify track number metadata of the copied file.


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
- `--prefix <STRING> (-p)`: The exact prefix string to remove from the beginning of filenames.
- `--ext <STRING> (-e)`: The file extension (without the dot) to target (e.g., mp3, flac). Only files with this extension will be considered.
- `<PATH>`: The path to the directory containing the files to process.

### analyze-music

Scans a source directory for music files, extracts metadata, and saves the analysis results. Only flac files are supported.

```bash
ffery analyze-music --result <OUTPUT_FILE_PATH> <SOURCE_DIRECTORY>
```

**Arguments:**
- `--result <PATH> (-r)`: The path where the analysis results will be saved (e.g., analysis.json).
- `<PATH>`: The path to the source directory containing music files to analyze. The scan is recursive.

### copy-music

Copies music files from a source to a destination, sorting them by metadata (album, disc, track) before copying to ensure playback order on simple devices. Allows filename customization using metadata tags and offers options for metadata pre-processing. Only flac files are supported.

**Arguments:**
- `--src <PATH> (-s)`: The path to the source directory containing music files. Recursively scans for files.
- `--dest <PATH> (-d)`: The path to the destination directory where files will be copied. The directory structure from the source is generally preserved.
- `--delay-ms <MILLISECONDS>`: (Optional) A small delay introduced between file copy operations. This can sometimes help ensure the filesystem registers the intended write order. Default: 30.
- `--override-files (o)`: (Optional) If present, existing files in the destination directory with the same name will be overwritten. Use with caution! Default: Off (files are skipped if they exist).
- `--fat-32`: (Optional) If present, sanitizes filenames to be compatible with FAT32 filesystems (e.g., removes or replaces characters like *, ?, :, etc., and ensures length limits). Default: Off.
- `--filename-template <TEMPLATE> (-t)`: (Optional) A mustache template string to format the output filenames. Default: `"{{#disc_number}}{{{disc_number}}}-{{/disc_number}}{{{track_number}}} {{{title}}}"`. Extension is automatically added at the end.
- `--pad-width <NUMBER>`: (Optional) The width to pad track and disc numbers with leading zeros in the filename template. Default: 2.
- `--metadata-track-number-modification <MODIFICATION_TYPE> (-m)`: (Optional) Modifies the track number of the copied file. Useful if some DAPs cannot handle more complex track numbers (e.g. 3/11) or if they do not take disc number into consideration during sorting. Default: none.
Possible values for `<MODIFICATION_TYPE>`:
    - `none`: No modification to the track number tag. The raw tag value is used.
    - `number`: Extracts only the numerical part of the track number tag (e.g., "1" from "01/12"). If there are padding zeros it removes them.
    - `padded-number`: Same as number, but also pads the extracted number with max 1 leading zero (e.g. "03" from "3").
    - `include-disc-number`: It prepends the disc number to the track number (e.g., track "5" on disc "1" becomes "105"; track "12" on disc "2" becomes "212"). The track number is padded with max 1 leading zero. If the disc number does not exist, it will use disc number "0" as default.

**Filename Template (--filename-template):**

This uses mustache syntax. The default template `"{{#disc_number}}{{{disc_number}}}-{{/disc_number}}{{{track_number}}} {{{title}}}"` means:
- If a disc_number tag exists, output `<disc_number>`.
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
    --metadata-track-number-modification include-disc-number \
    --pad-width 3 \
    --override-files
# Example output filename: Some Artist - Great Album - 005 Song Title.flac
```

*Example 3: Copy music to a FAT32 SD card*
```bash
ffery copy-music --src '/home/$USER/Music/Artists/' --dest '/run/media/$USER/disk/Artists/' -m include-disc-number --fat-32 -o
```

### unzip-music

TBD

```bash
ffery unzip-music --dir-template '{{album}}'  --dest '.' album.zip
```
