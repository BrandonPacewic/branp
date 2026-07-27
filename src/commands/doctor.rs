//! CLI adapter for environment and configuration diagnostics.

use std::io::{self, Write};

use clap::{Arg, ArgAction, ArgMatches, Command};

use crate::context::GlobalContext;
use crate::doctor::{self, CheckSelector, DoctorContext, Finding, Fix, Severity};
use crate::errors::{CliError, CliResult};

pub fn command() -> Command {
    Command::new("doctor")
        .about("Diagnose and repair local CLI integration issues")
        .subcommand_required(false)
        .arg_required_else_help(false)
        .arg(Arg::new("fix").long("fix").help("Apply available fixes").action(ArgAction::SetTrue).global(true))
        .arg(Arg::new("yes").short('y').long("yes").help("Apply fixes without prompting").action(ArgAction::SetTrue).global(true))
        .subcommand(Command::new("links").about("Check terminal hyperlink support"))
}

pub fn exec(gctx: &mut GlobalContext, args: &ArgMatches) -> CliResult {
    let fix = args.get_flag("fix");
    let yes = args.get_flag("yes");
    let selector = match args.subcommand() {
        Some(("links", _)) | None => CheckSelector::Links,
        Some((name, _)) => return Err(CliError::from(format!("unknown doctor check `{name}`"))),
    };
    let ctx = DoctorContext { home: gctx.home().clone() };

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

    if !yes && !confirm("Apply these fixes?")? {
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

fn confirm(prompt: &str) -> Result<bool, CliError> {
    eprint!("{prompt} [y/N] ");
    io::stderr().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(matches!(input.trim(), "y" | "Y" | "yes" | "YES"))
}
