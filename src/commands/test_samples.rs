use clap::{Arg, ArgMatches, Command};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command as ProcessCommand, Stdio};

use crate::context::GlobalContext;
use crate::errors::{CliError, CliResult};

pub fn command() -> Command {
    Command::new("test-samples")
        .about("Compile a C++ file and run it against sample input/output files.")
        .long_about(
            "Compile a C++ file and run it against sample input/output files.\n\
            \n\
            Expects <stem>_input.txt and <stem>_output.txt in the same directory as the source \
            file.\n\
            \n\
            Example:\n  \
            bp test-samples solution.cpp\n\
            \n\
            This will compile solution.cpp, feed solution_input.txt to stdin, and compare \
            stdout against solution_output.txt line by line.",
        )
        .arg(
            Arg::new("file")
                .help("The C++ file to test (with or without .cpp extension)")
                .required(true)
                .index(1),
        )
}

pub fn exec(gctx: &mut GlobalContext, args: &ArgMatches) -> CliResult {
    let file = args.get_one::<String>("file").unwrap();
    let file = if file.ends_with(".cpp") {
        file.clone()
    } else {
        format!("{}.cpp", file)
    };

    if !Path::new(&file).exists() {
        return Err(CliError::new(format!("{} not found.", file), 1));
    }

    let stem = file.trim_end_matches(".cpp");
    let input_path = format!("{}_input.txt", stem);
    let output_path = format!("{}_output.txt", stem);

    let test_input = fs::read_to_string(&input_path)
        .map_err(|_| CliError::new(format!("{} not found.", input_path), 1))?;
    let expected_output = fs::read_to_string(&output_path)
        .map_err(|_| CliError::new(format!("{} not found.", output_path), 1))?;

    let compile_command = format!("g++ -g -std=c++17 -Wall -DDBG_MODE {}", file);
    let status = ProcessCommand::new("sh")
        .arg("-c")
        .arg(&compile_command)
        .status()?;

    if !status.success() {
        return Err(CliError::from("compilation failed"));
    }

    let mut child = ProcessCommand::new("./a.out")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;

    child
        .stdin
        .take()
        .unwrap()
        .write_all(test_input.as_bytes())?;

    let output = child.wait_with_output()?;
    let actual_output = String::from_utf8_lossy(&output.stdout);

    let expected_lines: Vec<&str> = expected_output.lines().collect();
    let actual_lines: Vec<&str> = actual_output.lines().collect();

    gctx.shell().note("--------------\nExpected:");
    for line in &expected_lines {
        gctx.shell().note(line);
    }

    gctx.shell().note("--------------\nActual:");
    for line in &actual_lines {
        gctx.shell().note(line);
    }

    gctx.shell().note("--------------");

    let total = expected_lines.len();
    let mut good = 0;
    let mut mismatches: Vec<(usize, &str, &str)> = Vec::new();

    for (i, (actual, expected)) in actual_lines.iter().zip(expected_lines.iter()).enumerate() {
        if actual == expected {
            good += 1;
        } else {
            mismatches.push((i + 1, actual, expected));
        }
    }

    for (line_num, actual, expected) in &mismatches {
        gctx.shell().note(color_print::cformat!(
            "Line {}: <red>{}</> != <green>{}</>",
            line_num,
            actual,
            expected
        ));
    }

    if good == total && total > 0 {
        gctx.shell()
            .note(color_print::cformat!("<green>All tests passed!</>"));
    } else if good >= 1 {
        gctx.shell().note(color_print::cformat!(
            "<yellow>{} / {} tests passed.</>",
            good,
            total
        ));
    } else {
        gctx.shell()
            .note(color_print::cformat!("<red>No tests passed.</>"));
    }

    Ok(())
}
