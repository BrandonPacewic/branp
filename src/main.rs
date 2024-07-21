use clap::{ArgMatches, Command};

mod commands;
mod parser;

fn main() {
    let args = branp().try_get_matches().unwrap();
    let (cmd, subcommand_args) = match args.subcommand() {
        Some((cmd, args)) => (cmd, args),
        _ => {
            // No subcommand provided.
            let _ = branp().print_help();
            return;
        }
    };

    let exec = Exec::infer(cmd).expect("");
    exec.exec(subcommand_args);
}

enum Exec {
    Branp(commands::Exec),
}

impl Exec {
    fn infer(cmd: &str) -> Option<Self> {
        if let Some(exec) = commands::branp_exec(cmd) {
            Some(Self::Branp(exec))
        } else {
            // TODO: Add external commands
            None
        }
    }

    fn exec(self, subcommand_args: &ArgMatches) {
        match self {
            Self::Branp(exec) => exec(subcommand_args),
        }
    }
}

fn branp() -> Command {
    Command::new("branp").subcommands(commands::branp())
}
