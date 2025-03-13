//! Build script for `branp` to embed Git commit metadata into the binary.
//!
//! This script runs automatically before the build process and extracts Git commit
//! information from the current repository. The extracted metadata includes:
//! 
//! - The full commit hash (`BRANP_GIT_HASH`)
//! - A short commit hash (`BRANP_GIT_SHORT_HASH`)
//! - The commit date (`BRANP_GIT_DATE`)
//!
//! This information is embedded as environment variables and can be accessed at runtime
//! to provide more verbose version details with `bp --version`.
//!
//! This script runs before every build but will not affect caching unless the Git commit changes.
//! If `.git/` is missing (e.g., when using a source tarball), no commit info will be available
//! to the final binary and the git metadata will be omitted from the version output.
//! 

use std::path::Path;
use std::process::Command;

fn main() {
    if let Some(git) = commit_info_from_git() {
        println!("cargo:rustc-env=BRANP_GIT_HASH={}", git.hash);
        println!("cargo:rustc-env=BRANP_GIT_SHORT_HASH={}", git.short_hash);
        println!("cargo:rustc-env=BRANP_GIT_DATE={}", git.date);
    };
}

struct CommitInfo {
    hash: String,
    short_hash: String,
    date: String,
}

fn commit_info_from_git() -> Option<CommitInfo> {
    if !Path::new(".git").exists() {
        return None;
    }

    // Retrieve the latest commit information using `git log`.
    //
    // `--format=%H %h %cd` is used to output the commit metadata in a space-separated format:
    //   - `%H`: The full 40-character commit hash.
    //   - `%h`: The short commit hash (abbreviated version).
    //   - `%cd`: The commit date, formatted as a short date (`YYYY-MM-DD`).
    //
    // `--date=short` trims the commit date to `YYYY-MM-DD`, omitting the time component.
    // `--abbrev=9` flag prevents Git from dynamically adjusting the short hash length,
    // a length of 9 is good enough for consistency.
    //
    // The order of fields must align with the `CommitInfo` struct.
    //
    // If the `git log` command fails for any reason (e.g., not in a Git repository), we return `None`.
    // Verbose version output is built to handle this and the commit info will simply be omitted.
    let output = match Command::new("git")
        .arg("log")
        .arg("-1")
        .arg("--date=short")
        .arg("--format=%H %h %cd")
        .arg("--abbrev=9")
        .output()
    {
        Ok(output) => output,
        _ => return None,
    };

    let stdout = String::from_utf8(output.stdout).unwrap();
    let mut parts = stdout.split_whitespace().map(|s| s.to_string());
    Some(CommitInfo {
        hash: parts.next()?,
        short_hash: parts.next()?,
        date: parts.next()?,
    })
}
