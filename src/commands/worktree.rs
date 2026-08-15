//! Git worktree helpers.

use clap::{Arg, ArgAction, ArgMatches, Command};

use crate::context::GlobalContext;
use crate::errors::{CliError, CliResult};
use crate::ops::worktree as ops;

pub fn cli() -> Command {
    Command::new("worktree").about("Manage sibling Git worktrees").subcommand_required(true).arg_required_else_help(true).subcommands(builtin())
}

type WorktreeExec = fn(&mut GlobalContext, &ops::Repo, &ArgMatches) -> CliResult;

fn builtin() -> Vec<Command> {
    vec![list_cli(), path_cli(), new_cli(), remove_cli(), prune_cli(), gone_cli(), track_cli(), links_cli(), sync_cli()]
}

fn builtin_exec(cmd: &str) -> Option<WorktreeExec> {
    let exec = match cmd {
        "list" => list_exec,
        "path" => path_exec,
        "new" => new_exec,
        "remove" => remove_exec,
        "prune" => prune_exec,
        "gone" => gone_exec,
        "track" => track_exec,
        "links" => links_exec,
        "sync" => sync_exec,
        _ => return None,
    };

    Some(exec)
}

pub fn exec(gctx: &mut GlobalContext, args: &ArgMatches) -> CliResult {
    let repo = ops::Repo::discover(gctx.cwd())?;

    if let Some((cmd, sub)) = args.subcommand() {
        if let Some(exec) = builtin_exec(cmd) {
            return exec(gctx, &repo, sub);
        }
    }

    Err(CliError::from("no `worktree` subcommand provided"))
}

fn list_cli() -> Command {
    Command::new("list")
        .alias("ls")
        .about("List Git worktrees")
        .arg(Arg::new("no-pr").long("no-pr").help("Skip GitHub pull request lookup").action(ArgAction::SetTrue))
}

fn path_cli() -> Command {
    Command::new("path")
        .about("Print the path for a worktree name, or the base worktree for the default branch")
        .arg(Arg::new("name").required(true).value_name("NAME"))
}

fn new_cli() -> Command {
    Command::new("new")
        .about("Create a sibling worktree")
        .arg(Arg::new("name").required(true).value_name("NAME"))
        .arg(Arg::new("branch").value_name("BRANCH"))
        .arg(Arg::new("no-fetch").long("no-fetch").help("Do not fetch origin before creating the worktree").action(ArgAction::SetTrue))
        .arg(Arg::new("no-submodules").long("no-submodules").help("Skip submodule initialization").action(ArgAction::SetTrue))
        .arg(Arg::new("no-tmux").long("no-tmux").help("Do not start a tmux session").action(ArgAction::SetTrue))
}

fn remove_cli() -> Command {
    Command::new("remove")
        .alias("rm")
        .about("Remove a sibling worktree and its local branch")
        .arg(Arg::new("name").required(true).value_name("NAME"))
        .arg(Arg::new("force").short('f').long("force").help("Force worktree removal and branch deletion").action(ArgAction::SetTrue))
        .arg(Arg::new("keep-branch").long("keep-branch").help("Do not delete the local branch").action(ArgAction::SetTrue))
        .arg(Arg::new("no-tmux").long("no-tmux").help("Do not kill the matching tmux session").action(ArgAction::SetTrue))
}

fn prune_cli() -> Command {
    Command::new("prune").about("Run git worktree prune")
}

fn gone_cli() -> Command {
    Command::new("gone")
        .about("Remove sibling worktrees whose origin branch no longer exists")
        .arg(Arg::new("dry-run").short('n').long("dry-run").help("Show what would be removed").action(ArgAction::SetTrue))
        .arg(Arg::new("force").short('f').long("force").help("Force worktree removal and branch deletion").action(ArgAction::SetTrue))
        .arg(Arg::new("no-tmux").long("no-tmux").help("Do not kill matching tmux sessions").action(ArgAction::SetTrue))
}

fn track_cli() -> Command {
    Command::new("track")
        .about("Track untracked files that should be symlinked into every worktree")
        .arg(Arg::new("path").required(true).num_args(1..).value_name("PATH"))
        .arg(Arg::new("force").short('f').long("force").help("Replace existing files in other worktrees").action(ArgAction::SetTrue))
}

fn links_cli() -> Command {
    Command::new("links").about("List files linked across worktrees")
}

fn sync_cli() -> Command {
    Command::new("sync")
        .about("Create or repair symlinks for files tracked in .branp/worktree.toml")
        .arg(Arg::new("force").short('f').long("force").help("Replace existing non-symlink paths").action(ArgAction::SetTrue))
}

fn list_exec(gctx: &mut GlobalContext, repo: &ops::Repo, args: &ArgMatches) -> CliResult {
    ops::list(gctx, &ops::ListOptions { base: &repo.base, default_branch: &repo.default_branch, pr_lookup: !args.get_flag("no-pr") })
}

fn path_exec(gctx: &mut GlobalContext, repo: &ops::Repo, args: &ArgMatches) -> CliResult {
    ops::path(gctx, &ops::PathOptions { base: &repo.base, default_branch: &repo.default_branch, name: required(args, "name")? })
}

fn new_exec(gctx: &mut GlobalContext, repo: &ops::Repo, args: &ArgMatches) -> CliResult {
    let name = required(args, "name")?;
    let branch = args.get_one::<String>("branch").map_or(name, String::as_str);
    ops::new(
        gctx,
        &ops::NewOptions {
            base: &repo.base,
            current: &repo.current,
            default_branch: &repo.default_branch,
            name,
            branch,
            fetch: !args.get_flag("no-fetch"),
            submodules: !args.get_flag("no-submodules"),
            tmux: !args.get_flag("no-tmux"),
            session_name: repo.session_name(name),
        },
    )
}

fn remove_exec(gctx: &mut GlobalContext, repo: &ops::Repo, args: &ArgMatches) -> CliResult {
    let name = required(args, "name")?;
    ops::remove(
        gctx,
        &ops::RemoveOptions {
            base: &repo.base,
            name,
            force: args.get_flag("force"),
            delete_branch: !args.get_flag("keep-branch"),
            tmux: !args.get_flag("no-tmux"),
            session_name: repo.session_name(name),
        },
    )
}

fn prune_exec(_gctx: &mut GlobalContext, repo: &ops::Repo, _args: &ArgMatches) -> CliResult {
    ops::prune(&repo.base)
}

fn gone_exec(gctx: &mut GlobalContext, repo: &ops::Repo, args: &ArgMatches) -> CliResult {
    ops::gone(
        gctx,
        &ops::GoneOptions {
            base: &repo.base,
            default_branch: &repo.default_branch,
            dry_run: args.get_flag("dry-run"),
            force: args.get_flag("force"),
            tmux: !args.get_flag("no-tmux"),
        },
    )
}

fn track_exec(gctx: &mut GlobalContext, repo: &ops::Repo, args: &ArgMatches) -> CliResult {
    ops::track(
        gctx,
        &ops::TrackOptions {
            base: &repo.base,
            current: &repo.current,
            paths: required_many(args, "path")?,
            worktrees: repo.worktree_paths()?,
            force: args.get_flag("force"),
        },
    )
}

fn links_exec(gctx: &mut GlobalContext, repo: &ops::Repo, _args: &ArgMatches) -> CliResult {
    ops::links(gctx, &ops::LinksOptions { base: &repo.base, current: &repo.current })
}

fn sync_exec(gctx: &mut GlobalContext, repo: &ops::Repo, args: &ArgMatches) -> CliResult {
    ops::sync(
        gctx,
        &ops::SyncOptions {
            base: &repo.base,
            current: &repo.current,
            worktrees: repo.worktree_paths()?,
            quiet: false,
            force: args.get_flag("force"),
        },
    )
}

fn required<'a>(args: &'a ArgMatches, name: &str) -> Result<&'a str, CliError> {
    args.get_one::<String>(name).map(String::as_str).ok_or_else(|| CliError::from(format!("missing required argument `{name}`")))
}

fn required_many<'a>(args: &'a ArgMatches, name: &str) -> Result<Vec<&'a str>, CliError> {
    args.get_many::<String>(name)
        .map(|values| values.map(String::as_str).collect())
        .ok_or_else(|| CliError::from(format!("missing required argument `{name}`")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worktree_subcommands_have_exec_handlers() {
        for command in builtin() {
            assert!(builtin_exec(command.get_name()).is_some(), "{} is missing an exec handler", command.get_name());
        }
    }

    #[test]
    fn worktree_cli_definition_is_valid() {
        cli().debug_assert();
    }

    #[test]
    fn worktree_subcommand_aliases_resolve_to_canonical_commands() {
        let matches = cli().try_get_matches_from(["worktree", "ls"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("list"));

        let matches = cli().try_get_matches_from(["worktree", "rm", "example"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("remove"));
    }
}
