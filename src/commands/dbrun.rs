use clap::{Arg, ArgMatches, Command};
use std::path::Path;
use std::process::Command as ProcessCommand;
use std::time::Instant;

use crate::context::GlobalContext;

pub fn command() -> Command {
    Command::new("dbrun")
        .about("Compile and execute a standalone C++ file.")
        .arg(
            Arg::new("file")
                .help("The file to run on")
                .required(true)
                .index(1),
        )
}

pub fn exec(gctx: &mut GlobalContext, args: &ArgMatches) {
    let file = match args.get_one::<String>("file") {
        Some(val) => val.clone(),
        None => {
            return;
        }
    };

    let file = if !file.ends_with(".cpp") {
        format!("{}.cpp", file)
    } else {
        file
    };

    if !Path::new(&file).exists() {
        gctx.shell().error(format!("{} file not found.", file));
        return;
    }

    let start_time = Instant::now();
    let compile_command = format!("g++ -g -std=c++17 -Wall -DDBG_MODE {}", file);
    let status = ProcessCommand::new("sh")
        .arg("-c")
        .arg(&compile_command)
        .status()
        .expect("Failed to execute compile command!");

    if !status.success() {
        eprint!("Compilation failed!");
        return;
    }

    let duration = start_time.elapsed();
    gctx.shell().note(format!(
        "Successfully compiled in {}s\n--------------------",
        duration.as_secs_f32()
    ));

    let _ = ProcessCommand::new("./a.out")
        .status()
        .expect("Failed to execute compiled program!");
}
