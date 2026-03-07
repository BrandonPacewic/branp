use clap::{Arg, ArgMatches, Command};
use std::env;
use std::fs::File;
use std::process::Command as ProcessCommand;

use crate::context::GlobalContext;
use crate::errors::{CliError, CliResult};

pub fn command() -> Command {
    Command::new("template-gen")
        .about("Generate test case files for a given file name")
        .arg(
            Arg::new("file")
                .help("The file to run on")
                .required(true)
                .index(1),
        )
}

pub fn exec(_gctx: &mut GlobalContext, args: &ArgMatches) -> CliResult {
    let mut file = match args.get_one::<String>("file") {
        Some(val) => val.clone(),
        None => return Ok(()),
    };

    if file.ends_with(".cpp") {
        file = file.trim_end_matches(".cpp").to_string();
    }

    let test_input = format!("{}_input.txt", file);
    let test_output = format!("{}_output.txt", file);

    File::create(&test_input)?;
    File::create(&test_output)?;

    let editor =
        env::var("EDITOR").map_err(|_| CliError::from("EDITOR environment variable not set"))?;

    ProcessCommand::new(&editor).arg(&test_input).status()?;
    ProcessCommand::new(&editor).arg(&test_output).status()?;

    Ok(())
}
