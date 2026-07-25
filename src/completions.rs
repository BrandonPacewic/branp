use std::process::Command as ProcessCommand;

use clap::{Arg, Command};
use clap_complete::Shell;

use crate::errors::CliError;

pub fn completion_script(shell: Shell) -> Result<String, CliError> {
    let exe = std::env::current_exe()?;
    let shell_name = shell.to_string();
    let output = ProcessCommand::new(exe)
        .env("BP_COMPLETE", &shell_name)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(CliError::from(format!(
            "failed to generate {shell_name} completion script: {stderr}"
        )));
    }

    let mut script = String::from_utf8(output.stdout)
        .map_err(|e| CliError::from(format!("completion script was not valid UTF-8: {e}")))?;

    if shell == Shell::Zsh {
        script.push_str(&zsh_placeholder_widget());
    }

    Ok(script)
}

#[derive(Debug)]
struct PlaceholderCommand {
    path: Vec<String>,
    placeholders: Vec<String>,
    value_options: Vec<String>,
}

fn zsh_placeholder_widget() -> String {
    let commands = placeholder_commands();
    if commands.is_empty() {
        return String::new();
    }

    format!(
        r#"

# Insert selected positional placeholders before falling back to clap's dynamic completer.
function _bp_placeholder_or_complete() {{
    emulate -L zsh

    local -a bp_words
    bp_words=("${{(z)LBUFFER}}")

    if (( ${{#bp_words[@]}} < 2 )); then
        zle complete-word
        return
    fi

    if [[ "$LBUFFER" != *[[:space:]] && "${{bp_words[-1]}}" == -* ]]; then
        zle complete-word
        return
    fi

    local best_len=0
    local placeholders=""
    local value_options=""
    local candidate
    for candidate in {cases}; do
        local candidate_path="${{candidate%%:*}}"
        local rest="${{candidate#*:}}"
        local candidate_placeholders="${{rest%%:*}}"
        local candidate_value_options="${{rest#*:}}"
        local -a path_words
        path_words=("${{(s: :)candidate_path}}")

        if (( ${{#bp_words[@]}} < ${{#path_words[@]}} )); then
            continue
        fi

        local bp_index=0
        local matches=1
        for (( bp_index = 1; bp_index <= ${{#path_words[@]}}; bp_index++ )); do
            if [[ "${{bp_words[$bp_index]}}" != "${{path_words[$bp_index]}}" ]]; then
                matches=0
                break
            fi
        done

        if (( matches && ${{#path_words[@]}} > best_len )); then
            best_len=${{#path_words[@]}}
            placeholders="$candidate_placeholders"
            value_options="$candidate_value_options"
        fi
    done

    if (( best_len == 0 )); then
        zle complete-word
        return
    fi

    local positional_count=0
    local skip_next=0
    local bp_index=0
    local token=""
    for (( bp_index = best_len + 1; bp_index <= ${{#bp_words[@]}}; bp_index++ )); do
        token="${{bp_words[$bp_index]}}"

        if (( skip_next )); then
            skip_next=0
            continue
        fi

        if [[ "$token" == --*=* ]]; then
            continue
        fi

        if [[ "$token" == -* ]]; then
            if [[ " $value_options " == *" $token "* ]]; then
                skip_next=1
            fi
            continue
        fi

        (( positional_count++ ))
    done

    local -a placeholder_words
    placeholder_words=("${{(s: :)placeholders}}")
    if (( positional_count >= ${{#placeholder_words[@]}} )); then
        zle complete-word
        return
    fi

    local placeholder="${{placeholder_words[$(( positional_count + 1 ))]}}"
    local prefix=""
    if [[ "$LBUFFER" != *[[:space:]] ]]; then
        prefix=" "
    fi

    local mark=$(( ${{#LBUFFER}} + ${{#prefix}} ))
    LBUFFER+="${{prefix}}${{placeholder}}"
    MARK=$mark
    CURSOR=$(( mark + ${{#placeholder}} ))
    REGION_ACTIVE=1
    zle -R
}}

if [[ -o interactive && -n "${{widgets[complete-word]}}" ]]; then
    zle -N _bp_placeholder_or_complete
    bindkey '^I' _bp_placeholder_or_complete
fi
"#,
        cases = zsh_array_literal(commands.iter().map(zsh_case))
    )
}

fn placeholder_commands() -> Vec<PlaceholderCommand> {
    let mut commands = Vec::new();
    let root = crate::branp();
    let mut path = vec![root.get_name().to_string()];

    collect_placeholder_commands(&root, &mut path, &mut commands);
    commands
}

fn collect_placeholder_commands(
    command: &Command,
    path: &mut Vec<String>,
    commands: &mut Vec<PlaceholderCommand>,
) {
    let placeholders = positional_placeholders(command);

    if path.len() > 1 && !placeholders.is_empty() {
        let value_options = value_options(command);
        commands.extend(
            path_alias_variants(path)
                .into_iter()
                .map(|path| PlaceholderCommand {
                    path,
                    placeholders: placeholders.clone(),
                    value_options: value_options.clone(),
                }),
        );
    }

    for subcommand in command.get_subcommands() {
        path.push(command_names(subcommand).join("|"));
        collect_placeholder_commands(subcommand, path, commands);
        path.pop();
    }
}

fn command_names(command: &Command) -> Vec<String> {
    let mut names = vec![command.get_name().to_string()];
    names.extend(command.get_all_aliases().map(ToString::to_string));
    names
}

fn path_alias_variants(path: &[String]) -> Vec<Vec<String>> {
    let mut variants = vec![Vec::new()];

    for segment in path {
        let names = segment.split('|').collect::<Vec<_>>();
        let mut next = Vec::new();

        for variant in &variants {
            for name in &names {
                let mut variant = variant.clone();
                variant.push((*name).to_string());
                next.push(variant);
            }
        }

        variants = next;
    }

    variants
}

fn positional_placeholders(command: &Command) -> Vec<String> {
    let mut args = command
        .get_arguments()
        .filter(|arg| arg.is_positional() && !arg.is_hide_set())
        .collect::<Vec<_>>();

    args.sort_by_key(|arg| arg.get_index().unwrap_or(usize::MAX));

    args.into_iter().map(placeholder_name).collect()
}

fn placeholder_name(arg: &Arg) -> String {
    arg.get_value_names()
        .and_then(|names| names.first())
        .map(|name| name.as_str().to_ascii_lowercase())
        .unwrap_or_else(|| arg.get_id().as_str().to_string())
}

fn value_options(command: &Command) -> Vec<String> {
    let mut options = Vec::new();

    for arg in command
        .get_arguments()
        .filter(|arg| !arg.is_positional() && arg.get_action().takes_values())
    {
        if let Some(long) = arg.get_long() {
            options.push(format!("--{long}"));
        }
        if let Some(short) = arg.get_short() {
            options.push(format!("-{short}"));
        }
    }

    options
}

fn zsh_case(command: &PlaceholderCommand) -> String {
    format!(
        "{}:{}:{}",
        command.path.join(" "),
        command.placeholders.join(" "),
        command.value_options.join(" ")
    )
}

fn zsh_array_literal(values: impl Iterator<Item = String>) -> String {
    values.map(zsh_quote).collect::<Vec<_>>().join(" ")
}

fn zsh_quote(value: String) -> String {
    format!("'{}'", value.replace('\'', r#"'\''"#))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_worktree_new_placeholder_commands_from_clap() {
        let commands = placeholder_commands();
        let worktree_new = commands
            .iter()
            .find(|command| command.path == ["bp", "worktree", "new"])
            .expect("worktree new placeholder command");
        let wt_new = commands
            .iter()
            .find(|command| command.path == ["bp", "wt", "new"])
            .expect("wt new placeholder command");

        assert_eq!(worktree_new.placeholders, ["name", "branch"]);
        assert_eq!(wt_new.placeholders, ["name", "branch"]);
    }
}
