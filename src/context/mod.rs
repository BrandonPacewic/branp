use std::{env, path::PathBuf};

pub struct GlobalContext {
    home_path: PathBuf,
}

impl GlobalContext {
    pub fn new(homedir: PathBuf) -> Self {
        Self { home_path: homedir }
    }

    pub fn default() -> Option<Self> {
        let homedir = env::var("HOME").expect("HOME environment variable not set.");
        Some(Self::new(PathBuf::from(homedir)))
    }

    pub fn home(&self) -> &PathBuf {
        &self.home_path
    }
}
