//! Reusable diagnostics and repair primitives.

use std::path::PathBuf;

use crate::errors::CliError;

pub mod checks;
mod fixes;

pub use fixes::{apply_fix, dedupe_fixes, Fix};

pub struct DoctorContext {
    pub home: PathBuf,
}

#[derive(Clone, Copy)]
pub enum CheckSelector {
    Links,
}

pub struct CheckReport {
    pub name: &'static str,
    pub summary: &'static str,
    pub findings: Vec<Finding>,
}

pub trait DoctorCheck {
    fn name(&self) -> &'static str;
    fn summary(&self) -> &'static str;
    fn run(&self, ctx: &DoctorContext) -> Vec<Finding>;
}

#[derive(Clone, Copy)]
pub enum Severity {
    Ok,
    Info,
    Warning,
    Error,
}

pub struct Finding {
    pub id: &'static str,
    pub severity: Severity,
    pub title: String,
    pub details: Vec<String>,
    pub fix: Option<Fix>,
}

pub fn run(ctx: &DoctorContext, selector: CheckSelector) -> Vec<CheckReport> {
    checks_for(selector)
        .into_iter()
        .map(|check| CheckReport {
            name: check.name(),
            summary: check.summary(),
            findings: check.run(ctx),
        })
        .collect()
}

fn checks_for(selector: CheckSelector) -> Vec<Box<dyn DoctorCheck>> {
    match selector {
        CheckSelector::Links => vec![Box::new(checks::links::LinksCheck)],
    }
}

pub fn finding(
    id: &'static str,
    severity: Severity,
    title: impl Into<String>,
    details: Vec<String>,
    fix: Option<Fix>,
) -> Finding {
    Finding {
        id,
        severity,
        title: title.into(),
        details,
        fix,
    }
}

pub fn command_error_detail(error: CliError) -> Vec<String> {
    vec![error.message]
}
