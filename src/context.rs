use std::env;
use std::path::PathBuf;

pub struct GlobalContext {
    home_path: PathBuf,
    cwd: PathBuf,
}

impl GlobalContext {
    pub fn new(cwd: PathBuf, homedir: PathBuf) -> Self {
        Self {
            home_path: homedir,
            cwd,
        }
    }

    pub fn default() -> Option<Self> {
        let homedir = env::var("HOME").expect("HOME environment variable not set.");
        let cwd = env::current_dir().expect("Failed to get current directory.");
        Some(Self::new(cwd, PathBuf::from(homedir)))
    }

    pub fn configure(&mut self, _verbose: u32, _quiet: bool) {
        // Not doing anything with quiet for now. This will be expanded in the future when more things
        // need to be configurable.
    }

    pub fn home(&self) -> &PathBuf {
        &self.home_path
    }

    pub fn templates_dir(&self) -> PathBuf {
        self.home_path.join(".config/branp/templates")
    }

    pub fn cwd(&self) -> &PathBuf {
        &self.cwd
    }
}
