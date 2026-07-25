use std::path::Path;

use crate::errors::CliError;

#[derive(Debug)]
pub struct PullRequest {
    pub head: String,
    pub number: u64,
    pub url: String,
}

pub fn open_pull_requests(repo: &Path) -> Result<Vec<PullRequest>, CliError> {
    let output = crate::utils::command::output(
        "gh",
        &[
            "pr",
            "list",
            "--state",
            "open",
            "--json",
            "number,headRefName,url",
        ],
        Some(repo),
    )?;

    let prs: Vec<GhPullRequest> =
        serde_json::from_str(&output).map_err(|e| CliError::from(e.to_string()))?;
    Ok(prs
        .into_iter()
        .map(|pr| PullRequest {
            head: pr.head_ref_name,
            number: pr.number,
            url: pr.url,
        })
        .collect())
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhPullRequest {
    number: u64,
    head_ref_name: String,
    url: String,
}
