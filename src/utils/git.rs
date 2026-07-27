use std::collections::HashSet;
use std::path::Path;

use crate::errors::{CliError, CliResult};

#[derive(Clone, Debug)]
pub struct GitHubRepo {
    pub owner: String,
    pub name: String,
}

pub fn run(repo: &Path, args: &[&str]) -> CliResult {
    crate::utils::command::run("git", args, Some(repo))
}

pub fn output(repo: &Path, args: &[&str]) -> Result<String, CliError> {
    crate::utils::command::output("git", args, Some(repo))
}

pub fn status(repo: &Path, args: &[&str]) -> Result<bool, CliError> {
    crate::utils::command::status("git", args, Some(repo))
}

pub fn current_branch(repo: &Path) -> Result<Option<String>, CliError> {
    let output = output(repo, &["branch", "--show-current"])?;
    let branch = output.trim();
    Ok((!branch.is_empty()).then(|| branch.to_string()))
}

pub fn local_branch_exists(repo: &Path, branch: &str) -> Result<bool, CliError> {
    status(repo, &["show-ref", "--verify", "--quiet", &format!("refs/heads/{branch}")])
}

pub fn remote_branch_exists(repo: &Path, remote: &str, branch: &str) -> Result<bool, CliError> {
    status(repo, &["show-ref", "--quiet", &format!("refs/remotes/{remote}/{branch}")])
}

pub fn remote_branches(repo: &Path, remote: &str) -> Result<HashSet<String>, CliError> {
    Ok(output(repo, &["for-each-ref", "--format=%(refname:strip=3)", &format!("refs/remotes/{remote}")])?
        .lines()
        .filter(|branch| !branch.is_empty() && *branch != "HEAD")
        .map(str::to_string)
        .collect())
}

pub fn delete_local_branch(repo: &Path, branch: &str, force: bool) -> CliResult {
    if !local_branch_exists(repo, branch)? {
        return Ok(());
    }

    let flag = if force { "-D" } else { "-d" };
    run(repo, &["branch", flag, branch])
}

pub fn remote_url(repo: &Path, remote: &str) -> Result<Option<String>, CliError> {
    match output(repo, &["remote", "get-url", remote]) {
        Ok(url) => Ok(Some(url.trim().to_string())),
        Err(_) => Ok(None),
    }
}

pub fn github_remote(repo: &Path, remote: &str) -> Result<GitHubRepo, CliError> {
    let url = remote_url(repo, remote)?.ok_or_else(|| CliError::from(format!("Failed to get URL for remote `{remote}`")))?;
    parse_github_url(&url).ok_or_else(|| CliError::from(format!("Unsupported remote URL format: {url}")))
}

fn parse_github_url(url: &str) -> Option<GitHubRepo> {
    let url = url.trim();
    let path = url.strip_prefix("git@github.com:").or_else(|| url.strip_prefix("https://github.com/"))?;
    let path = path.trim_end_matches(".git");
    let (owner, name) = path.split_once('/')?;

    Some(GitHubRepo { owner: owner.to_string(), name: name.to_string() })
}
