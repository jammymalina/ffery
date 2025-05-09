use anyhow::{Context, anyhow};
use phf::phf_set;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

static FORBIDDEN_CHARS: phf::Set<char> = phf_set! {
    // Explicitly forbidden printable ASCII
    '<', '>', ':', '"', '/', '\\', '|', '?', '*',
    // ASCII Control Characters (0-31)
    '\x00', '\x01', '\x02', '\x03', '\x04', '\x05', '\x06', '\x07',
    '\x08', '\x09', '\x0A', '\x0B', '\x0C', '\x0D', '\x0E', '\x0F',
    '\x10', '\x11', '\x12', '\x13', '\x14', '\x15', '\x16', '\x17',
    '\x18', '\x19', '\x1A', '\x1B', '\x1C', '\x1D', '\x1E', '\x1F',
    // While not strictly a control character, DEL (127) can also be problematic
    '\x7F',
};

static RESERVED_NAMES_UPPERCASE: phf::Set<&'static str> = phf_set! {
    "CON", "PRN", "AUX", "NUL",
    "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
    "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
};

const MAX_FILENAME_LEN: usize = 255;
const REPLACEMENT_CHAR: char = '_';

pub fn sanitize_pathbuf_for_fat32(path: &Path) -> PathBuf {
    path.file_name().map_or_else(
        || path.to_path_buf(),
        |filename_osstr| {
            let filename_str = filename_osstr.to_string_lossy();
            let sanitized_filename: String = sanitize_filename_string(&filename_str);
            if filename_str == sanitized_filename {
                path.to_path_buf()
            } else {
                path.with_file_name(sanitized_filename)
            }
        },
    )
}

fn sanitize_filename_string(filename: &str) -> String {
    // Separate stem and extension
    let path_repr = Path::new(filename);
    let original_stem = path_repr
        .file_stem()
        .unwrap_or_else(|| OsStr::new(""))
        .to_string_lossy();
    let original_extension = path_repr
        .extension()
        .unwrap_or_else(|| OsStr::new(""))
        .to_string_lossy();

    let mut sanitized_stem: String = original_stem
        .chars()
        .map(|c| {
            if FORBIDDEN_CHARS.contains(&c) {
                REPLACEMENT_CHAR
            } else {
                c
            }
        })
        .collect();

    // 2. Trim trailing spaces and periods from stem
    while sanitized_stem.ends_with(' ') || sanitized_stem.ends_with('.') {
        sanitized_stem.pop();
    }

    // 3. Handle potentially empty stem after sanitization/trimming
    if sanitized_stem.is_empty() {
        sanitized_stem.push(REPLACEMENT_CHAR);
    }

    // 4. Check against reserved names (case-insensitive)
    if RESERVED_NAMES_UPPERCASE.contains(sanitized_stem.to_uppercase().as_str()) {
        sanitized_stem.push(REPLACEMENT_CHAR); // Append replacement char if reserved
    }

    // 5. Sanitize characters in extension
    let sanitized_extension: String = original_extension
        .chars()
        .map(|c| {
            // Use PHF set + explicit check for '.' and ' ' in extension
            if FORBIDDEN_CHARS.contains(&c) || c == '.' || c == ' ' {
                REPLACEMENT_CHAR
            } else {
                c
            }
        })
        .collect();

    // 6. Handle Length Constraint
    let ext_len = if sanitized_extension.is_empty() {
        0
    } else {
        sanitized_extension.chars().count()
    };
    let dot_len = usize::from(ext_len > 0);
    let max_stem_len = MAX_FILENAME_LEN
        .saturating_sub(ext_len)
        .saturating_sub(dot_len);

    if sanitized_stem.chars().count() > max_stem_len {
        sanitized_stem = sanitized_stem.chars().take(max_stem_len).collect();
        // Re-trim after truncation
        while sanitized_stem.ends_with(' ') || sanitized_stem.ends_with('.') {
            sanitized_stem.pop();
        }
        // Ensure stem is not empty after truncation/trimming
        if sanitized_stem.is_empty() {
            sanitized_stem.push(REPLACEMENT_CHAR);
        }
        // Re-check reserved names *if* truncation could have created one
        if RESERVED_NAMES_UPPERCASE.contains(sanitized_stem.to_uppercase().as_str()) {
            if sanitized_stem.chars().count() < max_stem_len {
                sanitized_stem.push(REPLACEMENT_CHAR);
            } else if max_stem_len > 0 {
                sanitized_stem.pop();
                sanitized_stem.push(REPLACEMENT_CHAR);
            }
        }
    }

    // 7. Reassemble the filename
    let mut final_filename = sanitized_stem;
    if !sanitized_extension.is_empty() {
        final_filename.push('.');
        final_filename.push_str(&sanitized_extension);
    }

    // Final check for empty result
    if final_filename.is_empty() {
        return REPLACEMENT_CHAR.to_string();
    }

    final_filename
}

pub fn validate_dir(dir: &Path) -> anyhow::Result<()> {
    let d = dir.to_str().unwrap_or("unknown");
    if !dir.exists() {
        return Err(anyhow!("Path '{d}' does not exist"));
    }

    if !dir.is_dir() {
        return Err(anyhow!("Path '{d}' must be a directory"));
    }

    Ok(())
}

pub fn count_files(dir: &PathBuf) -> anyhow::Result<u64> {
    count_files_recursive(dir, None)
}

pub fn count_files_by_extension(dir: &PathBuf, extensions: &[&str]) -> anyhow::Result<u64> {
    count_files_recursive(dir, Some(extensions))
}

pub fn file_has_extension(f: &Path, extensions: &[&str]) -> bool {
    let extension = f.extension();
    extension.is_some_and(|extension| {
        extension
            .to_str()
            .is_some_and(|extension| extensions.contains(&extension))
    })
}

pub fn copy_file(
    src: &PathBuf,
    dest: &PathBuf,
    override_file: bool,
    fat_32: bool,
) -> anyhow::Result<()> {
    if dest.exists() && !override_file {
        return Ok(());
    }

    let dest = if fat_32 {
        &sanitize_pathbuf_for_fat32(dest)
    } else {
        dest
    };

    if let Some(parent_dir) = dest.parent() {
        fs::create_dir_all(parent_dir)?;
    }

    fs::copy(src, dest).with_context(|| {
        format!(
            "Failed to copy file '{}' to '{}'",
            src.to_str().unwrap_or("unknown"),
            dest.to_str().unwrap_or("unknown"),
        )
    })?;

    Ok(())
}

pub fn remove_prefix_from_files(prefix: &str, ext: &str, dir: &PathBuf) -> anyhow::Result<()> {
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

fn count_files_recursive(dir: &PathBuf, extensions: Option<&[&str]>) -> anyhow::Result<u64> {
    let mut count = 0;

    validate_dir(dir)?;

    for entry_result in fs::read_dir(dir)? {
        let entry = entry_result?;
        let path = entry.path();

        if path.is_file() {
            if let Some(extensions) = extensions {
                if !file_has_extension(&path, extensions) {
                    continue;
                }
            }
            count += 1;
        } else if path.is_dir() {
            count += count_files_recursive(&path, extensions)?;
        }
    }

    Ok(count)
}
