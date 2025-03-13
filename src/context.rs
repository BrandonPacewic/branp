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

    pub fn configure(&mut self, _quiet: bool) {
        // Not doing anything with quiet for now. This will be expanded in the future when more things
        // need to be configurable.
    }

    pub fn home(&self) -> &PathBuf {
        &self.home_path
    }
}
