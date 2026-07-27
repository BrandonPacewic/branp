use clap::{Arg, ArgMatches, Command};
use std::path::Path;
use std::process::Command as ProcessCommand;
use std::time::Instant;

use crate::context::GlobalContext;
use crate::errors::{CliError, CliResult};

pub fn command() -> Command {
    Command::new("dbrun").about("Compile and execute a standalone C++ file.").arg(Arg::new("file").help("The file to run on").required(true).index(1))
}

pub fn exec(gctx: &mut GlobalContext, args: &ArgMatches) -> CliResult {
    let file = match args.get_one::<String>("file") {
        Some(val) => val.clone(),
        None => return Ok(()),
    };

    let file = if !file.ends_with(".cpp") { format!("{file}.cpp") } else { file };

    if !Path::new(&file).exists() {
        return Err(CliError::new(format!("{file} file not found."), 1));
    }

    let start_time = Instant::now();
    let compile_command = format!("g++ -g -std=c++17 -Wall -DDBG_MODE {file}");
    let status = ProcessCommand::new("sh").arg("-c").arg(&compile_command).status()?;

    if !status.success() {
        return Err(CliError::from("compilation failed"));
    }

    let duration = start_time.elapsed();
    gctx.shell().note(format!("Successfully compiled in {}s\n--------------------", duration.as_secs_f32()));

    ProcessCommand::new("./a.out").status()?;

    Ok(())
}
