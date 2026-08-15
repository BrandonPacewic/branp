use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use clap::{ArgMatches, Command};

use crate::context::GlobalContext;
use crate::errors::{CliError, CliResult};

pub fn cli() -> Command {
    Command::new("uninstall").about("Delete the running bp binary and shell completions").long_about(
        "\
Delete the running bp binary and installed shell completions from disk.

This command asks you to type yes before it removes anything.",
    )
}

pub fn exec(gctx: &mut GlobalContext, _args: &ArgMatches) -> CliResult {
    let exe = current_exe()?;
    let completion_paths = completion_paths(gctx.home());

    gctx.shell().note("This will permanently delete:");
    gctx.shell().note(format!("  {}", exe.display()));
    gctx.shell().note("  installed shell completions and zsh completion caches");
    gctx.shell().note("");

    let confirmation = prompt("Type `yes` to uninstall branp: ")?;
    if confirmation != "yes" {
        return Err(CliError::new("uninstall aborted", 1));
    }

    remove_completion_paths(gctx, &completion_paths)?;

    fs::remove_file(&exe).map_err(|e| CliError::new(format!("failed to delete {}: {e}", exe.display()), 1))?;

    gctx.shell().note(format!("Deleted bp binary at {}", exe.display()));

    Ok(())
}

fn current_exe() -> Result<PathBuf, CliError> {
    let exe = env::current_exe()?;

    if !exe.exists() {
        return Err(CliError::new(format!("current executable does not exist: {}", exe.display()), 1));
    }

    Ok(exe)
}

fn completion_paths(home: &PathBuf) -> Vec<PathBuf> {
    let mut paths = vec![
        home.join(".local/share/bash-completion/completions/bp"),
        home.join(".config/elvish/lib/bp-completions.elv"),
        home.join(".config/fish/completions/bp.fish"),
        home.join("Documents/PowerShell/Modules/bp/_bp.ps1"),
        home.join(".zsh/completions/_bp"),
        home.join(".oh-my-zsh/plugins/git/_bp"),
        home.join(".oh-my-zsh/completions/_bp"),
        home.join(".oh-my-zsh/custom/completions/_bp"),
        home.join(".oh-my-zsh/cache/completions/_bp"),
        home.join(".local/share/bp/completions/bp"),
        PathBuf::from("/opt/homebrew/share/zsh/site-functions/_bp"),
        PathBuf::from("/usr/local/share/zsh/site-functions/_bp"),
    ];

    paths.extend(zcompdump_paths(home));
    paths
}

fn zcompdump_paths(home: &PathBuf) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(home) else {
        return Vec::new();
    };

    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.file_name().and_then(|name| name.to_str()).is_some_and(|name| name.starts_with(".zcompdump")))
        .collect()
}

fn remove_completion_paths(gctx: &mut GlobalContext, paths: &[PathBuf]) -> CliResult {
    for path in paths {
        if !path.exists() {
            continue;
        }

        if path.is_dir() {
            gctx.shell().warn(format!("Skipping completion path because it is a directory: {}", path.display()));
            continue;
        }

        fs::remove_file(path).map_err(|e| CliError::new(format!("failed to delete completion {}: {e}", path.display()), 1))?;
    }

    Ok(())
}

fn prompt(message: impl std::fmt::Display) -> Result<String, CliError> {
    print!("{message}");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim_end_matches(['\r', '\n']).to_string())
}
