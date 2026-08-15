use std::collections::HashSet;
use std::path::Path;

use crate::errors::CliError;
use crate::utils::process::ProcessInfo;

/// The live signals that currently make a worktree in use.
///
/// This deliberately does not contain lifecycle reservations or durable
/// leases. Those are separate facts that future lifecycle operations can add
/// without changing process or session detection.
#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct InUseInspection {
    pub processes: Vec<ProcessInfo>,
    pub active_sessions: Vec<String>,
}

impl InUseInspection {
    pub fn is_in_use(&self) -> bool {
        !self.processes.is_empty() || !self.active_sessions.is_empty()
    }

    pub fn reasons(&self) -> Vec<String> {
        self.processes
            .iter()
            .map(|process| format!("process:{} ({})", process.pid, process.name))
            .chain(self.active_sessions.iter().map(|session| format!("tmux:{session}")))
            .collect()
    }
}

/// Inspect a worktree using the live process list and the active tmux server.
///
/// A missing or unusable tmux server is treated as having no active sessions,
/// matching the existing best-effort worktree status behavior.
pub fn inspect(path: &Path, session_name: Option<&str>) -> Result<InUseInspection, CliError> {
    let processes = crate::utils::process::processes_in_worktree(path)?;
    let active_sessions = crate::utils::tmux::sessions().unwrap_or_default();
    Ok(inspect_with_active_sessions(processes, session_name, &active_sessions))
}

pub fn inspect_with_active_sessions(processes: Vec<ProcessInfo>, session_name: Option<&str>, active_sessions: &[String]) -> InUseInspection {
    let active_sessions = session_name
        .filter(|session| active_sessions.iter().any(|active| active == *session))
        .map(|session| vec![session.to_string()])
        .unwrap_or_default();

    InUseInspection { processes, active_sessions }
}

pub fn active_sessions() -> HashSet<String> {
    crate::utils::tmux::sessions().unwrap_or_default().into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_only_the_exact_named_session() {
        let inspection =
            inspect_with_active_sessions(Vec::new(), Some("repo-feature"), &["repo-feature-extra".to_string(), "repo-feature".to_string()]);

        assert_eq!(inspection.active_sessions, ["repo-feature"]);
        assert!(inspection.is_in_use());
    }

    #[test]
    fn keeps_process_and_session_reasons_separate() {
        let inspection = InUseInspection {
            processes: vec![ProcessInfo { pid: 42, name: "shell".to_string(), command: "shell".to_string() }],
            active_sessions: vec!["repo-feature".to_string()],
        };

        assert_eq!(inspection.reasons(), ["process:42 (shell)", "tmux:repo-feature"]);
    }

    #[test]
    fn sibling_session_names_are_not_a_match() {
        let inspection = inspect_with_active_sessions(Vec::new(), Some("repo-feature"), &["repo-feature-extra".to_string()]);

        assert!(!inspection.is_in_use());
        assert!(inspection.active_sessions.is_empty());
    }
}
