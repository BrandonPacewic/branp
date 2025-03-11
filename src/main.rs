use std::ffi::OsString;

use clap::{Arg, ArgAction, ArgMatches, Command};

use crate::commands::branp_exec;
use crate::context::GlobalContext;

mod commands;
mod context;

fn main() {
    let args = branp().try_get_matches().unwrap();
    let (expanded_args, global_args) = expand_aliases(args, vec![]).unwrap();
    let mut gctx = match GlobalContext::default() {
        Some(gctx) => gctx,
        None => {
            panic!("Failed to create global context");
        }
    };

    if expanded_args.get_flag("version") {
        println!("Branp version: {}", env!("CARGO_PKG_VERSION"));
    } else {
        let (cmd, subcommand_args) = match expanded_args.subcommand() {
            Some((cmd, args)) => (cmd, args),
            _ => {
                // No subcommand provided.
                let _ = branp().print_help();
                return;
            }
        };

        let exec = Exec::infer(cmd).expect("");
        configure_gctx(&mut gctx, &expanded_args, subcommand_args, global_args);
        exec.exec(&mut gctx, subcommand_args);
    }
}

enum Exec {
    Branp(commands::Exec),
}

impl Exec {
    fn infer(cmd: &str) -> Option<Self> {
        commands::branp_exec(cmd).map(Self::Branp)
    }

    fn exec(self, gctx: &mut context::GlobalContext, subcommand_args: &ArgMatches) {
        match self {
            Self::Branp(exec) => exec(gctx, subcommand_args),
        }
    }
}

const BUILTIN_ALIASES: [(&str, &str, &str); 1] = [("f", "format", "alias: format")];

fn builtin_aliases_execs(cmd: &str) -> Option<&(&str, &str, &str)> {
    BUILTIN_ALIASES.iter().find(|alias| alias.0 == cmd)
}

fn aliased_command(command: &str) -> Option<Vec<String>> {
    builtin_aliases_execs(command).map(|alias| vec![alias.1.to_string()])
}

#[derive(Default)]
struct GlobalArgs {
    quiet: bool,
}

impl GlobalArgs {
    fn new(args: &ArgMatches) -> Self {
        Self {
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

    gctx.configure(quiet);
}

fn branp() -> Command {
    Command::new("branp")
        .allow_external_subcommands(true)
        .help_template(color_print::cstr!(
            "\
<green,bold>Usage:</> <cyan,bold>bp</> <cyan>[OPTIONS] [COMMAND]</>

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
            Arg::new("quiet")
                .short('q')
                .long("quiet")
                .help("Do not print log messages or output")
                .action(ArgAction::SetTrue)
                .global(true),
        )
        .subcommands(commands::branp())
}
