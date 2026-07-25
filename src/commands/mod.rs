use clap::{ArgMatches, Command};

use crate::context::GlobalContext;
use crate::errors::CliResult;

pub fn branp() -> Vec<Command> {
    vec![
        dbrun::command().visible_alias("r"),
        sample_gen::command(),
        test_samples::command(),
        format::command().visible_alias("f"),
        gen::command(),
        git::command().visible_alias("g"),
        uninstall::command(),
    ]
}

pub type Exec = fn(&mut GlobalContext, &ArgMatches) -> CliResult;

pub fn branp_exec(cmd: &str) -> Option<Exec> {
    let exec = match cmd {
        "dbrun" => dbrun::exec,
        "sample-gen" => sample_gen::exec,
        "test-samples" => test_samples::exec,
        "format" => format::exec,
        "gen" => gen::exec,
        "git" => git::exec,
        "uninstall" => uninstall::exec,
        _ => return None,
    };

    Some(exec)
}

pub mod dbrun;
pub mod format;
pub mod gen;
pub mod git;
pub mod sample_gen;
pub mod test_samples;
pub mod uninstall;
