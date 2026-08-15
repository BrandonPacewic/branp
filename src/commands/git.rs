//! `git` subcommands.
//!
//! This module provides the clap surface for Git-related helper commands.

use crate::context::GlobalContext;
use crate::errors::{CliError, CliResult};
use crate::ops::git as ops;
use clap::{Arg, ArgAction, ArgMatches, Command};

pub fn cli() -> Command {
    Command::new("git").about("Git-related helpers").subcommand_required(true).arg_required_else_help(true).subcommands(builtin())
}

type GitExec = fn(&mut GlobalContext, &ArgMatches) -> CliResult;

fn builtin() -> Vec<Command> {
    vec![coauthor_cli(), prs_cli(), check_cli(), fork_cli(), default_branch_cli(), ignore_cli(), open_cli()]
}

fn builtin_exec(cmd: &str) -> Option<GitExec> {
    let exec = match cmd {
        "coauthor" => coauthor,
        "prs" => prs,
        "check" => check,
        "fork" => fork,
        "default-branch" => default_branch,
        "ignore" => ignore,
        "open" => open,
        _ => return None,
    };

    Some(exec)
}

pub fn exec(gctx: &mut GlobalContext, args: &ArgMatches) -> CliResult {
    if let Some((cmd, sub)) = args.subcommand() {
        if let Some(exec) = builtin_exec(cmd) {
            return exec(gctx, sub);
        }
    }

    Err(CliError::from("no `git` subcommand provided"))
}

fn coauthor_cli() -> Command {
    Command::new("coauthor")
        .about("Generate GitHub no-reply co-author lines")
        .arg(Arg::new("usernames").help("One or more GitHub usernames").required(true).num_args(1..).value_name("USERNAME"))
}

fn prs_cli() -> Command {
    Command::new("prs")
        .about("List open pull requests in this GitHub repo")
        .arg(Arg::new("remote").short('r').long("remote").help("Git remote to use").default_value("origin"))
}

fn check_cli() -> Command {
    Command::new("check")
        .about("Monitor PR checks until the PR is merged")
        .arg(Arg::new("target").help("Pull request number, URL, or branch; defaults to the current branch PR").value_name("PR|URL|BRANCH"))
        .arg(
            Arg::new("interval")
                .short('i')
                .long("interval")
                .help("Refresh interval in seconds")
                .default_value("5")
                .value_parser(clap::value_parser!(u64).range(1..)),
        )
        .arg(Arg::new("once").long("once").help("Show the current PR status and checks without waiting for merge").action(ArgAction::SetTrue))
}

fn fork_cli() -> Command {
    Command::new("fork")
        .about("Add and track a fork remote branch for local PR edits")
        .arg(Arg::new("remote").help("Fork remote name to add or reuse").required(true).value_name("REMOTE"))
        .arg(Arg::new("url").help("Fork Git URL").required(true).value_name("URL"))
        .arg(
            Arg::new("branch").short('b').long("branch").help("Branch to switch to and track; defaults to the fork remote HEAD").value_name("BRANCH"),
        )
}

fn default_branch_cli() -> Command {
    Command::new("default-branch")
        .about("Print the default branch branp would use for this repo")
        .arg(Arg::new("remote").short('r').long("remote").help("Git remote to use").default_value("origin"))
}

fn ignore_cli() -> Command {
    Command::new("ignore")
        .about("Add repo-local ignore patterns without modifying .gitignore")
        .arg(Arg::new("patterns").help("Paths or patterns to ignore locally").num_args(1..).value_name("PATH|PATTERN"))
        .arg(Arg::new("raw").long("raw").help("Treat arguments as literal gitignore patterns").action(ArgAction::SetTrue))
        .arg(
            Arg::new("list")
                .long("list")
                .short('l')
                .help("List repo-local ignore patterns")
                .conflicts_with_all(["patterns", "remove", "raw"])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("remove")
                .long("remove")
                .short('r')
                .help("Remove matching repo-local ignore patterns")
                .requires("patterns")
                .action(ArgAction::SetTrue),
        )
}

fn open_cli() -> Command {
    Command::new("open")
        .about("Open this GitHub repo, issue, pull request, or commit in a browser")
        .arg(Arg::new("pr").help("Open the pull request for the current branch, number, or commit").long("pr").short('p').action(ArgAction::SetTrue))
        .arg(Arg::new("targets").help("Optional `pr`, issue/PR number, or commit hash").num_args(0..=2).value_name("TARGET"))
        .arg(Arg::new("remote").short('r').long("remote").help("Git remote to use").default_value("origin"))
}

fn coauthor(gctx: &mut GlobalContext, args: &ArgMatches) -> CliResult {
    let usernames = args.get_many::<String>("usernames").unwrap().map(String::as_str).collect();
    ops::coauthor(gctx, &ops::CoauthorOptions { usernames })
}

fn prs(gctx: &mut GlobalContext, args: &ArgMatches) -> CliResult {
    let remote = args.get_one::<String>("remote").unwrap();
    ops::prs(gctx, &ops::PrsOptions { remote })
}

fn check(gctx: &mut GlobalContext, args: &ArgMatches) -> CliResult {
    let options = ops::CheckOptions {
        target: args.get_one::<String>("target").map(String::as_str),
        interval: *args.get_one::<u64>("interval").unwrap(),
        once: args.get_flag("once"),
    };
    ops::check(gctx, &options)
}

fn fork(gctx: &mut GlobalContext, args: &ArgMatches) -> CliResult {
    let options = ops::ForkOptions {
        remote: required(args, "remote")?,
        url: required(args, "url")?,
        branch: args.get_one::<String>("branch").map(String::as_str),
    };
    ops::fork(gctx, &options)
}

fn default_branch(gctx: &mut GlobalContext, args: &ArgMatches) -> CliResult {
    let remote = args.get_one::<String>("remote").unwrap();
    ops::default_branch(gctx, remote)
}

fn ignore(gctx: &mut GlobalContext, args: &ArgMatches) -> CliResult {
    let options = ops::IgnoreOptions {
        patterns: args.get_many::<String>("patterns").map(|values| values.map(String::as_str).collect()).unwrap_or_default(),
        raw: args.get_flag("raw"),
        list: args.get_flag("list"),
        remove: args.get_flag("remove"),
    };
    ops::ignore(gctx, &options)
}

fn open(gctx: &mut GlobalContext, args: &ArgMatches) -> CliResult {
    let targets = args.get_many::<String>("targets").map(|values| values.map(String::as_str).collect::<Vec<_>>()).unwrap_or_default();
    let options = ops::OpenOptions { remote: args.get_one::<String>("remote").unwrap(), pr: args.get_flag("pr"), targets };
    ops::open(gctx, &options)
}

fn required<'a>(args: &'a ArgMatches, name: &str) -> Result<&'a str, CliError> {
    args.get_one::<String>(name).map(String::as_str).ok_or_else(|| CliError::from(format!("missing required argument `{name}`")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_subcommands_have_exec_handlers() {
        for command in builtin() {
            assert!(builtin_exec(command.get_name()).is_some(), "{} is missing an exec handler", command.get_name());
        }
    }

    #[test]
    fn git_cli_definition_is_valid() {
        cli().debug_assert();
    }
}
