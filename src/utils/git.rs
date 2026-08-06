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

pub fn local_branch_merged_for_delete(repo: &Path, branch: &str) -> Result<bool, CliError> {
    let upstream = branch_upstream(repo, branch);
    let merge_target = upstream.as_deref().unwrap_or("HEAD");

    status(repo, &["merge-base", "--is-ancestor", branch, merge_target])
}

fn branch_upstream(repo: &Path, branch: &str) -> Option<String> {
    output(repo, &["rev-parse", "--abbrev-ref", &format!("{branch}@{{upstream}}")]).ok().and_then(|value| non_empty_trimmed(&value))
}

pub fn default_branch(repo: &Path, remote: &str) -> Result<String, CliError> {
    choose_default_branch(
        local_config_value(repo, "branp.worktree.defaultBranch"),
        live_remote_head_branch(repo, remote),
        current_branch(repo).ok().flatten(),
        cached_remote_head_branch(repo, remote),
        local_config_value(repo, "init.defaultBranch"),
    )
}

fn choose_default_branch(
    explicit_config: Option<String>,
    live_remote_head: Option<String>,
    current_branch: Option<String>,
    cached_remote_head: Option<String>,
    init_default_branch: Option<String>,
) -> Result<String, CliError> {
    explicit_config
        .and_then(|branch| non_empty_trimmed(&branch))
        .or_else(|| live_remote_head.and_then(|branch| non_empty_trimmed(&branch)))
        .or_else(|| current_branch.and_then(|branch| non_empty_trimmed(&branch)))
        .or_else(|| cached_remote_head.and_then(|branch| non_empty_trimmed(&branch)))
        .or_else(|| init_default_branch.and_then(|branch| non_empty_trimmed(&branch)))
        .ok_or_else(|| {
            CliError::from(
                "could not determine default branch from branp.worktree.defaultBranch, origin HEAD, the current branch, origin/HEAD, or init.defaultBranch",
            )
        })
}

fn local_config_value(repo: &Path, key: &str) -> Option<String> {
    output(repo, &["config", "--local", "--get", key]).ok().and_then(|value| non_empty_trimmed(&value))
}

fn live_remote_head_branch(repo: &Path, remote: &str) -> Option<String> {
    output(repo, &["ls-remote", "--symref", remote, "HEAD"]).ok().and_then(|output| parse_remote_head(&output))
}

fn parse_remote_head(output: &str) -> Option<String> {
    output
        .lines()
        .find_map(|line| line.strip_prefix("ref: refs/heads/").and_then(|line| line.strip_suffix("\tHEAD")).map(str::to_string))
        .and_then(|branch| (!branch.is_empty()).then_some(branch))
}

fn cached_remote_head_branch(repo: &Path, remote: &str) -> Option<String> {
    output(repo, &["symbolic-ref", "--short", &format!("refs/remotes/{remote}/HEAD")])
        .ok()
        .and_then(|value| value.trim().strip_prefix(&format!("{remote}/")).map(str::to_string))
        .and_then(|branch| (!branch.is_empty()).then_some(branch))
}

fn non_empty_trimmed(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chooses_default_branch_from_best_available_signal() {
        assert_eq!(
            choose_default_branch(
                Some("configured".to_string()),
                Some("remote".to_string()),
                Some("base".to_string()),
                Some("cached".to_string()),
                Some("init".to_string()),
            )
            .unwrap(),
            "configured"
        );
        assert_eq!(
            choose_default_branch(None, Some("remote".to_string()), Some("base".to_string()), Some("cached".to_string()), Some("init".to_string()))
                .unwrap(),
            "remote"
        );
        assert_eq!(
            choose_default_branch(None, None, Some("base".to_string()), Some("cached".to_string()), Some("init".to_string())).unwrap(),
            "base"
        );
        assert_eq!(choose_default_branch(None, None, None, Some("cached".to_string()), Some("init".to_string())).unwrap(), "cached");
        assert_eq!(choose_default_branch(None, None, None, None, Some("init".to_string())).unwrap(), "init");
        assert!(choose_default_branch(None, None, None, None, None).is_err());
    }

    #[test]
    fn current_branch_beats_stale_cached_remote_head() {
        assert_eq!(choose_default_branch(None, None, Some("dev".to_string()), Some("master".to_string()), None).unwrap(), "dev");
    }

    #[test]
    fn parses_live_remote_head() {
        let output = "ref: refs/heads/dev\tHEAD\nf7dbd99e3616315c360db815f2a09e03d38ac669\tHEAD\n";
        assert_eq!(parse_remote_head(output).as_deref(), Some("dev"));
        assert_eq!(parse_remote_head("f7dbd99e3616315c360db815f2a09e03d38ac669\tHEAD\n"), None);
    }
}
