use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;

use crate::context::GlobalContext;
use crate::errors::{CliError, CliResult};
use crate::utils::browser;

pub struct CoauthorOptions<'a> {
    pub usernames: Vec<&'a str>,
}

pub struct PrsOptions<'a> {
    pub remote: &'a str,
}

pub struct CheckOptions<'a> {
    pub target: Option<&'a str>,
    pub interval: u64,
    pub once: bool,
}

pub struct ForkOptions<'a> {
    pub remote: &'a str,
    pub url: &'a str,
    pub branch: Option<&'a str>,
}

pub struct IgnoreOptions<'a> {
    pub patterns: Vec<&'a str>,
    pub raw: bool,
    pub list: bool,
    pub remove: bool,
}

pub struct OpenOptions<'a> {
    pub remote: &'a str,
    pub pr: bool,
    pub targets: Vec<&'a str>,
}

pub fn coauthor(gctx: &mut GlobalContext, options: &CoauthorOptions<'_>) -> CliResult {
    for user in &options.usernames {
        match fetch_github_user(user) {
            Ok(line) => gctx.shell().note(line),
            Err(e) => gctx.shell().error(format!("{user}: {e}")),
        }
    }

    Ok(())
}

pub fn prs(gctx: &mut GlobalContext, options: &PrsOptions<'_>) -> CliResult {
    let repo = crate::utils::git::github_remote(gctx.cwd(), options.remote)?;

    let client = reqwest::blocking::Client::new();
    let prs = fetch_open_prs(&client, &repo.owner, &repo.name).map_err(CliError::from)?;

    if prs.is_empty() {
        gctx.shell().note("No open pull requests found.");
        return Ok(());
    }

    for pr in prs {
        let changes = fetch_changes_count(&client, &repo.owner, &repo.name, pr.number).unwrap_or(0);

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

pub fn check(gctx: &mut GlobalContext, options: &CheckOptions<'_>) -> CliResult {
    let pr = current_pr(gctx, options.target)?;

    print_pr_status(gctx, &pr);
    if options.once {
        let checks = current_checks(gctx, options.target)?;
        print_check_summary(gctx, &checks);
        return Ok(());
    }

    wait_for_merge(gctx, options.target, &pr, options.interval)
}

pub fn fork(gctx: &mut GlobalContext, options: &ForkOptions<'_>) -> CliResult {
    add_or_reuse_remote(gctx, options.remote, options.url)?;
    crate::utils::git::run(gctx.cwd(), &["fetch", options.remote])?;

    let branch = match options.branch {
        Some(branch) => branch.to_string(),
        None => default_branch(gctx, options.remote)?,
    };

    switch_to_branch(gctx, options.remote, &branch)?;

    let upstream = format!("{}/{}", options.remote, branch);
    let upstream_arg = format!("--set-upstream-to={upstream}");
    crate::utils::git::run(gctx.cwd(), &["branch", &upstream_arg])?;
    gctx.shell()
        .note(format!("`{branch}` is now tracking `{upstream}`"));

    Ok(())
}

pub fn ignore(gctx: &mut GlobalContext, options: &IgnoreOptions<'_>) -> CliResult {
    let repo_root = repo_root(gctx.cwd())?;
    let exclude_path = local_exclude_path(gctx.cwd())?;
    let contents = read_optional_file(&exclude_path)?;

    if options.list {
        list_local_ignores(gctx, &contents);
        return Ok(());
    }

    if options.patterns.is_empty() {
        return Err(CliError::from(
            "expected `bp git ignore <path|pattern>...` or `bp git ignore --list`",
        ));
    }

    let patterns = ignore_patterns(gctx.cwd(), &repo_root, options)?;
    if options.remove {
        remove_local_ignores(gctx, &exclude_path, &contents, &patterns)
    } else {
        add_local_ignores(gctx, &exclude_path, &contents, &patterns)
    }
}

pub fn open(gctx: &mut GlobalContext, options: &OpenOptions<'_>) -> CliResult {
    let repo = crate::utils::git::github_remote(gctx.cwd(), options.remote)?;
    let base_url = format!("https://github.com/{}/{}", repo.owner, repo.name);
    let (pr_mode, target) = open_args(options.pr, &options.targets)?;
    let url = if pr_mode {
        pull_request_url(gctx.cwd(), &repo.owner, &repo.name, &base_url, target)?
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

fn repo_root(cwd: &Path) -> Result<PathBuf, CliError> {
    let output = crate::utils::git::output(cwd, &["rev-parse", "--show-toplevel"])?;
    Ok(PathBuf::from(output.trim()))
}

fn local_exclude_path(cwd: &Path) -> Result<PathBuf, CliError> {
    let output = crate::utils::git::output(cwd, &["rev-parse", "--git-path", "info/exclude"])?;
    let path = PathBuf::from(output.trim());
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(cwd.join(path))
    }
}

fn read_optional_file(path: &Path) -> Result<String, CliError> {
    match fs::read_to_string(path) {
        Ok(contents) => Ok(contents),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(e.into()),
    }
}

fn list_local_ignores(gctx: &mut GlobalContext, contents: &str) {
    let patterns = active_ignore_lines(contents).collect::<Vec<_>>();
    if patterns.is_empty() {
        gctx.shell().note("No repo-local ignore patterns found.");
        return;
    }

    for pattern in patterns {
        gctx.shell().note(pattern);
    }
}

fn ignore_patterns(
    cwd: &Path,
    repo_root: &Path,
    options: &IgnoreOptions<'_>,
) -> Result<Vec<String>, CliError> {
    let mut patterns = Vec::new();
    for pattern in &options.patterns {
        let pattern = if options.raw {
            raw_ignore_pattern(pattern)?
        } else {
            path_ignore_pattern(cwd, repo_root, pattern)?
        };
        if !patterns.contains(&pattern) {
            patterns.push(pattern);
        }
    }
    Ok(patterns)
}

fn raw_ignore_pattern(pattern: &str) -> Result<String, CliError> {
    let pattern = pattern.trim_end_matches(['\r', '\n']);
    if pattern.is_empty() {
        Err(CliError::from("ignore pattern cannot be empty"))
    } else {
        Ok(pattern.to_string())
    }
}

fn path_ignore_pattern(cwd: &Path, repo_root: &Path, input: &str) -> Result<String, CliError> {
    let trimmed = input.trim_end_matches(['\r', '\n']);
    if trimmed.is_empty() {
        return Err(CliError::from("ignore path cannot be empty"));
    }

    let path = Path::new(trimmed);
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    };
    let absolute = normalize_path(&absolute);
    let repo_root = normalize_path(repo_root);
    let relative = absolute.strip_prefix(&repo_root).map_err(|_| {
        CliError::from(format!(
            "`{trimmed}` is outside this repo; use --raw to add it as a pattern"
        ))
    })?;
    let mut pattern = path_to_gitignore_pattern(relative)?;

    if pattern.is_empty() {
        return Err(CliError::from("cannot ignore the repository root"));
    }

    if path_is_existing_dir(cwd, path) && !pattern.ends_with('/') {
        pattern.push('/');
    }

    Ok(pattern)
}

fn path_is_existing_dir(cwd: &Path, path: &Path) -> bool {
    if path.is_absolute() {
        path.is_dir()
    } else {
        cwd.join(path).is_dir()
    }
}

fn path_to_gitignore_pattern(path: &Path) -> Result<String, CliError> {
    let pattern = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/");

    if pattern.contains('\n') || pattern.contains('\r') {
        return Err(CliError::from("ignore path cannot contain a newline"));
    }

    Ok(pattern)
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

fn add_local_ignores(
    gctx: &mut GlobalContext,
    exclude_path: &Path,
    contents: &str,
    patterns: &[String],
) -> CliResult {
    let existing = active_ignore_lines(contents).collect::<Vec<_>>();
    let new_patterns = patterns
        .iter()
        .filter(|pattern| !existing.contains(&pattern.as_str()))
        .collect::<Vec<_>>();

    if new_patterns.is_empty() {
        gctx.shell()
            .note("All requested patterns are already ignored locally.");
        return Ok(());
    }

    let mut updated = contents.to_string();
    if !updated.is_empty() && !updated.ends_with('\n') {
        updated.push('\n');
    }
    for pattern in &new_patterns {
        updated.push_str(pattern);
        updated.push('\n');
    }

    if let Some(parent) = exclude_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(exclude_path, updated)?;

    for pattern in new_patterns {
        gctx.shell().note(format!("Ignored locally: {pattern}"));
    }
    gctx.shell()
        .note(format!("Updated {}", exclude_path.display()));
    Ok(())
}

fn remove_local_ignores(
    gctx: &mut GlobalContext,
    exclude_path: &Path,
    contents: &str,
    patterns: &[String],
) -> CliResult {
    let mut removed = Vec::new();
    let mut kept = Vec::new();

    for line in contents.lines() {
        if patterns.iter().any(|pattern| pattern == line) {
            removed.push(line.to_string());
        } else {
            kept.push(line);
        }
    }

    if removed.is_empty() {
        gctx.shell()
            .note("No matching repo-local ignore patterns found.");
        return Ok(());
    }

    let mut updated = kept.join("\n");
    if contents.ends_with('\n') && !updated.is_empty() {
        updated.push('\n');
    }
    fs::write(exclude_path, updated)?;

    removed.sort();
    removed.dedup();
    for pattern in removed {
        gctx.shell()
            .note(format!("Removed local ignore: {pattern}"));
    }
    gctx.shell()
        .note(format!("Updated {}", exclude_path.display()));
    Ok(())
}

fn active_ignore_lines(contents: &str) -> impl Iterator<Item = &str> {
    contents.lines().filter(|line| {
        let trimmed = line.trim();
        !trimmed.is_empty() && !trimmed.starts_with('#')
    })
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

    let output = crate::utils::gh::output(gctx.cwd(), &args).map_err(|e| {
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

    let output = crate::utils::gh::output_allowing_exit_codes(gctx.cwd(), &args, &[8])?;

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

fn add_or_reuse_remote(gctx: &mut GlobalContext, remote: &str, url: &str) -> CliResult {
    match crate::utils::git::remote_url(gctx.cwd(), remote)? {
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
            crate::utils::git::run(gctx.cwd(), &["remote", "add", remote, url])?;
            gctx.shell().note(format!("Added remote `{remote}`"));
            Ok(())
        }
    }
}

fn default_branch(gctx: &mut GlobalContext, remote: &str) -> Result<String, CliError> {
    let output =
        crate::utils::git::output(gctx.cwd(), &["remote", "show", remote]).map_err(|e| {
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
    match crate::utils::git::run(gctx.cwd(), &["switch", branch]) {
        Ok(()) => Ok(()),
        Err(_) => {
            let upstream = format!("{remote}/{branch}");
            crate::utils::git::run(gctx.cwd(), &["switch", "--track", "-c", branch, &upstream])
        }
    }
}

fn open_args<'a>(
    pr_flag: bool,
    targets: &'a [&'a str],
) -> Result<(bool, Option<&'a str>), CliError> {
    match targets {
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
    let output =
        crate::utils::gh::output(cwd, &["pr", "view", "--json", "number", "--jq", ".number"])
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
    let output = crate::utils::gh::output(
        cwd,
        &[
            "api",
            &api_path,
            "-H",
            "Accept: application/vnd.github+json",
            "--jq",
            ".[0].number",
        ],
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

#[derive(Deserialize)]
struct Review {
    state: String,
}

#[derive(Deserialize)]
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

#[derive(Deserialize)]
struct PullRequest {
    number: u64,
    title: String,
    html_url: String,
    draft: bool,
    assignees: Vec<SimpleUser>,
    requested_reviewers: Vec<SimpleUser>,
}

#[derive(Clone, Deserialize)]
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

#[derive(Deserialize)]
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn converts_paths_to_repo_relative_ignore_patterns() {
        let cwd = Path::new("repo/packages/app");
        let repo_root = Path::new("repo");

        assert_eq!(
            path_ignore_pattern(cwd, repo_root, "target/debug.log")
                .ok()
                .as_deref(),
            Some("packages/app/target/debug.log")
        );
        assert_eq!(
            path_ignore_pattern(cwd, repo_root, "../shared/cache")
                .ok()
                .as_deref(),
            Some("packages/shared/cache")
        );
    }

    #[test]
    fn rejects_non_raw_paths_outside_repo() {
        let err = match path_ignore_pattern(Path::new("repo"), Path::new("repo"), "../outside") {
            Ok(pattern) => panic!("outside path should be rejected, got {pattern}"),
            Err(err) => err,
        };

        assert!(err.message.contains("use --raw"));
    }

    #[test]
    fn preserves_active_ignore_lines() {
        let contents = "# comment\n\nbuild/\n  literal-space\n";
        let lines = active_ignore_lines(contents).collect::<Vec<_>>();

        assert_eq!(lines, ["build/", "  literal-space"]);
    }
}
