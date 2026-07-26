//! `git` subcommands.
//!
//! This module provides Git-related helper commands that integrate with GitHub.
//!

use crate::context::GlobalContext;
use crate::errors::{CliError, CliResult};
use crate::utils::browser;
use clap::{Arg, ArgAction, ArgMatches, Command};
use serde::Deserialize;

pub fn command() -> Command {
    Command::new("git")
        .about("Git-related helpers")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("coauthor")
                .about("Generate GitHub no-reply co-author lines")
                .arg(
                    Arg::new("usernames")
                        .help("One or more GitHub usernames")
                        .required(true)
                        .num_args(1..)
                        .value_name("USERNAME"),
                ),
        )
        .subcommand(
            Command::new("prs")
                .about("List open pull requests in this GitHub repo")
                .arg(
                    Arg::new("remote")
                        .short('r')
                        .long("remote")
                        .help("Git remote to use")
                        .default_value("origin"),
                ),
        )
        .subcommand(
            Command::new("fork")
                .about("Add and track a fork remote branch for local PR edits")
                .arg(
                    Arg::new("remote")
                        .help("Fork remote name to add or reuse")
                        .required(true)
                        .value_name("REMOTE"),
                )
                .arg(
                    Arg::new("url")
                        .help("Fork Git URL")
                        .required(true)
                        .value_name("URL"),
                )
                .arg(
                    Arg::new("branch")
                        .short('b')
                        .long("branch")
                        .help("Branch to switch to and track; defaults to the fork remote HEAD")
                        .value_name("BRANCH"),
                ),
        )
        .subcommand(
            Command::new("open")
                .about("Open this GitHub repo, issue, pull request, or commit in a browser")
                .arg(
                    Arg::new("pr")
                        .help("Open the pull request for the current branch, number, or commit")
                        .long("pr")
                        .short('p')
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("targets")
                        .help("Optional `pr`, issue/PR number, or commit hash")
                        .num_args(0..=2)
                        .value_name("TARGET"),
                )
                .arg(
                    Arg::new("remote")
                        .short('r')
                        .long("remote")
                        .help("Git remote to use")
                        .default_value("origin"),
                ),
        )
}

pub fn exec(gctx: &mut GlobalContext, args: &ArgMatches) -> CliResult {
    match args.subcommand() {
        Some(("coauthor", sub)) => coauthor(gctx, sub),
        Some(("prs", sub)) => prs(gctx, sub),
        Some(("fork", sub)) => fork(gctx, sub),
        Some(("open", sub)) => open(gctx, sub),
        _ => Err(CliError::from("no `git` subcommand provided")),
    }
}

fn coauthor(gctx: &mut GlobalContext, args: &ArgMatches) -> CliResult {
    for user in args.get_many::<String>("usernames").unwrap() {
        match fetch_github_user(user) {
            Ok(line) => gctx.shell().note(line),
            Err(e) => gctx.shell().error(format!("{user}: {e}")),
        }
    }

    Ok(())
}

fn prs(gctx: &mut GlobalContext, args: &ArgMatches) -> CliResult {
    let remote = args.get_one::<String>("remote").unwrap();
    let (owner, repo) = parse_owner_repo(remote).map_err(CliError::from)?;

    let client = reqwest::blocking::Client::new();
    let prs = fetch_open_prs(&client, &owner, &repo).map_err(CliError::from)?;

    if prs.is_empty() {
        gctx.shell().note("No open pull requests found.");
        return Ok(());
    }

    for pr in prs {
        let changes = fetch_changes_count(&client, &owner, &repo, pr.number).unwrap_or(0);

        let pr_label =
            crate::utils::terminal::hyperlink(&format!("PR #{}", pr.number), &pr.html_url);
        gctx.shell().note(format!(
            "{} {}{}",
            pr_label,
            if pr.draft { "[DRAFT] " } else { "" },
            pr.title
        ));
        gctx.shell().note(format!(
            "  Assignees : {}",
            linked_github_users(&pr.assignees)
        ));
        gctx.shell().note(format!(
            "  Reviewers : {}",
            linked_github_users(&pr.requested_reviewers)
        ));
        gctx.shell()
            .note(format!("  Change requests pending: {changes}"));
        gctx.shell().note("------------------------------");
    }

    Ok(())
}

fn fork(gctx: &mut GlobalContext, args: &ArgMatches) -> CliResult {
    let remote = required(args, "remote")?;
    let url = required(args, "url")?;

    add_or_reuse_remote(gctx, remote, url)?;
    run_git(gctx, &["fetch", remote])?;

    let branch = match args.get_one::<String>("branch") {
        Some(branch) => branch.to_string(),
        None => default_branch(remote)?,
    };

    switch_to_branch(gctx, remote, &branch)?;

    let upstream = format!("{remote}/{branch}");
    let upstream_arg = format!("--set-upstream-to={upstream}");
    run_git(gctx, &["branch", &upstream_arg])?;
    gctx.shell()
        .note(format!("`{branch}` is now tracking `{upstream}`"));

    Ok(())
}

fn add_or_reuse_remote(gctx: &mut GlobalContext, remote: &str, url: &str) -> CliResult {
    match remote_url(remote)? {
        Some(existing_url) if existing_url == url => {
            gctx.shell().note(format!(
                "Remote `{remote}` already exists with the requested URL"
            ));
            Ok(())
        }
        Some(existing_url) => Err(CliError::from(format!(
            "remote `{remote}` already exists with URL `{existing_url}`"
        ))),
        None => {
            run_git(gctx, &["remote", "add", remote, url])?;
            gctx.shell().note(format!("Added remote `{remote}`"));
            Ok(())
        }
    }
}

fn remote_url(remote: &str) -> Result<Option<String>, CliError> {
    match crate::utils::command::output("git", &["remote", "get-url", remote], None) {
        Ok(url) => Ok(Some(url.trim().to_string())),
        Err(_) => Ok(None),
    }
}

fn default_branch(remote: &str) -> Result<String, CliError> {
    let output =
        crate::utils::command::output("git", &["remote", "show", remote], None).map_err(|e| {
            CliError::from(format!(
                "failed to determine default branch for `{remote}`: {}; pass --branch",
                e.message
            ))
        })?;

    output
        .lines()
        .find_map(|line| line.trim().strip_prefix("HEAD branch: "))
        .filter(|branch| !branch.is_empty() && *branch != "(unknown)")
        .map(str::to_string)
        .ok_or_else(|| {
            CliError::from(format!(
                "failed to determine default branch for `{remote}`; pass --branch"
            ))
        })
}

fn switch_to_branch(gctx: &mut GlobalContext, remote: &str, branch: &str) -> CliResult {
    match run_git(gctx, &["switch", branch]) {
        Ok(()) => Ok(()),
        Err(_) => {
            let upstream = format!("{remote}/{branch}");
            run_git(gctx, &["switch", "--track", "-c", branch, &upstream])
        }
    }
}

fn open(gctx: &mut GlobalContext, args: &ArgMatches) -> CliResult {
    let remote = args.get_one::<String>("remote").unwrap();
    let (owner, repo) = parse_owner_repo(remote).map_err(CliError::from)?;
    let base_url = format!("https://github.com/{owner}/{repo}");
    let (pr_mode, target) = open_args(args)?;
    let url = if pr_mode {
        pull_request_url(gctx.cwd(), &owner, &repo, &base_url, target)?
    } else {
        target_url(&base_url, target)?
    };

    browser::open_url(&url)?;
    gctx.shell().note(format!(
        "Opened {}",
        crate::utils::terminal::hyperlink(&url, &url)
    ));
    Ok(())
}

fn open_args(args: &ArgMatches) -> Result<(bool, Option<&str>), CliError> {
    let pr_flag = args.get_flag("pr");
    let targets = args
        .get_many::<String>("targets")
        .map(|values| values.map(String::as_str).collect::<Vec<_>>())
        .unwrap_or_default();

    match targets.as_slice() {
        ["pr"] => Ok((true, None)),
        ["pr", target] => Ok((true, Some(*target))),
        [] => Ok((pr_flag, None)),
        [target] => Ok((pr_flag, Some(*target))),
        _ => Err(CliError::from(
            "expected `bp git open [pr] [issue|pull-request|commit]`",
        )),
    }
}

fn target_url(base_url: &str, target: Option<&str>) -> Result<String, CliError> {
    match target {
        None => Ok(base_url.to_string()),
        Some(target) if is_issue_number(target) => Ok(format!("{base_url}/issues/{target}")),
        Some(target) if is_commit_hash(target) => Ok(format!("{base_url}/commit/{target}")),
        Some(target) => Err(CliError::from(format!(
            "`{target}` is not a pull request/issue number or commit hash"
        ))),
    }
}

fn pull_request_url(
    cwd: &std::path::Path,
    owner: &str,
    repo: &str,
    base_url: &str,
    target: Option<&str>,
) -> Result<String, CliError> {
    let number = match target {
        Some(target) if is_issue_number(target) => target.to_string(),
        Some(target) if is_commit_hash(target) => {
            pull_request_for_commit(cwd, owner, repo, target)?
        }
        Some(target) => {
            return Err(CliError::from(format!(
                "`{target}` is not a pull request number or commit hash"
            )))
        }
        None => pull_request_for_current_branch(cwd)?,
    };

    Ok(format!("{base_url}/pull/{number}"))
}

fn pull_request_for_current_branch(cwd: &std::path::Path) -> Result<String, CliError> {
    let output = crate::utils::command::output(
        "gh",
        &["pr", "view", "--json", "number", "--jq", ".number"],
        Some(cwd),
    )
    .map_err(|e| {
        CliError::from(format!(
            "failed to find a pull request for the current branch: {}",
            e.message
        ))
    })?;
    non_empty_output(output, "no pull request found for the current branch")
}

fn pull_request_for_commit(
    cwd: &std::path::Path,
    owner: &str,
    repo: &str,
    commit: &str,
) -> Result<String, CliError> {
    let api_path = format!("repos/{owner}/{repo}/commits/{commit}/pulls");
    let output = crate::utils::command::output(
        "gh",
        &[
            "api",
            &api_path,
            "-H",
            "Accept: application/vnd.github+json",
            "--jq",
            ".[0].number",
        ],
        Some(cwd),
    )
    .map_err(|e| {
        CliError::from(format!(
            "failed to find a pull request containing `{commit}`: {}",
            e.message
        ))
    })?;
    non_empty_output(
        output,
        format!("no pull request found containing `{commit}`"),
    )
}

fn non_empty_output(output: String, message: impl Into<String>) -> Result<String, CliError> {
    let output = output.trim();
    if output.is_empty() || output == "null" {
        Err(CliError::from(message.into()))
    } else {
        Ok(output.to_string())
    }
}

fn is_issue_number(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|c| c.is_ascii_digit())
}

fn is_commit_hash(value: &str) -> bool {
    (7..=40).contains(&value.len()) && value.chars().all(|c| c.is_ascii_hexdigit())
}

fn fetch_github_user(username: &str) -> Result<String, String> {
    let url = format!("https://api.github.com/users/{username}");
    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(&url)
        .header("User-Agent", "branp-git-coauthor")
        .send()
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("GitHub API returned {}", resp.status()));
    }

    let u: User = resp.json().map_err(|e| e.to_string())?;
    let name = u.name.unwrap_or_else(|| u.login.clone());
    let email = format!("{}+{}@users.noreply.github.com", u.id, u.login);
    Ok(format!("Co-authored-by: {name} <{email}>"))
}

fn run_git(gctx: &mut GlobalContext, args: &[&str]) -> CliResult {
    crate::utils::command::run("git", args, Some(gctx.cwd()))
}

fn required<'a>(args: &'a ArgMatches, name: &str) -> Result<&'a str, CliError> {
    args.get_one::<String>(name)
        .map(String::as_str)
        .ok_or_else(|| CliError::from(format!("missing required argument `{name}`")))
}

/// Parse `git remote get-url <remote>` into (owner, repo)
fn parse_owner_repo(remote: &str) -> Result<(String, String), String> {
    let url = crate::utils::command::output("git", &["remote", "get-url", remote], None)
        .map_err(|_| format!("Failed to get URL for remote `{remote}`"))?;
    let url = url.trim();
    let path = if let Some(stripped) = url.strip_prefix("git@github.com:") {
        stripped.to_string()
    } else if let Some(stripped) = url.strip_prefix("https://github.com/") {
        stripped.to_string()
    } else {
        return Err(format!("Unsupported remote URL format: {url}"));
    };

    let path = path.trim_end_matches(".git");
    let mut parts = path.splitn(2, '/');
    let owner = parts.next().ok_or("Missing owner")?.to_string();
    let repo = parts.next().ok_or("Missing repo")?.to_string();
    Ok((owner, repo))
}

fn fetch_open_prs(
    client: &reqwest::blocking::Client,
    owner: &str,
    repo: &str,
) -> Result<Vec<PullRequest>, String> {
    let url = format!("https://api.github.com/repos/{owner}/{repo}/pulls?state=open");
    let resp = client
        .get(&url)
        .header("User-Agent", "branp-git-prs")
        .send()
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("GitHub API returned {}", resp.status()));
    }

    resp.json::<Vec<PullRequest>>().map_err(|e| e.to_string())
}

fn fetch_changes_count(
    client: &reqwest::blocking::Client,
    owner: &str,
    repo: &str,
    pr_number: u64,
) -> Result<usize, String> {
    let url = format!("https://api.github.com/repos/{owner}/{repo}/pulls/{pr_number}/reviews");
    let resp = client
        .get(&url)
        .header("User-Agent", "branp-git-prs")
        .send()
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("GitHub API returned {}", resp.status()));
    }

    let reviews: Vec<Review> = resp.json().map_err(|e| e.to_string())?;
    Ok(reviews
        .into_iter()
        .filter(|r| r.state == "CHANGES_REQUESTED")
        .count())
}

#[derive(Deserialize)]
struct User {
    id: u64,
    login: String,
    name: Option<String>,
}

#[derive(serde::Deserialize)]
struct Review {
    state: String,
}

#[derive(serde::Deserialize)]
struct SimpleUser {
    login: String,
}

fn linked_github_users(users: &[SimpleUser]) -> String {
    users
        .iter()
        .map(|user| {
            crate::utils::terminal::hyperlink(
                &user.login,
                &format!("https://github.com/{}", user.login),
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[derive(serde::Deserialize)]
struct PullRequest {
    number: u64,
    title: String,
    html_url: String,
    draft: bool,
    assignees: Vec<SimpleUser>,
    requested_reviewers: Vec<SimpleUser>,
}
