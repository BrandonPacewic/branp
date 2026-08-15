//! Git worktree helpers.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;

use clap::{Arg, ArgAction, ArgMatches, Command};

use crate::context::GlobalContext;
use crate::errors::{CliError, CliResult};
use crate::ops::worktree as ops;
use crate::utils::git;

pub fn cli() -> Command {
    Command::new("worktree").about("Manage sibling Git worktrees").subcommand_required(true).arg_required_else_help(true).subcommands(builtin())
}

type WorktreeExec = fn(&mut GlobalContext, &Repo, &ArgMatches) -> CliResult;

fn builtin() -> Vec<Command> {
    vec![list_cli(), path_cli(), new_cli(), remove_cli(), prune_cli(), gone_cli(), track_cli(), links_cli(), sync_cli()]
}

fn builtin_exec(cmd: &str) -> Option<WorktreeExec> {
    let exec = match cmd {
        "list" => list_exec,
        "path" => path_exec,
        "new" => new_exec,
        "remove" => remove_exec,
        "prune" => prune_exec,
        "gone" => gone_exec,
        "track" => track_exec,
        "links" => links_exec,
        "sync" => sync_exec,
        _ => return None,
    };

    Some(exec)
}

pub fn exec(gctx: &mut GlobalContext, args: &ArgMatches) -> CliResult {
    let repo = Repo::discover(gctx.cwd())?;

    if let Some((cmd, sub)) = args.subcommand() {
        if let Some(exec) = builtin_exec(cmd) {
            return exec(gctx, &repo, sub);
        }
    }

    Err(CliError::from("no `worktree` subcommand provided"))
}

fn list_cli() -> Command {
    Command::new("list")
        .alias("ls")
        .about("List Git worktrees")
        .arg(Arg::new("no-pr").long("no-pr").help("Skip GitHub pull request lookup").action(ArgAction::SetTrue))
}

fn path_cli() -> Command {
    Command::new("path")
        .about("Print the path for a worktree name, or the base worktree for the default branch")
        .arg(Arg::new("name").required(true).value_name("NAME"))
}

fn new_cli() -> Command {
    Command::new("new")
        .about("Create a sibling worktree")
        .arg(Arg::new("name").required(true).value_name("NAME"))
        .arg(Arg::new("branch").value_name("BRANCH"))
        .arg(Arg::new("no-fetch").long("no-fetch").help("Do not fetch origin before creating the worktree").action(ArgAction::SetTrue))
        .arg(Arg::new("no-submodules").long("no-submodules").help("Skip submodule initialization").action(ArgAction::SetTrue))
        .arg(Arg::new("no-tmux").long("no-tmux").help("Do not start a tmux session").action(ArgAction::SetTrue))
}

fn remove_cli() -> Command {
    Command::new("remove")
        .alias("rm")
        .about("Remove a sibling worktree and its local branch")
        .arg(Arg::new("name").required(true).value_name("NAME"))
        .arg(Arg::new("force").short('f').long("force").help("Force worktree removal and branch deletion").action(ArgAction::SetTrue))
        .arg(Arg::new("keep-branch").long("keep-branch").help("Do not delete the local branch").action(ArgAction::SetTrue))
        .arg(Arg::new("no-tmux").long("no-tmux").help("Do not kill the matching tmux session").action(ArgAction::SetTrue))
}

fn prune_cli() -> Command {
    Command::new("prune").about("Run git worktree prune")
}

fn gone_cli() -> Command {
    Command::new("gone")
        .about("Remove sibling worktrees whose origin branch no longer exists")
        .arg(Arg::new("dry-run").short('n').long("dry-run").help("Show what would be removed").action(ArgAction::SetTrue))
        .arg(Arg::new("force").short('f').long("force").help("Force worktree removal and branch deletion").action(ArgAction::SetTrue))
        .arg(Arg::new("no-tmux").long("no-tmux").help("Do not kill matching tmux sessions").action(ArgAction::SetTrue))
}

fn track_cli() -> Command {
    Command::new("track")
        .about("Track untracked files that should be symlinked into every worktree")
        .arg(Arg::new("path").required(true).num_args(1..).value_name("PATH"))
        .arg(Arg::new("force").short('f').long("force").help("Replace existing files in other worktrees").action(ArgAction::SetTrue))
}

fn links_cli() -> Command {
    Command::new("links").about("List files linked across worktrees")
}

fn sync_cli() -> Command {
    Command::new("sync")
        .about("Create or repair symlinks for files tracked in .branp/worktree.toml")
        .arg(Arg::new("force").short('f').long("force").help("Replace existing non-symlink paths").action(ArgAction::SetTrue))
}

fn list_exec(gctx: &mut GlobalContext, repo: &Repo, args: &ArgMatches) -> CliResult {
    list(gctx, repo, !args.get_flag("no-pr"))
}

fn path_exec(gctx: &mut GlobalContext, repo: &Repo, args: &ArgMatches) -> CliResult {
    ops::path(gctx, &ops::PathOptions { base: &repo.base, default_branch: &repo.default_branch, name: required(args, "name")? })
}

fn new_exec(gctx: &mut GlobalContext, repo: &Repo, args: &ArgMatches) -> CliResult {
    let name = required(args, "name")?;
    let branch = args.get_one::<String>("branch").map_or(name, String::as_str);
    ops::new(
        gctx,
        &ops::NewOptions {
            base: &repo.base,
            current: &repo.current,
            default_branch: &repo.default_branch,
            name,
            branch,
            fetch: !args.get_flag("no-fetch"),
            submodules: !args.get_flag("no-submodules"),
            tmux: !args.get_flag("no-tmux"),
            session_name: repo.session_name(name),
        },
    )
}

fn remove_exec(gctx: &mut GlobalContext, repo: &Repo, args: &ArgMatches) -> CliResult {
    let name = required(args, "name")?;
    ops::remove(
        gctx,
        &ops::RemoveOptions {
            base: &repo.base,
            name,
            force: args.get_flag("force"),
            delete_branch: !args.get_flag("keep-branch"),
            tmux: !args.get_flag("no-tmux"),
            session_name: repo.session_name(name),
        },
    )
}

fn prune_exec(_gctx: &mut GlobalContext, repo: &Repo, _args: &ArgMatches) -> CliResult {
    ops::prune(&repo.base)
}

fn gone_exec(gctx: &mut GlobalContext, repo: &Repo, args: &ArgMatches) -> CliResult {
    remove_gone(gctx, repo, args.get_flag("dry-run"), args.get_flag("force"), !args.get_flag("no-tmux"))
}

fn track_exec(gctx: &mut GlobalContext, repo: &Repo, args: &ArgMatches) -> CliResult {
    ops::track(
        gctx,
        &ops::TrackOptions {
            base: &repo.base,
            current: &repo.current,
            paths: required_many(args, "path")?,
            worktrees: worktree_paths(repo)?,
            force: args.get_flag("force"),
        },
    )
}

fn links_exec(gctx: &mut GlobalContext, repo: &Repo, _args: &ArgMatches) -> CliResult {
    ops::links(gctx, &ops::LinksOptions { base: &repo.base, current: &repo.current })
}

fn sync_exec(gctx: &mut GlobalContext, repo: &Repo, args: &ArgMatches) -> CliResult {
    ops::sync(
        gctx,
        &ops::SyncOptions { base: &repo.base, current: &repo.current, worktrees: worktree_paths(repo)?, quiet: false, force: args.get_flag("force") },
    )
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
            thread::spawn(move || (index, status_summary(&path).unwrap_or_default(), ops::has_submodules(&path)))
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
            let _ = tx.send(WorktreeUpdate::Status {
                index,
                status: status_summary(&path).unwrap_or_default(),
                has_submodules: ops::has_submodules(&path),
            });
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
            ops::remove(
                gctx,
                &ops::RemoveOptions { base: &repo.base, name: &name, force, delete_branch: true, tmux, session_name: repo.session_name(&name) },
            )?;
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

    fn session_name(&self, name: &str) -> String {
        session_name_for(&self.base, name)
    }

    fn name_from_path(&self, path: &Path) -> Option<String> {
        name_from_worktree_path(&self.base, path)
    }
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

fn required<'a>(args: &'a ArgMatches, name: &str) -> Result<&'a str, CliError> {
    args.get_one::<String>(name).map(String::as_str).ok_or_else(|| CliError::from(format!("missing required argument `{name}`")))
}

fn required_many<'a>(args: &'a ArgMatches, name: &str) -> Result<Vec<&'a str>, CliError> {
    args.get_many::<String>(name)
        .map(|values| values.map(String::as_str).collect())
        .ok_or_else(|| CliError::from(format!("missing required argument `{name}`")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worktree_subcommands_have_exec_handlers() {
        for command in builtin() {
            assert!(builtin_exec(command.get_name()).is_some(), "{} is missing an exec handler", command.get_name());
        }
    }

    #[test]
    fn worktree_cli_definition_is_valid() {
        cli().debug_assert();
    }

    #[test]
    fn worktree_subcommand_aliases_resolve_to_canonical_commands() {
        let matches = cli().try_get_matches_from(["worktree", "ls"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("list"));

        let matches = cli().try_get_matches_from(["worktree", "rm", "example"]).unwrap();
        assert_eq!(matches.subcommand_name(), Some("remove"));
    }
}
