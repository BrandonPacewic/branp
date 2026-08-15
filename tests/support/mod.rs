use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct TestWorkspace {
    root: PathBuf,
}

impl TestWorkspace {
    pub fn new(name: &str) -> Self {
        let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("branp-{name}-{}-{unique}", std::process::id()));
        fs::create_dir_all(&root).unwrap();

        Self { root }
    }

    pub fn git_repo(&self, name: &str, default_branch: &str) -> GitRepo {
        let repo = self.root.join(name);
        let output = command("git").args(["init", &format!("--initial-branch={default_branch}"), name]).current_dir(&self.root).output().unwrap();
        assert_success(&output, "git init");

        let repo = GitRepo { path: repo };
        repo.git(["config", "user.name", "Branp Test"]);
        repo.git(["config", "user.email", "branp-test@example.com"]);
        repo.git(["config", "commit.gpgsign", "false"]);
        repo.git(["config", "branp.worktree.defaultBranch", default_branch]);
        repo
    }
}

impl Drop for TestWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub struct GitRepo {
    path: PathBuf,
}

impl GitRepo {
    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn sibling(&self, name: &str) -> PathBuf {
        PathBuf::from(format!("{}-{name}", self.path.display()))
    }

    pub fn git<I, S>(&self, args: I) -> Output
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let output = command("git").args(args).current_dir(&self.path).output().unwrap();
        assert_success(&output, "git");
        output
    }

    pub fn bp<I, S>(&self, args: I) -> Output
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        command(env!("CARGO_BIN_EXE_bp")).args(args).current_dir(&self.path).stdin(Stdio::null()).output().unwrap()
    }

    pub fn bp_with_env<I, S>(&self, args: I, envs: &[(&str, &str)]) -> Output
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        command(env!("CARGO_BIN_EXE_bp")).args(args).current_dir(&self.path).envs(envs.iter().copied()).stdin(Stdio::null()).output().unwrap()
    }

    pub fn commit_file(&self, path: &str, contents: &str, message: &str) {
        let path = self.path.join(path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, contents).unwrap();
        self.git(["add", "."]);
        self.git(["commit", "-m", message]);
    }

    pub fn read_file(&self, path: &str) -> String {
        fs::read_to_string(self.path.join(path)).unwrap()
    }
}

pub fn assert_success(output: &Output, context: &str) {
    assert!(
        output.status.success(),
        "{context} failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        stdout(output),
        stderr(output)
    );
}

pub fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

pub fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn command(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);
    command.env_remove("GIT_DIR");
    command.env_remove("GIT_WORK_TREE");
    command
}
