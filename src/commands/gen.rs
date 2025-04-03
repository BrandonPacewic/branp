//! `gen` subcommand.
//!
//! This command generates one or more template files from the `branp/templates` directory.
//! It is variadic: you can provide multiple file or directory names in a single invocation,
//! separated by spaces.
//!
//! For example:
//! ```bash
//! $ bp gen test.cpp
//! ```
//! This will copy `branp/templates/test.cpp` into the current directory as `test.cpp`.
//!
//! You can also generate multiple items at once:
//! ```bash
//! $ bp gen makes clang-format default.clang-format
//! ```
//! If `makes` is a directory, all files from `branp/templates/makes` will be copied. The
//! command will also copy `branp/templates/clang-format/default.clang-format` if it exists, given that
//! `clang-format` is a directory. This allows for the structuring of templates in subdirectories.
//!
//! If no matching templates are found for a given filename or directory, a warning is
//! printed and no files are copied.

use clap::{Arg, ArgMatches, Command};
use std::collections::HashSet;
use std::fs;

use crate::context::GlobalContext;

pub fn command() -> Command {
    Command::new("gen")
        .about("Generate a template file(s) from <config dir>/branp/templates")
        .arg(
            Arg::new("args")
                .help("The file(s) / directory(s) to generate")
                .required(true)
                .num_args(1..),
        )
}

pub fn exec(gctx: &mut GlobalContext, args: &ArgMatches) {
    let templates_dir = gctx.templates_dir();
    let case_insensitive_paths: HashSet<String> = args
        .get_many::<String>("args")
        .unwrap()
        .map(|s| s.to_string().to_ascii_lowercase())
        .collect();

    let entries = fs::read_dir(&templates_dir)
        .unwrap_or_else(|_| panic!("Failed to read templates directory"));

    let expanded_paths: Vec<String> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let file_name = entry.file_name();
            let name = file_name.to_str()?;
            if case_insensitive_paths.contains(&name.to_string().to_ascii_lowercase()) {
                Some(entry.path().to_string_lossy().to_string())
            } else {
                None
            }
        })
        .collect();

    if expanded_paths.is_empty() {
        gctx.shell()
            .warn("No templates found for the given file(s) / directory(s)");
        return;
    }

    for path in expanded_paths {
        let file_name = path.split('/').next_back().unwrap();
        let dest_path = format!("{}/{}", gctx.cwd().to_string_lossy(), file_name);
        if fs::copy(&path, &dest_path).is_err() {
            gctx.shell().error(format!(
                "Failed to copy template file ({}) destination {}",
                path, dest_path
            ));
        } else {
            gctx.shell()
                .note(format!("Generated template file: {}", file_name));
        }
    }
}
