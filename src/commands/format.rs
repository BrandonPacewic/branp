//! `format` subcommand.
//!
//! This command is used to format a given directory with a supported auto formatter.
//! Example:
//! ```bash
//! $ bp format
//! ```
//! branp will format all files in the current directory and its subdirectories that
//! it possibly can. Skipping over files that do not have a supported auto formatter for
//! their respective file extension.
//!
//! If the auto formatter you want to call has a configuration file, you can specify it with
//! the `--config` flag. This can also be used to specify a custom configuration file that
//! you have saved inside of `<config dir>/branp/format`.
//! Example:
//! ```bash
//! $ bp format --config <config>
//! ```

use clap::{Arg, ArgMatches, Command};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

use crate::context::GlobalContext;

pub fn command() -> Command {
    Command::new("format")
        .about("Format a C++ file using clang-format.")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .help("The clang-format config preset file to use")
                .long_help(
                    "branp will search your <config dir>/branp directory for a .clang-format file",
                )
                .required(false),
        )
}

/// Tag for a collection of files and their respective supported formatter.
/// 
/// This struct contains a reference to the globally defined [`CodeFormatter`]
/// and a vector of [`PathBuf`]s that represent the files that are to be formatted.
struct FormatCollection<'a> {
    files: Vec<PathBuf>,
    formatter: &'a CodeFormatter,
}

pub fn exec(gctx: &mut GlobalContext, _args: &ArgMatches) {
    let entries = fs::read_dir(".").unwrap_or_else(|_| panic!("Failed to read current directory"));
    let formatters = get_formatters(entries, vec![]);

    for fc in formatters {
        gctx.shell().note(format!(
            "Formatting {} file(s) with {}...",
            fc.files.len(),
            fc.formatter.name
        ));
        for file in fc.files {
            let mut command = (fc.formatter.runner)(&file, None);
            let output = command.output().unwrap_or_else(|_| {
                panic!("Failed to execute formatter for file: {}", file.display())
            });

            if !output.status.success() {
                gctx.shell().error(format!(
                    "Failed to format file: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
        }
    }

    gctx.shell().note("Done!");
}

fn get_formatters(
    entries: fs::ReadDir,
    mut formatters: Vec<FormatCollection>,
) -> Vec<FormatCollection> {
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            let sub_entries =
                fs::read_dir(&path).unwrap_or_else(|_| panic!("Failed to read directory"));
            formatters = get_formatters(sub_entries, formatters);
        } else if let Some(formatter) = get_code_formatter(&path) {
            if let Some(created_formatter) = formatters
                .iter_mut()
                .find(|f| f.formatter.name == formatter.name)
            {
                created_formatter.files.push(path);
            } else {
                formatters.push(FormatCollection {
                    files: vec![path],
                    formatter,
                });
            }
        }
    }

    formatters
}

fn get_clang_format_config(gctx: &mut GlobalContext, args: &ArgMatches) -> Option<String> {
    let config = args.get_one::<String>("config")?;
    let config_dir = Path::new(&gctx.home()).join(".config/branp/format");

    let mut config_files = Vec::new();
    if let Ok(entries) = fs::read_dir(&config_dir) {
        for entry in entries.filter_map(Result::ok) {
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(".clang-format") {
                    config_files.push(entry.path());
                }
            }
        }
    }

    let config_names: Vec<String> = config_files
        .iter()
        .filter_map(|p| p.file_name())
        .filter_map(|s| s.to_str())
        .map(|s| s.split('.').next().unwrap_or("").to_lowercase())
        .collect();

    let config_file = config_files
        .iter()
        .zip(config_names.iter())
        .find(|(_, name)| **name == config.to_lowercase())
        .map(|(path, _)| path.to_string_lossy().into_owned());

    if config_file.is_none() {
        gctx.shell().warn(format!(
            "No config file found for {}. Available configs: {:?}",
            config, config_names
        ));
        return None;
    }

    config_file
}

struct CodeFormatter {
    name: &'static str,
    valid_extensions: &'static [&'static str],
    runner: fn(&PathBuf, Option<String>) -> ProcessCommand,
}

fn clang_format_command(file: &PathBuf, config: Option<String>) -> ProcessCommand {
    let mut command = ProcessCommand::new("clang-format");
    command.arg("-i");
    if let Some(config) = config {
        command.arg(format!("--style=file:{}", config));
    } else {
        command.arg("--style=file");
    }

    command.arg(file);
    command
}

const CLANG: CodeFormatter = CodeFormatter {
    name: "clang-format",
    valid_extensions: &["cpp", "h", "cc", "hpp", "cxx", "c", "cs"],
    runner: clang_format_command,
};

fn autopep8_format_command(file: &PathBuf, _config: Option<String>) -> ProcessCommand {
    // Note that pep8autoformat does not accept a configuration file.
    // In the future customization to this particular call should be done via
    // the global context.
    let mut command = ProcessCommand::new("autopep8");
    command.arg("--in-place");
    command.arg("--max-line-length");
    command.arg("120");
    command.arg("--aggressive");
    command.arg("--aggressive");
    command.arg(file);
    command
}

const AUTOPEP8: CodeFormatter = CodeFormatter {
    name: "autopep8",
    valid_extensions: &["py"],
    runner: autopep8_format_command,
};

const FORMATTERS: [CodeFormatter; 2] = [CLANG, AUTOPEP8];

fn get_code_formatter(file: &PathBuf) -> Option<&'static CodeFormatter> {
    let ext = file.extension()?.to_str()?;
    FORMATTERS
        .iter()
        .find(|formatter| formatter.valid_extensions.contains(&ext))
}
