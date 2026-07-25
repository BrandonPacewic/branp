use core::fmt;
use std::io::Write;
use std::path::PathBuf;
use std::{env, io};

use crate::errors::{CliError, CliResult};

pub struct GlobalContext {
    /// [`PathBuf`] to the user's home directory at runtime.
    home_path: PathBuf,
    /// [`Shell`] instance for console output.
    shell: Shell,
    /// [`PathBuf`] to the current working directory at runtime.
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

    pub fn default() -> Result<Self, CliError> {
        let shell = Shell::new();
        let homedir =
            env::var("HOME").map_err(|_| CliError::from("HOME environment variable not set"))?;
        let cwd = env::current_dir()?;
        Ok(Self::new(shell, cwd, PathBuf::from(homedir)))
    }

    pub fn configure(&mut self, verbose: u32, quiet: bool) -> CliResult {
        let _extra_verbose = verbose >= 2;
        let verbose = verbose > 0;
        let verbosity = match (verbose, quiet) {
            (true, true) => return Err(CliError::from("cannot set both --verbose and --quiet")),
            (true, false) => Verbosity::Verbose,
            (false, true) => Verbosity::Quiet,
            // In the future this should attempt to source the default verbosity from a configuration
            // file, this is not yet supported so we default to normal verbosity (i.e. verbose).
            (false, false) => Verbosity::Verbose,
        };
        self.shell().set_verbosity(verbosity);
        Ok(())
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
            Some(message) => writeln!(buffer, "{message}").unwrap(),
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

    pub fn warn<T: fmt::Display>(&mut self, message: T) {
        eprintln!(
            "{}",
            color_print::cformat!("<yellow,bold>warning</>: {}", message)
        );
    }

    pub fn error<T: fmt::Display>(&mut self, message: T) {
        eprintln!(
            "{}",
            color_print::cformat!("<red,bold>error</>: {}", message)
        );
    }

    pub fn set_verbosity(&mut self, verbosity: Verbosity) {
        self.verbosity = verbosity;
    }
}

#[derive(PartialEq)]
pub enum Verbosity {
    Verbose,
    Quiet,
}
