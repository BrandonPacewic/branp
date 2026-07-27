use clap::{Arg, ArgMatches, Command};

use crate::context::GlobalContext;
use crate::errors::CliResult;
use crate::ops::cloc as ops;

pub fn command() -> Command {
    Command::new("cloc")
        .about("Count lines of code quickly")
        .arg(
            Arg::new("paths")
                .help("Files or directories to count")
                .num_args(0..)
                .default_value("."),
        )
}

pub fn exec(gctx: &mut GlobalContext, args: &ArgMatches) -> CliResult {
    let paths = args
        .get_many::<String>("paths")
        .into_iter()
        .flatten()
        .map(String::as_str)
        .collect();

    ops::cloc(gctx, &ops::ClocOptions { paths })
}
