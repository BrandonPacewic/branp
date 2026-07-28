use std::fs;
use std::io;
use std::path::Path;

use crate::errors::{CliError, CliResult};

pub fn path_exists_or_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

pub fn remove_path(path: &Path) -> CliResult {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn ensure_symlink(source: &Path, link: &Path, force: bool) -> CliResult {
    if is_same_link_target(link, source)? {
        return Ok(());
    }
    if path_exists_or_symlink(link) {
        if force {
            remove_path(link)?;
        } else {
            return Err(CliError::from(format!("path already exists: {}; pass --force to replace it", link.display())));
        }
    }
    if let Some(parent) = link.parent() {
        fs::create_dir_all(parent)?;
    }
    symlink_path(source, link)
}

pub fn is_same_link_target(path: &Path, target: &Path) -> Result<bool, CliError> {
    match fs::read_link(path) {
        Ok(link_target) => Ok(link_target == target),
        Err(error) if error.kind() == io::ErrorKind::InvalidInput || error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

#[cfg(unix)]
fn symlink_path(source: &Path, link: &Path) -> CliResult {
    std::os::unix::fs::symlink(source, link)?;
    Ok(())
}

#[cfg(windows)]
fn symlink_path(source: &Path, link: &Path) -> CliResult {
    if fs::metadata(source)?.is_dir() {
        std::os::windows::fs::symlink_dir(source, link)?;
    } else {
        std::os::windows::fs::symlink_file(source, link)?;
    }
    Ok(())
}
