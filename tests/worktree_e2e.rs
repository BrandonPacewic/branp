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
fn worktree_enter_opens_exact_base_named_and_nested_scratch_targets_without_mutation() {
    use std::os::unix::fs::PermissionsExt;

    let workspace = TestWorkspace::new("worktree-enter-existing");
    let repo = workspace.git_repo("repo", "mega");
    repo.commit_file("file.txt", "base\n", "initial");
    repo.git(["branch", "feature"]);

    let named = repo.sibling("feature");
    repo.git(["worktree", "add", named.to_str().unwrap(), "feature"]);
    fs::write(named.join("file.txt"), "named\n").unwrap();

    let home = repo.sibling("nested/home");
    let scratch = add_detached_scratch(&repo, &home, "scratch-enter");
    fs::write(scratch.join("file.txt"), "scratch\n").unwrap();
    let state_path = home.join(".bp/worktrees/state.toml");
    let state_before = fs::read_to_string(&state_path).unwrap();

    let shell = home.join("record-shell");
    let shell_log = home.join("shell.log");
    fs::create_dir_all(&home).unwrap();
    fs::write(
        &shell,
        "#!/bin/sh\nprintf '%s|' \"$PWD\" >> \"$BP_TEST_SHELL_LOG\"\ngit branch --show-current | tr '\\n' '|' >> \"$BP_TEST_SHELL_LOG\"\ngit status --short | tr '\\n' ';' >> \"$BP_TEST_SHELL_LOG\"\nprintf '\\n' >> \"$BP_TEST_SHELL_LOG\"\n",
    )
    .unwrap();
    fs::set_permissions(&shell, fs::Permissions::from_mode(0o755)).unwrap();

    for target in ["mega", "feature", "scratch-enter"] {
        let output = repo.bp_with_env(
            ["worktree", "enter", target],
            &[("HOME", home.to_str().unwrap()), ("SHELL", shell.to_str().unwrap()), ("BP_TEST_SHELL_LOG", shell_log.to_str().unwrap())],
        );
        assert_success(&output, "enter worktree by name");
    }
    for target in [named.to_str().unwrap(), scratch.to_str().unwrap()] {
        let output = repo.bp_with_env(
            ["worktree", "enter", target],
            &[("HOME", home.to_str().unwrap()), ("SHELL", shell.to_str().unwrap()), ("BP_TEST_SHELL_LOG", shell_log.to_str().unwrap())],
        );
        assert_success(&output, "enter worktree by path");
    }

    let log = fs::read_to_string(&shell_log).unwrap();
    assert!(log.lines().any(|line| line.contains("|mega|")), "shell did not enter the base worktree with its branch:\n{log}");
    assert!(
        log.lines().any(|line| line.contains("|feature| M file.txt;")),
        "shell did not enter the named worktree with its files and branch:\n{log}"
    );
    assert!(log.lines().any(|line| line.contains("| M file.txt;")), "shell did not enter the detached scratch worktree with its files:\n{log}");
    assert_eq!(fs::read_to_string(repo.path().join("file.txt")).unwrap(), "base\n");
    assert_eq!(fs::read_to_string(named.join("file.txt")).unwrap(), "named\n");
    assert_eq!(fs::read_to_string(scratch.join("file.txt")).unwrap(), "scratch\n");
    assert_eq!(stdout(&GitRepo::from_path(&named).git(["branch", "--show-current"])), "feature\n");
    assert_eq!(stdout(&GitRepo::from_path(&scratch).git(["branch", "--show-current"])), "");
    assert_eq!(fs::read_to_string(state_path).unwrap(), state_before);
}

#[test]
fn worktree_enter_refuses_missing_unregistered_and_ambiguous_targets() {
    let workspace = TestWorkspace::new("worktree-enter-refusals");
    let repo = workspace.git_repo("repo", "mega");
    repo.commit_file("file.txt", "base\n", "initial");
    repo.git(["branch", "scratch-collision"]);
    let home = repo.sibling("nested/home");
    let scratch = add_detached_scratch(&repo, &home, "scratch-collision");
    let named = repo.sibling("scratch-collision");
    repo.git(["worktree", "add", named.to_str().unwrap(), "scratch-collision"]);

    let missing = repo.bp(["worktree", "enter", "does-not-exist"]);
    assert!(!missing.status.success());
    assert!(stderr(&missing).contains("not a registered worktree"), "missing target refusal was unclear:\n{}", stderr(&missing));

    let unregistered = repo.sibling("unregistered");
    fs::create_dir_all(&unregistered).unwrap();
    let unregistered_output = repo.bp(["worktree", "enter", unregistered.to_str().unwrap()]);
    assert!(!unregistered_output.status.success());
    assert!(
        stderr(&unregistered_output).contains("not a registered worktree"),
        "unregistered target refusal was unclear:\n{}",
        stderr(&unregistered_output)
    );

    let ambiguous = repo.bp_with_env(["worktree", "enter", "scratch-collision"], &[("HOME", home.to_str().unwrap())]);
    assert!(!ambiguous.status.success());
    assert!(stderr(&ambiguous).contains("ambiguous"), "ambiguous target refusal was unclear:\n{}", stderr(&ambiguous));
    assert!(scratch.is_dir() && named.is_dir());
}

#[test]
fn worktree_path_resolves_registered_names_and_paths_with_clean_output() {
    let workspace = TestWorkspace::new("worktree-path-targets");
    let repo = workspace.git_repo("repo", "mega");
    repo.commit_file("file.txt", "base\n", "initial");
    repo.git(["branch", "feature"]);

    let named = repo.sibling("feature");
    repo.git(["worktree", "add", named.to_str().unwrap(), "feature"]);
    let home = repo.sibling("nested/home");
    let scratch = add_detached_scratch(&repo, &home, "scratch-path");
    let state_path = home.join(".bp/worktrees/state.toml");
    let state_before = fs::read_to_string(&state_path).unwrap();
    let worktrees_before = stdout(&repo.git(["worktree", "list", "--porcelain"]));

    let cases = [("mega", repo.path().to_path_buf()), ("feature", named.clone()), ("scratch-path", scratch.clone())];
    for (target, expected) in cases {
        let output = repo.bp_with_env(["worktree", "path", target], &[("HOME", home.to_str().unwrap())]);
        assert_success(&output, "path by registered name");
        assert_eq!(stdout(&output), format!("{}\n", fs::canonicalize(expected).unwrap().display()));
        assert_eq!(stderr(&output), "");
    }

    for expected in [repo.path().to_path_buf(), named.clone(), scratch.clone()] {
        let target = expected.to_str().unwrap();
        let output = repo.bp_with_env(["worktree", "path", target], &[("HOME", home.to_str().unwrap())]);
        assert_success(&output, "path by registered path");
        assert_eq!(stdout(&output), format!("{}\n", fs::canonicalize(expected).unwrap().display()));
        assert_eq!(stderr(&output), "");
    }

    assert_eq!(fs::read_to_string(&state_path).unwrap(), state_before);
    assert_eq!(stdout(&repo.git(["worktree", "list", "--porcelain"])), worktrees_before);
}

#[test]
fn worktree_path_refuses_unregistered_missing_and_colliding_targets() {
    let workspace = TestWorkspace::new("worktree-path-refusals");
    let repo = workspace.git_repo("repo", "mega");
    repo.commit_file("file.txt", "base\n", "initial");

    let home = repo.sibling("nested/home");
    repo.git(["branch", "scratch-collision"]);
    let scratch = add_detached_scratch(&repo, &home, "scratch-collision");
    let named = repo.sibling("scratch-collision");
    repo.git(["worktree", "add", named.to_str().unwrap(), "scratch-collision"]);

    let missing = repo.sibling("missing");
    repo.git(["worktree", "add", "--detach", missing.to_str().unwrap(), "HEAD"]);
    fs::remove_dir_all(&missing).unwrap();
    let unregistered = repo.sibling("unregistered");
    fs::create_dir_all(&unregistered).unwrap();

    let missing_name = repo.bp_with_env(["worktree", "path", "does-not-exist"], &[("HOME", home.to_str().unwrap())]);
    assert!(!missing_name.status.success());
    assert_eq!(stdout(&missing_name), "");
    assert!(stderr(&missing_name).contains("not a registered worktree"), "missing name refusal was unclear:\n{}", stderr(&missing_name));

    let unregistered_path = repo.bp_with_env(["worktree", "path", unregistered.to_str().unwrap()], &[("HOME", home.to_str().unwrap())]);
    assert!(!unregistered_path.status.success());
    assert_eq!(stdout(&unregistered_path), "");
    assert!(
        stderr(&unregistered_path).contains("not a registered worktree"),
        "unregistered path refusal was unclear:\n{}",
        stderr(&unregistered_path)
    );

    let missing_path = repo.bp_with_env(["worktree", "path", "missing"], &[("HOME", home.to_str().unwrap())]);
    assert!(!missing_path.status.success());
    assert_eq!(stdout(&missing_path), "");
    assert!(stderr(&missing_path).contains("is missing"), "missing registered path refusal was unclear:\n{}", stderr(&missing_path));

    let collision = repo.bp_with_env(["worktree", "path", "scratch-collision"], &[("HOME", home.to_str().unwrap())]);
    assert!(!collision.status.success());
    assert_eq!(stdout(&collision), "");
    assert!(stderr(&collision).contains("ambiguous"), "name collision refusal was unclear:\n{}", stderr(&collision));
    assert!(scratch.is_dir() && named.is_dir());
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

#[test]
fn worktree_remove_resolves_exact_scratch_name_and_path_without_deleting_branches() {
    let workspace = TestWorkspace::new("worktree-remove-scratch-targets");
    let repo = workspace.git_repo("repo", "mega");
    repo.commit_file("file.txt", "base\n", "initial");
    repo.git(["branch", "feature"]);

    let home = repo.sibling("nested/home");
    let by_name = add_detached_scratch(&repo, &home, "scratch-by-name");
    let by_path = add_detached_scratch(&repo, &home, "scratch-by-path");

    let output = repo.bp_with_env(["worktree", "remove", "scratch-by-name", "--no-tmux"], &[("HOME", home.to_str().unwrap())]);
    assert_success(&output, "remove scratch by exact name");
    assert!(!by_name.exists());

    let output = repo.bp_with_env(["worktree", "remove", by_path.to_str().unwrap(), "--no-tmux"], &[("HOME", home.to_str().unwrap())]);
    assert_success(&output, "remove scratch by exact path");
    assert!(!by_path.exists());
    assert_eq!(stdout(&repo.git(["branch", "--list", "feature"])), "  feature\n");
}

#[test]
fn worktree_remove_dry_run_reports_target_and_scratch_protections() {
    let workspace = TestWorkspace::new("worktree-remove-dry-run");
    let repo = workspace.git_repo("repo", "mega");
    repo.commit_file("file.txt", "base\n", "initial");
    let home = repo.sibling("home");
    let scratch = add_detached_scratch(&repo, &home, "scratch-preview");

    let output = repo.bp_with_env(["worktree", "remove", scratch.to_str().unwrap(), "--dry-run", "--no-tmux"], &[("HOME", home.to_str().unwrap())]);
    assert_success(&output, "preview scratch removal");
    let text = stdout(&output);
    assert!(text.contains("dry-run: would remove scratch worktree"), "preview omitted operation:\n{text}");
    assert!(text.contains(&scratch.display().to_string()), "preview omitted exact target:\n{text}");
    assert!(text.contains("--include-dirty"), "preview omitted dirty protection:\n{text}");
    assert!(text.contains("--include-in-use"), "preview omitted in-use protection:\n{text}");
    assert!(text.contains("branch deletion: skipped"), "preview omitted detached branch protection:\n{text}");
    assert!(scratch.is_dir(), "dry run changed the scratch path");
    assert!(stdout(&repo.git(["worktree", "list", "--porcelain"])).contains(scratch.to_str().unwrap()));
}

#[test]
fn worktree_remove_refuses_dirty_scratch_until_dirty_override_is_explicit() {
    let workspace = TestWorkspace::new("worktree-remove-dirty");
    let repo = workspace.git_repo("repo", "mega");
    repo.commit_file("file.txt", "base\n", "initial");
    let home = repo.sibling("home");
    let scratch = add_detached_scratch(&repo, &home, "scratch-dirty");
    fs::write(scratch.join("file.txt"), "dirty\n").unwrap();
    fs::write(scratch.join("untracked.txt"), "untracked\n").unwrap();

    let refused = repo.bp_with_env(["worktree", "remove", scratch.to_str().unwrap(), "--no-tmux"], &[("HOME", home.to_str().unwrap())]);
    assert!(!refused.status.success(), "dirty scratch removal unexpectedly succeeded");
    assert!(stderr(&refused).contains("--include-dirty"), "refusal omitted dirty override:\n{}", stderr(&refused));
    assert!(scratch.is_dir());

    let removed =
        repo.bp_with_env(["worktree", "remove", scratch.to_str().unwrap(), "--include-dirty", "--no-tmux"], &[("HOME", home.to_str().unwrap())]);
    assert_success(&removed, "remove dirty scratch with explicit override");
    assert!(!scratch.exists());
}

#[cfg(unix)]
#[test]
fn worktree_remove_refuses_in_use_scratch_until_in_use_override_is_explicit() {
    let workspace = TestWorkspace::new("worktree-remove-in-use");
    let repo = workspace.git_repo("repo", "mega");
    repo.commit_file("file.txt", "base\n", "initial");
    let home = repo.sibling("home");
    let scratch = add_detached_scratch(&repo, &home, "scratch-in-use");

    let mut child =
        Command::new("sleep").arg("30").current_dir(&scratch).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap();
    let refused = (0..80).find_map(|_| {
        let output = repo.bp_with_env(["worktree", "remove", scratch.to_str().unwrap(), "--no-tmux"], &[("HOME", home.to_str().unwrap())]);
        if !output.status.success() && stderr(&output).contains("--include-in-use") {
            Some(output)
        } else {
            thread::sleep(Duration::from_millis(25));
            None
        }
    });
    let Some(refused) = refused else {
        child.kill().unwrap();
        child.wait().unwrap();
        panic!("in-use scratch removal did not report a refusal");
    };
    assert!(stderr(&refused).contains("sleep"), "refusal omitted process detail:\n{}", stderr(&refused));
    assert!(scratch.is_dir());

    let removed =
        repo.bp_with_env(["worktree", "remove", scratch.to_str().unwrap(), "--include-in-use", "--no-tmux"], &[("HOME", home.to_str().unwrap())]);
    child.kill().unwrap();
    child.wait().unwrap();
    assert_success(&removed, "remove in-use scratch with explicit override");
    assert!(!scratch.exists());
}

#[test]
fn worktree_remove_refuses_unregistered_base_named_missing_and_ambiguous_targets() {
    let workspace = TestWorkspace::new("worktree-remove-target-refusals");
    let repo = workspace.git_repo("repo", "mega");
    repo.commit_file("file.txt", "base\n", "initial");
    repo.git(["branch", "feature"]);
    repo.git(["branch", "scratch-collision"]);
    let named = repo.sibling("feature");
    repo.git(["worktree", "add", named.to_str().unwrap(), "feature"]);
    let home = repo.sibling("home");
    let collision = add_detached_scratch(&repo, &home, "scratch-collision");
    let named_collision = repo.sibling("scratch-collision");
    repo.git(["worktree", "add", named_collision.to_str().unwrap(), "scratch-collision"]);

    let base = repo.bp(["worktree", "remove", "mega", "--no-tmux"]);
    assert!(!base.status.success());
    assert!(stderr(&base).contains("base worktree"));

    let missing = repo.bp(["worktree", "remove", "scratch-missing", "--no-tmux"]);
    assert!(!missing.status.success());
    assert!(stderr(&missing).contains("not registered"));

    let ambiguous = repo.bp_with_env(["worktree", "remove", "scratch-collision", "--no-tmux"], &[("HOME", home.to_str().unwrap())]);
    assert!(!ambiguous.status.success());
    assert!(stderr(&ambiguous).contains("ambiguous"));
    assert!(collision.is_dir());
    assert!(named.is_dir());
    assert!(named_collision.is_dir());
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
fn worktree_prune_previews_then_removes_clean_scratch_with_confirmation() {
    let workspace = TestWorkspace::new("worktree-prune-scratch");
    let repo = workspace.git_repo("repo", "mega");
    repo.commit_file("file.txt", "base\n", "initial");
    let home = repo.sibling("home");
    let scratch = add_detached_scratch(&repo, &home, "scratch-clean");

    let preview = repo.bp_with_env(["worktree", "prune"], &[("HOME", home.to_str().unwrap())]);
    assert_success(&preview, "preview scratch pruning");
    let preview_text = format!("{}{}", stdout(&preview), stderr(&preview));
    assert!(preview_text.contains("dry-run: no worktrees removed"), "preview did not explain its safety default:\n{preview_text}");
    assert!(preview_text.contains("would remove scratch worktree"), "preview omitted the clean scratch worktree:\n{preview_text}");
    assert!(preview_text.contains("reclaimed disk space"), "preview omitted disk-space reporting:\n{preview_text}");
    assert!(scratch.is_dir(), "dry run removed the scratch worktree");

    let confirmed = repo.bp_with_env(["worktree", "prune", "--confirm"], &[("HOME", home.to_str().unwrap())]);
    assert_success(&confirmed, "confirmed scratch pruning");
    let confirmed_text = format!("{}{}", stdout(&confirmed), stderr(&confirmed));
    assert!(confirmed_text.contains("removed scratch worktree"), "confirmed prune omitted the removal:\n{confirmed_text}");
    assert!(confirmed_text.contains("reclaimed disk space:"), "confirmed prune omitted reclaimed space:\n{confirmed_text}");
    assert!(!scratch.exists(), "confirmed prune did not remove the clean scratch worktree");
}

#[test]
fn worktree_prune_protects_dirty_named_and_young_worktrees() {
    let workspace = TestWorkspace::new("worktree-prune-protected");
    let repo = workspace.git_repo("repo", "mega");
    repo.commit_file("file.txt", "base\n", "initial");
    repo.git(["branch", "feature"]);
    let named = repo.sibling("feature");
    repo.git(["worktree", "add", named.to_str().unwrap(), "feature"]);

    let home = repo.sibling("home");
    let dirty = add_detached_scratch(&repo, &home, "scratch-dirty");
    fs::write(dirty.join("untracked.txt"), "keep me\n").unwrap();
    let young = add_detached_scratch(&repo, &home, "scratch-young");

    let output = repo.bp_with_env(["worktree", "prune", "--confirm", "--older-than", "1d"], &[("HOME", home.to_str().unwrap())]);
    assert_success(&output, "protected scratch pruning");
    let text = format!("{}{}", stdout(&output), stderr(&output));
    assert!(text.contains("dirty changes"), "dirty skip omitted an actionable reason:\n{text}");
    assert!(text.contains("younger than 1d"), "age skip omitted an actionable reason:\n{text}");
    assert!(text.contains("named worktrees are protected"), "named skip omitted an actionable reason:\n{text}");
    assert!(dirty.is_dir(), "prune removed a dirty scratch worktree");
    assert!(young.is_dir(), "prune removed a scratch worktree younger than the requested age");
    assert!(named.is_dir(), "prune removed a named worktree");
}

#[cfg(unix)]
#[test]
fn worktree_prune_protects_in_use_scratch_worktrees() {
    let workspace = TestWorkspace::new("worktree-prune-in-use");
    let repo = workspace.git_repo("repo", "mega");
    repo.commit_file("file.txt", "base\n", "initial");
    let home = repo.sibling("home");
    let scratch = add_detached_scratch(&repo, &home, "scratch-in-use");

    let mut child =
        Command::new("sleep").arg("30").current_dir(&scratch).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap();
    let output = (0..80).find_map(|_| {
        let output = repo.bp_with_env(["worktree", "prune", "--confirm"], &[("HOME", home.to_str().unwrap())]);
        if format!("{}{}", stdout(&output), stderr(&output)).contains("worktree is in use") {
            Some(output)
        } else {
            thread::sleep(Duration::from_millis(25));
            None
        }
    });

    let Some(output) = output else {
        child.kill().unwrap();
        child.wait().unwrap();
        panic!("prune did not report the in-use scratch worktree");
    };
    let text = format!("{}{}", stdout(&output), stderr(&output));
    assert!(text.contains("sleep"), "in-use skip omitted process detail:\n{text}");
    assert!(scratch.is_dir(), "prune removed an in-use scratch worktree");
    child.kill().unwrap();
    child.wait().unwrap();
}

#[test]
fn worktree_return_infers_current_scratch_and_preserves_ignored_files() {
    let workspace = TestWorkspace::new("worktree-return-clean");
    let repo = workspace.git_repo("repo", "mega");
    repo.commit_file("file.txt", "base\n", "initial");
    repo.commit_file(".gitignore", "build/\n", "ignore build cache");

    let home = repo.sibling("home");
    let scratch = add_detached_scratch(&repo, &home, "scratch-clean");
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
    let scratch = add_detached_scratch(&repo, &home, "scratch-dirty");
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
    let scratch = add_detached_scratch(&repo, &home, "scratch-in-use");

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
#[test]
fn interrupted_scratch_acquisition_is_not_reused_until_explicit_recovery() {
    use std::os::unix::fs::PermissionsExt;

    let workspace = TestWorkspace::new("worktree-state-interrupted");
    let repo = workspace.git_repo("repo", "mega");
    repo.commit_file("file.txt", "base\n", "initial");
    let home = repo.sibling("home");
    let shell = home.join("interrupt-shell");
    let shell_pid = home.join("interrupt-shell.pid");
    fs::create_dir_all(&home).unwrap();
    fs::write(&shell, format!("#!/bin/sh\necho $$ > {}\nexec sleep 30 >/dev/null 2>&1\n", shell_pid.display())).unwrap();
    fs::set_permissions(&shell, fs::Permissions::from_mode(0o755)).unwrap();

    let mut interrupted = Command::new(env!("CARGO_BIN_EXE_bp"))
        .args(["worktree", "new", "--no-submodules"])
        .current_dir(repo.path())
        .env("HOME", home.to_str().unwrap())
        .env("SHELL", shell.to_str().unwrap())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let state_path = home.join(".bp/worktrees/state.toml");
    let ready = (0..120).any(|_| {
        fs::read_to_string(&state_path).map(|state| state.contains("availability = \"leased\"") && shell_pid.is_file()).unwrap_or(false) || {
            thread::sleep(Duration::from_millis(25));
            false
        }
    });
    assert!(ready, "scratch acquisition never wrote a leased state");
    interrupted.kill().unwrap();
    let _interrupted_output = interrupted.wait_with_output().unwrap();
    if let Ok(pid) = fs::read_to_string(&shell_pid) {
        let _ = Command::new("kill").args(["-TERM", pid.trim()]).status();
    }
    let canonical_home = fs::canonicalize(&home).unwrap();
    let first_path = stdout(&repo.git(["worktree", "list", "--porcelain"]))
        .lines()
        .filter_map(|line| line.strip_prefix("worktree "))
        .find(|line| line.starts_with(canonical_home.to_str().unwrap()) && line.contains("/.bp/worktrees/scratch-"))
        .map(PathBuf::from)
        .expect("interrupted scratch worktree was not registered");
    assert!(first_path.is_dir(), "interrupted acquisition removed its worktree: {}", first_path.display());

    let next = repo.bp_with_env(["worktree", "new", "--no-submodules"], &[("HOME", home.to_str().unwrap()), ("SHELL", "/bin/sh")]);
    assert_success(&next, "new scratch after interruption");
    assert_ne!(PathBuf::from(stdout(&next).trim()), first_path);
    assert!(stderr(&next).contains("state is leased"), "interrupted state was not reported as non-reusable:\n{}", stderr(&next));

    let released = repo.bp_with_env(["worktree", "recover", "release", first_path.to_str().unwrap()], &[("HOME", home.to_str().unwrap())]);
    assert_success(&released, "release interrupted scratch");
    let reused = repo.bp_with_env(["worktree", "new", "--no-submodules"], &[("HOME", home.to_str().unwrap()), ("SHELL", "/bin/sh")]);
    assert_success(&reused, "reuse recovered scratch");
    assert_eq!(fs::canonicalize(PathBuf::from(stdout(&reused).trim())).unwrap(), fs::canonicalize(first_path).unwrap());
}

#[test]
fn missing_or_corrupt_scratch_state_quarantines_existing_worktrees() {
    let workspace = TestWorkspace::new("worktree-state-quarantine");
    let repo = workspace.git_repo("repo", "mega");
    repo.commit_file("file.txt", "base\n", "initial");
    let home = repo.sibling("home");
    let missing = add_detached_scratch(&repo, &home, "scratch-missing-state");
    let state_path = home.join(".bp/worktrees/state.toml");
    fs::remove_file(&state_path).unwrap();

    let missing_output = repo.bp_with_env(["worktree", "new", "--no-submodules"], &[("HOME", home.to_str().unwrap()), ("SHELL", "/bin/sh")]);
    assert_success(&missing_output, "new scratch with missing state");
    let missing_text = format!("{}{}", stdout(&missing_output), stderr(&missing_output));
    assert!(missing_text.contains("state is missing") && missing_text.contains("quarantined"), "missing state lacked diagnostics:\n{missing_text}");
    let released = repo.bp_with_env(["worktree", "recover", "release", missing.to_str().unwrap()], &[("HOME", home.to_str().unwrap())]);
    assert_success(&released, "release missing-state scratch");

    fs::write(&state_path, "version = 999\n").unwrap();
    let corrupt_output = repo.bp_with_env(["worktree", "new", "--no-submodules"], &[("HOME", home.to_str().unwrap()), ("SHELL", "/bin/sh")]);
    assert_success(&corrupt_output, "new scratch with corrupt state");
    let corrupt_text = format!("{}{}", stdout(&corrupt_output), stderr(&corrupt_output));
    assert!(corrupt_text.contains("state is corrupt") && corrupt_text.contains("quarantined"), "corrupt state lacked diagnostics:\n{corrupt_text}");
    assert!(fs::read_dir(home.join(".bp/worktrees")).unwrap().any(|entry| entry
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with("state.toml.corrupt.")));
    let inspected = repo.bp_with_env(["worktree", "recover", "inspect", missing.to_str().unwrap()], &[("HOME", home.to_str().unwrap())]);
    assert_success(&inspected, "inspect quarantined scratch");
    assert!(stdout(&inspected).contains("quarantined scratch worktrees"));
}

#[test]
fn destroying_quarantined_scratch_requires_confirmation_and_is_visible() {
    let workspace = TestWorkspace::new("worktree-state-destroy");
    let repo = workspace.git_repo("repo", "mega");
    repo.commit_file("file.txt", "base\n", "initial");
    let home = repo.sibling("home");
    let scratch = add_detached_scratch(&repo, &home, "scratch-quarantine-destroy");
    fs::remove_file(home.join(".bp/worktrees/state.toml")).unwrap();

    let refused = repo.bp_with_env(["worktree", "recover", "destroy", scratch.to_str().unwrap()], &[("HOME", home.to_str().unwrap())]);
    assert!(!refused.status.success());
    assert!(stderr(&refused).contains("requires --confirm"));
    assert!(scratch.is_dir());

    let destroyed = repo
        .bp_with_env(["worktree", "recover", "destroy", scratch.to_str().unwrap(), "--confirm", "--no-tmux"], &[("HOME", home.to_str().unwrap())]);
    assert_success(&destroyed, "destroy quarantined scratch");
    let text = format!("{}{}", stdout(&destroyed), stderr(&destroyed));
    assert!(text.contains("DESTRUCTIVE: destroying quarantined scratch worktree"));
    assert!(!scratch.exists());
}

fn add_detached_scratch(repo: &GitRepo, home: &Path, name: &str) -> PathBuf {
    let scratch = home.join(".bp/worktrees").join(name);
    fs::create_dir_all(scratch.parent().unwrap()).unwrap();
    repo.git(["worktree", "add", "--detach", scratch.to_str().unwrap(), "HEAD"]);
    let output = repo.bp_with_env(["worktree", "recover", "release", name], &[("HOME", home.to_str().unwrap())]);
    assert_success(&output, "register scratch state");
    scratch
}
