use std::fs;
use std::path::{Path, PathBuf};

use crate::errors::CliError;

/// A process whose current working directory is inside a requested path.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub command: String,
}

/// Find processes whose current working directory is `path` or one of its descendants.
pub fn processes_in_worktree(path: &Path) -> Result<Vec<ProcessInfo>, CliError> {
    let path = canonical_path(path)?;
    platform::processes_in_worktree(&path)
}

fn canonical_path(path: &Path) -> Result<PathBuf, CliError> {
    fs::canonicalize(path).map_err(|error| CliError::from(format!("could not resolve process inspection path {}: {error}", path.display())))
}

fn path_is_within(path: &Path, root: &Path) -> bool {
    path == root || path.starts_with(root)
}

#[cfg(target_os = "linux")]
mod platform {
    use super::{canonical_path, path_is_within, Path, ProcessInfo};
    use crate::errors::CliError;
    use std::fs;

    pub(super) fn processes_in_worktree(root: &Path) -> Result<Vec<ProcessInfo>, CliError> {
        let mut processes = Vec::new();

        for entry in fs::read_dir("/proc")? {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };
            let Some(pid) = entry.file_name().to_str().and_then(|name| name.parse::<u32>().ok()) else {
                continue;
            };

            let process_dir = entry.path();
            let Ok(cwd) = canonical_path(&process_dir.join("cwd")) else {
                continue;
            };
            if !path_is_within(&cwd, root) {
                continue;
            }

            let name = fs::read_to_string(process_dir.join("comm")).map(|name| name.trim().to_string()).unwrap_or_else(|_| "unknown".to_string());
            let command = fs::read(process_dir.join("cmdline"))
                .ok()
                .map(|command| {
                    command
                        .split(|byte| *byte == 0)
                        .filter(|argument| !argument.is_empty())
                        .map(String::from_utf8_lossy)
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .filter(|command| !command.is_empty())
                .unwrap_or_else(|| name.clone());

            processes.push(ProcessInfo { pid, name, command });
        }

        processes.sort_by_key(|process| process.pid);
        Ok(processes)
    }
}

#[cfg(all(unix, not(target_os = "linux")))]
mod platform {
    use super::{canonical_path, path_is_within, Path, PathBuf, ProcessInfo};
    use crate::errors::CliError;
    use std::process::Command;

    pub(super) fn processes_in_worktree(root: &Path) -> Result<Vec<ProcessInfo>, CliError> {
        let output = Command::new("lsof").args(["-a", "-d", "cwd", "+D"]).arg(root).args(["-F", "pcn", "-w", "-n"]).output()?;

        if !output.status.success() && output.stdout.is_empty() {
            // lsof uses exit code 1 when no process has an open file below the path.
            if output.status.code() == Some(1) {
                return Ok(Vec::new());
            }
            let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(CliError::from(if detail.is_empty() { "lsof failed".to_string() } else { format!("lsof failed: {detail}") }));
        }

        let mut processes = Vec::new();
        let mut current = LsofProcess::default();
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let Some((field, value)) = line.split_at_checked(1) else {
                continue;
            };
            match field {
                "p" => {
                    add_process(&mut processes, &mut current, root);
                    current.pid = value.parse().ok();
                }
                "c" => current.name = Some(value.to_string()),
                "n" => current.cwd = Some(PathBuf::from(value)),
                _ => {}
            }
        }
        add_process(&mut processes, &mut current, root);

        processes.sort_by_key(|process| process.pid);
        processes.dedup_by_key(|process| process.pid);
        Ok(processes)
    }

    #[derive(Default)]
    struct LsofProcess {
        pid: Option<u32>,
        name: Option<String>,
        cwd: Option<PathBuf>,
    }

    fn add_process(processes: &mut Vec<ProcessInfo>, current: &mut LsofProcess, root: &Path) {
        let (Some(pid), Some(name), Some(cwd)) = (current.pid.take(), current.name.take(), current.cwd.take()) else {
            return;
        };
        let Ok(cwd) = canonical_path(&cwd) else {
            return;
        };
        if path_is_within(&cwd, root) {
            processes.push(ProcessInfo { pid, command: name.clone(), name });
        }
    }
}

#[cfg(not(unix))]
mod platform {
    use super::Path;
    use crate::errors::CliError;

    pub(super) fn processes_in_worktree(_root: &Path) -> Result<Vec<super::ProcessInfo>, CliError> {
        Err(CliError::from("process cwd inspection is unsupported on this platform"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Child, Command, Stdio};
    use std::thread;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    #[test]
    fn path_containment_respects_directory_boundaries() {
        let root = Path::new("/tmp/worktree");
        assert!(path_is_within(Path::new("/tmp/worktree"), root));
        assert!(path_is_within(Path::new("/tmp/worktree/src"), root));
        assert!(!path_is_within(Path::new("/tmp/worktree-sibling"), root));
    }

    #[cfg(unix)]
    #[test]
    fn finds_process_with_exact_cwd() {
        let dir = test_directory("exact-cwd");
        let mut child = SleepProcess::spawn(&dir);
        let process = wait_for_process(&dir, child.id());

        assert_eq!(process.pid, child.id());
        assert!(!process.name.is_empty());
        assert!(!process.command.is_empty());
        child.stop();
        remove_test_directory(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn finds_process_with_descendant_cwd() {
        let dir = test_directory("descendant-cwd");
        let descendant = dir.join("nested");
        fs::create_dir(&descendant).unwrap();
        let mut child = SleepProcess::spawn(&descendant);
        let process = wait_for_process(&dir, child.id());

        assert_eq!(process.pid, child.id());
        child.stop();
        remove_test_directory(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn excludes_process_with_sibling_prefix_cwd() {
        let dir = test_directory("prefix");
        let sibling = dir.with_file_name(format!("{}-sibling", dir.file_name().unwrap().to_string_lossy()));
        fs::create_dir(&sibling).unwrap();
        let mut child = SleepProcess::spawn(&sibling);
        wait_for_process(&sibling, child.id());

        let processes = processes_in_worktree(&dir).unwrap();
        assert!(!processes.iter().any(|process| process.pid == child.id()));
        child.stop();
        remove_test_directory(&dir);
        remove_test_directory(&sibling);
    }

    #[cfg(unix)]
    #[test]
    fn returns_no_processes_for_unused_path() {
        let dir = test_directory("unused");
        assert!(processes_in_worktree(&dir).unwrap().is_empty());
        remove_test_directory(&dir);
    }

    #[cfg(unix)]
    struct SleepProcess(Child);

    #[cfg(unix)]
    impl SleepProcess {
        fn spawn(cwd: &Path) -> Self {
            Self(Command::new("sleep").arg("30").current_dir(cwd).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap())
        }

        fn id(&self) -> u32 {
            self.0.id()
        }

        fn stop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    #[cfg(unix)]
    impl Drop for SleepProcess {
        fn drop(&mut self) {
            self.stop();
        }
    }

    #[cfg(unix)]
    fn wait_for_process(path: &Path, pid: u32) -> ProcessInfo {
        for _ in 0..80 {
            if let Some(process) = processes_in_worktree(path).unwrap().into_iter().find(|process| process.pid == pid) {
                return process;
            }
            thread::sleep(Duration::from_millis(25));
        }
        panic!("process {pid} was not found in {}", path.display());
    }

    fn test_directory(name: &str) -> PathBuf {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("branp-process-{name}-{}-{timestamp}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn remove_test_directory(path: &Path) {
        let _ = fs::remove_dir_all(path);
    }
}
