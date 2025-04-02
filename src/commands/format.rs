use clap::{Arg, ArgMatches, Command};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

use crate::context::GlobalContext;

const FILE_TARGETS: [&str; 7] = ["cpp", "h", "cc", "hpp", "cxx", "c", "cs"];

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

pub fn exec(gctx: &mut GlobalContext, args: &ArgMatches) {
    gctx.shell().note("Indexing...");

    let config = get_clang_format_config(gctx, args);
    let mut files: Vec<PathBuf> = Vec::new();
    let paths = vec![PathBuf::from(".")];

    for path in paths {
        if path.is_dir() {
            walk_directory_for_files(&path, &mut files);
        }
    }

    gctx.shell()
        .note(format!("Formatting {} file(s)...", files.len()));

    for file in files {
        let mut command = ProcessCommand::new("clang-format");
        command.arg("-i");
        if let Some(config) = &config {
            command.arg(format!("--style=file:{}", config));
        } else {
            command.arg("--style=file");
        }

        command.arg(&file);
        let output = command.output();
        if let Ok(output) = output {
            if !output.status.success() {
                gctx.shell().error(format!(
                    "Failed to format file: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
        } else {
            gctx.shell().error(format!(
                "Failed to execute clang-format for file: {}",
                file.display()
            ));
        }
    }

    gctx.shell().note("Done!");
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

fn walk_directory_for_files(dir: &Path, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                walk_directory_for_files(&path, files);
            } else if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if FILE_TARGETS.contains(&ext) {
                    files.push(path);
                }
            }
        }
    }
}
