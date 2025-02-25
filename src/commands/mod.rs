use clap::{ArgMatches, Command};

pub fn branp() -> Vec<Command> {
    vec![dbrun::command(), template_gen::command(), format::command()]
}

pub type Exec = fn(&ArgMatches);

pub fn branp_exec(cmd: &str) -> Option<Exec> {
    let exec = match cmd {
        "dbrun" => dbrun::exec,
        "template-gen" => template_gen::exec,
        "format" => format::exec,
        _ => return None,
    };

    Some(exec)
}

pub mod dbrun;
pub mod format;
pub mod template_gen;
