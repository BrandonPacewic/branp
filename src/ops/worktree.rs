use std::fs;
use std::path::{Component, Path, PathBuf};

use toml_edit::{value, Array, DocumentMut, Item};

use crate::config::RepoConfig;
use crate::errors::{CliError, CliResult};

pub struct LinkStore {
    config: RepoConfig,
    current: PathBuf,
}

impl LinkStore {
    pub fn new(base: impl Into<PathBuf>, current: impl Into<PathBuf>) -> Self {
        Self { config: RepoConfig::new(base), current: current.into() }
    }

    pub fn track(&self, paths: &[&str], worktrees: &[PathBuf], force: bool) -> Result<Vec<PathBuf>, CliError> {
        let mut config = LinkConfig::read(self)?;
        let mut tracked = Vec::new();

        for path in paths {
            let rel = normalize_link_path(path)?;
            let source = self.current.join(&rel);
            if !crate::utils::fs::path_exists_or_symlink(&source) {
                return Err(CliError::from(format!("cannot track missing path: {}", source.display())));
            }

            let storage = self.link_store_dir()?.join(&rel);
            if !crate::utils::fs::path_exists_or_symlink(&storage) {
                if let Some(parent) = storage.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::rename(&source, &storage)?;
            } else if !crate::utils::fs::is_same_link_target(&source, &storage)? {
                if force {
                    crate::utils::fs::remove_path(&source)?;
                } else {
                    return Err(CliError::from(format!(
                        "shared copy already exists for {}; pass --force to replace this worktree path with a symlink",
                        rel.display()
                    )));
                }
            }

            config.add(&rel)?;
            tracked.push(rel);
        }

        config.write(self)?;
        self.sync(worktrees, force)?;
        Ok(tracked)
    }

    pub fn linked_paths(&self) -> Result<Vec<PathBuf>, CliError> {
        Ok(LinkConfig::read(self)?.paths)
    }

    pub fn sync(&self, worktrees: &[PathBuf], force: bool) -> CliResult {
        let config = LinkConfig::read(self)?;
        for rel in config.paths {
            let storage = self.link_store_dir()?.join(&rel);
            if !crate::utils::fs::path_exists_or_symlink(&storage) {
                return Err(CliError::from(format!("linked source is missing for {}: {}", rel.display(), storage.display())));
            }

            for worktree in worktrees {
                crate::utils::fs::ensure_symlink(&storage, &worktree.join(&rel), force)?;
            }
        }
        Ok(())
    }

    fn link_store_dir(&self) -> Result<PathBuf, CliError> {
        self.config.data_dir("worktree-files")
    }
}

struct LinkConfig {
    document: DocumentMut,
    paths: Vec<PathBuf>,
}

impl LinkConfig {
    fn read(store: &LinkStore) -> Result<Self, CliError> {
        let document = store.config.read_document("worktree")?;
        let paths = linked_paths_from_document(&document)?;

        Ok(Self { document, paths })
    }

    fn add(&mut self, rel: &Path) -> Result<(), CliError> {
        if self.paths.iter().any(|path| path == rel) {
            return Ok(());
        }
        self.paths.push(rel.to_path_buf());
        self.paths.sort();

        let mut array = Array::default();
        for path in &self.paths {
            array.push(path_to_config_string(path)?);
        }
        self.document["worktree"].or_insert(Item::Table(toml_edit::Table::new()));
        self.document["worktree"]["linked"] = value(array);
        Ok(())
    }

    fn write(&self, store: &LinkStore) -> CliResult {
        store.config.write_document("worktree", &self.document)
    }
}

fn linked_paths_from_document(document: &DocumentMut) -> Result<Vec<PathBuf>, CliError> {
    let Some(linked) = document.get("worktree").and_then(|worktree| worktree.get("linked")) else {
        return Ok(Vec::new());
    };
    let linked = linked.as_array().ok_or_else(|| CliError::from("expected `worktree.linked` to be an array"))?;
    let mut paths = Vec::new();

    for item in linked {
        let path = item.as_str().ok_or_else(|| CliError::from("expected every `worktree.linked` item to be a string"))?;
        paths.push(normalize_link_path(path)?);
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn normalize_link_path(path: &str) -> Result<PathBuf, CliError> {
    let path = Path::new(path);
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err(CliError::from(format!("linked path must be relative: {}", path.display())));
    }

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => normalized.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(CliError::from(format!("linked path cannot escape the repository: {}", path.display())));
            }
        }
    }

    if normalized.as_os_str().is_empty() || normalized.starts_with(".git") || normalized.starts_with(".branp") {
        return Err(CliError::from(format!("unsupported linked path: {}", path.display())));
    }
    Ok(normalized)
}

fn path_to_config_string(path: &Path) -> Result<String, CliError> {
    path.to_str().map(str::to_string).ok_or_else(|| CliError::from(format!("linked path must be valid UTF-8: {}", path.display())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_link_paths_inside_repo() {
        assert_eq!(normalize_link_path("./.env").unwrap(), PathBuf::from(".env"));
        assert_eq!(normalize_link_path("config/local.toml").unwrap(), PathBuf::from("config/local.toml"));
        assert!(normalize_link_path("../secret").is_err());
        assert!(normalize_link_path("/tmp/secret").is_err());
        assert!(normalize_link_path(".git/config").is_err());
        assert!(normalize_link_path(".branp/worktree.toml").is_err());
    }

    #[test]
    fn reads_sorted_unique_link_paths_from_toml() {
        let document = r#"
[worktree]
linked = ["z.env", "./a.env", "z.env"]
"#
        .parse::<DocumentMut>()
        .unwrap();

        assert_eq!(linked_paths_from_document(&document).unwrap(), vec![PathBuf::from("a.env"), PathBuf::from("z.env")]);
    }
}
