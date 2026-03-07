//! `git` subcommands.
//!
//! This module provides Git-related helper commands that integrate with GitHub.
//!

use crate::context::GlobalContext;
use crate::errors::{CliError, CliResult};
use clap::{Arg, ArgMatches, Command};
use serde::Deserialize;

pub fn command() -> Command {
    Command::new("git")
        .about("Git-related helpers")
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
}

pub fn exec(gctx: &mut GlobalContext, args: &ArgMatches) -> CliResult {
    if let Some(sub) = args.subcommand_matches("coauthor") {
        for user in sub.get_many::<String>("usernames").unwrap() {
            match fetch_github_user(user) {
                Ok(line) => gctx.shell().note(line),
                Err(e) => gctx.shell().error(format!("{}: {}", user, e)),
            }
        }
    } else if let Some(sub) = args.subcommand_matches("prs") {
        let remote = sub.get_one::<String>("remote").unwrap();
        let (owner, repo) = match parse_owner_repo(remote) {
            Ok(pair) => pair,
            Err(e) => return Err(CliError::from(e)),
        };

        let client = reqwest::blocking::Client::new();
        let prs = match fetch_open_prs(&client, &owner, &repo) {
            Ok(v) => v,
            Err(e) => return Err(CliError::from(e)),
        };

        if prs.is_empty() {
            gctx.shell().note("No open pull requests found.");
            return Ok(());
        }

        for pr in prs {
            let changes = fetch_changes_count(&client, &owner, &repo, pr.number).unwrap_or(0);

            gctx.shell().note(format!(
                "PR #{} {}{}",
                pr.number,
                if pr.draft { "[DRAFT] " } else { "" },
                pr.title
            ));
            gctx.shell().note(format!(
                "  Assignees : {}",
                pr.assignees
                    .iter()
                    .map(|u| u.login.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
            gctx.shell().note(format!(
                "  Reviewers : {}",
                pr.requested_reviewers
                    .iter()
                    .map(|u| u.login.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
            gctx.shell()
                .note(format!("  Change requests pending: {}", changes));
            gctx.shell().note("------------------------------");
        }
    } else {
        return Err(CliError::from("no `git` subcommand provided"));
    }

    Ok(())
}

fn fetch_github_user(username: &str) -> Result<String, String> {
    let url = format!("https://api.github.com/users/{}", username);
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
    Ok(format!("Co-authored-by: {} <{}>", name, email))
}

/// Parse `git remote get-url <remote>` into (owner, repo)
fn parse_owner_repo(remote: &str) -> Result<(String, String), String> {
    let out = std::process::Command::new("git")
        .arg("remote")
        .arg("get-url")
        .arg(remote)
        .output()
        .map_err(|e| e.to_string())?;

    if !out.status.success() {
        return Err(format!("Failed to get URL for remote `{}`", remote));
    }

    let url = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let path = if let Some(stripped) = url.strip_prefix("git@github.com:") {
        stripped.to_string()
    } else if let Some(stripped) = url.strip_prefix("https://github.com/") {
        stripped.to_string()
    } else {
        return Err(format!("Unsupported remote URL format: {}", url));
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
    let url = format!(
        "https://api.github.com/repos/{}/{}/pulls?state=open",
        owner, repo
    );
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
    let url = format!(
        "https://api.github.com/repos/{}/{}/pulls/{}/reviews",
        owner, repo, pr_number
    );
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

#[derive(serde::Deserialize)]
struct PullRequest {
    number: u64,
    title: String,
    draft: bool,
    assignees: Vec<SimpleUser>,
    requested_reviewers: Vec<SimpleUser>,
}
