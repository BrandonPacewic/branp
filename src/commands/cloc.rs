use clap::{Arg, ArgAction, ArgMatches, Command};

use crate::context::GlobalContext;
use crate::errors::CliResult;
use crate::ops::cloc as ops;

pub fn command() -> Command {
    Command::new("cloc")
        .about("Count lines of code quickly")
        .arg(Arg::new("paths").help("Files or directories to count").num_args(0..).default_value("."))
        .arg(Arg::new("git-diff").long("git-diff").help("Count changed lines in the current git diff").action(ArgAction::SetTrue))
        .arg(Arg::new("no-live").long("no-live").alias("no-live-update").help("Disable live updating output").action(ArgAction::SetTrue))
        .arg(Arg::new("no-commas").long("no-commas").help("Omit commas from output numbers").action(ArgAction::SetTrue))
}

pub fn exec(gctx: &mut GlobalContext, args: &ArgMatches) -> CliResult {
    let paths = args.get_many::<String>("paths").into_iter().flatten().map(String::as_str).collect();
    let target = if args.get_flag("git-diff") {
        ops::ClocTarget::Git(ops::GitClocTarget::WorktreeDiff { pathspecs: paths })
    } else {
        ops::ClocTarget::Paths(paths)
    };

    ops::cloc(gctx, &ops::ClocOptions { target, live: !args.get_flag("no-live"), commas: !args.get_flag("no-commas") })
}
