use std::fmt;

pub struct CommitInfo {
    pub commit_hash: String,
    pub short_commit_hash: String,
    pub commit_date: String,
}

pub struct VersionInfo {
    pub version: String,

    /// [`None`] if branp was not built from a detectable Git repo.
    pub commit_info: Option<CommitInfo>,
}

impl fmt::Display for VersionInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.version)?;
        Ok(())
    }
}

pub fn get_version_info() -> VersionInfo {
    macro_rules! option_env_str {
        ($name:expr) => {
            option_env!($name).map(|s| s.to_string())
        };
    }

    let version = option_env_str!("CARGO_PKG_VERSION").unwrap_or_else(|| "unknown".to_string());

    // Use `commit_hash` to determine if branp was built with git information. If `commit_hash` is
    // `None`, then its safe to assume that the commit info is not available for the current binary.
    let commit_info = option_env_str!("BRANP_GIT_HASH").map(|commit_hash| CommitInfo {
        commit_hash,
        short_commit_hash: option_env_str!("BRANP_GIT_SHORT_HASH").unwrap_or_default(),
        commit_date: option_env_str!("BRANP_GIT_DATE").unwrap_or_default(),
    });

    VersionInfo { version, commit_info }
}
