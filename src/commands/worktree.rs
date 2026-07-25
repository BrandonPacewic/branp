//! Git worktree helpers.

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use clap::{Arg, ArgAction, ArgMatches, Command};

use crate::context::GlobalContext;
use crate::errors::{CliError, CliResult};

pub fn command() -> Command {
    Command::new("worktree")
        .about("Manage sibling Git worktrees")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("list")
                .alias("ls")
                .about("List Git worktrees")
                .arg(
                    Arg::new("no-pr")
                        .long("no-pr")
                        .help("Skip GitHub pull request lookup")
                        .action(ArgAction::SetTrue),
                ),
        )
        .subcommand(
            Command::new("path")
                .about("Print the path for a worktree name, or the base worktree for the default branch")
                .arg(Arg::new("name").required(true).value_name("NAME")),
        )
        .subcommand(
            Command::new("new")
                .about("Create a sibling worktree")
                .arg(Arg::new("name").required(true).value_name("NAME"))
                .arg(Arg::new("branch").value_name("BRANCH"))
                .arg(
                    Arg::new("no-fetch")
                        .long("no-fetch")
                        .help("Do not fetch origin before creating the worktree")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("no-submodules")
                        .long("no-submodules")
                        .help("Skip submodule initialization")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("no-tmux")
                        .long("no-tmux")
                        .help("Do not start a tmux session")
                        .action(ArgAction::SetTrue),
                ),
        )
        .subcommand(
            Command::new("remove")
                .alias("rm")
                .about("Remove a sibling worktree and its local branch")
                .arg(Arg::new("name").required(true).value_name("NAME"))
                .arg(
                    Arg::new("force")
                        .short('f')
                        .long("force")
                        .help("Force worktree removal and branch deletion")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("keep-branch")
                        .long("keep-branch")
                        .help("Do not delete the local branch")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("no-tmux")
                        .long("no-tmux")
                        .help("Do not kill the matching tmux session")
                        .action(ArgAction::SetTrue),
                ),
        )
        .subcommand(Command::new("prune").about("Run git worktree prune"))
        .subcommand(
            Command::new("gone")
                .about("Remove sibling worktrees whose origin branch no longer exists")
                .arg(
                    Arg::new("dry-run")
                        .short('n')
                        .long("dry-run")
                        .help("Show what would be removed")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("force")
                        .short('f')
                        .long("force")
                        .help("Force worktree removal and branch deletion")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("no-tmux")
                        .long("no-tmux")
                        .help("Do not kill matching tmux sessions")
                        .action(ArgAction::SetTrue),
                ),
        )
}

pub fn exec(gctx: &mut GlobalContext, args: &ArgMatches) -> CliResult {
    let repo = Repo::discover(gctx.cwd())?;

    match args.subcommand() {
        Some(("list", sub)) => list(gctx, &repo, !sub.get_flag("no-pr")),
        Some(("path", sub)) => {
            let name = required(sub, "name")?;
            let path = if name == repo.default_branch {
                repo.base.clone()
            } else {
                repo.target_dir(name)
            };
            gctx.shell().note(path.display());
            Ok(())
        }
        Some(("new", sub)) => {
            let name = required(sub, "name")?;
            let branch = sub.get_one::<String>("branch").map_or(name, String::as_str);
            create(
                gctx,
                &repo,
                name,
                branch,
                !sub.get_flag("no-fetch"),
                !sub.get_flag("no-submodules"),
                !sub.get_flag("no-tmux"),
            )
        }
        Some(("remove", sub)) => remove(
            gctx,
            &repo,
            required(sub, "name")?,
            sub.get_flag("force"),
            !sub.get_flag("keep-branch"),
            !sub.get_flag("no-tmux"),
        ),
        Some(("prune", _)) => {
            run_git(&repo.base, &["worktree", "prune"])?;
            Ok(())
        }
        Some(("gone", sub)) => remove_gone(
            gctx,
            &repo,
            sub.get_flag("dry-run"),
            sub.get_flag("force"),
            !sub.get_flag("no-tmux"),
        ),
        _ => Err(CliError::from("no `worktree` subcommand provided")),
    }
}

fn list(gctx: &mut GlobalContext, repo: &Repo, pr_lookup: bool) -> CliResult {
    let worktrees = worktrees(&repo.base)?;
    let path_root = common_parent(&worktrees);
    let verbose = gctx.is_verbose();
    let pr_numbers = if pr_lookup {
        crate::utils::gh::open_pull_requests(&repo.base).unwrap_or_default()
    } else {
        Vec::new()
    };
    let mut rows = Vec::new();

    for worktree in worktrees {
        let branch = worktree.branch.as_deref().unwrap_or("detached");
        let status = status_summary(&worktree.path)?;
        let dirty = status.is_dirty();
        let base = worktree.path == repo.base;
        let remote_exists = worktree
            .branch
            .as_deref()
            .is_some_and(|branch| remote_branch_exists(&repo.base, branch).unwrap_or(false));
        let remote_gone = !base
            && worktree
                .branch
                .as_deref()
                .is_some_and(|branch| branch != repo.default_branch)
            && !remote_exists;
        let pr_number = worktree
            .branch
            .as_deref()
            .and_then(|branch| pr_numbers.iter().find(|pr| pr.head == branch))
            .map(|pr| pr.number);
        let has_submodules = has_submodules(&worktree.path);
        let tmux_session = repo
            .name_from_path(&worktree.path)
            .map(|name| repo.session_name(&name))
            .filter(|session| crate::utils::tmux::has_session(session).unwrap_or(false));

        rows.push(WorktreeRow {
            path: display_path(&worktree.path, &path_root),
            branch: branch.to_string(),
            head: worktree
                .head
                .as_deref()
                .and_then(|head| head.get(..7))
                .unwrap_or("")
                .to_string(),
            dirty,
            remote_gone,
            remote_exists,
            base,
            verbose,
            pr_number,
            has_submodules,
            tmux_session,
            status,
        });
    }

    let [path_width, branch_width, head_width] = crate::utils::table::column_widths(
        rows.iter()
            .map(|row| [row.path.as_str(), row.branch.as_str(), row.head.as_str()]),
    );
    let mut badge_widths = [0; 6];
    for row in &rows {
        let badges = row.badges();
        for (index, width) in badges.widths().into_iter().enumerate() {
            badge_widths[index] = badge_widths[index].max(width);
        }
    }

    for row in rows {
        gctx.shell()
            .note(row.format(path_width, branch_width, head_width, badge_widths));
    }

    Ok(())
}

fn create(
    gctx: &mut GlobalContext,
    repo: &Repo,
    name: &str,
    branch: &str,
    fetch: bool,
    submodules: bool,
    tmux: bool,
) -> CliResult {
    let target = repo.target_dir(name);
    if target.exists() {
        return Err(CliError::from(format!(
            "worktree already exists: {}",
            target.display()
        )));
    }

    if fetch {
        let _ = run_git(&repo.base, &["fetch", "origin", branch]);
    }

    if local_branch_exists(&repo.base, branch)? {
        return Err(CliError::from(format!(
            "local branch already exists: {branch}"
        )));
    }

    if remote_branch_exists(&repo.base, branch)? {
        run_git(
            &repo.base,
            &[
                "worktree",
                "add",
                path_arg(&target).as_str(),
                "-b",
                branch,
                &format!("origin/{branch}"),
            ],
        )?;
    } else {
        run_git(
            &repo.base,
            &["worktree", "add", path_arg(&target).as_str(), "-b", branch],
        )?;
    }

    if submodules {
        run_git(&target, &["submodule", "update", "--init", "--recursive"])?;
    }

    if tmux {
        crate::utils::tmux::ensure_session(gctx, &repo.session_name(name), &target)?;
    }

    gctx.shell().note(target.display());
    Ok(())
}

fn remove(
    gctx: &mut GlobalContext,
    repo: &Repo,
    name: &str,
    force: bool,
    delete_branch: bool,
    tmux: bool,
) -> CliResult {
    let target = repo.target_dir(name);
    if !target.is_dir() {
        return Err(CliError::from(format!(
            "worktree not found: {}",
            target.display()
        )));
    }

    let branch = current_branch(&target)?;
    let confirmed_lossy_remove = confirm_lossy_remove(gctx, &target)?;
    deinit_submodules(gctx, &target)?;

    let mut remove_args = vec!["worktree", "remove"];
    if force || confirmed_lossy_remove {
        remove_args.push("--force");
    }
    let target_arg = path_arg(&target);
    remove_args.push(target_arg.as_str());
    run_git(&repo.base, &remove_args)?;
    run_git(&repo.base, &["worktree", "prune"])?;

    if delete_branch {
        if let Some(branch) = branch {
            delete_local_branch(&repo.base, &branch, force)?;
        }
    }

    if tmux {
        crate::utils::tmux::kill_session(gctx, &repo.session_name(name))?;
    }

    Ok(())
}

fn remove_gone(
    gctx: &mut GlobalContext,
    repo: &Repo,
    dry_run: bool,
    force: bool,
    tmux: bool,
) -> CliResult {
    run_git(&repo.base, &["fetch", "--prune"])?;
    let worktrees = worktrees(&repo.base)?;

    for worktree in worktrees {
        if worktree.path == repo.base {
            continue;
        }

        let Some(branch) = worktree.branch else {
            continue;
        };
        if branch == repo.default_branch || remote_branch_exists(&repo.base, &branch)? {
            continue;
        }

        let Some(name) = repo.name_from_path(&worktree.path) else {
            gctx.shell().warn(format!(
                "skipping non-sibling worktree for branch `{branch}`: {}",
                worktree.path.display()
            ));
            continue;
        };

        if dry_run {
            gctx.shell().note(format!("{name} ({branch})"));
        } else {
            remove(gctx, repo, &name, force, true, tmux)?;
        }
    }

    Ok(())
}

struct Repo {
    base: PathBuf,
    default_branch: String,
}

impl Repo {
    fn discover(cwd: &Path) -> Result<Self, CliError> {
        let root = PathBuf::from(git_output(cwd, &["rev-parse", "--show-toplevel"])?.trim());
        let base = worktrees(&root)?
            .into_iter()
            .next()
            .map(|w| w.path)
            .ok_or("could not determine base worktree")?;
        let default_branch = git_output(
            &base,
            &["symbolic-ref", "--short", "refs/remotes/origin/HEAD"],
        )
        .ok()
        .and_then(|s| s.trim().strip_prefix("origin/").map(str::to_string))
        .unwrap_or_else(|| "main".to_string());

        Ok(Self {
            base,
            default_branch,
        })
    }

    fn target_dir(&self, name: &str) -> PathBuf {
        PathBuf::from(format!("{}-{name}", self.base.display()))
    }

    fn session_name(&self, name: &str) -> String {
        let repo_name = self
            .base
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("worktree");
        format!("{repo_name}-{name}")
    }

    fn name_from_path(&self, path: &Path) -> Option<String> {
        path.to_str()?
            .strip_prefix(&format!("{}-", self.base.display()))
            .map(str::to_string)
    }
}

struct Worktree {
    path: PathBuf,
    head: Option<String>,
    branch: Option<String>,
}

fn worktrees(repo: &Path) -> Result<Vec<Worktree>, CliError> {
    let output = git_output(repo, &["worktree", "list", "--porcelain"])?;
    let mut items = Vec::new();
    let mut path = None;
    let mut head = None;
    let mut branch = None;

    for line in output.lines().chain(std::iter::once("")) {
        if line.is_empty() {
            if let Some(path) = path.take() {
                items.push(Worktree {
                    path,
                    head: head.take(),
                    branch: branch.take(),
                });
            }
        } else if let Some(value) = line.strip_prefix("worktree ") {
            path = Some(PathBuf::from(value));
        } else if let Some(value) = line.strip_prefix("HEAD ") {
            head = Some(value.to_string());
        } else if let Some(value) = line.strip_prefix("branch refs/heads/") {
            branch = Some(value.to_string());
        }
    }

    Ok(items)
}

#[derive(Default)]
struct StatusSummary {
    staged: usize,
    modified: usize,
    deleted: usize,
    untracked: usize,
    submodule: usize,
}

impl StatusSummary {
    fn is_dirty(&self) -> bool {
        self.staged > 0
            || self.modified > 0
            || self.deleted > 0
            || self.untracked > 0
            || self.submodule > 0
    }

    fn tracked_badge(&self) -> String {
        let mut statuses = Vec::new();
        let staged = count_badge("staged", self.staged);
        if !staged.is_empty() {
            statuses.push(staged);
        }
        let modified = count_badge("modified", self.modified);
        if !modified.is_empty() {
            statuses.push(modified);
        }
        let deleted = count_badge("deleted", self.deleted);
        if !deleted.is_empty() {
            statuses.push(deleted);
        }
        let submodule = count_badge("submodule-dirty", self.submodule);
        if !submodule.is_empty() {
            statuses.push(submodule);
        }

        statuses.join(",")
    }
}

struct WorktreeRow {
    path: String,
    branch: String,
    head: String,
    dirty: bool,
    remote_gone: bool,
    remote_exists: bool,
    base: bool,
    verbose: bool,
    pr_number: Option<u64>,
    has_submodules: bool,
    tmux_session: Option<String>,
    status: StatusSummary,
}

impl WorktreeRow {
    fn badges(&self) -> WorktreeBadges {
        let (state, state_color) = if self.base {
            ("base".to_string(), BadgeColor::Green)
        } else if !self.dirty {
            ("clean".to_string(), BadgeColor::Green)
        } else {
            (self.status.tracked_badge(), BadgeColor::YellowBold)
        };

        WorktreeBadges {
            state,
            state_color,
            untracked: count_badge("untracked", self.status.untracked),
            remote: if self.remote_gone {
                "prunable".to_string()
            } else if self.remote_exists {
                "remote-ok".to_string()
            } else {
                String::new()
            },
            pr: self
                .pr_number
                .map(|number| format!("PR #{number}"))
                .unwrap_or_default(),
            submodules: if self.has_submodules && self.verbose {
                "submodules".to_string()
            } else {
                String::new()
            },
            tmux: self
                .tmux_session
                .as_deref()
                .map(|session| format!("tmux:{session}"))
                .unwrap_or_default(),
        }
    }

    fn format(
        &self,
        path_width: usize,
        branch_width: usize,
        head_width: usize,
        badge_widths: [usize; 6],
    ) -> String {
        let path = crate::utils::table::pad_cell(&self.path, path_width);
        let path = if self.dirty {
            color_print::cformat!("<yellow,bold>{path}</>")
        } else if self.base {
            color_print::cformat!("<green,bold>{path}</>")
        } else {
            color_print::cformat!("<cyan>{path}</>")
        };

        let branch = crate::utils::table::pad_cell(&self.branch, branch_width);
        let branch = color_print::cformat!("<blue>{branch}</>");

        let head = crate::utils::table::pad_cell(&self.head, head_width);
        let head = color_print::cformat!("<black!>{head}</>");

        let badges = self.badges();
        let badge_cells = [
            color_badge(&badges.state, badge_widths[0], badges.state_color),
            color_badge(&badges.untracked, badge_widths[1], BadgeColor::MagentaBold),
            if self.remote_gone {
                color_badge(&badges.remote, badge_widths[2], BadgeColor::RedBold)
            } else {
                color_badge(&badges.remote, badge_widths[2], BadgeColor::Green)
            },
            color_badge(&badges.pr, badge_widths[3], BadgeColor::Magenta),
            color_badge(&badges.submodules, badge_widths[4], BadgeColor::Yellow),
            color_badge(&badges.tmux, badge_widths[5], BadgeColor::Cyan),
        ];

        format!("{path}  {branch}  {head}  {}", badge_cells.join("  "))
    }
}

struct WorktreeBadges {
    state: String,
    state_color: BadgeColor,
    untracked: String,
    remote: String,
    pr: String,
    submodules: String,
    tmux: String,
}

impl WorktreeBadges {
    fn widths(&self) -> [usize; 6] {
        [
            self.state.len(),
            self.untracked.len(),
            self.remote.len(),
            self.pr.len(),
            self.submodules.len(),
            self.tmux.len(),
        ]
    }
}

fn count_badge(label: &str, count: usize) -> String {
    if count > 0 {
        format!("{label}:{count}")
    } else {
        String::new()
    }
}

#[derive(Clone, Copy)]
enum BadgeColor {
    Green,
    Yellow,
    YellowBold,
    RedBold,
    Magenta,
    MagentaBold,
    Cyan,
}

fn color_badge(value: &str, width: usize, color: BadgeColor) -> String {
    let value = crate::utils::table::pad_cell(value, width);
    if value.trim().is_empty() {
        value
    } else {
        match color {
            BadgeColor::Green => color_print::cformat!("<green>{value}</>"),
            BadgeColor::Yellow => color_print::cformat!("<yellow>{value}</>"),
            BadgeColor::YellowBold => color_print::cformat!("<yellow,bold>{value}</>"),
            BadgeColor::RedBold => color_print::cformat!("<red,bold>{value}</>"),
            BadgeColor::Magenta => color_print::cformat!("<magenta>{value}</>"),
            BadgeColor::MagentaBold => color_print::cformat!("<magenta,bold>{value}</>"),
            BadgeColor::Cyan => color_print::cformat!("<cyan>{value}</>"),
        }
    }
}

fn common_parent(worktrees: &[Worktree]) -> PathBuf {
    let Some(first) = worktrees.first() else {
        return PathBuf::new();
    };
    let mut common = first.path.parent().unwrap_or(&first.path).to_path_buf();

    for worktree in &worktrees[1..] {
        while !worktree.path.starts_with(&common) {
            if !common.pop() {
                return PathBuf::new();
            }
        }
    }

    common
}

fn display_path(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .ok()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(path)
        .display()
        .to_string()
}

fn status_summary(repo: &Path) -> Result<StatusSummary, CliError> {
    let output = git_output(repo, &["status", "--porcelain"])?;
    let mut summary = StatusSummary::default();

    for line in output.lines() {
        let x = line.chars().next().unwrap_or(' ');
        let y = line.chars().nth(1).unwrap_or(' ');

        if line.starts_with("??") {
            summary.untracked += 1;
            continue;
        }

        if x != ' ' {
            summary.staged += 1;
        }

        match y {
            'M' => summary.modified += 1,
            'D' => summary.deleted += 1,
            '?' => summary.untracked += 1,
            'm' | 'U' => summary.submodule += 1,
            _ => {}
        }
    }

    Ok(summary)
}

fn current_branch(repo: &Path) -> Result<Option<String>, CliError> {
    let output = git_output(repo, &["branch", "--show-current"])?;
    let branch = output.trim();
    Ok((!branch.is_empty()).then(|| branch.to_string()))
}

fn local_branch_exists(repo: &Path, branch: &str) -> Result<bool, CliError> {
    git_status(
        repo,
        &[
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ],
    )
}

fn remote_branch_exists(repo: &Path, branch: &str) -> Result<bool, CliError> {
    git_status(
        repo,
        &[
            "show-ref",
            "--quiet",
            &format!("refs/remotes/origin/{branch}"),
        ],
    )
}

fn delete_local_branch(repo: &Path, branch: &str, force: bool) -> CliResult {
    if !local_branch_exists(repo, branch)? {
        return Ok(());
    }

    let flag = if force { "-D" } else { "-d" };
    run_git(repo, &["branch", flag, branch])
}

fn confirm_lossy_remove(gctx: &mut GlobalContext, repo: &Path) -> Result<bool, CliError> {
    let changes = removal_risk_changes(repo)?;
    if changes.is_empty() {
        return Ok(false);
    }

    gctx.shell().warn(format!(
        "removing this worktree will lose unstaged changes in {}:",
        repo.display()
    ));
    for change in changes {
        gctx.shell().warn(format!("  {change}"));
    }

    eprint!("Type `yes` to remove the worktree anyway: ");
    io::stderr().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    if input.trim() == "yes" {
        Ok(true)
    } else {
        Err(CliError::from("worktree removal cancelled"))
    }
}

fn removal_risk_changes(repo: &Path) -> Result<Vec<String>, CliError> {
    let mut changes = collect_status_changes(repo, None)?;
    for submodule in submodule_paths(repo)? {
        let submodule_repo = repo.join(&submodule);
        if !submodule_repo.is_dir() {
            continue;
        }

        changes.extend(collect_status_changes(&submodule_repo, Some(&submodule))?);
    }

    Ok(changes)
}

fn collect_status_changes(repo: &Path, prefix: Option<&str>) -> Result<Vec<String>, CliError> {
    let output = git_output(repo, &["status", "--porcelain"])?;
    let mut changes = Vec::new();

    for line in output.lines() {
        if is_lossy_status(line) {
            changes.push(format_status_line(line, prefix));
        }
    }

    Ok(changes)
}

fn is_lossy_status(line: &str) -> bool {
    line.starts_with("??") || line.chars().nth(1).is_some_and(|status| status != ' ')
}

fn format_status_line(line: &str, prefix: Option<&str>) -> String {
    let status = line.get(..2).unwrap_or("").trim();
    let path = line.get(3..).unwrap_or("").trim();

    if let Some(prefix) = prefix {
        format!("{status} {prefix}/{path}")
    } else {
        format!("{status} {path}")
    }
}

fn submodule_paths(repo: &Path) -> Result<Vec<String>, CliError> {
    if !has_submodules(repo) {
        return Ok(Vec::new());
    }

    let output = git_output(
        repo,
        &[
            "config",
            "--file",
            ".gitmodules",
            "--get-regexp",
            r"^submodule\..*\.path$",
        ],
    )?;

    Ok(output
        .lines()
        .filter_map(|line| line.split_once(' ').map(|(_, path)| path.to_string()))
        .collect())
}

fn deinit_submodules(gctx: &mut GlobalContext, repo: &Path) -> CliResult {
    if !has_submodules(repo) {
        return Ok(());
    }

    run_git(repo, &["submodule", "deinit", "--force", "--all"])?;
    gctx.shell().note("submodules: deinitialized");
    Ok(())
}

fn has_submodules(repo: &Path) -> bool {
    repo.join(".gitmodules").is_file()
}

fn run_git(repo: &Path, args: &[&str]) -> CliResult {
    crate::utils::command::run("git", args, Some(repo))
}

fn git_output(repo: &Path, args: &[&str]) -> Result<String, CliError> {
    crate::utils::command::output("git", args, Some(repo))
}

fn git_status(repo: &Path, args: &[&str]) -> Result<bool, CliError> {
    crate::utils::command::status("git", args, Some(repo))
}

fn required<'a>(args: &'a ArgMatches, name: &str) -> Result<&'a str, CliError> {
    args.get_one::<String>(name)
        .map(String::as_str)
        .ok_or_else(|| CliError::from(format!("missing required argument `{name}`")))
}

fn path_arg(path: &Path) -> String {
    path.to_string_lossy().to_string()
}
