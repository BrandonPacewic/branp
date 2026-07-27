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
//!
//! By default branp will not include submodules in the formatting process.
//! This can be manually enabled with `--include-submodules`.
//! Example:
//! ```bash
//! $ bp format --include-submodules
//! ```

use clap::{Arg, ArgAction, ArgMatches, Command};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

use crate::context::GlobalContext;
use crate::errors::{CliError, CliResult};

pub fn command() -> Command {
    Command::new("format")
        .about("Format a C++ file using clang-format.")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .help("The clang-format config preset file to use")
                .long_help("branp will search your <config dir>/branp directory for a .clang-format file")
                .required(false),
        )
        .arg(
            Arg::new("include-submodules")
                .short('s')
                .long("include-submodules")
                .help("Include submodules in the formatting process")
                .long_help("branp will recursively include submodules in the formatting process")
                .action(ArgAction::SetTrue)
                .required(false),
        )
}

/// Tag for a collection of files and their respective supported formatter.
///
/// This struct contains a reference to the globally defined [`CodeFormatter`]
/// and a vector of [`PathBuf`]s that represent the files that are to be formatted.
struct FormatCollection {
    files: Vec<PathBuf>,
    formatter: &'static CodeFormatter,
}

pub fn exec(gctx: &mut GlobalContext, args: &ArgMatches) -> CliResult {
    let include_submodules = args.get_flag("include-submodules");

    let entries = fs::read_dir(".")?;
    let formatters = get_formatters(entries, vec![], include_submodules)?;
    let config = get_clang_format_config(gctx, args);

    for fc in formatters {
        gctx.shell().note(format!("Formatting {} file(s) with {}...", fc.files.len(), fc.formatter.name));
        for file in fc.files {
            let mut command = (fc.formatter.runner)(&file, &config);
            let output = command.output().map_err(|e| CliError::new(format!("failed to run formatter on {}: {}", file.display(), e), 1))?;

            if !output.status.success() {
                gctx.shell().error(format!("Failed to format file: {}", String::from_utf8_lossy(&output.stderr)));
            }
        }
    }

    gctx.shell().note("Done!");
    Ok(())
}

/// Generate a list of files 'tagged' with their respective formatters ([`CodeFormatter`]).
///
/// This function recursively traverses the directory tree starting from the given
/// `entries` and collects files that are associated with a specific formatter.
///
/// Each [`CodeFormatter`] is simply a pointer to the global static definition of the formatter.
fn get_formatters(entries: fs::ReadDir, mut formatters: Vec<FormatCollection>, include_submodules: bool) -> Result<Vec<FormatCollection>, CliError> {
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            if !include_submodules && is_git_dir(path.as_path()) {
                continue;
            }

            let sub_entries = fs::read_dir(&path)?;
            formatters = get_formatters(sub_entries, formatters, include_submodules)?;
        } else if let Some(formatter) = get_code_formatter(&path) {
            if let Some(created_formatter) = formatters.iter_mut().find(|f| f.formatter.name == formatter.name) {
                created_formatter.files.push(path);
            } else {
                formatters.push(FormatCollection { files: vec![path], formatter });
            }
        }
    }

    Ok(formatters)
}

/// Checks if the given path slice is a git directory.
///
/// This is done on the basis of the presence of a `.git` directory.
fn is_git_dir(path: &Path) -> bool {
    path.join(".git").exists()
}

/// Get the clang-format config file from the command line arguments.
///
/// Currently the only formatter that supports a config file is `clang-format`.
/// i.e. [`get_clang_format_config`].
///
/// As this expands in the future, with more formatters that support a configuration file,
/// a new method of handling this logic needs to be implemented.
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
        gctx.shell().warn(format!("No config file found for {config}. Available configs: {config_names:?}"));
        return None;
    }

    config_file
}

/// A static representation of a formatter.
struct CodeFormatter {
    name: &'static str,
    valid_extensions: &'static [&'static str],
    runner: fn(&PathBuf, &Option<String>) -> ProcessCommand,
}

/// Command: `clang-format -i --style=<file,config> <file>`
fn clang_format_command(file: &PathBuf, config: &Option<String>) -> ProcessCommand {
    let mut command = ProcessCommand::new("clang-format");
    command.arg("-i");
    if let Some(config) = config {
        command.arg(format!("--style=file:{config}"));
    } else {
        command.arg("--style=file");
    }

    command.arg(file);
    command
}

// For more information about the clang-format auto formatter visit:
// https://clang.llvm.org/docs/ClangFormat.html
const CLANG: CodeFormatter =
    CodeFormatter { name: "clang-format", valid_extensions: &["cpp", "h", "cc", "hpp", "cxx", "c", "cs"], runner: clang_format_command };

/// Command: `autopep8 --in-place --max-line-length 120 --aggressive --aggressive <file>`
fn autopep8_format_command(file: &PathBuf, _config: &Option<String>) -> ProcessCommand {
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

// For more information about the autopep8 auto formatter visit:
// https://pypi.org/project/autopep8/
const AUTOPEP8: CodeFormatter = CodeFormatter { name: "autopep8", valid_extensions: &["py"], runner: autopep8_format_command };

const FORMATTERS: [&CodeFormatter; 2] = [&CLANG, &AUTOPEP8];

fn get_code_formatter(file: &Path) -> Option<&'static CodeFormatter> {
    let ext = file.extension()?.to_str()?;
    FORMATTERS
        .iter()
        .copied() // Dereference `&&'static CodeFormatter` to `&'static CodeFormatter`
        .find(|formatter| formatter.valid_extensions.contains(&ext))
}
