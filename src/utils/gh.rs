use std::path::Path;
use std::process::Command as ProcessCommand;
use std::process::Output;

use crate::errors::CliError;

#[derive(Clone, Debug)]
pub struct PullRequest {
    pub head: String,
    pub number: u64,
    pub url: String,
}

pub fn open_pull_requests(repo: &Path) -> Result<Vec<PullRequest>, CliError> {
    let output = output(repo, &["pr", "list", "--state", "open", "--json", "number,headRefName,url"])?;

    let prs: Vec<GhPullRequest> = serde_json::from_str(&output).map_err(|e| CliError::from(e.to_string()))?;
    Ok(prs.into_iter().map(|pr| PullRequest { head: pr.head_ref_name, number: pr.number, url: pr.url }).collect())
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhPullRequest {
    number: u64,
    head_ref_name: String,
    url: String,
}

pub fn output(repo: &Path, args: &[&str]) -> Result<String, CliError> {
    crate::utils::command::output("gh", args, Some(repo))
}

pub fn output_allowing_exit_codes(repo: &Path, args: &[&str], allowed_codes: &[i32]) -> Result<Output, CliError> {
    let output = ProcessCommand::new("gh").args(args).current_dir(repo).output()?;

    if output.status.success() || output.status.code().is_some_and(|code| allowed_codes.contains(&code)) {
        Ok(output)
    } else {
        Err(command_output_error("gh", args, &output))
    }
}

fn command_output_error(program: &str, args: &[&str], output: &Output) -> CliError {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let detail = if !stderr.trim().is_empty() { stderr.trim() } else { stdout.trim() };

    CliError::from(format!("{} {} failed{}{}", program, args.join(" "), if detail.is_empty() { "" } else { ": " }, detail))
}
