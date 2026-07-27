use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use clap::{Arg, ArgAction, ArgMatches, Command};
use clap_complete::{CompleteEnv, Shell};

use crate::commands::branp_exec;
use crate::completions::completion_script;
use crate::context::GlobalContext;
use crate::errors::{CliError, CliResult};
use crate::version::get_version_info;

mod commands;
mod completions;
mod context;
mod doctor;
mod errors;
mod ops;
mod utils;
mod version;

fn main() {
    let mut gctx = match GlobalContext::default() {
        Ok(gctx) => gctx,
        Err(e) => {
            let mut shell = context::Shell::new();
            shell.error(e.message);
            std::process::exit(e.code);
        }
    };

    let args = std::env::args_os();
    let current_dir = std::env::current_dir().ok();
    if CompleteEnv::with_factory(branp)
        .var("BP_COMPLETE")
        .try_complete(args, current_dir.as_deref())
        .unwrap_or_else(|e| e.exit())
    {
        return;
    }

    if let Err(e) = run(&mut gctx) {
        gctx.shell().error(e.message);
        std::process::exit(e.code);
    }
}

fn run(gctx: &mut GlobalContext) -> CliResult {
    let args = branp().try_get_matches().unwrap_or_else(|e| e.exit());

    let (expanded_args, global_args) = expand_aliases(gctx, args, vec![])?;
    let is_verbose = expanded_args.get_count("verbose") > 0;

    if expanded_args.get_flag("version") {
        let version = get_version_string(is_verbose);
        print!("{version}");
    } else {
        let (cmd, subcommand_args) = match expanded_args.subcommand() {
            Some((cmd, args)) => (cmd, args),
            _ => {
                // No subcommand provided.
                let _ = branp().print_help();
                return Ok(());
            }
        };

        if cmd == "completion" {
            configure_gctx(gctx, &expanded_args, subcommand_args, global_args)?;
            install_completion(gctx, subcommand_args)?;
        } else {
            let exec = Exec::infer(cmd)?;
            configure_gctx(gctx, &expanded_args, subcommand_args, global_args)?;
            exec.exec(gctx, subcommand_args)?;
        }
    }

    Ok(())
}

enum Exec {
    Branp(commands::Exec),
}

impl Exec {
    fn infer(cmd: &str) -> Result<Self, CliError> {
        if let Some(exec) = commands::branp_exec(cmd) {
            Ok(Self::Branp(exec))
        } else {
            Err(CliError::new(
                format!("`{cmd}` is not a valid subcommand"),
                1,
            ))
        }
    }

    fn exec(self, gctx: &mut context::GlobalContext, subcommand_args: &ArgMatches) -> CliResult {
        match self {
            Self::Branp(exec) => exec(gctx, subcommand_args),
        }
    }
}

const BUILTIN_ALIASES: [(&str, &str, &str); 3] = [
    ("f", "format", "alias: format"),
    ("g", "git", "alias: git"),
    ("r", "dbrun", "alias: dbrun"),
];

fn builtin_aliases_execs(cmd: &str) -> Option<&(&str, &str, &str)> {
    BUILTIN_ALIASES.iter().find(|alias| alias.0 == cmd)
}

fn aliased_command(command: &str) -> Option<Vec<String>> {
    builtin_aliases_execs(command).map(|alias| vec![alias.1.to_string()])
}

#[derive(Default)]
struct GlobalArgs {
    verbose: u32,
    quiet: bool,
}

impl GlobalArgs {
    fn new(args: &ArgMatches) -> Self {
        Self {
            verbose: args.get_count("verbose") as u32,
            quiet: args.get_flag("quiet"),
        }
    }
}

fn expand_aliases(
    gctx: &mut GlobalContext,
    args: ArgMatches,
    mut already_expanded: Vec<String>,
) -> Result<(ArgMatches, GlobalArgs), CliError> {
    if let Some((cmd, subcommand_args)) = args.subcommand() {
        let exec = branp_exec(cmd);
        let aliased_cmd = aliased_command(cmd);

        match (exec, aliased_cmd) {
            (Some(_), Some(_)) => {
                // User alias conflicts with built-in subcommand.
                gctx.shell().warn(format!(
                    "user-defined alias `{cmd}` is ignored as it conflicts with a built-in subcommand"
                ));
            }
            (Some(_), None) => {} // Found a subcommand with no overlapping alias, do nothing.
            // Not a subcommand and not an alias, an error but not for the responsibility of this function.
            (None, None) => {}
            (None, Some(alias)) => {
                let mut alias = alias.into_iter().map(OsString::from).collect::<Vec<_>>();
                alias.extend(
                    subcommand_args
                        .get_many::<OsString>("")
                        .unwrap_or_default()
                        .cloned(),
                );

                // `new_args` captures everything after the subcommand and drops the rest, so
                // we need to get the global args beforehand.
                // Note that any alias to an external subcommand will not receive these arguments.
                let global_args = GlobalArgs::new(subcommand_args);
                let new_args = branp()
                    .no_binary_name(true)
                    .try_get_matches_from(alias)
                    .unwrap_or_else(|e| e.exit());
                let Some(new_command) = new_args.subcommand_name() else {
                    return Err(CliError::new(
                        format!("alias `{cmd}` must resolve to a subcommand"),
                        1,
                    ));
                };

                already_expanded.push(cmd.to_string());
                if already_expanded.contains(&new_command.to_string()) {
                    return Err(CliError::new(format!("alias `{cmd}` is recursive"), 1));
                }

                let (expanded_args, _) = expand_aliases(gctx, new_args, already_expanded)?;
                return Ok((expanded_args, global_args));
            }
        }
    }

    Ok((args, GlobalArgs::default()))
}

fn configure_gctx(
    gctx: &mut GlobalContext,
    args: &ArgMatches,
    subcommand_args: &ArgMatches,
    global_args: GlobalArgs,
) -> CliResult {
    let quiet = args.get_flag("quiet") || subcommand_args.get_flag("quiet") || global_args.quiet;
    let verbose = global_args.verbose + args.get_count("verbose") as u32;

    gctx.configure(verbose, quiet)
}

fn get_version_string(is_verbose: bool) -> String {
    let version = get_version_info();
    let mut version_string = format!("branp {version}");
    if is_verbose {
        version_string.push_str(" (rust)\n");

        if let Some(ref ci) = version.commit_info {
            version_string.push_str(&format!("commit-hash: {}\n", ci.commit_hash));
            version_string.push_str(&format!("short-hash:  {}\n", ci.short_commit_hash));
            version_string.push_str(&format!("commit-date: {}\n", ci.commit_date));
        } else {
            version_string.push_str("commit-info: unknown\n");
        }
    } else {
        version_string.push('\n');
    }

    version_string
}

fn branp() -> Command {
    Command::new("bp")
        .allow_external_subcommands(true)
        .help_template(color_print::cstr!(
            "\
<green,bold>Usage:</> <cyan,bold>bp</> <cyan>[OPTIONS] [COMMAND]</>

<green,bold>Options:</>
{options}

<green,bold>Commands:</>
    <cyan,bold>doctor</>          Diagnose and repair local CLI integration issues
    <cyan,bold>cloc</>            Count lines in files quickly
    <cyan,bold>format</>, <cyan,bold>f</>       Format the current working directory
    <cyan,bold>git</>, <cyan,bold>g</>          Git-related commands
    <cyan,bold>worktree</>, <cyan,bold>wt</>    Manage sibling Git worktrees
    <cyan,bold>dbrun</>, <cyan,bold>r</>        Run a standalone C++ code file
    <cyan,bold>sample-gen</>      Generate sample input/output files for a C++ file
    <cyan,bold>test-samples</>    Compile a C++ file and run it against sample input/output
    <cyan,bold>gen</>             Generate template file(s) from the config templates dir
    <cyan,bold>uninstall</>       Delete the running bp binary and shell completions
    <cyan,bold>completion</>      Generate shell completion scripts"
        ))
        .arg(
            Arg::new("version")
                .short('V')
                .long("version")
                .help("Print version info and exit")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Use verbose output (-vv very verbose)")
                .action(ArgAction::Count)
                .global(true),
        )
        .arg(
            Arg::new("quiet")
                .short('q')
                .long("quiet")
                .help("Do not print log messages or output")
                .action(ArgAction::SetTrue)
                .global(true),
        )
        .subcommand(
            Command::new("completion")
                .about("Generate shell completion scripts")
                .arg(
                    Arg::new("shell")
                        .help("Shell to generate completions for")
                        .required(true)
                        .value_parser(clap::value_parser!(Shell)),
                )
                .arg(
                    Arg::new("print")
                        .long("print")
                        .help("Print the completion script instead of installing it")
                        .action(ArgAction::SetTrue),
                ),
        )
        .subcommands(commands::branp())
}

fn install_completion(gctx: &mut GlobalContext, args: &ArgMatches) -> CliResult {
    let shell = args
        .get_one::<Shell>("shell")
        .expect("required by clap")
        .to_owned();

    if args.get_flag("print") {
        print!("{}", completion_script(shell)?);
        return Ok(());
    }

    let path = completion_path(gctx.home(), shell);
    let Some(parent) = path.parent() else {
        return Err(CliError::from(
            "completion path does not have a parent directory",
        ));
    };

    fs::create_dir_all(parent)?;

    fs::write(&path, completion_script(shell)?)?;
    if shell == Shell::Zsh {
        remove_zsh_completion_caches(gctx.home())?;
    }

    gctx.shell()
        .note(format!("Installed completion script to {}", path.display()));
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
        let is_zcompdump = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(".zcompdump"));

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
        _ => {
            "Restart your shell after wiring the installed completion file into your shell config."
        }
    };

    gctx.shell().note(hint);
}
