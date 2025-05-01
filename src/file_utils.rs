use anyhow::{Context, anyhow};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

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

pub fn copy_file(src: &PathBuf, dest: &PathBuf, override_file: bool) -> anyhow::Result<()> {
    if dest.exists() && !override_file {
        return Ok(());
    }

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
        let entry = entry_result?; // Handle potential error reading a specific entry
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
