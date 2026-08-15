use std::ffi::OsString;

use branp_cli::context::GlobalContext;
use branp_cli::errors::{CliError, CliResult};
use branp_cli::version::get_version_info;
use clap::{Arg, ArgAction, ArgMatches, Command};
use clap_complete::CompleteEnv;

pub use branp_cli::{context, doctor, errors, ops, utils};

mod commands;
mod completions;

fn main() {
    let mut gctx = match GlobalContext::from_env() {
        Ok(gctx) => gctx,
        Err(e) => {
            let mut shell = context::Shell::new();
            shell.error(e.message);
            std::process::exit(e.code);
        }
    };

    let args = std::env::args_os();
    let current_dir = std::env::current_dir().ok();
    if CompleteEnv::with_factory(branp).var("BP_COMPLETE").try_complete(args, current_dir.as_deref()).unwrap_or_else(|e| e.exit()) {
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

        let exec = Exec::infer(cmd)?;
        configure_gctx(gctx, &expanded_args, subcommand_args, global_args)?;
        exec.exec(gctx, subcommand_args)?;
    }

    Ok(())
}

enum Exec {
    Builtin(commands::Exec),
}

impl Exec {
    fn infer(cmd: &str) -> Result<Self, CliError> {
        if let Some(exec) = commands::builtin_exec(cmd) {
            Ok(Self::Builtin(exec))
        } else {
            Err(CliError::new(format!("`{cmd}` is not a valid subcommand"), 1))
        }
    }

    fn exec(self, gctx: &mut context::GlobalContext, subcommand_args: &ArgMatches) -> CliResult {
        match self {
            Self::Builtin(exec) => exec(gctx, subcommand_args),
        }
    }
}

#[derive(Default)]
struct GlobalArgs {
    verbose: u32,
    quiet: bool,
}

impl GlobalArgs {
    fn new(args: &ArgMatches) -> Self {
        Self { verbose: args.get_count("verbose") as u32, quiet: args.get_flag("quiet") }
    }
}

fn expand_aliases(gctx: &mut GlobalContext, args: ArgMatches, mut already_expanded: Vec<String>) -> Result<(ArgMatches, GlobalArgs), CliError> {
    if let Some((cmd, subcommand_args)) = args.subcommand() {
        let exec = commands::builtin_exec(cmd);
        let aliased_cmd = commands::aliased_command(cmd);

        match (exec, aliased_cmd) {
            (Some(_), Some(_)) => {
                // User alias conflicts with built-in subcommand.
                gctx.shell().warn(format!("user-defined alias `{cmd}` is ignored as it conflicts with a built-in subcommand"));
            }
            (Some(_), None) => {} // Found a subcommand with no overlapping alias, do nothing.
            // Not a subcommand and not an alias, an error but not for the responsibility of this function.
            (None, None) => {}
            (None, Some(alias)) => {
                let mut alias = alias.into_iter().map(OsString::from).collect::<Vec<_>>();
                alias.extend(subcommand_args.get_many::<OsString>("").unwrap_or_default().cloned());

                // `new_args` captures everything after the subcommand and drops the rest, so
                // we need to get the global args beforehand.
                // Note that any alias to an external subcommand will not receive these arguments.
                let global_args = GlobalArgs::new(subcommand_args);
                let new_args = branp().no_binary_name(true).try_get_matches_from(alias).unwrap_or_else(|e| e.exit());
                let Some(new_command) = new_args.subcommand_name() else {
                    return Err(CliError::new(format!("alias `{cmd}` must resolve to a subcommand"), 1));
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

fn configure_gctx(gctx: &mut GlobalContext, args: &ArgMatches, subcommand_args: &ArgMatches, global_args: GlobalArgs) -> CliResult {
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
        .disable_help_subcommand(true)
        .help_template(color_print::cstr!(
            "\
<green,bold>Usage:</> <cyan,bold>bp</> <cyan>[OPTIONS] [COMMAND]</>

<green,bold>Options:</>
{options}

<green,bold>Commands:</>
{subcommands}"
        ))
        .arg(Arg::new("version").short('V').long("version").help("Print version info and exit").action(ArgAction::SetTrue))
        .arg(Arg::new("verbose").short('v').long("verbose").help("Use verbose output (-vv very verbose)").action(ArgAction::Count).global(true))
        .arg(Arg::new("quiet").short('q').long("quiet").help("Do not print log messages or output").action(ArgAction::SetTrue).global(true))
        .subcommands(commands::builtin())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_level_help_uses_registered_command_metadata() {
        let help = branp().render_help().to_string();

        assert!(help.contains("Format the current working directory [alias: f]"));
        assert!(help.contains("Manage sibling Git worktrees [alias: wt]"));
        assert!(!help.contains("Format a C++ file using clang-format."));
    }

    #[test]
    fn cli_definition_is_valid() {
        branp().debug_assert();
    }
}
