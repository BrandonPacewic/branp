//! CLI adapter for environment and configuration diagnostics.

use clap::{Arg, ArgAction, ArgMatches, Command};

use crate::context::GlobalContext;
use crate::doctor::{self, CheckSelector, DoctorContext, Finding, Fix, Severity};
use crate::errors::{CliError, CliResult};

pub fn cli() -> Command {
    Command::new("doctor")
        .about("Diagnose and repair local CLI integration issues")
        .subcommand_required(false)
        .arg_required_else_help(false)
        .arg(Arg::new("fix").long("fix").help("Apply available fixes").action(ArgAction::SetTrue).global(true))
        .arg(Arg::new("yes").short('y').long("yes").help("Apply fixes without prompting").action(ArgAction::SetTrue).global(true))
        .subcommands(builtin())
}

type DoctorExec = fn(&mut GlobalContext, &ArgMatches) -> CliResult;

fn builtin() -> Vec<Command> {
    vec![links_cli()]
}

fn builtin_exec(cmd: &str) -> Option<DoctorExec> {
    let exec = match cmd {
        "links" => links_exec,
        _ => return None,
    };

    Some(exec)
}

fn links_cli() -> Command {
    Command::new("links").about("Check terminal hyperlink support")
}

pub fn exec(gctx: &mut GlobalContext, args: &ArgMatches) -> CliResult {
    if let Some((cmd, sub)) = args.subcommand() {
        if let Some(exec) = builtin_exec(cmd) {
            return exec(gctx, sub);
        }

        return Err(CliError::from(format!("unknown doctor check `{cmd}`")));
    }

    run_check(gctx, CheckSelector::Links, args)
}

fn links_exec(gctx: &mut GlobalContext, args: &ArgMatches) -> CliResult {
    run_check(gctx, CheckSelector::Links, args)
}

fn run_check(gctx: &mut GlobalContext, selector: CheckSelector, args: &ArgMatches) -> CliResult {
    let ctx = DoctorContext { home: gctx.home().clone() };
    let fix = args.get_flag("fix");
    let yes = args.get_flag("yes");

    for report in doctor::run(&ctx, selector) {
        gctx.shell().note(color_print::cformat!("<cyan,bold>{}</>", report.name));
        gctx.shell().note(format!("  {}", report.summary));

        let mut fixes = Vec::new();
        for finding in report.findings {
            print_finding(gctx, &finding);
            if let Some(fix) = finding.fix {
                fixes.push(fix);
            }
        }

        let fixes = doctor::dedupe_fixes(fixes);
        if fix {
            apply_fixes(gctx, fixes, yes)?;
        } else if !fixes.is_empty() {
            gctx.shell().note("  Run `bp doctor links --fix` to apply available fixes.");
        }
    }

    Ok(())
}

fn print_finding(gctx: &mut GlobalContext, finding: &Finding) {
    let label = match finding.severity {
        Severity::Ok => color_print::cformat!("<green>ok</>"),
        Severity::Info => color_print::cformat!("<cyan>info</>"),
        Severity::Warning => color_print::cformat!("<yellow,bold>warning</>"),
        Severity::Error => color_print::cformat!("<red,bold>error</>"),
    };

    gctx.shell().note(format!("  {label} [{}] {}", finding.id, finding.title));
    for detail in &finding.details {
        if !detail.trim().is_empty() {
            gctx.shell().note(format!("    {detail}"));
        }
    }
}

fn apply_fixes(gctx: &mut GlobalContext, fixes: Vec<Fix>, yes: bool) -> CliResult {
    if fixes.is_empty() {
        gctx.shell().note("  No fixes needed.");
        return Ok(());
    }

    gctx.shell().note("  Available fixes:");
    for fix in &fixes {
        gctx.shell().note(format!("    - {}", fix.description()));
    }

    if !yes && !gctx.shell().confirm("Apply these fixes?")? {
        gctx.shell().note("  Fixes skipped.");
        return Ok(());
    }

    for fix in fixes {
        let outcome = doctor::apply_fix(fix)?;
        gctx.shell().note(format!("  Applied: {}", outcome.description));
        for warning in outcome.warnings {
            gctx.shell().warn(warning);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doctor_subcommands_have_exec_handlers() {
        for command in builtin() {
            assert!(builtin_exec(command.get_name()).is_some(), "{} is missing an exec handler", command.get_name());
        }
    }

    #[test]
    fn doctor_cli_definition_is_valid() {
        cli().debug_assert();
    }

    #[test]
    fn doctor_allows_default_check_without_subcommand() {
        let matches = cli().try_get_matches_from(["doctor"]).expect("default doctor command should parse");

        assert_eq!(matches.subcommand_name(), None);
    }
}
