use std::process::Command as ProcessCommand;

use crate::errors::{CliError, CliResult};

pub fn open_url(url: &str) -> CliResult {
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = ProcessCommand::new("open");
        command.arg(url);
        command
    };

    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = ProcessCommand::new("cmd");
        command.args(["/C", "start", "", url]);
        command
    };

    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut command = ProcessCommand::new("xdg-open");
        command.arg(url);
        command
    };

    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(CliError::from(format!("failed to open {url}")))
    }
}
