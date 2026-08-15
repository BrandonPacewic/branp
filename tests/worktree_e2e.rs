mod support;

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
