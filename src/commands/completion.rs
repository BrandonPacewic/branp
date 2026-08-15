use std::fs;
use std::path::{Path, PathBuf};

use clap::{Arg, ArgAction, ArgMatches, Command};
use clap_complete::Shell;

use crate::completions::completion_script;
use crate::context::GlobalContext;
use crate::errors::{CliError, CliResult};

pub fn cli() -> Command {
    Command::new("completion")
        .about("Generate shell completion scripts")
        .arg(Arg::new("shell").help("Shell to generate completions for").required(true).value_parser(clap::value_parser!(Shell)))
        .arg(Arg::new("print").long("print").help("Print the completion script instead of installing it").action(ArgAction::SetTrue))
}

pub fn exec(gctx: &mut GlobalContext, args: &ArgMatches) -> CliResult {
    let shell = args.get_one::<Shell>("shell").expect("required by clap").to_owned();

    if args.get_flag("print") {
        print!("{}", completion_script(shell)?);
        return Ok(());
    }

    let path = completion_path(gctx.home(), shell);
    let Some(parent) = path.parent() else {
        return Err(CliError::from("completion path does not have a parent directory"));
    };

    fs::create_dir_all(parent)?;

    fs::write(&path, completion_script(shell)?)?;
    if shell == Shell::Zsh {
        remove_zsh_completion_caches(gctx.home())?;
    }

    gctx.shell().note(format!("Installed completion script to {}", path.display()));
    print_shell_reload_hint(gctx, shell);

    Ok(())
}

fn completion_path(home: &Path, shell: Shell) -> PathBuf {
    match shell {
        Shell::Bash => home.join(".local/share/bash-completion/completions/bp"),
        Shell::Elvish => home.join(".config/elvish/lib/bp-completions.elv"),
        Shell::Fish => home.join(".config/fish/completions/bp.fish"),
        Shell::PowerShell => home.join("Documents/PowerShell/Modules/bp/_bp.ps1"),
        Shell::Zsh => zsh_completion_path(home),
        _ => home.join(".local/share/bp/completions/bp"),
    }
}

fn zsh_completion_path(home: &Path) -> PathBuf {
    let oh_my_zsh = home.join(".oh-my-zsh");
    if oh_my_zsh.exists() {
        return oh_my_zsh.join("custom/completions/_bp");
    }

    home.join(".zsh/completions/_bp")
}

fn remove_zsh_completion_caches(home: &Path) -> CliResult {
    let entries = match fs::read_dir(home) {
        Ok(entries) => entries,
        Err(_) => return Ok(()),
    };

    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let is_zcompdump = path.file_name().and_then(|name| name.to_str()).is_some_and(|name| name.starts_with(".zcompdump"));

        if is_zcompdump && path.is_file() {
            fs::remove_file(&path)?;
        }
    }

    Ok(())
}

fn print_shell_reload_hint(gctx: &mut GlobalContext, shell: Shell) {
    let hint = match shell {
        Shell::Bash => "Restart your shell, or source the installed completion file.",
        Shell::Elvish => "Restart elvish, or add `use bp-completions` to ~/.config/elvish/rc.elv.",
        Shell::Fish => "Restart fish, or run `exec fish`.",
        Shell::PowerShell => "Source the installed _bp.ps1 file from your PowerShell profile.",
        Shell::Zsh => "Restart zsh, or run `autoload -Uz compinit && compinit`.",
        _ => "Restart your shell after wiring the installed completion file into your shell config.",
    };

    gctx.shell().note(hint);
}
