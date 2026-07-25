use std::path::Path;
use std::process::{Command as ProcessCommand, Output, Stdio};

use crate::errors::{CliError, CliResult};

pub fn run(program: &str, args: &[&str], cwd: Option<&Path>) -> CliResult {
    let output = process(program, args, cwd).output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(command_error(program, args, &output))
    }
}

pub fn output(program: &str, args: &[&str], cwd: Option<&Path>) -> Result<String, CliError> {
    let output = process(program, args, cwd).output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(command_error(program, args, &output))
    }
}

pub fn status(program: &str, args: &[&str], cwd: Option<&Path>) -> Result<bool, CliError> {
    let status = process(program, args, cwd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;

    Ok(status.success())
}

fn process(program: &str, args: &[&str], cwd: Option<&Path>) -> ProcessCommand {
    let mut command = ProcessCommand::new(program);
    command.args(args);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    command
}

fn command_error(program: &str, args: &[&str], output: &Output) -> CliError {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let detail = if !stderr.trim().is_empty() {
        stderr.trim()
    } else {
        stdout.trim()
    };

    CliError::from(format!(
        "{} {} failed{}{}",
        program,
        args.join(" "),
        if detail.is_empty() { "" } else { ": " },
        detail
    ))
}
