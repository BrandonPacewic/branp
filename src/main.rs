use std::ffi::OsString;

use clap::{Arg, ArgAction, ArgMatches, Command};

use crate::commands::branp_exec;
use crate::context::GlobalContext;
use crate::version::get_version_info;

mod commands;
mod context;
mod version;

fn main() {
    let args = branp().try_get_matches().unwrap();
    let (expanded_args, global_args) = expand_aliases(args, vec![]).unwrap();
    let mut gctx = match GlobalContext::default() {
        Some(gctx) => gctx,
        None => {
            panic!("Failed to create global context");
        }
    };

    let is_verbose = expanded_args.get_count("verbose") > 0;

    if expanded_args.get_flag("version") {
        let version = get_version_string(is_verbose);
        print!("{}", version);
    } else {
        let (cmd, subcommand_args) = match expanded_args.subcommand() {
            Some((cmd, args)) => (cmd, args),
            _ => {
                // No subcommand provided.
                let _ = branp().print_help();
                return;
            }
        };

        let exec = Exec::infer(cmd);
        configure_gctx(&mut gctx, &expanded_args, subcommand_args, global_args);
        exec.exec(&mut gctx, subcommand_args);
    }
}

enum Exec {
    Branp(commands::Exec),
}

impl Exec {
    fn infer(cmd: &str) -> Self {
        if let Some(exec) = commands::branp_exec(cmd) {
            Self::Branp(exec)
        } else {
            color_print::cprintln!(
                "<red,bold>error</>: <yellow>{}</> is not a valid subcommand",
                cmd
            );
            std::process::exit(1);
        }
    }

    fn exec(self, gctx: &mut context::GlobalContext, subcommand_args: &ArgMatches) {
        match self {
            Self::Branp(exec) => exec(gctx, subcommand_args),
        }
    }
}

const BUILTIN_ALIASES: [(&str, &str, &str); 2] =
    [("f", "format", "alias: format"), ("g", "gen", "alias: gen")];

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
    args: ArgMatches,
    mut already_expanded: Vec<String>,
) -> Option<(ArgMatches, GlobalArgs)> {
    if let Some((cmd, subcommand_args)) = args.subcommand() {
        let exec = branp_exec(cmd);
        let aliased_cmd = aliased_command(cmd);

        match (exec, aliased_cmd) {
            (Some(_), Some(_)) => {
                // User alias conflicts with built-in subcommand.
                println!(
                    "user-defined alias `{}` is ignored as it conflicts with a built-in subcommand",
                    cmd
                );
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
                    .unwrap();
                let Some(new_command) = new_args.subcommand_name() else {
                    panic!(
                        "subcommand is required, add a subcommand to the command alias `alias.{}",
                        cmd
                    );
                };

                already_expanded.push(cmd.to_string());
                if already_expanded.contains(&new_command.to_string()) {
                    panic!("subcommand alias `{}` is recursive", cmd);
                }

                let (expanded_args, _) = expand_aliases(new_args, already_expanded)?;
                return Some((expanded_args, global_args));
            }
        }
    }

    Some((args, GlobalArgs::default()))
}

fn configure_gctx(
    gctx: &mut GlobalContext,
    args: &ArgMatches,
    subcommand_args: &ArgMatches,
    global_args: GlobalArgs,
) {
    let quiet = args.get_flag("quiet") || subcommand_args.get_flag("quiet") || global_args.quiet;
    let verbose = global_args.verbose + args.get_count("verbose") as u32;

    gctx.configure(verbose, quiet);
}

fn get_version_string(is_verbose: bool) -> String {
    let version = get_version_info();
    let mut version_string = format!("branp {}", version);
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
    Command::new("branp")
        .allow_external_subcommands(true)
        .help_template(color_print::cstr!(
            "\
<green,bold>Usage:</> <cyan,bold>bp</> <cyan>[OPTIONS] [COMMAND]</>

<green,bold>Options:</>
{options}

<green,bold>Commands:</>
    <cyan,bold>format</>, <cyan,bold>f</>       Format the current working directory
    <cyan,bold>dbrun</>           Run a standalone C++ code file
    <cyan,bold>template-gen</>    Generate test case files for a given file name"
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
        .subcommands(commands::branp())
}
