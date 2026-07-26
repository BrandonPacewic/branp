//! `git` subcommands.
//!
//! This module provides Git-related helper commands that integrate with GitHub.
//!

use crate::context::GlobalContext;
use crate::errors::{CliError, CliResult};
use crate::utils::browser;
use clap::{Arg, ArgAction, ArgMatches, Command};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::process::Output;
use std::time::Duration;

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
            Command::new("check")
                .about("Monitor PR checks until the PR is merged")
                .arg(
                    Arg::new("target")
                        .help("Pull request number, URL, or branch; defaults to the current branch PR")
                        .value_name("PR|URL|BRANCH"),
                )
                .arg(
                    Arg::new("interval")
                        .short('i')
                        .long("interval")
                        .help("Refresh interval in seconds")
                        .default_value("5")
                        .value_parser(clap::value_parser!(u64).range(1..)),
                )
                .arg(
                    Arg::new("once")
                        .long("once")
                        .help("Show the current PR status and checks without waiting for merge")
                        .action(ArgAction::SetTrue),
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
        Some(("check", sub)) => check(gctx, sub),
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

fn check(gctx: &mut GlobalContext, args: &ArgMatches) -> CliResult {
    let target = args.get_one::<String>("target").map(String::as_str);
    let interval = *args.get_one::<u64>("interval").unwrap();
    let once = args.get_flag("once");
    let pr = current_pr(gctx, target)?;

    print_pr_status(gctx, &pr);
    if once {
        let checks = current_checks(gctx, target)?;
        print_check_summary(gctx, &checks);
        return Ok(());
    }

    wait_for_merge(gctx, target, &pr, interval)
}

fn print_pr_status(gctx: &mut GlobalContext, pr: &PrStatus) {
    let draft = if pr.is_draft {
        format!(" {}", color_print::cformat!("<black!>[DRAFT]</>"))
    } else {
        String::new()
    };
    gctx.shell().note(format!(
        "{}{} {} -> {}",
        crate::utils::terminal::hyperlink(&colored_pr_label(pr.number), &pr.url),
        draft,
        color_print::cformat!("<cyan>{}</>", pr.head_ref_name),
        color_print::cformat!("<black!>{}</>", pr.base_ref_name)
    ));
    gctx.shell().note(format!("  Title : {}", pr.title));
    gctx.shell()
        .note(format!("  State : {}", pr.status_label()));
    if let Some(merged_at) = &pr.merged_at {
        gctx.shell().note(format!(
            "  Merged: {}",
            color_print::cformat!("<black!>{merged_at}</>")
        ));
    }
}

fn wait_for_merge(
    gctx: &mut GlobalContext,
    target: Option<&str>,
    initial_pr: &PrStatus,
    interval: u64,
) -> CliResult {
    if initial_pr.is_merged() {
        gctx.shell().note(merged_message(
            initial_pr.number,
            initial_pr.merged_at.as_deref(),
        ));
        return Ok(());
    }

    let mut monitor = PrMonitor::new(initial_pr.clone(), current_checks(gctx, target)?);
    let mut frame = crate::utils::terminal::DynamicRenderLoop::start(&monitor.render(Some('-')))?;

    loop {
        for _ in 0..spinner_ticks(interval) {
            std::thread::sleep(frame.tick_interval());
            frame.advance(&monitor.render(Some(frame.spinner())))?;
        }

        let pr = current_pr(gctx, target)?;
        let checks = current_checks(gctx, target)?;
        let is_merged = pr.is_merged();
        let is_closed = pr.state == "CLOSED";
        let merged_at = pr.merged_at.clone();
        let number = pr.number;

        monitor.update(pr, checks);
        frame.replace(&monitor.render(Some(frame.spinner())))?;

        if is_merged {
            frame.replace(&monitor.render(None))?;
            gctx.shell()
                .note(merged_message(number, merged_at.as_deref()));
            return Ok(());
        }

        if is_closed {
            frame.replace(&monitor.render(None))?;
            return Err(CliError::from(format!(
                "PR #{number} was closed without merging"
            )));
        }
    }
}

fn spinner_ticks(interval: u64) -> u64 {
    let tick_ms = 120;
    (Duration::from_secs(interval).as_millis() as u64 / tick_ms).max(1)
}

fn merged_message(number: u64, merged_at: Option<&str>) -> String {
    format!(
        "{} {}{}",
        colored_pr_label(number),
        color_print::cformat!("<magenta,bold>merged</>"),
        merged_at
            .map(|merged_at| format!(" at {}", color_print::cformat!("<black!>{merged_at}</>")))
            .unwrap_or_default()
    )
}

fn colored_pr_label(number: u64) -> String {
    color_print::cformat!("<cyan,bold>PR #{number}</>")
}

fn current_pr(gctx: &mut GlobalContext, target: Option<&str>) -> Result<PrStatus, CliError> {
    let mut args = vec![
        "pr",
        "view",
        "--json",
        "number,title,url,state,isDraft,mergeStateStatus,baseRefName,headRefName,mergedAt",
    ];
    if let Some(target) = target {
        args.insert(2, target);
    }

    let output = crate::utils::command::output("gh", &args, Some(gctx.cwd())).map_err(|e| {
        let subject = target.unwrap_or("the current branch");
        CliError::from(format!(
            "failed to find a pull request for {subject}: {}",
            e.message
        ))
    })?;

    serde_json::from_str(&output)
        .map_err(|e| CliError::from(format!("failed to parse GitHub PR status: {e}")))
}

fn current_checks(
    gctx: &mut GlobalContext,
    target: Option<&str>,
) -> Result<Vec<PrCheck>, CliError> {
    let mut args = vec![
        "pr",
        "checks",
        "--json",
        "bucket,name,state,workflow,completedAt,link",
    ];
    if let Some(target) = target {
        args.insert(2, target);
    }

    let output = std::process::Command::new("gh")
        .args(&args)
        .current_dir(gctx.cwd())
        .output()?;

    if !output.status.success() && output.status.code() != Some(8) {
        return Err(command_output_error("gh", &args, &output));
    }

    serde_json::from_slice(&output.stdout)
        .map_err(|e| CliError::from(format!("failed to parse GitHub PR checks: {e}")))
}

fn print_check_summary(gctx: &mut GlobalContext, checks: &[PrCheck]) {
    gctx.shell()
        .note(format!("  Checks: {}", check_summary(checks)));
    for check in checks {
        gctx.shell().note(format!(
            "    {} {}",
            check.bucket_symbol(),
            check.display_label()
        ));
    }
}

fn check_summary(checks: &[PrCheck]) -> String {
    if checks.is_empty() {
        return "none".to_string();
    }

    let mut buckets = BTreeMap::new();
    for check in checks {
        *buckets.entry(check.bucket.as_str()).or_insert(0usize) += 1;
    }

    ["pending", "fail", "cancel", "pass", "skipping"]
        .into_iter()
        .filter_map(|bucket| buckets.get(bucket).map(|count| (*count, bucket)))
        .map(|(count, bucket)| colored_check_bucket_count(bucket, count))
        .collect::<Vec<_>>()
        .join(", ")
}

fn active_checks(checks: &[PrCheck]) -> String {
    let names = checks
        .iter()
        .filter(|check| check.bucket == "pending")
        .take(3)
        .map(PrCheck::display_name)
        .collect::<Vec<_>>();

    if names.is_empty() {
        String::new()
    } else {
        format!(
            " | {} {}",
            color_print::cformat!("<yellow>waiting on</>"),
            names.join(", ")
        )
    }
}

fn command_output_error(program: &str, args: &[&str], output: &Output) -> CliError {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let detail = if !stderr.trim().is_empty() {
        stderr.trim()
    } else {
        stdout.trim()
    };

    CliError::from(format!(
        "{} {} failed{}{}",
        program,
        args.join(" "),
        if detail.is_empty() { "" } else { ": " },
        detail
    ))
}

struct PrMonitor {
    pr: PrStatus,
    checks: Vec<PrCheck>,
}

impl PrMonitor {
    fn new(pr: PrStatus, checks: Vec<PrCheck>) -> Self {
        Self { pr, checks }
    }

    fn update(&mut self, pr: PrStatus, checks: Vec<PrCheck>) {
        self.pr = pr;
        self.checks = checks;
    }

    fn render(&self, spinner: Option<char>) -> Vec<String> {
        let marker = spinner.unwrap_or(' ');
        vec![format!(
            "{} {} {} | checks: {}{}",
            color_print::cformat!("<yellow>{marker}</>"),
            colored_pr_label(self.pr.number),
            self.pr.status_label(),
            check_summary(&self.checks),
            active_checks(&self.checks)
        )]
    }
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

#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PrStatus {
    number: u64,
    title: String,
    url: String,
    state: String,
    is_draft: bool,
    merge_state_status: Option<String>,
    base_ref_name: String,
    head_ref_name: String,
    merged_at: Option<String>,
}

impl PrStatus {
    fn is_merged(&self) -> bool {
        self.state == "MERGED" || self.merged_at.is_some()
    }

    fn status_label(&self) -> String {
        if self.is_merged() {
            return color_print::cformat!("<magenta,bold>merged</>");
        }

        let state = match self.state.as_str() {
            "OPEN" => color_print::cformat!("<cyan>open</>"),
            "CLOSED" => color_print::cformat!("<red,bold>closed</>"),
            other => other.to_string(),
        };

        match self.merge_state_status.as_deref() {
            Some("BLOCKED") => format!("{state}, {}", color_print::cformat!("<yellow>blocked</>")),
            Some("BEHIND") => {
                format!(
                    "{state}, {}",
                    color_print::cformat!("<yellow>behind base</>")
                )
            }
            Some("CLEAN") => {
                format!(
                    "{state}, {}",
                    color_print::cformat!("<green>ready to merge</>")
                )
            }
            Some("DIRTY") => {
                format!(
                    "{state}, {}",
                    color_print::cformat!("<red,bold>has conflicts</>")
                )
            }
            Some("DRAFT") => format!("{state}, {}", color_print::cformat!("<black!>draft</>")),
            Some("HAS_HOOKS") => {
                format!(
                    "{state}, {}",
                    color_print::cformat!("<yellow>waiting on hooks</>")
                )
            }
            Some("UNKNOWN") | None => state.to_string(),
            Some(status) => format!(
                "{state}, {}",
                color_print::cformat!("<yellow>{}</>", enum_label(status))
            ),
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PrCheck {
    bucket: String,
    name: String,
    state: String,
    workflow: Option<String>,
}

impl PrCheck {
    fn display_name(&self) -> String {
        match self.workflow.as_deref() {
            Some(workflow) if !workflow.is_empty() => format!("{workflow} / {}", self.name),
            _ => self.name.clone(),
        }
    }

    fn display_label(&self) -> String {
        format!(
            "{} ({})",
            self.display_name(),
            colored_check_state(&self.bucket, &self.state)
        )
    }

    fn bucket_symbol(&self) -> String {
        match self.bucket.as_str() {
            "pass" => color_print::cformat!("<green,bold>PASS</>"),
            "fail" => color_print::cformat!("<red,bold>FAIL</>"),
            "pending" => color_print::cformat!("<yellow,bold>WAIT</>"),
            "skipping" => color_print::cformat!("<black!,bold>SKIP</>"),
            "cancel" => color_print::cformat!("<red>CANCEL</>"),
            _ => color_print::cformat!("<black!>INFO</>"),
        }
    }
}

fn colored_check_bucket_count(bucket: &str, count: usize) -> String {
    let label = match bucket {
        "pass" => "passed",
        "fail" => "failed",
        "pending" => "pending",
        "skipping" => "skipped",
        "cancel" => "cancelled",
        _ => "unknown",
    };

    let value = format!("{count} {label}");
    match bucket {
        "pass" => color_print::cformat!("<green>{value}</>"),
        "fail" => color_print::cformat!("<red,bold>{value}</>"),
        "pending" => color_print::cformat!("<yellow>{value}</>"),
        "skipping" => color_print::cformat!("<black!>{value}</>"),
        "cancel" => color_print::cformat!("<red>{value}</>"),
        _ => color_print::cformat!("<black!>{value}</>"),
    }
}

fn colored_check_state(bucket: &str, state: &str) -> String {
    let label = enum_label(state);
    match bucket {
        "pass" => color_print::cformat!("<green>{label}</>"),
        "fail" => color_print::cformat!("<red,bold>{label}</>"),
        "pending" => color_print::cformat!("<yellow>{label}</>"),
        "skipping" => color_print::cformat!("<black!>{label}</>"),
        "cancel" => color_print::cformat!("<red>{label}</>"),
        _ => color_print::cformat!("<black!>{label}</>"),
    }
}

fn enum_label(value: &str) -> String {
    value.to_ascii_lowercase().replace('_', " ")
}
