use std::fmt;

pub type CliResult = Result<(), CliError>;

pub struct CliError {
    pub message: String,
    pub code: i32,
}

impl CliError {
    pub fn new(message: impl fmt::Display, code: i32) -> Self {
        Self { message: message.to_string(), code }
    }
}

impl From<String> for CliError {
    fn from(s: String) -> Self {
        Self { message: s, code: 1 }
    }
}

impl From<&str> for CliError {
    fn from(s: &str) -> Self {
        Self { message: s.to_string(), code: 1 }
    }
}

impl From<std::io::Error> for CliError {
    fn from(e: std::io::Error) -> Self {
        Self { message: e.to_string(), code: 1 }
    }
}

impl From<std::env::VarError> for CliError {
    fn from(e: std::env::VarError) -> Self {
        Self { message: e.to_string(), code: 1 }
    }
}
