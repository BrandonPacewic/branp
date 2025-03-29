use core::fmt;
use std::io::Write;
use std::path::PathBuf;
use std::{env, io};

pub struct GlobalContext {
    home_path: PathBuf,
    shell: Shell,
    cwd: PathBuf,
}

impl GlobalContext {
    pub fn new(shell: Shell, cwd: PathBuf, homedir: PathBuf) -> Self {
        Self {
            home_path: homedir,
            shell,
            cwd,
        }
    }

    pub fn default() -> Option<Self> {
        let shell = Shell::new();
        let homedir = env::var("HOME").expect("HOME environment variable not set.");
        let cwd = env::current_dir().expect("Failed to get current directory.");
        Some(Self::new(shell, cwd, PathBuf::from(homedir)))
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

    pub fn shell(&mut self) -> &mut Shell {
        &mut self.shell
    }
}

/// An abstraction around console output.
/// Capable of remembering preferences for verbosity / quietness and color.
pub struct Shell {
    stdout: io::Stdout,
    verbosity: Verbosity,
}

impl Shell {
    pub fn new() -> Self {
        Self {
            stdout: std::io::stdout(),
            verbosity: Verbosity::Verbose,
        }
    }

    fn print(&mut self, message: Option<&dyn fmt::Display>) {
        if self.verbosity == Verbosity::Quiet {
            return;
        }

        let mut buffer = Vec::new();
        match message {
            Some(message) => writeln!(buffer, "{}", message).unwrap(),
            None => writeln!(buffer).unwrap(),
        }

        self.stdout().write_all(&buffer).unwrap();
    }

    fn stdout(&mut self) -> &mut io::Stdout {
        &mut self.stdout
    }

    pub fn note<T: fmt::Display>(&mut self, message: T) {
        self.print(Some(&message));
    }
}

#[derive(PartialEq)]
pub enum Verbosity {
    Verbose,
    Quiet,
}
