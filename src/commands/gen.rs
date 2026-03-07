use clap::{Arg, ArgMatches, Command};
use std::collections::HashSet;
use std::fs;

use crate::context::GlobalContext;
use crate::errors::{CliError, CliResult};

const LONG_ABOUT: &str = "\
Copy template files from ~/.config/branp/templates into the current directory.

Template names are matched case-insensitively against files in the templates \
directory (flat, no subdirectory search). Multiple templates can be specified \
in one invocation.

EXAMPLES:
    # Copy a single template
    bp gen test.cpp

    # Copy multiple templates
    bp gen test.cpp makefile

    # Generate 3 copies named A.cpp, B.cpp, C.cpp
    bp gen test.cpp -n 3";

pub fn command() -> Command {
    Command::new("gen")
        .about("Generate template file(s) from <config dir>/branp/templates")
        .long_about(LONG_ABOUT)
        .arg(
            Arg::new("args")
                .help("The file(s) / directory(s) to generate")
                .required(true)
                .num_args(1..),
        )
        .arg(
            Arg::new("count")
                .short('n')
                .long("count")
                .help("Generate N copies named A.ext, B.ext, etc. (requires exactly one template arg)")
                .value_name("N")
                .value_parser(clap::value_parser!(u8)),
        )
}

pub fn exec(gctx: &mut GlobalContext, args: &ArgMatches) -> CliResult {
    let templates_dir = gctx.templates_dir();
    let count = args.get_one::<u8>("count").copied();

    let template_args: Vec<String> = args
        .get_many::<String>("args")
        .unwrap()
        .map(|s| s.to_string())
        .collect();

    if count.is_some() && template_args.len() != 1 {
        return Err(CliError::from(
            "--count requires exactly one template argument",
        ));
    }

    let case_insensitive_paths: HashSet<String> = template_args
        .iter()
        .map(|s| s.to_ascii_lowercase())
        .collect();

    let entries = fs::read_dir(&templates_dir)
        .map_err(|_| CliError::from("failed to read templates directory"))?;

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
        return Ok(());
    }

    if let Some(n) = count {
        let path = &expanded_paths[0];
        let ext = path.rsplit('.').next().unwrap_or("");
        for i in 0..n {
            let letter = (b'A' + i) as char;
            let file_name = format!("{}.{}", letter, ext);
            let dest_path = format!("{}/{}", gctx.cwd().to_string_lossy(), file_name);
            if fs::copy(path, &dest_path).is_err() {
                gctx.shell().error(format!(
                    "Failed to copy template file ({}) to {}",
                    path, dest_path
                ));
            } else {
                gctx.shell()
                    .note(format!("Generated template file: {}", file_name));
            }
        }
    } else {
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

    Ok(())
}
