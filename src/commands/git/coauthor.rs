use clap::{Arg, ArgMatches, Command};

use crate::context::GlobalContext;
use crate::errors::CliResult;
use crate::ops::git as ops;

pub fn cli() -> Command {
    Command::new("coauthor")
        .about("Generate GitHub no-reply co-author lines")
        .arg(Arg::new("usernames").help("One or more GitHub usernames").required(true).num_args(1..).value_name("USERNAME"))
}

pub fn exec(gctx: &mut GlobalContext, args: &ArgMatches) -> CliResult {
    let usernames = args.get_many::<String>("usernames").unwrap().map(String::as_str).collect();
    ops::coauthor(gctx, &ops::CoauthorOptions { usernames })
}
