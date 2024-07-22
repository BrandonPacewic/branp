use clap::{Arg, ArgMatches, Command};
use std::env;
use std::fs::File;
use std::process::Command as ProcessCommand;

pub fn command() -> Command {
    Command::new("template-gen")
        .about("Generate templates from global template files.")
        .arg(
            Arg::new("file")
                .help("The file to run on")
                .required(true)
                .index(1),
        )
}

pub fn exec(args: &ArgMatches) {
    let mut file = match args.get_one::<String>("file") {
        Some(val) => val.clone(),
        None => {
            return;
        }
    };

    // Modify the file name to remove .cpp extension if necessary
    if file.ends_with(".cpp") {
        file = file.trim_end_matches(".cpp").to_string();
    }

    let test_input = format!("{}_input.txt", file);
    let test_output = format!("{}_output.txt", file);

    // Create the test input and output files
    File::create(&test_input).expect("Failed to create test input file");
    File::create(&test_output).expect("Failed to create test output file");

    // Get the default editor from the environment variable
    let editor = env::var("EDITOR").expect("EDITOR environment variable not set");

    // Open the test input and output files in the default editor
    ProcessCommand::new(&editor)
        .arg(&test_input)
        .status()
        .expect("Failed to open test input file in editor");

    ProcessCommand::new(&editor)
        .arg(&test_output)
        .status()
        .expect("Failed to open test output file in editor");
}
