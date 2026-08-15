mod support;

use std::fs;
use std::path::{Path, PathBuf};
#[cfg(unix)] use std::process::{Command, Stdio};
#[cfg(unix)] use std::thread;
#[cfg(unix)] use std::time::Duration;

use support::{assert_success, stderr, stdout, GitRepo, TestWorkspace};

#[test]
fn worktree_new_reuses_existing_local_branch_without_source_prompt() {
    let workspace = TestWorkspace::new("worktree-new-existing-local-branch");
    let repo = workspace.git_repo("repo", "mega");

    repo.commit_file("file.txt", "base\n", "initial");
    repo.git(["branch", "feature"]);
    repo.git(["switch", "-c", "other"]);
    repo.commit_file("file.txt", "other\n", "other branch change");

    let output = repo.bp(["worktree", "new", "feature", "--no-fetch", "--no-submodules", "--no-tmux"]);
    assert_success(&output, "bp worktree new");

    let stderr = stderr(&output);
    assert!(!stderr.contains("Continue creating the worktree?"), "unexpected source-branch prompt:\n{stderr}");
    assert!(!stderr.contains("the new worktree will branch from the current base HEAD"), "unexpected source-branch warning:\n{stderr}");

    let worktree = repo.sibling("feature");
    assert!(stdout(&output).contains(&worktree.display().to_string()), "stdout did not contain worktree path:\n{}", stdout(&output));

    let feature = GitRepo::from_path(&worktree);
    assert_eq!(stdout(&feature.git(["branch", "--show-current"])), "feature\n");
    assert_eq!(feature.read_file("file.txt"), "base\n");
    assert_eq!(stdout(&feature.git(["status", "--short"])), "");
}

#[cfg(unix)]
#[test]
fn worktree_new_without_name_enters_each_detached_scratch_worktree() {
    use std::os::unix::fs::PermissionsExt;

    let workspace = TestWorkspace::new("worktree-new-scratch");
    let repo = workspace.git_repo("repo", "mega");
    repo.commit_file("file.txt", "base\n", "initial");

    let fake_shell_dir = std::env::temp_dir().join(format!("branp-fake-shell-{}", std::process::id()));
    let _ = fs::remove_dir_all(&fake_shell_dir);
    fs::create_dir_all(&fake_shell_dir).unwrap();
    let fake_shell = fake_shell_dir.join("shell");
    let shell_cwd_file = fake_shell_dir.join("cwd");
    fs::write(&fake_shell, "#!/bin/sh\npwd >> \"$BP_TEST_SHELL_CWD_FILE\"\nexit 0\n").unwrap();
    fs::set_permissions(&fake_shell, fs::Permissions::from_mode(0o755)).unwrap();
    let home = repo.sibling("home");
    let home_string = home.to_str().unwrap();
    let shell_string = fake_shell.to_str().unwrap();
    let shell_cwd_file_string = shell_cwd_file.to_str().unwrap();

    let first = repo.bp_with_env(
        ["worktree", "new", "--no-submodules"],
        &[("HOME", home_string), ("SHELL", shell_string), ("BP_TEST_SHELL_CWD_FILE", shell_cwd_file_string)],
    );
    assert_success(&first, "bp worktree new scratch");
    let second = repo.bp_with_env(
        ["worktree", "n", "--no-submodules"],
        &[("HOME", home_string), ("SHELL", shell_string), ("BP_TEST_SHELL_CWD_FILE", shell_cwd_file_string)],
    );
    assert_success(&second, "bp worktree n scratch");

    let first_path = Path::new(stdout(&first).trim()).to_path_buf();
    let second_path = Path::new(stdout(&second).trim()).to_path_buf();
    let scratch_dir = fs::canonicalize(home.join(".bp/worktrees")).unwrap();
    let first_parent = fs::canonicalize(first_path.parent().unwrap()).unwrap();
    let second_parent = fs::canonicalize(second_path.parent().unwrap()).unwrap();
    assert_eq!(first_parent, scratch_dir);
    assert_eq!(second_parent, scratch_dir);
    assert_ne!(first_path, second_path);
    assert!(first_path.file_name().unwrap().to_string_lossy().starts_with("scratch-"));
    assert!(second_path.file_name().unwrap().to_string_lossy().starts_with("scratch-"));

    let first_worktree = GitRepo::from_path(&first_path);
    let second_worktree = GitRepo::from_path(&second_path);
    assert_eq!(stdout(&first_worktree.git(["branch", "--show-current"])), "");
    assert_eq!(stdout(&second_worktree.git(["branch", "--show-current"])), "");
    assert_eq!(stdout(&first_worktree.git(["status", "--short"])), "");
    assert_eq!(stdout(&second_worktree.git(["status", "--short"])), "");

    let shell_cwds = fs::read_to_string(&shell_cwd_file).unwrap();
    let shell_cwds = shell_cwds.lines().map(Path::new).map(fs::canonicalize).collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(shell_cwds, vec![fs::canonicalize(&first_path).unwrap(), fs::canonicalize(&second_path).unwrap()]);

    fs::remove_dir_all(fake_shell_dir).unwrap();
}

#[cfg(unix)]
#[test]
fn worktree_new_reuses_clean_scratch_and_syncs_links() {
    let workspace = TestWorkspace::new("worktree-new-reuses-scratch");
    let repo = workspace.git_repo("repo", "mega");
    repo.commit_file("file.txt", "base\n", "initial");
    repo.commit_file(".gitignore", "shared.txt\n", "ignore shared link");

    let home = repo.sibling("home");
    let scratch = add_detached_scratch(&repo, &home, "scratch-reusable");
    fs::write(repo.path().join("shared.txt"), "shared\n").unwrap();
    assert_success(&repo.bp(["worktree", "track", "shared.txt"]), "track shared link");
    fs::remove_file(scratch.join("shared.txt")).unwrap();

    let output = repo.bp_with_env(["worktree", "new", "--no-submodules"], &[("HOME", home.to_str().unwrap()), ("SHELL", "/bin/sh")]);
    assert_success(&output, "reuse clean scratch worktree");
    assert_eq!(fs::canonicalize(Path::new(stdout(&output).trim())).unwrap(), fs::canonicalize(&scratch).unwrap());
    assert!(scratch.join("shared.txt").is_symlink(), "reuse did not run link synchronization");
}

#[cfg(unix)]
#[test]
fn worktree_new_selects_scratch_candidates_deterministically() {
    let workspace = TestWorkspace::new("worktree-new-scratch-order");
    let repo = workspace.git_repo("repo", "mega");
    repo.commit_file("file.txt", "base\n", "initial");

    let home = repo.sibling("home");
    let _later = add_detached_scratch(&repo, &home, "scratch-z");
    let earlier = add_detached_scratch(&repo, &home, "scratch-a");

    let output = repo.bp_with_env(["worktree", "n", "--no-submodules"], &[("HOME", home.to_str().unwrap()), ("SHELL", "/bin/sh")]);
    assert_success(&output, "deterministic scratch reuse");
    assert_eq!(fs::canonicalize(Path::new(stdout(&output).trim())).unwrap(), fs::canonicalize(earlier).unwrap());
}

#[cfg(unix)]
#[test]
fn worktree_new_skips_dirty_in_use_and_named_scratch_candidates() {
    let workspace = TestWorkspace::new("worktree-new-scratch-skips");
    let repo = workspace.git_repo("repo", "mega");
    repo.commit_file("file.txt", "base\n", "initial");

    let home = repo.sibling("home");
    let dirty = add_detached_scratch(&repo, &home, "scratch-a-dirty");
    fs::write(dirty.join("file.txt"), "dirty\n").unwrap();
    let in_use = add_detached_scratch(&repo, &home, "scratch-b-in-use");
    repo.git(["branch", "scratch-c-named"]);
    let named = home.join(".bp/worktrees/scratch-c-named");
    fs::create_dir_all(named.parent().unwrap()).unwrap();
    repo.git(["worktree", "add", named.to_str().unwrap(), "scratch-c-named"]);

    let mut child =
        Command::new("sleep").arg("30").current_dir(&in_use).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap();
    let pid = child.id();
    let detected = (0..80).any(|_| {
        let output = repo.bp(["worktree", "list", "--no-pr"]);
        if stdout(&output).contains(&format!("in-use:process:{pid}")) {
            true
        } else {
            thread::sleep(Duration::from_millis(25));
            false
        }
    });
    assert!(detected, "in-use scratch process was not detected");

    let output = repo.bp_with_env(["worktree", "new", "--no-submodules"], &[("HOME", home.to_str().unwrap()), ("SHELL", "/bin/sh")]);
    child.kill().unwrap();
    child.wait().unwrap();

    assert_success(&output, "allocate scratch after skipping unavailable candidates");
    let allocated = PathBuf::from(stdout(&output).trim());
    assert_ne!(allocated, dirty);
    assert_ne!(allocated, in_use);
    assert_ne!(allocated, named);
    assert!(allocated.starts_with(home.join(".bp/worktrees")));
    let diagnostics = stderr(&output);
    assert!(diagnostics.contains("scratch-a-dirty") && diagnostics.contains("dirty"), "dirty skip lacked a reason:\n{diagnostics}");
    assert!(diagnostics.contains("scratch-b-in-use") && diagnostics.contains("in use"), "in-use skip lacked a reason:\n{diagnostics}");
    assert!(diagnostics.contains("scratch-c-named") && diagnostics.contains("named"), "named skip lacked a reason:\n{diagnostics}");
    assert_eq!(stdout(&GitRepo::from_path(&named).git(["branch", "--show-current"])), "scratch-c-named\n");
}

#[cfg(unix)]
#[test]
fn concurrent_scratch_acquisitions_do_not_claim_one_candidate() {
    use std::os::unix::fs::PermissionsExt;

    let workspace = TestWorkspace::new("worktree-new-scratch-concurrent");
    let repo = workspace.git_repo("repo", "mega");
    repo.commit_file("file.txt", "base\n", "initial");
    let home = repo.sibling("home");
    let candidate = add_detached_scratch(&repo, &home, "scratch-concurrent");

    let shell_dir = home.join("shell");
    fs::create_dir_all(&shell_dir).unwrap();
    let shell = shell_dir.join("sleep-shell");
    fs::write(&shell, "#!/bin/sh\nsleep 1\n").unwrap();
    fs::set_permissions(&shell, fs::Permissions::from_mode(0o755)).unwrap();

    let spawn = || {
        Command::new(env!("CARGO_BIN_EXE_bp"))
            .args(["worktree", "new", "--no-submodules"])
            .current_dir(repo.path())
            .env("HOME", home.to_str().unwrap())
            .env("SHELL", shell.to_str().unwrap())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap()
    };

    let first = spawn();
    let reservation = home.join(".bp/worktrees/.scratch-concurrent.reservation");
    let claimed = (0..80).any(|_| {
        if reservation.is_file() {
            true
        } else {
            thread::sleep(Duration::from_millis(25));
            false
        }
    });
    assert!(claimed, "first acquisition did not hold the scratch reservation");

    let second = spawn().wait_with_output().unwrap();
    let first = first.wait_with_output().unwrap();
    assert_success(&first, "first concurrent scratch acquisition");
    assert_success(&second, "second concurrent scratch acquisition");
    assert_eq!(fs::canonicalize(Path::new(stdout(&first).trim())).unwrap(), fs::canonicalize(candidate).unwrap());
    assert_ne!(stdout(&first).trim(), stdout(&second).trim(), "concurrent acquisitions claimed the same scratch worktree");
}

#[test]
fn worktree_remove_deletes_clean_worktree_and_branch() {
    let workspace = TestWorkspace::new("worktree-remove-clean");
    let repo = workspace.git_repo("repo", "mega");

    repo.commit_file("file.txt", "base\n", "initial");
    repo.git(["branch", "feature"]);

    let output = repo.bp(["worktree", "new", "feature", "--no-fetch", "--no-submodules", "--no-tmux"]);
    assert_success(&output, "bp worktree new");

    let worktree = repo.sibling("feature");
    assert!(worktree.is_dir(), "expected worktree to exist at {}", worktree.display());

    let output = repo.bp(["worktree", "remove", "feature", "--no-tmux"]);
    assert_success(&output, "bp worktree remove");

    assert!(!worktree.exists(), "expected worktree to be removed from {}", worktree.display());
    assert_eq!(stdout(&repo.git(["branch", "--list", "feature"])), "");
}

#[cfg(unix)]
#[test]
fn worktree_list_reports_process_use_without_matching_a_sibling_worktree() {
    let workspace = TestWorkspace::new("worktree-list-in-use");
    let repo = workspace.git_repo("repo", "mega");

    repo.commit_file("file.txt", "base\n", "initial");
    repo.git(["branch", "feature"]);
    repo.git(["branch", "feature-extra"]);

    assert_success(&repo.bp(["worktree", "new", "feature", "--no-fetch", "--no-submodules", "--no-tmux"]), "create feature worktree");
    assert_success(&repo.bp(["worktree", "new", "feature-extra", "--no-fetch", "--no-submodules", "--no-tmux"]), "create sibling worktree");

    let feature = repo.sibling("feature");
    let mut child = Command::new("sleep")
        .arg("30")
        .current_dir(&feature)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("start process in worktree");
    let pid = child.id().to_string();

    let output = (0..80).find_map(|_| {
        let output = repo.bp(["worktree", "list", "--no-pr"]);
        if stdout(&output).contains(&format!("in-use:process:{pid}")) {
            Some(output)
        } else {
            thread::sleep(Duration::from_millis(25));
            None
        }
    });
    let Some(output) = output else {
        child.kill().expect("stop process after detection timeout");
        child.wait().expect("reap process after detection timeout");
        panic!("worktree list did not report process {pid}");
    };

    assert_success(&output, "list worktrees with process use");
    let text = stdout(&output);
    let feature_line = text.lines().find(|line| line.contains("repo-feature")).expect("feature worktree row");
    assert!(feature_line.contains(&format!("in-use:process:{pid}")), "feature row did not report process use:\n{text}");
    assert!(feature_line.contains("sleep"), "feature row did not report the process name:\n{text}");

    let sibling_line = text.lines().find(|line| line.contains("repo-feature-extra")).expect("sibling worktree row");
    assert!(!sibling_line.contains("in-use:"), "sibling row was falsely reported in use:\n{text}");

    child.kill().expect("stop process in worktree");
    child.wait().expect("reap process in worktree");
}

#[test]
fn worktree_prune_does_not_require_default_branch_discovery() {
    let workspace = TestWorkspace::new("worktree-prune-detached-without-default");
    let repo = workspace.git_repo("repo", "mega");

    repo.commit_file("file.txt", "base\n", "initial");
    repo.git(["config", "--unset-all", "branp.worktree.defaultBranch"]);
    repo.git(["switch", "--detach", "HEAD"]);

    let output = repo.bp(["worktree", "prune"]);
    assert_success(&output, "bp worktree prune");
}

#[test]
fn worktree_return_infers_current_scratch_and_preserves_ignored_files() {
    let workspace = TestWorkspace::new("worktree-return-clean");
    let repo = workspace.git_repo("repo", "mega");
    repo.commit_file("file.txt", "base\n", "initial");
    repo.commit_file(".gitignore", "build/\n", "ignore build cache");

    let home = repo.sibling("home");
    let scratch = home.join(".bp/worktrees/scratch-clean");
    fs::create_dir_all(scratch.parent().unwrap()).unwrap();
    repo.git(["worktree", "add", "--detach", scratch.to_str().unwrap(), "HEAD"]);
    fs::create_dir_all(scratch.join("build")).unwrap();
    fs::write(scratch.join("build/cache.bin"), "cache\n").unwrap();

    let output = GitRepo::from_path(&scratch).bp_with_env(["worktree", "return"], &[("HOME", home.to_str().unwrap())]);
    assert_success(&output, "return current scratch worktree");
    assert!(stdout(&output).contains("returned"), "return output did not identify the target:\n{}", stdout(&output));

    let scratch_repo = GitRepo::from_path(&scratch);
    assert_eq!(stdout(&scratch_repo.git(["branch", "--show-current"])), "");
    assert_eq!(stdout(&scratch_repo.git(["status", "--short"])), "");
    assert_eq!(scratch_repo.read_file("file.txt"), "base\n");
    assert_eq!(scratch_repo.read_file("build/cache.bin"), "cache\n");
}

#[test]
fn worktree_return_refuses_dirty_scratch_until_explicitly_allowed() {
    let workspace = TestWorkspace::new("worktree-return-dirty");
    let repo = workspace.git_repo("repo", "mega");
    repo.commit_file("file.txt", "base\n", "initial");
    let home = repo.sibling("home");
    let scratch = home.join(".bp/worktrees/scratch-dirty");
    fs::create_dir_all(scratch.parent().unwrap()).unwrap();
    repo.git(["worktree", "add", "--detach", scratch.to_str().unwrap(), "HEAD"]);
    fs::write(scratch.join("file.txt"), "dirty\n").unwrap();
    fs::write(scratch.join("untracked.txt"), "untracked\n").unwrap();

    let scratch_repo = GitRepo::from_path(&scratch);
    let refused = repo.bp_with_env(["worktree", "return", scratch.to_str().unwrap()], &[("HOME", home.to_str().unwrap())]);
    assert!(!refused.status.success(), "dirty return unexpectedly succeeded:\n{}", stdout(&refused));
    assert!(stderr(&refused).contains("--discard-changes"), "refusal did not explain the override:\n{}", stderr(&refused));
    assert_eq!(scratch_repo.read_file("file.txt"), "dirty\n");

    let returned = repo.bp_with_env(["worktree", "return", scratch.to_str().unwrap(), "--discard-changes"], &[("HOME", home.to_str().unwrap())]);
    assert_success(&returned, "return dirty scratch worktree with explicit override");
    assert_eq!(stdout(&scratch_repo.git(["status", "--short"])), "");
    assert!(!scratch.join("untracked.txt").exists());
}

#[test]
fn worktree_return_refuses_named_worktrees() {
    let workspace = TestWorkspace::new("worktree-return-named");
    let repo = workspace.git_repo("repo", "mega");
    repo.commit_file("file.txt", "base\n", "initial");
    repo.git(["branch", "feature"]);
    let named = repo.sibling("feature");
    repo.git(["worktree", "add", named.to_str().unwrap(), "feature"]);

    let home = repo.sibling("home");
    let base_output = repo.bp_with_env(["worktree", "return"], &[("HOME", home.to_str().unwrap())]);
    assert!(!base_output.status.success(), "base worktree return unexpectedly succeeded:\n{}", stdout(&base_output));
    assert!(stderr(&base_output).contains("base worktree"), "base refusal did not identify base lifecycle:\n{}", stderr(&base_output));

    let output = repo.bp_with_env(["worktree", "return", "feature"], &[("HOME", home.to_str().unwrap())]);
    assert!(!output.status.success(), "named worktree return unexpectedly succeeded:\n{}", stdout(&output));
    assert!(stderr(&output).contains("named worktree"), "refusal did not identify named lifecycle:\n{}", stderr(&output));
    assert!(named.is_dir());
}

#[cfg(unix)]
#[test]
fn worktree_return_refuses_in_use_scratch_worktrees() {
    let workspace = TestWorkspace::new("worktree-return-in-use");
    let repo = workspace.git_repo("repo", "mega");
    repo.commit_file("file.txt", "base\n", "initial");
    let home = repo.sibling("home");
    let scratch = home.join(".bp/worktrees/scratch-in-use");
    fs::create_dir_all(scratch.parent().unwrap()).unwrap();
    repo.git(["worktree", "add", "--detach", scratch.to_str().unwrap(), "HEAD"]);

    let mut child =
        Command::new("sleep").arg("30").current_dir(&scratch).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap();
    let output = (0..80).find_map(|_| {
        let output = repo.bp_with_env(["worktree", "return", scratch.to_str().unwrap()], &[("HOME", home.to_str().unwrap())]);
        if !output.status.success() && stderr(&output).contains("in use") {
            Some(output)
        } else {
            thread::sleep(Duration::from_millis(25));
            None
        }
    });
    let Some(output) = output else {
        child.kill().unwrap();
        child.wait().unwrap();
        panic!("return did not report in-use scratch worktree");
    };
    assert!(stderr(&output).contains("sleep"), "in-use refusal omitted process detail:\n{}", stderr(&output));
    child.kill().unwrap();
    child.wait().unwrap();
}

#[test]
fn worktree_list_reports_base_named_and_detached_worktrees_with_state() {
    let workspace = TestWorkspace::new("worktree-list-status");
    let repo = workspace.git_repo("repo", "mega");

    repo.commit_file("file.txt", "base\n", "initial");
    repo.git(["branch", "feature"]);

    let named_path = repo.sibling("feature");
    repo.git(["worktree", "add", named_path.to_str().unwrap(), "feature"]);
    let scratch_path = repo.sibling("scratch");
    repo.git(["worktree", "add", "--detach", scratch_path.to_str().unwrap(), "HEAD"]);

    std::fs::write(named_path.join("untracked.txt"), "dirty\n").unwrap();
    repo.git(["config", "status.showUntrackedFiles", "no"]);

    let output = GitRepo::from_path(&named_path).bp(["worktree", "list", "--no-pr"]);
    assert_success(&output, "bp worktree list");
    let text = stdout(&output);

    let base_line = text.lines().find(|line| line.contains("repo  ")).expect("base worktree row");
    assert!(base_line.contains("base,clean"), "base row did not report clean state:\n{text}");

    let named_line = text.lines().find(|line| line.contains("repo-feature")).expect("named worktree row");
    assert!(named_line.contains("feature"), "named row did not report its branch:\n{text}");
    assert!(named_line.contains("dirty"), "named row did not report dirty state:\n{text}");
    assert!(named_line.contains("current"), "named row did not report current state:\n{text}");

    let scratch_line = text.lines().find(|line| line.contains("repo-scratch")).expect("detached worktree row");
    assert!(scratch_line.contains("detached"), "scratch row did not report detached state:\n{text}");
    assert!(scratch_line.contains("clean"), "scratch row did not report clean state:\n{text}");
}

#[cfg(unix)]
fn add_detached_scratch(repo: &GitRepo, home: &Path, name: &str) -> PathBuf {
    let scratch = home.join(".bp/worktrees").join(name);
    fs::create_dir_all(scratch.parent().unwrap()).unwrap();
    repo.git(["worktree", "add", "--detach", scratch.to_str().unwrap(), "HEAD"]);
    scratch
}
