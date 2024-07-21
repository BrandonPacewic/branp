use clap::{ArgMatches, Command};

pub fn branp() -> Vec<Command> {
    vec![dbrun::command()]
}

pub type Exec = fn(&ArgMatches);

pub fn branp_exec(cmd: &str) -> Option<Exec> {
    let exec = match cmd {
        "dbrun" => dbrun::exec,
        _ => return None,
    };
    Some(exec)
}

pub mod dbrun;
