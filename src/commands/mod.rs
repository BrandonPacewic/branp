use clap::{ArgMatches, Command};

use crate::context::GlobalContext;
use crate::errors::CliResult;

pub struct BuiltinAlias {
    pub alias: &'static str,
    pub command: &'static str,
}

/// Table for defining the aliases which come built into `bp`.
///
/// Keep this as the single source for built-in top-level subcommand aliases so
/// alias expansion and clap registration cannot drift apart.
pub const BUILTIN_ALIASES: &[BuiltinAlias] = &[
    BuiltinAlias { alias: "f", command: "format" },
    BuiltinAlias { alias: "g", command: "git" },
    BuiltinAlias { alias: "r", command: "dbrun" },
    BuiltinAlias { alias: "wt", command: "worktree" },
];

pub fn builtin() -> Vec<Command> {
    vec![
        with_builtin_aliases(cloc::cli()),
        with_builtin_aliases(completion::cli()),
        with_builtin_aliases(doctor::cli()),
        with_builtin_aliases(dbrun::cli()),
        with_builtin_aliases(sample_gen::cli()),
        with_builtin_aliases(test_samples::cli()),
        with_builtin_aliases(format::cli()),
        with_builtin_aliases(gen::cli()),
        with_builtin_aliases(git::cli()),
        with_builtin_aliases(worktree::cli()),
        with_builtin_aliases(uninstall::cli()),
    ]
}

fn with_builtin_aliases(mut command: Command) -> Command {
    let name = command.get_name().to_string();
    for alias in BUILTIN_ALIASES.iter().filter(|alias| alias.command == name) {
        command = command.visible_alias(alias.alias);
    }
    command
}

pub type Exec = fn(&mut GlobalContext, &ArgMatches) -> CliResult;

pub fn builtin_exec(cmd: &str) -> Option<Exec> {
    let exec = match cmd {
        "cloc" => cloc::exec,
        "completion" => completion::exec,
        "doctor" => doctor::exec,
        "dbrun" => dbrun::exec,
        "sample-gen" => sample_gen::exec,
        "test-samples" => test_samples::exec,
        "format" => format::exec,
        "gen" => gen::exec,
        "git" => git::exec,
        "worktree" => worktree::exec,
        "uninstall" => uninstall::exec,
        _ => return None,
    };

    Some(exec)
}

pub fn builtin_alias(cmd: &str) -> Option<&'static BuiltinAlias> {
    BUILTIN_ALIASES.iter().find(|alias| alias.alias == cmd)
}

pub fn aliased_command(command: &str) -> Option<Vec<String>> {
    builtin_alias(command).map(|alias| vec![alias.command.to_string()])
}

pub mod cloc;
pub mod completion;
pub mod dbrun;
pub mod doctor;
pub mod format;
pub mod gen;
pub mod git;
pub mod sample_gen;
pub mod test_samples;
pub mod uninstall;
pub mod worktree;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_aliases_drive_alias_expansion() {
        assert_eq!(aliased_command("f"), Some(vec!["format".to_string()]));
        assert_eq!(aliased_command("g"), Some(vec!["git".to_string()]));
        assert_eq!(aliased_command("r"), Some(vec!["dbrun".to_string()]));
        assert_eq!(aliased_command("wt"), Some(vec!["worktree".to_string()]));
    }

    #[test]
    fn builtin_aliases_are_registered_with_clap_commands() {
        let command = builtin().into_iter().find(|command| command.get_name() == "worktree").expect("worktree command");

        assert!(command.get_all_aliases().any(|alias| alias == "wt"));
    }

    #[test]
    fn builtin_commands_have_exec_handlers() {
        for command in builtin() {
            assert!(builtin_exec(command.get_name()).is_some(), "{} is missing an exec handler", command.get_name());
        }
    }

    #[test]
    fn builtin_aliases_target_registered_commands() {
        let commands = builtin().into_iter().map(|command| command.get_name().to_string()).collect::<Vec<_>>();

        for alias in BUILTIN_ALIASES {
            assert!(commands.iter().any(|command| command == alias.command), "{} aliases unknown command {}", alias.alias, alias.command);
        }
    }
}
