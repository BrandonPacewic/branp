use std::collections::HashSet;
use std::fs;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::mpsc;
use std::thread;

use toml_edit::{value, Array, DocumentMut, Item};

use crate::config::RepoConfig;
use crate::context::GlobalContext;
use crate::errors::{CliError, CliResult};

pub struct PathOptions<'a> {
    pub base: &'a Path,
    pub default_branch: &'a str,
    pub name: &'a str,
}

pub struct ListOptions<'a> {
    pub base: &'a Path,
    pub default_branch: &'a str,
    pub pr_lookup: bool,
}

pub struct PruneOptions<'a> {
    pub base: &'a Path,
}

pub struct TrackOptions<'a> {
    pub base: &'a Path,
    pub current: &'a Path,
    pub paths: Vec<&'a str>,
    pub worktrees: Vec<PathBuf>,
    pub force: bool,
}

pub struct LinksOptions<'a> {
    pub base: &'a Path,
    pub current: &'a Path,
}

pub struct SyncOptions<'a> {
    pub base: &'a Path,
    pub current: &'a Path,
    pub worktrees: Vec<PathBuf>,
    pub quiet: bool,
    pub force: bool,
}

pub struct RemoveOptions<'a> {
    pub base: &'a Path,
    pub name: &'a str,
    pub force: bool,
    pub delete_branch: bool,
    pub tmux: bool,
    pub session_name: String,
}

pub struct NewOptions<'a> {
    pub base: &'a Path,
    pub current: &'a Path,
    pub default_branch: &'a str,
    pub name: &'a str,
    pub branch: &'a str,
    pub fetch: bool,
    pub submodules: bool,
    pub tmux: bool,
    pub session_name: String,
}

pub struct GoneOptions<'a> {
    pub base: &'a Path,
    pub default_branch: &'a str,
    pub dry_run: bool,
    pub force: bool,
    pub tmux: bool,
}

pub struct Worktree {
    pub path: PathBuf,
    pub head: Option<String>,
    pub branch: Option<String>,
}

pub struct Repo {
    pub base: PathBuf,
    pub current: PathBuf,
}

impl Repo {
    pub fn discover(cwd: &Path) -> Result<Self, CliError> {
        let root = PathBuf::from(crate::utils::git::output(cwd, &["rev-parse", "--show-toplevel"])?.trim());
        let base = worktrees(&root)?.into_iter().next().map(|w| w.path).ok_or("could not determine base worktree")?;

        Ok(Self { base, current: root })
    }

    pub fn default_branch(&self) -> Result<String, CliError> {
        crate::utils::git::default_branch(&self.base, "origin")
    }

    pub fn session_name(&self, name: &str) -> String {
        session_name_for(&self.base, name)
    }

    pub fn worktree_paths(&self) -> Result<Vec<PathBuf>, CliError> {
        Ok(worktrees(&self.base)?.into_iter().map(|worktree| worktree.path).collect())
    }
}

pub fn path(gctx: &mut GlobalContext, options: &PathOptions<'_>) -> CliResult {
    let path = if options.name == options.default_branch { options.base.to_path_buf() } else { target_dir(options.base, options.name) };
    gctx.shell().note(path.display());
    Ok(())
}

pub fn prune(_gctx: &mut GlobalContext, options: &PruneOptions<'_>) -> CliResult {
    crate::utils::git::run(options.base, &["worktree", "prune"])?;
    Ok(())
}

pub fn list(gctx: &mut GlobalContext, options: &ListOptions<'_>) -> CliResult {
    if gctx.shell().is_quiet() {
        return Ok(());
    }

    let worktrees = worktrees(options.base)?;
    let path_root = common_parent(&worktrees);
    let verbose = gctx.is_verbose();
    if !crate::utils::terminal::supports_dynamic_lines() {
        let rows = resolve_worktree_rows(options.base, options.default_branch, &worktrees, &path_root, verbose)?;
        let pr_numbers = if options.pr_lookup { crate::utils::gh::open_pull_requests(options.base).unwrap_or_default() } else { Vec::new() };
        let pr_render = if options.pr_lookup { PrRender::Resolved(&pr_numbers) } else { PrRender::Disabled };

        for line in render_worktree_rows(&rows, pr_render, None) {
            gctx.shell().note(line);
        }
        return Ok(());
    }

    render_worktree_rows_dynamic(options.base, options.default_branch, worktrees, &path_root, verbose, options.pr_lookup)
}

pub fn track(gctx: &mut GlobalContext, options: &TrackOptions<'_>) -> CliResult {
    let tracked = LinkStore::new(options.base, options.current).track(&options.paths, &options.worktrees, options.force)?;

    for rel in tracked {
        gctx.shell().note(format!("linked {}", rel.display()));
    }
    Ok(())
}

pub fn links(gctx: &mut GlobalContext, options: &LinksOptions<'_>) -> CliResult {
    for rel in LinkStore::new(options.base, options.current).linked_paths()? {
        gctx.shell().note(rel.display());
    }
    Ok(())
}

pub fn sync(gctx: &mut GlobalContext, options: &SyncOptions<'_>) -> CliResult {
    LinkStore::new(options.base, options.current).sync(&options.worktrees, options.force)?;

    if !options.quiet {
        gctx.shell().note("worktree links synced");
    }
    Ok(())
}

pub fn remove(gctx: &mut GlobalContext, options: &RemoveOptions<'_>) -> CliResult {
    let target = target_dir(options.base, options.name);
    if !target.is_dir() {
        return Err(CliError::from(format!("worktree not found: {}", target.display())));
    }

    let branch = crate::utils::git::current_branch(&target)?;
    if options.delete_branch {
        if let Some(branch) = &branch {
            preflight_branch_delete(options.base, branch, options.force)?;
        }
    }
    let target_has_submodules = has_submodules(&target);
    let confirmed_lossy_remove = confirm_lossy_remove(gctx, &target)?;
    deinit_submodules(gctx, &target)?;

    let target_arg = path_arg(&target);
    let remove_args = worktree_remove_args(target_arg.as_str(), options.force, confirmed_lossy_remove, target_has_submodules);
    crate::utils::git::run(options.base, &remove_args)?;
    crate::utils::git::run(options.base, &["worktree", "prune"])?;

    if options.delete_branch {
        if let Some(branch) = branch {
            crate::utils::git::delete_local_branch(options.base, &branch, options.force)?;
        }
    }

    if options.tmux {
        crate::utils::tmux::kill_session(gctx, &options.session_name)?;
    }

    Ok(())
}

pub fn new(gctx: &mut GlobalContext, options: &NewOptions<'_>) -> CliResult {
    let target = target_dir(options.base, options.name);
    if target.exists() {
        return Err(CliError::from(format!("worktree already exists: {}", target.display())));
    }

    if options.fetch {
        let _ = crate::utils::git::run(options.base, &["fetch", "origin", options.branch]);
    }

    let target_arg = path_arg(&target);
    if crate::utils::git::local_branch_exists(options.base, options.branch)? {
        let args = worktree_add_existing_branch_args(target_arg.as_str(), options.branch);
        crate::utils::git::run(options.base, &args)?;
    } else if crate::utils::git::remote_branch_exists(options.base, "origin", options.branch)? {
        crate::utils::git::run(options.base, &["worktree", "add", target_arg.as_str(), "-b", options.branch, &format!("origin/{}", options.branch)])?;
    } else {
        let start_point = confirm_default_source_branch(gctx, options)?;
        if let Some(start_point) = start_point {
            crate::utils::git::run(options.base, &["worktree", "add", target_arg.as_str(), "-b", options.branch, &start_point])?;
        } else {
            crate::utils::git::run(options.base, &["worktree", "add", target_arg.as_str(), "-b", options.branch])?;
        }
    }

    if options.submodules {
        crate::utils::git::run(&target, &["submodule", "update", "--init", "--recursive"])?;
    }

    sync(gctx, &SyncOptions { base: options.base, current: options.current, worktrees: vec![target.clone()], quiet: true, force: false })?;

    if options.tmux {
        crate::utils::tmux::ensure_session(gctx, &options.session_name, &target)?;
    }

    gctx.shell().note(target.display());
    Ok(())
}

pub fn gone(gctx: &mut GlobalContext, options: &GoneOptions<'_>) -> CliResult {
    crate::utils::git::run(options.base, &["fetch", "--prune"])?;
    let worktrees = worktrees(options.base)?;

    for worktree in worktrees {
        if worktree.path == options.base {
            continue;
        }

        let Some(branch) = worktree.branch else {
            continue;
        };
        if branch == options.default_branch || crate::utils::git::remote_branch_exists(options.base, "origin", &branch)? {
            continue;
        }

        let Some(name) = name_from_worktree_path(options.base, &worktree.path) else {
            gctx.shell().warn(format!("skipping non-sibling worktree for branch `{branch}`: {}", worktree.path.display()));
            continue;
        };

        if options.dry_run {
            gctx.shell().note(format!("{name} ({branch})"));
        } else {
            remove(
                gctx,
                &RemoveOptions {
                    base: options.base,
                    name: &name,
                    force: options.force,
                    delete_branch: true,
                    tmux: options.tmux,
                    session_name: session_name_for(options.base, &name),
                },
            )?;
        }
    }

    Ok(())
}

fn resolve_worktree_rows(
    base: &Path,
    default_branch: &str,
    worktrees: &[Worktree],
    path_root: &Path,
    verbose: bool,
) -> Result<Vec<WorktreeRow>, CliError> {
    let remote_branches = crate::utils::git::remote_branches(base, "origin").unwrap_or_default();
    let tmux_sessions: HashSet<_> = crate::utils::tmux::sessions().unwrap_or_default().into_iter().collect();
    let mut rows: Vec<_> = worktrees.iter().map(|worktree| WorktreeRow::from_worktree(base, worktree, path_root, verbose)).collect();
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
        row.apply_remote_branches(&remote_branches, default_branch);
        row.apply_tmux_sessions(&tmux_sessions);
    }

    Ok(rows)
}

fn render_worktree_rows_dynamic(
    base: &Path,
    default_branch: &str,
    worktrees: Vec<Worktree>,
    path_root: &Path,
    verbose: bool,
    pr_lookup: bool,
) -> CliResult {
    let rows: Vec<_> = worktrees.iter().map(|worktree| WorktreeRow::from_worktree(base, worktree, path_root, verbose)).collect();
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
        let repo_base = base.to_path_buf();
        thread::spawn(move || {
            let _ = tx.send(WorktreeUpdate::RemoteBranches(crate::utils::git::remote_branches(&repo_base, "origin").unwrap_or_default()));
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
        let repo_base = base.to_path_buf();
        thread::spawn(move || {
            let _ = tx.send(WorktreeUpdate::PullRequests(crate::utils::gh::open_pull_requests(&repo_base).unwrap_or_default()));
        });
        pending += 1;
    }
    drop(tx);

    let mut state = WorktreeRenderState { rows, default_branch: default_branch.to_string(), pr_lookup, pr_numbers: None };
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
    fn from_worktree(base: &Path, worktree: &Worktree, path_root: &Path, verbose: bool) -> Self {
        let branch = worktree.branch.as_deref().unwrap_or("detached");
        Self {
            path: display_path(&worktree.path, path_root),
            branch: branch.to_string(),
            head: worktree.head.as_deref().and_then(|head| head.get(..7)).unwrap_or("").to_string(),
            source_path: worktree.path.clone(),
            source_base: base.to_path_buf(),
            base: worktree.path == base,
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
    let output = crate::utils::git::output(repo, &["status", "--porcelain"])?;
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

pub struct LinkStore {
    config: RepoConfig,
    current: PathBuf,
}

impl LinkStore {
    pub fn new(base: impl Into<PathBuf>, current: impl Into<PathBuf>) -> Self {
        Self { config: RepoConfig::new(base), current: current.into() }
    }

    pub fn track(&self, paths: &[&str], worktrees: &[PathBuf], force: bool) -> Result<Vec<PathBuf>, CliError> {
        let mut config = LinkConfig::read(self)?;
        let mut tracked = Vec::new();

        for path in paths {
            let rel = normalize_link_path(path)?;
            let source = self.current.join(&rel);
            if !crate::utils::fs::path_exists_or_symlink(&source) {
                return Err(CliError::from(format!("cannot track missing path: {}", source.display())));
            }

            let storage = self.link_store_dir()?.join(&rel);
            if !crate::utils::fs::path_exists_or_symlink(&storage) {
                if let Some(parent) = storage.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::rename(&source, &storage)?;
            } else if !crate::utils::fs::is_same_link_target(&source, &storage)? {
                if force {
                    crate::utils::fs::remove_path(&source)?;
                } else {
                    return Err(CliError::from(format!(
                        "shared copy already exists for {}; pass --force to replace this worktree path with a symlink",
                        rel.display()
                    )));
                }
            }

            config.add(&rel)?;
            tracked.push(rel);
        }

        config.write(self)?;
        self.sync(worktrees, force)?;
        Ok(tracked)
    }

    pub fn linked_paths(&self) -> Result<Vec<PathBuf>, CliError> {
        Ok(LinkConfig::read(self)?.paths)
    }

    pub fn sync(&self, worktrees: &[PathBuf], force: bool) -> CliResult {
        let config = LinkConfig::read(self)?;
        for rel in config.paths {
            let storage = self.link_store_dir()?.join(&rel);
            if !crate::utils::fs::path_exists_or_symlink(&storage) {
                return Err(CliError::from(format!("linked source is missing for {}: {}", rel.display(), storage.display())));
            }

            for worktree in worktrees {
                crate::utils::fs::ensure_symlink(&storage, &worktree.join(&rel), force)?;
            }
        }
        Ok(())
    }

    fn link_store_dir(&self) -> Result<PathBuf, CliError> {
        self.config.data_dir("worktree-files")
    }
}

struct LinkConfig {
    document: DocumentMut,
    paths: Vec<PathBuf>,
}

impl LinkConfig {
    fn read(store: &LinkStore) -> Result<Self, CliError> {
        let document = store.config.read_document("worktree")?;
        let paths = linked_paths_from_document(&document)?;

        Ok(Self { document, paths })
    }

    fn add(&mut self, rel: &Path) -> Result<(), CliError> {
        if self.paths.iter().any(|path| path == rel) {
            return Ok(());
        }
        self.paths.push(rel.to_path_buf());
        self.paths.sort();

        let mut array = Array::default();
        for path in &self.paths {
            array.push(path_to_config_string(path)?);
        }
        self.document["worktree"].or_insert(Item::Table(toml_edit::Table::new()));
        self.document["worktree"]["linked"] = value(array);
        Ok(())
    }

    fn write(&self, store: &LinkStore) -> CliResult {
        store.config.write_document("worktree", &self.document)
    }
}

fn linked_paths_from_document(document: &DocumentMut) -> Result<Vec<PathBuf>, CliError> {
    let Some(linked) = document.get("worktree").and_then(|worktree| worktree.get("linked")) else {
        return Ok(Vec::new());
    };
    let linked = linked.as_array().ok_or_else(|| CliError::from("expected `worktree.linked` to be an array"))?;
    let mut paths = Vec::new();

    for item in linked {
        let path = item.as_str().ok_or_else(|| CliError::from("expected every `worktree.linked` item to be a string"))?;
        paths.push(normalize_link_path(path)?);
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn normalize_link_path(path: &str) -> Result<PathBuf, CliError> {
    let path = Path::new(path);
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err(CliError::from(format!("linked path must be relative: {}", path.display())));
    }

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => normalized.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(CliError::from(format!("linked path cannot escape the repository: {}", path.display())));
            }
        }
    }

    if normalized.as_os_str().is_empty() || normalized.starts_with(".git") || normalized.starts_with(".branp") {
        return Err(CliError::from(format!("unsupported linked path: {}", path.display())));
    }
    Ok(normalized)
}

fn path_to_config_string(path: &Path) -> Result<String, CliError> {
    path.to_str().map(str::to_string).ok_or_else(|| CliError::from(format!("linked path must be valid UTF-8: {}", path.display())))
}

pub fn has_submodules(repo: &Path) -> bool {
    repo.join(".gitmodules").is_file()
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
    let output = crate::utils::git::output(repo, &["status", "--porcelain"])?;
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

    let output = crate::utils::git::output(repo, &["config", "--file", ".gitmodules", "--get-regexp", r"^submodule\..*\.path$"])?;

    Ok(output.lines().filter_map(|line| line.split_once(' ').map(|(_, path)| path.to_string())).collect())
}

fn deinit_submodules(gctx: &mut GlobalContext, repo: &Path) -> CliResult {
    if !has_submodules(repo) {
        return Ok(());
    }

    crate::utils::git::run(repo, &["submodule", "deinit", "--force", "--all"])?;
    gctx.shell().note("submodules: deinitialized");
    Ok(())
}

fn preflight_branch_delete(repo: &Path, branch: &str, force: bool) -> CliResult {
    if force || !crate::utils::git::local_branch_exists(repo, branch)? || crate::utils::git::local_branch_merged_for_delete(repo, branch)? {
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

fn path_arg(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn worktree_add_existing_branch_args<'a>(target: &'a str, branch: &'a str) -> Vec<&'a str> {
    vec!["worktree", "add", target, branch]
}

fn confirm_default_source_branch(gctx: &mut GlobalContext, options: &NewOptions<'_>) -> Result<Option<String>, CliError> {
    let current_branch = crate::utils::git::current_branch(options.base)?;
    if current_branch.as_deref() == Some(options.default_branch) {
        return Ok(None);
    }

    let current_branch = current_branch.unwrap_or_else(|| "detached HEAD".to_string());
    gctx.shell().warn(format!(
        "base worktree is on `{current_branch}`, not the default branch `{}`; the new worktree will branch from the current base HEAD",
        options.default_branch
    ));
    if gctx.shell().confirm("Continue creating the worktree?")? {
        return Ok(None);
    }

    if gctx.shell().confirm(format!("Create the worktree from `{}` instead?", options.default_branch))? {
        Ok(Some(options.default_branch.to_string()))
    } else {
        Err(CliError::from("worktree creation cancelled"))
    }
}

fn target_dir(base: &Path, name: &str) -> PathBuf {
    PathBuf::from(format!("{}-{name}", base.display()))
}

pub fn worktrees(repo: &Path) -> Result<Vec<Worktree>, CliError> {
    let output = crate::utils::git::output(repo, &["worktree", "list", "--porcelain"])?;
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

pub fn session_name_for(base: &Path, name: &str) -> String {
    let repo_name = base.file_name().and_then(|name| name.to_str()).unwrap_or("worktree");
    format!("{repo_name}-{name}")
}

pub fn name_from_worktree_path(base: &Path, path: &Path) -> Option<String> {
    path.to_str()?.strip_prefix(&format!("{}-", base.display())).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_link_paths_inside_repo() {
        assert_eq!(normalize_link_path("./.env").unwrap(), PathBuf::from(".env"));
        assert_eq!(normalize_link_path("config/local.toml").unwrap(), PathBuf::from("config/local.toml"));
        assert!(normalize_link_path("../secret").is_err());
        assert!(normalize_link_path("/tmp/secret").is_err());
        assert!(normalize_link_path(".git/config").is_err());
        assert!(normalize_link_path(".branp/worktree.toml").is_err());
    }

    #[test]
    fn reads_sorted_unique_link_paths_from_toml() {
        let document = r#"
[worktree]
linked = ["z.env", "./a.env", "z.env"]
"#
        .parse::<DocumentMut>()
        .unwrap();

        assert_eq!(linked_paths_from_document(&document).unwrap(), vec![PathBuf::from("a.env"), PathBuf::from("z.env")]);
    }

    #[test]
    fn target_dir_appends_worktree_name_to_base_path() {
        assert_eq!(target_dir(Path::new("/tmp/example"), "feature"), PathBuf::from("/tmp/example-feature"));
    }

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

    #[test]
    fn existing_local_branch_worktree_add_reuses_branch() {
        assert_eq!(worktree_add_existing_branch_args("/tmp/example", "feature"), vec!["worktree", "add", "/tmp/example", "feature"]);
    }
}
