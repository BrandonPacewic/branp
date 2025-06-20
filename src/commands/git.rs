//! `git` subcommands.
//!
//! This module provides Git-related helper commands that integrate with GitHub.
//!

use crate::context::GlobalContext;
use clap::{Arg, ArgMatches, Command};
use serde::Deserialize;

pub fn command() -> Command {
    Command::new("git").about("Git-related helpers").subcommand(
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
}

pub fn exec(gctx: &mut GlobalContext, args: &ArgMatches) {
    if let Some(sub) = args.subcommand_matches("coauthor") {
        for user in sub.get_many::<String>("usernames").unwrap() {
            match fetch_github_user(user) {
                Ok(line) => gctx.shell().note(line),
                Err(e) => gctx.shell().error(format!("{}: {}", user, e)),
            }
        }
    } else {
        gctx.shell().error("No `git` subcommand provided");
    }
}

#[derive(Deserialize)]
struct User {
    id: u64,
    login: String,
    name: Option<String>,
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
