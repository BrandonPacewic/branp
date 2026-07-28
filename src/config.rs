use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use toml_edit::DocumentMut;

use crate::errors::{CliError, CliResult};

pub const REPO_CONFIG_DIR: &str = ".branp";

pub struct RepoConfig {
    root: PathBuf,
}

impl RepoConfig {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn dir(&self) -> PathBuf {
        self.root.join(REPO_CONFIG_DIR)
    }

    pub fn data_dir(&self, name: &str) -> Result<PathBuf, CliError> {
        validate_config_name(name)?;
        Ok(self.dir().join(name))
    }

    pub fn read_document(&self, name: &str) -> Result<DocumentMut, CliError> {
        let path = self.document_path(name)?;
        let source = match fs::read_to_string(&path) {
            Ok(source) => source,
            Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
            Err(error) => return Err(error.into()),
        };

        if source.trim().is_empty() {
            Ok(DocumentMut::new())
        } else {
            source.parse::<DocumentMut>().map_err(|error| CliError::from(format!("failed to parse {}: {error}", path.display())))
        }
    }

    pub fn write_document(&self, name: &str, document: &DocumentMut) -> CliResult {
        fs::create_dir_all(self.dir())?;
        fs::write(self.document_path(name)?, document.to_string())?;
        Ok(())
    }

    fn document_path(&self, name: &str) -> Result<PathBuf, CliError> {
        validate_config_name(name)?;
        Ok(self.dir().join(format!("{name}.toml")))
    }
}

fn validate_config_name(name: &str) -> Result<(), CliError> {
    let path = Path::new(name);
    if name.is_empty() || path.components().count() != 1 || path.file_name().and_then(|part| part.to_str()) != Some(name) {
        return Err(CliError::from(format!("invalid .branp config name: {name}")));
    }
    Ok(())
}
