//! Git worktree helpers.

use std::collections::HashSet;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;

use clap::{Arg, ArgAction, ArgMatches, Command};

use crate::context::GlobalContext;
use crate::errors::{CliError, CliResult};
use crate::ops::worktree::LinkStore;
use crate::utils::git;

pub fn command() -> Command {
    Command::new("worktree")
        .about("Manage sibling Git worktrees")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("list")
                .alias("ls")
                .about("List Git worktrees")
                .arg(Arg::new("no-pr").long("no-pr").help("Skip GitHub pull request lookup").action(ArgAction::SetTrue)),
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
                .arg(Arg::new("no-fetch").long("no-fetch").help("Do not fetch origin before creating the worktree").action(ArgAction::SetTrue))
                .arg(Arg::new("no-submodules").long("no-submodules").help("Skip submodule initialization").action(ArgAction::SetTrue))
                .arg(Arg::new("no-tmux").long("no-tmux").help("Do not start a tmux session").action(ArgAction::SetTrue)),
        )
        .subcommand(
            Command::new("remove")
                .alias("rm")
                .about("Remove a sibling worktree and its local branch")
                .arg(Arg::new("name").required(true).value_name("NAME"))
                .arg(Arg::new("force").short('f').long("force").help("Force worktree removal and branch deletion").action(ArgAction::SetTrue))
                .arg(Arg::new("keep-branch").long("keep-branch").help("Do not delete the local branch").action(ArgAction::SetTrue))
                .arg(Arg::new("no-tmux").long("no-tmux").help("Do not kill the matching tmux session").action(ArgAction::SetTrue)),
        )
        .subcommand(Command::new("prune").about("Run git worktree prune"))
        .subcommand(
            Command::new("gone")
                .about("Remove sibling worktrees whose origin branch no longer exists")
                .arg(Arg::new("dry-run").short('n').long("dry-run").help("Show what would be removed").action(ArgAction::SetTrue))
                .arg(Arg::new("force").short('f').long("force").help("Force worktree removal and branch deletion").action(ArgAction::SetTrue))
                .arg(Arg::new("no-tmux").long("no-tmux").help("Do not kill matching tmux sessions").action(ArgAction::SetTrue)),
        )
        .subcommand(
            Command::new("track")
                .about("Track untracked files that should be symlinked into every worktree")
                .arg(Arg::new("path").required(true).num_args(1..).value_name("PATH"))
                .arg(Arg::new("force").short('f').long("force").help("Replace existing files in other worktrees").action(ArgAction::SetTrue)),
        )
        .subcommand(Command::new("links").about("List files linked across worktrees"))
        .subcommand(
            Command::new("sync")
                .about("Create or repair symlinks for files tracked in .branp/worktree.toml")
                .arg(Arg::new("force").short('f').long("force").help("Replace existing non-symlink paths").action(ArgAction::SetTrue)),
        )
}

pub fn exec(gctx: &mut GlobalContext, args: &ArgMatches) -> CliResult {
    let repo = Repo::discover(gctx.cwd())?;

    match args.subcommand() {
        Some(("list", sub)) => list(gctx, &repo, !sub.get_flag("no-pr")),
        Some(("path", sub)) => {
            let name = required(sub, "name")?;
            let path = if name == repo.default_branch { repo.base.clone() } else { repo.target_dir(name) };
            gctx.shell().note(path.display());
            Ok(())
        }
        Some(("new", sub)) => {
            let name = required(sub, "name")?;
            let branch = sub.get_one::<String>("branch").map_or(name, String::as_str);
            create(gctx, &repo, name, branch, !sub.get_flag("no-fetch"), !sub.get_flag("no-submodules"), !sub.get_flag("no-tmux"))
        }
        Some(("remove", sub)) => {
            remove(gctx, &repo, required(sub, "name")?, sub.get_flag("force"), !sub.get_flag("keep-branch"), !sub.get_flag("no-tmux"))
        }
        Some(("prune", _)) => {
            git::run(&repo.base, &["worktree", "prune"])?;
            Ok(())
        }
        Some(("gone", sub)) => remove_gone(gctx, &repo, sub.get_flag("dry-run"), sub.get_flag("force"), !sub.get_flag("no-tmux")),
        Some(("track", sub)) => track_links(gctx, &repo, required_many(sub, "path")?, sub.get_flag("force")),
        Some(("links", _)) => list_links(gctx, &repo),
        Some(("sync", sub)) => sync_links(gctx, &repo, None, sub.get_flag("force")),
        _ => Err(CliError::from("no `worktree` subcommand provided")),
    }
}

fn list(gctx: &mut GlobalContext, repo: &Repo, pr_lookup: bool) -> CliResult {
    if gctx.shell().is_quiet() {
        return Ok(());
    }

    let worktrees = worktrees(&repo.base)?;
    let path_root = common_parent(&worktrees);
    let verbose = gctx.is_verbose();
    if !crate::utils::terminal::supports_dynamic_lines() {
        let rows = resolve_worktree_rows(repo, &worktrees, &path_root, verbose)?;
        let pr_numbers = if pr_lookup { crate::utils::gh::open_pull_requests(&repo.base).unwrap_or_default() } else { Vec::new() };
        let pr_render = if pr_lookup { PrRender::Resolved(&pr_numbers) } else { PrRender::Disabled };

        for line in render_worktree_rows(&rows, pr_render, None) {
            gctx.shell().note(line);
        }
        return Ok(());
    }

    render_worktree_rows_dynamic(repo, worktrees, &path_root, verbose, pr_lookup)
}

fn resolve_worktree_rows(repo: &Repo, worktrees: &[Worktree], path_root: &Path, verbose: bool) -> Result<Vec<WorktreeRow>, CliError> {
    let remote_branches = git::remote_branches(&repo.base, "origin").unwrap_or_default();
    let tmux_sessions: HashSet<_> = crate::utils::tmux::sessions().unwrap_or_default().into_iter().collect();
    let mut rows: Vec<_> = worktrees.iter().map(|worktree| WorktreeRow::from_worktree(repo, worktree, path_root, verbose)).collect();
    let handles: Vec<_> = worktrees
        .iter()
        .enumerate()
        .map(|(index, worktree)| {
            let path = worktree.path.clone();
            thread::spawn(move || (index, status_summary(&path).unwrap_or_default(), has_submodules(&path)))
        })
        .collect();

    for handle in handles {
        let (index, status, has_submodules) = handle.join().map_err(|_| CliError::from("worktree status lookup panicked"))?;
        if let Some(row) = rows.get_mut(index) {
            row.apply_status(status);
            row.has_submodules = Some(has_submodules);
        }
    }

    for row in &mut rows {
        row.apply_remote_branches(&remote_branches, &repo.default_branch);
        row.apply_tmux_sessions(&tmux_sessions);
    }

    Ok(rows)
}

fn render_worktree_rows_dynamic(repo: &Repo, worktrees: Vec<Worktree>, path_root: &Path, verbose: bool, pr_lookup: bool) -> CliResult {
    let rows: Vec<_> = worktrees.iter().map(|worktree| WorktreeRow::from_worktree(repo, worktree, path_root, verbose)).collect();
    let (tx, rx) = mpsc::channel();
    let mut pending = 0;

    for (index, worktree) in worktrees.iter().enumerate() {
        let tx = tx.clone();
        let path = worktree.path.clone();
        thread::spawn(move || {
            let _ =
                tx.send(WorktreeUpdate::Status { index, status: status_summary(&path).unwrap_or_default(), has_submodules: has_submodules(&path) });
        });
        pending += 1;
    }

    {
        let tx = tx.clone();
        let repo_base = repo.base.clone();
        thread::spawn(move || {
            let _ = tx.send(WorktreeUpdate::RemoteBranches(git::remote_branches(&repo_base, "origin").unwrap_or_default()));
        });
        pending += 1;
    }

    {
        let tx = tx.clone();
        thread::spawn(move || {
            let sessions = crate::utils::tmux::sessions().unwrap_or_default().into_iter().collect();
            let _ = tx.send(WorktreeUpdate::TmuxSessions(sessions));
        });
        pending += 1;
    }

    if pr_lookup {
        let tx = tx.clone();
        let repo_base = repo.base.clone();
        thread::spawn(move || {
            let _ = tx.send(WorktreeUpdate::PullRequests(crate::utils::gh::open_pull_requests(&repo_base).unwrap_or_default()));
        });
        pending += 1;
    }
    drop(tx);

    let mut state = WorktreeRenderState { rows, default_branch: repo.default_branch.clone(), pr_lookup, pr_numbers: None };
    crate::utils::terminal::render_dynamic_updates(
        &mut state,
        pending,
        rx,
        |state, spinner| {
            render_worktree_rows(&state.rows, pr_render_state(state.pr_lookup, state.pr_numbers.as_deref(), spinner.unwrap_or('-')), spinner)
        },
        WorktreeRenderState::apply,
    )?;
    Ok(())
}

fn create(gctx: &mut GlobalContext, repo: &Repo, name: &str, branch: &str, fetch: bool, submodules: bool, tmux: bool) -> CliResult {
    let target = repo.target_dir(name);
    if target.exists() {
        return Err(CliError::from(format!("worktree already exists: {}", target.display())));
    }

    let start_point = confirm_default_source_branch(gctx, repo)?;

    if fetch {
        let _ = git::run(&repo.base, &["fetch", "origin", branch]);
    }

    if git::local_branch_exists(&repo.base, branch)? {
        return Err(CliError::from(format!("local branch already exists: {branch}")));
    }

    if git::remote_branch_exists(&repo.base, "origin", branch)? {
        git::run(&repo.base, &["worktree", "add", path_arg(&target).as_str(), "-b", branch, &format!("origin/{branch}")])?;
    } else if let Some(start_point) = start_point {
        git::run(&repo.base, &["worktree", "add", path_arg(&target).as_str(), "-b", branch, &start_point])?;
    } else {
        git::run(&repo.base, &["worktree", "add", path_arg(&target).as_str(), "-b", branch])?;
    }

    if submodules {
        git::run(&target, &["submodule", "update", "--init", "--recursive"])?;
    }

    sync_links(gctx, repo, Some(&target), false)?;

    if tmux {
        crate::utils::tmux::ensure_session(gctx, &repo.session_name(name), &target)?;
    }

    gctx.shell().note(target.display());
    Ok(())
}

fn remove(gctx: &mut GlobalContext, repo: &Repo, name: &str, force: bool, delete_branch: bool, tmux: bool) -> CliResult {
    let target = repo.target_dir(name);
    if !target.is_dir() {
        return Err(CliError::from(format!("worktree not found: {}", target.display())));
    }

    let branch = git::current_branch(&target)?;
    if delete_branch {
        if let Some(branch) = &branch {
            preflight_branch_delete(&repo.base, branch, force)?;
        }
    }
    let target_has_submodules = has_submodules(&target);
    let confirmed_lossy_remove = confirm_lossy_remove(gctx, &target)?;
    deinit_submodules(gctx, &target)?;

    let target_arg = path_arg(&target);
    let remove_args = worktree_remove_args(target_arg.as_str(), force, confirmed_lossy_remove, target_has_submodules);
    git::run(&repo.base, &remove_args)?;
    git::run(&repo.base, &["worktree", "prune"])?;

    if delete_branch {
        if let Some(branch) = branch {
            git::delete_local_branch(&repo.base, &branch, force)?;
        }
    }

    if tmux {
        crate::utils::tmux::kill_session(gctx, &repo.session_name(name))?;
    }

    Ok(())
}

fn remove_gone(gctx: &mut GlobalContext, repo: &Repo, dry_run: bool, force: bool, tmux: bool) -> CliResult {
    git::run(&repo.base, &["fetch", "--prune"])?;
    let worktrees = worktrees(&repo.base)?;

    for worktree in worktrees {
        if worktree.path == repo.base {
            continue;
        }

        let Some(branch) = worktree.branch else {
            continue;
        };
        if branch == repo.default_branch || git::remote_branch_exists(&repo.base, "origin", &branch)? {
            continue;
        }

        let Some(name) = repo.name_from_path(&worktree.path) else {
            gctx.shell().warn(format!("skipping non-sibling worktree for branch `{branch}`: {}", worktree.path.display()));
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
    current: PathBuf,
    default_branch: String,
}

impl Repo {
    fn discover(cwd: &Path) -> Result<Self, CliError> {
        let root = PathBuf::from(git::output(cwd, &["rev-parse", "--show-toplevel"])?.trim());
        let base = worktrees(&root)?.into_iter().next().map(|w| w.path).ok_or("could not determine base worktree")?;
        let default_branch = default_branch(&base)?;

        Ok(Self { base, current: root, default_branch })
    }

    fn target_dir(&self, name: &str) -> PathBuf {
        PathBuf::from(format!("{}-{name}", self.base.display()))
    }

    fn session_name(&self, name: &str) -> String {
        session_name_for(&self.base, name)
    }

    fn name_from_path(&self, path: &Path) -> Option<String> {
        name_from_worktree_path(&self.base, path)
    }

    fn link_store(&self) -> LinkStore {
        LinkStore::new(&self.base, &self.current)
    }
}

fn track_links(gctx: &mut GlobalContext, repo: &Repo, paths: Vec<&str>, force: bool) -> CliResult {
    let tracked = repo.link_store().track(&paths, &worktree_paths(repo)?, force)?;

    for rel in tracked {
        gctx.shell().note(format!("linked {}", rel.display()));
    }
    Ok(())
}

fn list_links(gctx: &mut GlobalContext, repo: &Repo) -> CliResult {
    for rel in repo.link_store().linked_paths()? {
        gctx.shell().note(rel.display());
    }
    Ok(())
}

fn sync_links(gctx: &mut GlobalContext, repo: &Repo, only_worktree: Option<&Path>, force: bool) -> CliResult {
    let worktrees = if let Some(worktree) = only_worktree { vec![worktree.to_path_buf()] } else { worktree_paths(repo)? };
    repo.link_store().sync(&worktrees, force)?;

    if only_worktree.is_none() {
        gctx.shell().note("worktree links synced");
    }
    Ok(())
}

fn default_branch(repo: &Path) -> Result<String, CliError> {
    git::default_branch(repo, "origin")
}

fn session_name_for(base: &Path, name: &str) -> String {
    let repo_name = base.file_name().and_then(|name| name.to_str()).unwrap_or("worktree");
    format!("{repo_name}-{name}")
}

fn name_from_worktree_path(base: &Path, path: &Path) -> Option<String> {
    path.to_str()?.strip_prefix(&format!("{}-", base.display())).map(str::to_string)
}

struct Worktree {
    path: PathBuf,
    head: Option<String>,
    branch: Option<String>,
}

fn worktrees(repo: &Path) -> Result<Vec<Worktree>, CliError> {
    let output = git::output(repo, &["worktree", "list", "--porcelain"])?;
    let mut items = Vec::new();
    let mut path = None;
    let mut head = None;
    let mut branch = None;

    for line in output.lines().chain(std::iter::once("")) {
        if line.is_empty() {
            if let Some(path) = path.take() {
                items.push(Worktree { path, head: head.take(), branch: branch.take() });
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

fn worktree_paths(repo: &Repo) -> Result<Vec<PathBuf>, CliError> {
    Ok(worktrees(&repo.base)?.into_iter().map(|worktree| worktree.path).collect())
}

#[derive(Clone, Default)]
struct StatusSummary {
    staged: usize,
    modified: usize,
    deleted: usize,
    untracked: usize,
    submodule: usize,
}

impl StatusSummary {
    fn is_dirty(&self) -> bool {
        self.staged > 0 || self.modified > 0 || self.deleted > 0 || self.untracked > 0 || self.submodule > 0
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
    source_path: PathBuf,
    source_base: PathBuf,
    base: bool,
    verbose: bool,
    status: Option<StatusSummary>,
    remote_gone: Option<bool>,
    remote_exists: Option<bool>,
    has_submodules: Option<bool>,
    tmux_session: Option<String>,
}

impl WorktreeRow {
    fn from_worktree(repo: &Repo, worktree: &Worktree, path_root: &Path, verbose: bool) -> Self {
        let branch = worktree.branch.as_deref().unwrap_or("detached");
        Self {
            path: display_path(&worktree.path, path_root),
            branch: branch.to_string(),
            head: worktree.head.as_deref().and_then(|head| head.get(..7)).unwrap_or("").to_string(),
            source_path: worktree.path.clone(),
            source_base: repo.base.clone(),
            base: worktree.path == repo.base,
            verbose,
            status: None,
            remote_gone: None,
            remote_exists: None,
            has_submodules: None,
            tmux_session: None,
        }
    }

    fn apply_status(&mut self, status: StatusSummary) {
        self.status = Some(status);
    }

    fn apply_remote_branches(&mut self, remote_branches: &HashSet<String>, default_branch: &str) {
        let remote_exists = remote_branches.contains(&self.branch);
        self.remote_exists = Some(remote_exists);
        self.remote_gone = Some(!self.base && self.branch != "detached" && self.branch != default_branch && !remote_exists);
    }

    fn apply_tmux_sessions(&mut self, tmux_sessions: &HashSet<String>) {
        self.tmux_session = name_from_worktree_path(&self.source_base, &self.source_path)
            .map(|name| session_name_for(&self.source_base, &name))
            .filter(|session| tmux_sessions.contains(session));
    }

    fn is_dirty(&self) -> Option<bool> {
        self.status.as_ref().map(StatusSummary::is_dirty)
    }

    fn badges(&self, pr: DynamicBadge, spinner: Option<char>) -> WorktreeBadges {
        let (state, state_color) = if self.base {
            ("base".to_string(), BadgeColor::Green)
        } else {
            match &self.status {
                Some(status) if status.is_dirty() => (status.tracked_badge(), BadgeColor::YellowBold),
                Some(_) => ("clean".to_string(), BadgeColor::Green),
                None => (loading_badge("status", spinner), BadgeColor::Black),
            }
        };

        WorktreeBadges {
            state,
            state_color,
            untracked: self.status.as_ref().map(|status| count_badge("untracked", status.untracked)).unwrap_or_default(),
            remote: match (self.remote_gone, self.remote_exists) {
                (Some(true), _) => "prunable".to_string(),
                (_, Some(true)) => "remote-ok".to_string(),
                (_, Some(false)) => String::new(),
                _ => loading_badge("remote", spinner),
            },
            pr,
            submodules: match self.has_submodules {
                Some(true) if self.verbose => "submodules".to_string(),
                Some(_) => String::new(),
                None if self.verbose => loading_badge("submodules", spinner),
                None => String::new(),
            },
            tmux: self.tmux_session.as_deref().map(|session| format!("tmux:{session}")).unwrap_or_default(),
        }
    }

    fn format(
        &self,
        path_width: usize,
        branch_width: usize,
        head_width: usize,
        badge_widths: [usize; 6],
        pr: DynamicBadge,
        spinner: Option<char>,
    ) -> String {
        let path = crate::utils::table::pad_cell(&self.path, path_width);
        let path = match self.is_dirty() {
            Some(true) => color_print::cformat!("<yellow,bold>{path}</>"),
            _ if self.base => color_print::cformat!("<green,bold>{path}</>"),
            _ => color_print::cformat!("<cyan>{path}</>"),
        };

        let branch = crate::utils::table::pad_cell(&self.branch, branch_width);
        let branch = color_print::cformat!("<blue>{branch}</>");

        let head = crate::utils::table::pad_cell(&self.head, head_width);
        let head = color_print::cformat!("<black!>{head}</>");

        let badges = self.badges(pr, spinner);
        let badge_cells = [
            color_badge(&badges.state, badge_widths[0], badges.state_color),
            color_badge(&badges.untracked, badge_widths[1], BadgeColor::MagentaBold),
            if self.remote_gone == Some(true) {
                color_badge(&badges.remote, badge_widths[2], BadgeColor::RedBold)
            } else {
                color_badge(&badges.remote, badge_widths[2], BadgeColor::Green)
            },
            color_badge_link(&badges.pr.text, badge_widths[3], BadgeColor::Magenta, badges.pr.url.as_deref()),
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
    pr: DynamicBadge,
    submodules: String,
    tmux: String,
}

impl WorktreeBadges {
    fn widths(&self) -> [usize; 6] {
        [self.state.len(), self.untracked.len(), self.remote.len(), self.pr.text.len(), self.submodules.len(), self.tmux.len()]
    }
}

#[derive(Clone)]
struct DynamicBadge {
    text: String,
    url: Option<String>,
}

impl DynamicBadge {
    fn empty() -> Self {
        Self { text: String::new(), url: None }
    }

    fn text(text: impl Into<String>) -> Self {
        Self { text: text.into(), url: None }
    }

    fn link(text: impl Into<String>, url: impl Into<String>) -> Self {
        Self { text: text.into(), url: Some(url.into()) }
    }
}

enum PrRender<'a> {
    Disabled,
    Loading(char),
    Resolved(&'a [crate::utils::gh::PullRequest]),
}

enum WorktreeUpdate {
    Status { index: usize, status: StatusSummary, has_submodules: bool },
    RemoteBranches(HashSet<String>),
    TmuxSessions(HashSet<String>),
    PullRequests(Vec<crate::utils::gh::PullRequest>),
}

struct WorktreeRenderState {
    rows: Vec<WorktreeRow>,
    default_branch: String,
    pr_lookup: bool,
    pr_numbers: Option<Vec<crate::utils::gh::PullRequest>>,
}

impl WorktreeRenderState {
    fn apply(&mut self, update: WorktreeUpdate) {
        match update {
            WorktreeUpdate::Status { index, status, has_submodules } => {
                if let Some(row) = self.rows.get_mut(index) {
                    row.apply_status(status);
                    row.has_submodules = Some(has_submodules);
                }
            }
            WorktreeUpdate::RemoteBranches(branches) => {
                for row in &mut self.rows {
                    row.apply_remote_branches(&branches, &self.default_branch);
                }
            }
            WorktreeUpdate::TmuxSessions(sessions) => {
                for row in &mut self.rows {
                    row.apply_tmux_sessions(&sessions);
                }
            }
            WorktreeUpdate::PullRequests(prs) => {
                self.pr_numbers = Some(prs);
            }
        }
    }
}

fn pr_render_state<'a>(pr_lookup: bool, prs: Option<&'a [crate::utils::gh::PullRequest]>, spinner: char) -> PrRender<'a> {
    if !pr_lookup {
        PrRender::Disabled
    } else if let Some(prs) = prs {
        PrRender::Resolved(prs)
    } else {
        PrRender::Loading(spinner)
    }
}

fn render_worktree_rows(rows: &[WorktreeRow], pr_render: PrRender<'_>, spinner: Option<char>) -> Vec<String> {
    let pr_badges: Vec<_> = rows.iter().map(|row| row.pr_badge(&pr_render)).collect();
    let [path_width, branch_width, head_width] =
        crate::utils::table::column_widths(rows.iter().map(|row| [row.path.as_str(), row.branch.as_str(), row.head.as_str()]));
    let mut badge_widths = [0; 6];

    for (row, pr) in rows.iter().zip(&pr_badges) {
        let badges = row.badges(pr.clone(), spinner);
        for (index, width) in badges.widths().into_iter().enumerate() {
            badge_widths[index] = badge_widths[index].max(width);
        }
    }

    rows.iter().zip(pr_badges).map(|(row, pr)| row.format(path_width, branch_width, head_width, badge_widths, pr, spinner)).collect()
}

impl WorktreeRow {
    fn pr_badge(&self, pr_render: &PrRender<'_>) -> DynamicBadge {
        match pr_render {
            PrRender::Disabled => DynamicBadge::empty(),
            PrRender::Loading(spinner) => {
                if self.can_have_pr() {
                    DynamicBadge::text(format!("PR {spinner}"))
                } else {
                    DynamicBadge::empty()
                }
            }
            PrRender::Resolved(prs) => prs
                .iter()
                .find(|pr| pr.head == self.branch)
                .map(|pr| DynamicBadge::link(format!("PR #{}", pr.number), pr.url.clone()))
                .unwrap_or_else(DynamicBadge::empty),
        }
    }

    fn can_have_pr(&self) -> bool {
        !self.base && self.branch != "detached"
    }
}

fn loading_badge(label: &str, spinner: Option<char>) -> String {
    spinner.map(|spinner| format!("{label} {spinner}")).unwrap_or_default()
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
    Black,
}

fn color_badge(value: &str, width: usize, color: BadgeColor) -> String {
    color_badge_link(value, width, color, None)
}

fn color_badge_link(value: &str, width: usize, color: BadgeColor, url: Option<&str>) -> String {
    if value.is_empty() {
        return crate::utils::table::pad_cell(value, width);
    }

    let padding = " ".repeat(width.saturating_sub(value.len()));
    let value = if let Some(url) = url { format!("{}{padding}", crate::utils::terminal::hyperlink(value, url)) } else { format!("{value}{padding}") };

    match color {
        BadgeColor::Green => color_print::cformat!("<green>{value}</>"),
        BadgeColor::Yellow => color_print::cformat!("<yellow>{value}</>"),
        BadgeColor::YellowBold => color_print::cformat!("<yellow,bold>{value}</>"),
        BadgeColor::RedBold => color_print::cformat!("<red,bold>{value}</>"),
        BadgeColor::Magenta => color_print::cformat!("<magenta>{value}</>"),
        BadgeColor::MagentaBold => color_print::cformat!("<magenta,bold>{value}</>"),
        BadgeColor::Cyan => color_print::cformat!("<cyan>{value}</>"),
        BadgeColor::Black => color_print::cformat!("<black!>{value}</>"),
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
    path.strip_prefix(root).ok().filter(|path| !path.as_os_str().is_empty()).unwrap_or(path).display().to_string()
}

fn status_summary(repo: &Path) -> Result<StatusSummary, CliError> {
    let output = git::output(repo, &["status", "--porcelain"])?;
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

fn confirm_lossy_remove(gctx: &mut GlobalContext, repo: &Path) -> Result<bool, CliError> {
    let changes = removal_risk_changes(repo)?;
    if changes.is_empty() {
        return Ok(false);
    }

    gctx.shell().warn(format!("removing this worktree will lose local changes in {}:", repo.display()));
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

fn confirm_default_source_branch(gctx: &mut GlobalContext, repo: &Repo) -> Result<Option<String>, CliError> {
    let current_branch = git::current_branch(&repo.base)?;
    if current_branch.as_deref() == Some(repo.default_branch.as_str()) {
        return Ok(None);
    }

    let current_branch = current_branch.unwrap_or_else(|| "detached HEAD".to_string());
    gctx.shell().warn(format!(
        "base worktree is on `{current_branch}`, not the default branch `{}`; the new worktree will branch from the current base HEAD",
        repo.default_branch
    ));
    if gctx.shell().confirm("Continue creating the worktree?")? {
        return Ok(None);
    }

    if gctx.shell().confirm(format!("Create the worktree from `{}` instead?", repo.default_branch))? {
        Ok(Some(repo.default_branch.clone()))
    } else {
        Err(CliError::from("worktree creation cancelled"))
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
    let output = git::output(repo, &["status", "--porcelain"])?;
    let mut changes = Vec::new();

    for line in output.lines() {
        if is_lossy_status(line) {
            changes.push(format_status_line(line, prefix));
        }
    }

    Ok(changes)
}

fn is_lossy_status(line: &str) -> bool {
    line.starts_with("??") || line.get(..2).unwrap_or("").chars().any(|status| status != ' ')
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

    let output = git::output(repo, &["config", "--file", ".gitmodules", "--get-regexp", r"^submodule\..*\.path$"])?;

    Ok(output.lines().filter_map(|line| line.split_once(' ').map(|(_, path)| path.to_string())).collect())
}

fn deinit_submodules(gctx: &mut GlobalContext, repo: &Path) -> CliResult {
    if !has_submodules(repo) {
        return Ok(());
    }

    git::run(repo, &["submodule", "deinit", "--force", "--all"])?;
    gctx.shell().note("submodules: deinitialized");
    Ok(())
}

fn has_submodules(repo: &Path) -> bool {
    repo.join(".gitmodules").is_file()
}

fn preflight_branch_delete(repo: &Path, branch: &str, force: bool) -> CliResult {
    if force || !git::local_branch_exists(repo, branch)? || git::local_branch_merged_for_delete(repo, branch)? {
        return Ok(());
    }

    Err(CliError::from(branch_delete_preflight_message(branch)))
}

fn branch_delete_preflight_message(branch: &str) -> String {
    format!("local branch `{branch}` is not fully merged; pass --force to delete it anyway or --keep-branch to remove only the worktree")
}

fn worktree_remove_args(target: &str, force: bool, confirmed_lossy_remove: bool, has_submodules: bool) -> Vec<&str> {
    let mut args = vec!["worktree", "remove"];
    if force || confirmed_lossy_remove || has_submodules {
        args.push("--force");
    }
    args.push(target);
    args
}

fn required<'a>(args: &'a ArgMatches, name: &str) -> Result<&'a str, CliError> {
    args.get_one::<String>(name).map(String::as_str).ok_or_else(|| CliError::from(format!("missing required argument `{name}`")))
}

fn required_many<'a>(args: &'a ArgMatches, name: &str) -> Result<Vec<&'a str>, CliError> {
    args.get_many::<String>(name)
        .map(|values| values.map(String::as_str).collect())
        .ok_or_else(|| CliError::from(format!("missing required argument `{name}`")))
}

fn path_arg(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn staged_changes_are_lossy_for_worktree_removal() {
        assert!(is_lossy_status("M  src/main.rs"));
        assert!(is_lossy_status("A  src/main.rs"));
        assert!(is_lossy_status(" M src/main.rs"));
        assert!(is_lossy_status("?? scratch.txt"));
        assert!(!is_lossy_status("   clean-looking"));
    }

    #[test]
    fn submodules_force_git_worktree_remove_after_preflight() {
        assert_eq!(worktree_remove_args("/tmp/example", false, false, false), vec!["worktree", "remove", "/tmp/example"]);
        assert_eq!(worktree_remove_args("/tmp/example", false, false, true), vec!["worktree", "remove", "--force", "/tmp/example"]);
        assert_eq!(worktree_remove_args("/tmp/example", false, true, false), vec!["worktree", "remove", "--force", "/tmp/example"]);
        assert_eq!(worktree_remove_args("/tmp/example", true, false, false), vec!["worktree", "remove", "--force", "/tmp/example"]);
    }

    #[test]
    fn branch_delete_preflight_error_explains_choices() {
        assert_eq!(
            branch_delete_preflight_message("feature"),
            "local branch `feature` is not fully merged; pass --force to delete it anyway or --keep-branch to remove only the worktree"
        );
    }
}
