use anyhow::anyhow;
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
