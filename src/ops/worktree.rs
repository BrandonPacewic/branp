use std::collections::HashSet;
use std::fs;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use fs2::FileExt;
use toml_edit::{value, Array, DocumentMut, Item};

use crate::config::RepoConfig;
use crate::context::GlobalContext;
use crate::errors::{CliError, CliResult};
use crate::ops::scratch_state::{self, Availability, Entry, Registered, State, Store};

pub struct PathOptions<'a> {
    pub base: &'a Path,
    pub default_branch: &'a str,
    pub name: &'a str,
}

pub struct EnterOptions<'a> {
    pub base: &'a Path,
    pub current: &'a Path,
    pub cwd: &'a Path,
    pub home: &'a Path,
    pub default_branch: &'a str,
    pub target: &'a str,
}

pub struct ListOptions<'a> {
    pub base: &'a Path,
    pub current: &'a Path,
    pub default_branch: &'a str,
    pub pr_lookup: bool,
}

pub struct PruneOptions<'a> {
    pub base: &'a Path,
    pub home: &'a Path,
    pub confirm: bool,
    pub older_than: Option<Duration>,
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
    pub current: &'a Path,
    pub cwd: &'a Path,
    pub home: &'a Path,
    pub name: &'a str,
    pub force: bool,
    pub delete_branch: bool,
    pub tmux: bool,
    pub dry_run: bool,
    pub include_dirty: bool,
    pub include_in_use: bool,
    pub allow_state_transition: bool,
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

pub struct ScratchOptions<'a> {
    pub home: &'a Path,
    pub base: &'a Path,
    pub current: &'a Path,
    pub submodules: bool,
}

pub struct ReturnOptions<'a> {
    pub home: &'a Path,
    pub base: &'a Path,
    pub current: &'a Path,
    pub target: Option<&'a str>,
    pub discard_changes: bool,
}

pub struct GoneOptions<'a> {
    pub base: &'a Path,
    pub default_branch: &'a str,
    pub dry_run: bool,
    pub force: bool,
    pub tmux: bool,
}

pub struct RecoveryOptions<'a> {
    pub home: &'a Path,
    pub base: &'a Path,
    pub cwd: &'a Path,
    pub target: Option<&'a str>,
}

#[derive(Clone)]
struct Worktree {
    path: PathBuf,
    head: Option<String>,
    branch: Option<String>,
}

struct ResolvedRemoveTarget {
    path: PathBuf,
    worktree: Worktree,
    scratch: bool,
}

struct ScratchRemoveInspection {
    changes: Vec<String>,
    in_use: crate::utils::in_use::InUseInspection,
}

struct PruneScratchCandidate {
    path: PathBuf,
    size: u64,
    reservation: ScratchReservation,
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

pub fn enter(gctx: &mut GlobalContext, options: &EnterOptions<'_>) -> CliResult {
    let registered = worktrees(options.base)?;
    let target = resolve_enter_target(options, &registered)?;
    gctx.shell().note(target.display());
    enter_worktree_shell(&target)
}

pub fn prune(gctx: &mut GlobalContext, options: &PruneOptions<'_>) -> CliResult {
    let registered = worktrees(options.base)?;
    let durable_state = state_snapshot(gctx, options.home, options.base)?;
    let scratch_root = scratch_dir(options.home).canonicalize().ok();
    let mut candidates = Vec::new();
    let mut skipped = 0;
    let mut estimated_reclaimed = 0;

    for worktree in &registered {
        if same_worktree_path(&worktree.path, options.base) {
            gctx.shell().warn(format!("skipping worktree {}: base worktree is protected", worktree.path.display()));
            skipped += 1;
            continue;
        }

        let Some(root) = scratch_root.as_deref() else {
            gctx.shell().warn(format!(
                "skipping worktree {}: named worktrees are protected; use `bp worktree remove` or `bp worktree gone`",
                worktree.path.display()
            ));
            skipped += 1;
            continue;
        };

        let path = worktree.path.canonicalize().unwrap_or_else(|_| worktree.path.clone());
        let direct_child = path.parent() == Some(root);
        let name = path.file_name().and_then(|name| name.to_str()).unwrap_or_default();
        if !direct_child || !name.starts_with("scratch-") {
            gctx.shell().warn(format!(
                "skipping worktree {}: named worktrees are protected; use `bp worktree remove` or `bp worktree gone`",
                worktree.path.display()
            ));
            skipped += 1;
            continue;
        }
        if !worktree.path.exists() {
            gctx.shell().warn(format!(
                "skipping scratch worktree {}: registered path is missing; use `--confirm` to prune stale Git metadata",
                worktree.path.display()
            ));
            skipped += 1;
            continue;
        }
        if worktree.head.is_none() {
            gctx.shell().warn(format!("skipping scratch worktree {}: HEAD could not be verified", worktree.path.display()));
            skipped += 1;
            continue;
        }
        if worktree.branch.is_some() {
            gctx.shell().warn(format!("skipping scratch worktree {}: worktree is named and not detached", worktree.path.display()));
            skipped += 1;
            continue;
        }
        let Some(state_entry) = durable_state.entries.get(name) else {
            gctx.shell().warn(format!("skipping scratch worktree {}: state is missing; use `bp worktree recover inspect`", path.display()));
            skipped += 1;
            continue;
        };
        if state_entry.availability != Availability::Available {
            gctx.shell().warn(format!(
                "skipping scratch worktree {}: durable state is {}; use `bp worktree recover inspect`",
                path.display(),
                state_entry.availability.as_str()
            ));
            skipped += 1;
            continue;
        }

        let Some(reservation) = try_reserve_scratch(root, &path)? else {
            gctx.shell().warn(format!(
                "skipping scratch worktree {}: short-lived lifecycle reservation is held; retry after the other operation completes",
                path.display()
            ));
            skipped += 1;
            continue;
        };

        let changes = match removal_risk_changes(&path) {
            Ok(changes) => changes,
            Err(error) => {
                gctx.shell().warn(format!("skipping scratch worktree {}: could not verify cleanliness: {}", path.display(), error.message));
                skipped += 1;
                drop(reservation);
                continue;
            }
        };
        if !changes.is_empty() {
            gctx.shell().warn(format!(
                "skipping scratch worktree {}: dirty changes ({}) - use `bp worktree remove --include-dirty` for explicit destruction",
                path.display(),
                changes.join(", ")
            ));
            skipped += 1;
            drop(reservation);
            continue;
        }

        let session_name = name_from_worktree_path(options.base, &path)
            .or_else(|| path.file_name().and_then(|name| name.to_str()).map(str::to_string))
            .map(|name| session_name_for(options.base, &name));
        let mut in_use = match crate::utils::in_use::inspect(&path, session_name.as_deref()) {
            Ok(in_use) => in_use,
            Err(error) => {
                gctx.shell().warn(format!("skipping scratch worktree {}: could not verify availability: {}", path.display(), error.message));
                skipped += 1;
                drop(reservation);
                continue;
            }
        };
        in_use.processes.retain(|process| process.pid != std::process::id());
        if in_use.is_in_use() {
            gctx.shell().warn(format!(
                "skipping scratch worktree {}: worktree is in use ({}) - stop the process or tmux session and retry",
                path.display(),
                in_use.reasons().join(", ")
            ));
            skipped += 1;
            drop(reservation);
            continue;
        }

        let age = match fs::metadata(&path).and_then(|metadata| metadata.modified()) {
            Ok(modified) => SystemTime::now().duration_since(modified).unwrap_or_default(),
            Err(error) => {
                gctx.shell().warn(format!("skipping scratch worktree {}: could not verify age: {}", path.display(), error));
                skipped += 1;
                drop(reservation);
                continue;
            }
        };
        if let Some(older_than) = options.older_than {
            if age < older_than {
                gctx.shell().warn(format!(
                    "skipping scratch worktree {}: younger than {}; use a shorter `--older-than` duration or omit the age filter",
                    path.display(),
                    format_duration(older_than)
                ));
                skipped += 1;
                drop(reservation);
                continue;
            }
        }

        let size = match directory_size(&path) {
            Ok(size) => size,
            Err(error) => {
                gctx.shell().warn(format!("skipping scratch worktree {}: could not measure disk usage: {}", path.display(), error.message));
                skipped += 1;
                drop(reservation);
                continue;
            }
        };
        estimated_reclaimed += size;
        candidates.push(PruneScratchCandidate { path, size, reservation });
    }

    let stale_metadata = crate::utils::git::output(options.base, &["worktree", "prune", "--dry-run"])?;
    for line in stale_metadata.lines().filter(|line| !line.trim().is_empty()) {
        gctx.shell().note(format!("stale Git worktree metadata: {line}"));
    }

    if !options.confirm {
        gctx.shell().note("dry-run: no worktrees removed; pass `--confirm` to apply pruning");
        for candidate in &candidates {
            gctx.shell().note(format!(
                "would remove scratch worktree {} (clean, available, detached, {})",
                candidate.path.display(),
                format_bytes(candidate.size)
            ));
        }
        gctx.shell().note(format!("reclaimed disk space: 0 B (dry run; would reclaim {})", format_bytes(estimated_reclaimed)));
        gctx.shell().note(format!("prune summary: {} scratch worktree(s) eligible, {skipped} skipped", candidates.len()));
        return Ok(());
    }

    let mut reclaimed = 0;
    let mut removed = 0;
    for candidate in candidates {
        let path_string = candidate.path.display().to_string();
        if let Err(error) = transition_scratch_state(
            gctx,
            options.home,
            options.base,
            &candidate.path,
            Availability::Destroying,
            Some(scratch_state::owner_token()),
            None,
        ) {
            skipped += 1;
            gctx.shell().warn(format!("skipping scratch worktree {path_string}: could not persist destruction state: {}", error.message));
            drop(candidate.reservation);
            continue;
        }
        let result = remove(
            gctx,
            &RemoveOptions {
                base: options.base,
                current: options.base,
                cwd: options.base,
                home: options.home,
                name: &path_string,
                force: false,
                delete_branch: false,
                tmux: true,
                dry_run: false,
                include_dirty: false,
                include_in_use: false,
                allow_state_transition: true,
                session_name: session_name_for(options.base, &path_string),
            },
        );
        drop(candidate.reservation);
        match result {
            Ok(()) => {
                removed += 1;
                reclaimed += candidate.size;
                gctx.shell().note(format!("removed scratch worktree {} ({})", path_string, format_bytes(candidate.size)));
            }
            Err(error) => {
                skipped += 1;
                gctx.shell().warn(format!("skipping scratch worktree {path_string}: removal failed after safety recheck: {}", error.message));
            }
        }
    }

    crate::utils::git::run(options.base, &["worktree", "prune"])?;
    gctx.shell().note(format!("reclaimed disk space: {}", format_bytes(reclaimed)));
    gctx.shell().note(format!("prune summary: {removed} scratch worktree(s) removed, {skipped} skipped"));
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
        let rows = resolve_worktree_rows(options.base, options.current, options.default_branch, &worktrees, &path_root, verbose)?;
        let pr_numbers = if options.pr_lookup { crate::utils::gh::open_pull_requests(options.base).unwrap_or_default() } else { Vec::new() };
        let pr_render = if options.pr_lookup { PrRender::Resolved(&pr_numbers) } else { PrRender::Disabled };

        for line in render_worktree_rows(&rows, pr_render, None) {
            gctx.shell().note(line);
        }
        return Ok(());
    }

    render_worktree_rows_dynamic(options.base, options.current, options.default_branch, worktrees, &path_root, verbose, options.pr_lookup)
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
    let registered = worktrees(options.base)?;
    let target = resolve_remove_target(options, &registered)?;
    let session_name = remove_session_name(options, &target.path);

    if target.scratch {
        let state = state_snapshot(gctx, options.home, options.base)?;
        let name =
            target.path.file_name().and_then(|value| value.to_str()).ok_or_else(|| CliError::from("scratch worktree path has no valid name"))?;
        let state_entry = state.entries.get(name).ok_or_else(|| {
            CliError::from(format!(
                "cannot remove scratch worktree {}: durable state is missing; use `bp worktree recover destroy --confirm`",
                target.path.display()
            ))
        })?;
        let inspection = inspect_scratch_removal(&target.path, &session_name)?;
        if options.dry_run {
            render_scratch_remove_preview(gctx, &target.path, &inspection);
            gctx.shell().note(format!("durable state: {}", state_entry.availability.as_str()));
            return Ok(());
        }

        if state_entry.availability != Availability::Available
            && !(options.allow_state_transition && state_entry.availability == Availability::Destroying)
        {
            return Err(CliError::from(format!(
                "cannot remove scratch worktree {}: durable state is {}; use `bp worktree recover inspect` and explicitly recover it",
                target.path.display(),
                state_entry.availability.as_str()
            )));
        }

        let mut refusals = Vec::new();
        if !inspection.changes.is_empty() && !options.include_dirty {
            refusals.push(format!("dirty changes: {} (pass --include-dirty)", inspection.changes.join(", ")));
        }
        if inspection.in_use.is_in_use() && !options.include_in_use {
            refusals.push(format!("in use: {} (pass --include-in-use)", inspection.in_use.reasons().join(", ")));
        }
        if !refusals.is_empty() {
            return Err(CliError::from(format!("cannot remove scratch worktree {}: {}", target.path.display(), refusals.join("; "))));
        }

        let reservation = if options.allow_state_transition {
            None
        } else {
            let root = scratch_dir(options.home).canonicalize()?;
            Some(try_reserve_scratch(&root, &target.path)?.ok_or_else(|| {
                CliError::from(format!("cannot remove scratch worktree {} while another lifecycle operation is running", target.path.display()))
            })?)
        };
        if !options.allow_state_transition {
            transition_scratch_state(
                gctx,
                options.home,
                options.base,
                &target.path,
                Availability::Destroying,
                Some(scratch_state::owner_token()),
                None,
            )?;
        }

        let result = (|| {
            let target_has_submodules = has_submodules(&target.path);
            deinit_submodules(gctx, &target.path)?;
            let target_arg = path_arg(&target.path);
            let remove_args = worktree_remove_args(target_arg.as_str(), false, options.include_dirty, target_has_submodules);
            crate::utils::git::run(options.base, &remove_args)?;
            crate::utils::git::run(options.base, &["worktree", "prune"])?;
            Ok::<(), CliError>(())
        })();

        if let Err(error) = &result {
            let _ = transition_scratch_state(
                gctx,
                options.home,
                options.base,
                &target.path,
                Availability::Quarantined,
                None,
                Some(format!("destruction did not complete: {}", error.message)),
            );
        } else {
            remove_scratch_state(gctx, options.home, options.base, &target.path)?;
        }
        drop(reservation);
        result?;

        if options.tmux {
            crate::utils::tmux::kill_session(gctx, &session_name)?;
        }
        return Ok(());
    }

    let branch = target.worktree.branch.clone();
    if options.delete_branch {
        if let Some(branch) = &branch {
            preflight_branch_delete(options.base, branch, options.force)?;
        }
    }
    if options.dry_run {
        render_named_remove_preview(gctx, &target.path, branch.as_deref(), options.delete_branch);
        return Ok(());
    }

    let target_has_submodules = has_submodules(&target.path);
    let confirmed_lossy_remove = confirm_lossy_remove(gctx, &target.path)?;
    deinit_submodules(gctx, &target.path)?;

    let target_arg = path_arg(&target.path);
    let remove_args = worktree_remove_args(target_arg.as_str(), options.force, confirmed_lossy_remove, target_has_submodules);
    crate::utils::git::run(options.base, &remove_args)?;
    crate::utils::git::run(options.base, &["worktree", "prune"])?;

    if options.delete_branch {
        if let Some(branch) = branch {
            crate::utils::git::delete_local_branch(options.base, &branch, options.force)?;
        }
    }

    if options.tmux {
        crate::utils::tmux::kill_session(gctx, &session_name)?;
    }

    Ok(())
}

fn resolve_remove_target(options: &RemoveOptions<'_>, registered: &[Worktree]) -> Result<ResolvedRemoveTarget, CliError> {
    let input = Path::new(options.name);
    let matches: Vec<_> = registered.iter().filter(|worktree| remove_input_matches(options, input, &worktree.path)).cloned().collect();

    if matches.len() > 1 {
        let paths = matches.iter().map(|worktree| worktree.path.display().to_string()).collect::<Vec<_>>().join(", ");
        return Err(CliError::from(format!("ambiguous worktree target `{}`; matches: {paths}; use an exact path", options.name)));
    }

    let Some(worktree) = matches.into_iter().next() else {
        if input.components().count() == 1 && crate::utils::git::current_branch(options.base).ok().flatten().as_deref() == Some(options.name) {
            return Err(CliError::from(format!(
                "refusing to remove the base worktree {}; target `{}` names its current branch",
                options.base.display(),
                options.name
            )));
        }
        return Err(CliError::from(format!("worktree target `{}` is not registered; use an exact registered worktree name or path", options.name)));
    };

    let path = worktree.path.clone();
    if same_worktree_path(&path, options.base) {
        return Err(CliError::from(format!("refusing to remove the base worktree {}", path.display())));
    }

    let scratch_root = scratch_dir(options.home).canonicalize().ok();
    let path_for_checks = path.canonicalize().unwrap_or_else(|_| path.clone());
    if let Some(root) = scratch_root {
        if path_for_checks.starts_with(&root) {
            if path_for_checks.parent() != Some(root.as_path()) {
                return Err(CliError::from(format!(
                    "refusing to remove {}; scratch worktrees must be directly under {}",
                    path.display(),
                    root.display()
                )));
            }

            let name = path_for_checks.file_name().and_then(|name| name.to_str()).unwrap_or_default();
            if !name.starts_with("scratch-") {
                return Err(CliError::from(format!(
                    "refusing to remove {}; registered worktree under {} is not clearly identifiable as scratch",
                    path.display(),
                    root.display()
                )));
            }
            if worktree.branch.is_some() {
                return Err(CliError::from(format!("refusing to remove {}; scratch worktrees must be detached", path.display())));
            }

            return Ok(ResolvedRemoveTarget { path, worktree, scratch: true });
        }
    }

    Ok(ResolvedRemoveTarget { path, worktree, scratch: false })
}

fn remove_input_matches(options: &RemoveOptions<'_>, input: &Path, registered: &Path) -> bool {
    let mut candidates = Vec::new();
    if input.is_absolute() {
        candidates.push(input.to_path_buf());
    } else {
        candidates.push(options.cwd.join(input));
        candidates.push(options.current.join(input));
        if input.components().count() == 1 {
            if let Some(name) = input.to_str() {
                candidates.push(target_dir(options.base, name));
                candidates.push(scratch_dir(options.home).join(name));
            }
        }
    }

    candidates.into_iter().any(|candidate| same_worktree_path(&candidate, registered))
}

fn resolve_enter_target(options: &EnterOptions<'_>, registered: &[Worktree]) -> Result<PathBuf, CliError> {
    let input = Path::new(options.target);
    let matches: Vec<_> = registered.iter().filter(|worktree| enter_input_matches(options, input, worktree)).collect();

    if matches.len() > 1 {
        let paths = matches.iter().map(|worktree| worktree.path.display().to_string()).collect::<Vec<_>>().join(", ");
        return Err(CliError::from(format!("ambiguous worktree target `{}`; matches: {paths}; use an exact path", options.target)));
    }

    let Some(worktree) = matches.into_iter().next() else {
        return Err(CliError::from(format!(
            "worktree target `{}` is not a registered worktree; use an exact registered worktree name or path",
            options.target
        )));
    };

    if !worktree.path.is_dir() {
        return Err(CliError::from(format!("worktree target `{}` is missing: {}", options.target, worktree.path.display())));
    }

    Ok(worktree.path.clone())
}

fn enter_input_matches(options: &EnterOptions<'_>, input: &Path, worktree: &Worktree) -> bool {
    let mut candidates = Vec::new();
    if input.is_absolute() {
        candidates.push(input.to_path_buf());
    } else {
        candidates.push(options.cwd.join(input));
        candidates.push(options.current.join(input));
    }

    if input.components().count() == 1 {
        let Some(name) = input.to_str() else {
            return false;
        };

        if name == options.default_branch || worktree.branch.as_deref() == Some(name) {
            candidates.push(options.base.to_path_buf());
        }
        candidates.push(target_dir(options.base, name));

        let scratch_root = scratch_dir(options.home);
        let scratch = scratch_root.join(name);
        let is_scratch = worktree.branch.is_none()
            && worktree.path.parent().is_some_and(|parent| same_worktree_path(parent, &scratch_root))
            && scratch.file_name().and_then(|value| value.to_str()).is_some_and(|value| value.starts_with("scratch-"));
        if is_scratch {
            candidates.push(scratch);
        }
    }

    candidates.into_iter().any(|candidate| same_worktree_path(&candidate, &worktree.path))
}

fn inspect_scratch_removal(path: &Path, session_name: &str) -> Result<ScratchRemoveInspection, CliError> {
    let mut in_use = crate::utils::in_use::inspect(path, Some(session_name))?;
    in_use.processes.retain(|process| process.pid != std::process::id());
    Ok(ScratchRemoveInspection { changes: removal_risk_changes(path)?, in_use })
}

fn render_scratch_remove_preview(gctx: &mut GlobalContext, path: &Path, inspection: &ScratchRemoveInspection) {
    gctx.shell().note(format!("dry-run: would remove scratch worktree {}", path.display()));
    gctx.shell().note("protection: dirty changes require --include-dirty");
    gctx.shell().note("protection: in-use processes or tmux sessions require --include-in-use");
    gctx.shell().note("branch deletion: skipped because scratch worktrees are detached");
    if inspection.changes.is_empty() {
        gctx.shell().note("state: clean");
    } else {
        gctx.shell().note(format!("state: dirty ({})", inspection.changes.join(", ")));
    }
    if inspection.in_use.is_in_use() {
        gctx.shell().note(format!("state: in-use ({})", inspection.in_use.reasons().join(", ")));
    } else {
        gctx.shell().note("state: available");
    }
}

fn render_named_remove_preview(gctx: &mut GlobalContext, path: &Path, branch: Option<&str>, delete_branch: bool) {
    gctx.shell().note(format!("dry-run: would remove named worktree {}", path.display()));
    if delete_branch {
        if let Some(branch) = branch {
            gctx.shell().note(format!("branch deletion: would delete local branch `{branch}`"));
        } else {
            gctx.shell().note("branch deletion: none (worktree is detached)");
        }
    } else {
        gctx.shell().note("branch deletion: skipped by --keep-branch");
    }
}

fn remove_session_name(options: &RemoveOptions<'_>, path: &Path) -> String {
    name_from_worktree_path(options.base, path)
        .or_else(|| path.file_name().and_then(|name| name.to_str()).map(str::to_string))
        .map(|name| session_name_for(options.base, &name))
        .unwrap_or_else(|| options.session_name.clone())
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

pub fn scratch(gctx: &mut GlobalContext, options: &ScratchOptions<'_>) -> CliResult {
    let root = scratch_dir(options.home);
    fs::create_dir_all(&root)?;
    let registered_worktrees = worktrees(options.base)?;

    if let Some((target, reservation)) = find_reusable_scratch(gctx, options.base, &root, &registered_worktrees)? {
        return setup_scratch(gctx, options, target, reservation);
    }

    let target = reserve_scratch_target_in(&root, &format_scratch_prefix())?;
    let reservation = try_reserve_scratch(&root, &target)?.ok_or_else(|| CliError::from("could not reserve a new scratch worktree"))?;
    let name =
        target.file_name().and_then(|value| value.to_str()).ok_or_else(|| CliError::from("scratch worktree path has no valid name"))?.to_string();
    let registered = scratch_registered_worktrees(options.base, &root)?;
    let ((), notices) = scratch_store(options.home).transaction(&registered, |state| {
        state.entries.insert(
            name,
            Entry {
                path: target.canonicalize().unwrap_or_else(|_| target.clone()),
                owner: Some(scratch_state::owner_token()),
                availability: Availability::Provisioning,
                last_used: Some(scratch_state::now_seconds()),
                reason: None,
            },
        );
        Ok(())
    })?;
    report_state_notices(gctx, notices);
    let target_arg = path_arg(&target);
    if let Err(error) = crate::utils::git::run(options.base, &["worktree", "add", "--detach", target_arg.as_str()]) {
        let _ = transition_scratch_state(
            gctx,
            options.home,
            options.base,
            &target,
            Availability::Quarantined,
            None,
            Some(format!("scratch creation was interrupted: {}", error.message)),
        );
        return Err(error);
    }

    setup_scratch(gctx, options, target, reservation)
}

fn setup_scratch(gctx: &mut GlobalContext, options: &ScratchOptions<'_>, target: PathBuf, reservation: ScratchReservation) -> CliResult {
    let setup = (|| {
        if options.submodules {
            crate::utils::git::run(&target, &["submodule", "update", "--init", "--recursive"])?;
        }

        sync(gctx, &SyncOptions { base: options.base, current: options.current, worktrees: vec![target.clone()], quiet: true, force: false })?;
        transition_scratch_state(gctx, options.home, options.base, &target, Availability::Leased, Some(scratch_state::owner_token()), None)?;
        Ok::<(), CliError>(())
    })();
    if let Err(error) = setup {
        let _ = transition_scratch_state(
            gctx,
            options.home,
            options.base,
            &target,
            Availability::Quarantined,
            None,
            Some(format!("scratch setup did not complete: {}", error.message)),
        );
        drop(reservation);
        return Err(error);
    }

    gctx.shell().note(target.display());
    let shell_result = enter_worktree_shell(&target);
    let state_result = match &shell_result {
        Ok(()) => transition_scratch_state(gctx, options.home, options.base, &target, Availability::Available, None, None),
        Err(error) => transition_scratch_state(
            gctx,
            options.home,
            options.base,
            &target,
            Availability::Quarantined,
            None,
            Some(format!("scratch shell did not exit cleanly: {}", error.message)),
        ),
    };
    drop(reservation);
    state_result.and(shell_result)
}

pub fn return_worktree(gctx: &mut GlobalContext, options: &ReturnOptions<'_>) -> CliResult {
    let worktrees = worktrees(options.base)?;
    let target = resolve_return_target(options, &worktrees)?;
    let root = scratch_dir(options.home).canonicalize()?;
    let reservation = try_reserve_scratch(&root, &target)?
        .ok_or_else(|| CliError::from(format!("cannot return scratch worktree {} while another lifecycle operation is running", target.display())))?;

    let session_name = target.file_name().and_then(|name| name.to_str()).map(|name| session_name_for(options.base, name));
    let mut in_use = crate::utils::in_use::inspect(&target, session_name.as_deref())?;
    // Do not treat the return command itself as an external owner.
    in_use.processes.retain(|process| process.pid != std::process::id());
    if in_use.is_in_use() {
        drop(reservation);
        return Err(CliError::from(format!(
            "cannot return scratch worktree {} while it is in use: {}; stop the process or tmux session and try again",
            target.display(),
            in_use.reasons().join(", ")
        )));
    }

    let changes = removal_risk_changes(&target)?;
    if !changes.is_empty() && !options.discard_changes {
        drop(reservation);
        return Err(CliError::from(format!(
            "cannot return dirty scratch worktree {}: {}; rerun with --discard-changes to remove local changes",
            target.display(),
            changes.join(", ")
        )));
    }

    let registered = scratch_registered_worktrees(options.base, &root)?;
    let ((), notices) = scratch_store(options.home).transaction(&registered, |state| {
        let name = target.file_name().and_then(|value| value.to_str()).ok_or_else(|| CliError::from("scratch worktree path has no valid name"))?;
        let entry = state.entries.get_mut(name).ok_or_else(|| {
            CliError::from(format!(
                "cannot return scratch worktree {}: durable state is missing; use `bp worktree recover inspect`",
                target.display()
            ))
        })?;
        if entry.availability == Availability::Quarantined {
            return Err(CliError::from(format!(
                "cannot return quarantined scratch worktree {}; use `bp worktree recover release` or `destroy --confirm`",
                target.display()
            )));
        }
        entry.availability = Availability::Cleaning;
        entry.owner = Some(scratch_state::owner_token());
        entry.reason = None;
        Ok(())
    })?;
    report_state_notices(gctx, notices);

    let result = (|| {
        let base_head = crate::utils::git::output(options.base, &["rev-parse", "HEAD"])?;
        reset_for_return(&target, base_head.trim())?;
        sync(gctx, &SyncOptions { base: options.base, current: options.current, worktrees: vec![target.clone()], quiet: true, force: false })?;

        if crate::utils::git::current_branch(&target)?.is_some() {
            return Err(CliError::from(format!("returned worktree is not detached: {}", target.display())));
        }
        Ok::<(), CliError>(())
    })();
    let state_result = match &result {
        Ok(()) => transition_scratch_state(gctx, options.home, options.base, &target, Availability::Available, None, None),
        Err(error) => transition_scratch_state(
            gctx,
            options.home,
            options.base,
            &target,
            Availability::Quarantined,
            None,
            Some(format!("return did not complete: {}", error.message)),
        ),
    };
    drop(reservation);
    state_result?;
    result?;

    gctx.shell().note(format!("returned {}", target.display()));
    Ok(())
}

pub fn recover_inspect(gctx: &mut GlobalContext, options: &RecoveryOptions<'_>) -> CliResult {
    let state = state_snapshot(gctx, options.home, options.base)?;
    let entries = if let Some(target) = options.target {
        let path = resolve_recovery_target(options, target)?;
        let name = path.file_name().and_then(|value| value.to_str()).ok_or_else(|| CliError::from("worktree path has no valid name"))?;
        state.entries.get(name).map(|entry| vec![(name.to_string(), entry.clone())]).unwrap_or_default()
    } else {
        state.entries.into_iter().filter(|(_, entry)| entry.availability != Availability::Available).collect::<Vec<_>>()
    };

    if entries.is_empty() {
        gctx.shell().note("no quarantined scratch worktrees");
        return Ok(());
    }

    gctx.shell().note("quarantined scratch worktrees:");
    for (name, entry) in entries {
        let owner = entry.owner.as_deref().unwrap_or("none");
        let last_used = entry.last_used.map_or_else(|| "never".to_string(), |value| value.to_string());
        let reason = entry.reason.as_deref().unwrap_or("state is not reusable");
        gctx.shell().note(format!("  {name}: {} ({}, owner={owner}, last-used={last_used})", entry.path.display(), entry.availability.as_str()));
        gctx.shell().note(format!("    reason: {reason}"));
    }
    gctx.shell().note("inspect live safety before using bp worktree recover release or the destructive destroy --confirm");
    Ok(())
}

pub fn recover_release(gctx: &mut GlobalContext, options: &RecoveryOptions<'_>) -> CliResult {
    let target_name = options.target.ok_or_else(|| CliError::from("missing recovery target"))?;
    let target = resolve_recovery_target(options, target_name)?;
    let state = state_snapshot(gctx, options.home, options.base)?;
    let name = target.file_name().and_then(|value| value.to_str()).ok_or_else(|| CliError::from("worktree path has no valid name"))?;
    let entry = state.entries.get(name).ok_or_else(|| CliError::from(format!("no durable state entry for {}", target.display())))?;
    if entry.availability == Availability::Available {
        return Err(CliError::from(format!("scratch worktree {} is already available", target.display())));
    }

    if crate::utils::git::current_branch(&target)?.is_some() {
        return Err(CliError::from(format!(
            "cannot release {}: worktree is named; recovery only accepts detached scratch worktrees",
            target.display()
        )));
    }
    let changes = removal_risk_changes(&target)?;
    if !changes.is_empty() {
        return Err(CliError::from(format!(
            "cannot release quarantined scratch worktree {}: dirty changes ({}); inspect or use destroy --confirm --include-dirty",
            target.display(),
            changes.join(", ")
        )));
    }
    let session_name = target.file_name().and_then(|value| value.to_str()).map(|value| session_name_for(options.base, value));
    let mut in_use = crate::utils::in_use::inspect(&target, session_name.as_deref())?;
    in_use.processes.retain(|process| process.pid != std::process::id());
    if in_use.is_in_use() {
        return Err(CliError::from(format!(
            "cannot release quarantined scratch worktree {} while it is in use: {}",
            target.display(),
            in_use.reasons().join(", ")
        )));
    }

    transition_scratch_state(gctx, options.home, options.base, &target, Availability::Available, None, None)?;
    gctx.shell().note(format!("released quarantined scratch worktree {}", target.display()));
    Ok(())
}

pub fn recover_destroy(
    gctx: &mut GlobalContext,
    options: &RecoveryOptions<'_>,
    confirm: bool,
    include_dirty: bool,
    include_in_use: bool,
    tmux: bool,
) -> CliResult {
    if !confirm {
        return Err(CliError::from("destructive recovery requires --confirm; use bp worktree recover inspect first"));
    }
    let target_name = options.target.ok_or_else(|| CliError::from("missing recovery target"))?;
    let target = resolve_recovery_target(options, target_name)?;
    let state = state_snapshot(gctx, options.home, options.base)?;
    let name = target.file_name().and_then(|value| value.to_str()).ok_or_else(|| CliError::from("worktree path has no valid name"))?;
    let entry = state.entries.get(name).ok_or_else(|| CliError::from(format!("no durable state entry for {}", target.display())))?;
    if entry.availability == Availability::Available {
        return Err(CliError::from(format!("scratch worktree {} is available; use bp worktree remove for normal destruction", target.display())));
    }

    let session_name = target
        .file_name()
        .and_then(|value| value.to_str())
        .map(|value| session_name_for(options.base, value))
        .unwrap_or_else(|| options.base.display().to_string());
    let inspection = inspect_scratch_removal(&target, &session_name)?;
    let mut refusals = Vec::new();
    if !inspection.changes.is_empty() && !include_dirty {
        refusals.push(format!("dirty changes: {} (pass --include-dirty)", inspection.changes.join(", ")));
    }
    if inspection.in_use.is_in_use() && !include_in_use {
        refusals.push(format!("in use: {} (pass --include-in-use)", inspection.in_use.reasons().join(", ")));
    }
    if !refusals.is_empty() {
        return Err(CliError::from(format!("cannot destroy quarantined scratch worktree {}: {}", target.display(), refusals.join("; "))));
    }

    let root = scratch_dir(options.home).canonicalize()?;
    let reservation = try_reserve_scratch(&root, &target)?.ok_or_else(|| {
        CliError::from(format!("cannot destroy scratch worktree {} while another lifecycle operation is running", target.display()))
    })?;
    transition_scratch_state(gctx, options.home, options.base, &target, Availability::Destroying, Some(scratch_state::owner_token()), None)?;
    gctx.shell().warn(format!("DESTRUCTIVE: destroying quarantined scratch worktree {}", target.display()));

    let result = (|| {
        let target_has_submodules = has_submodules(&target);
        deinit_submodules(gctx, &target)?;
        let target_arg = path_arg(&target);
        let remove_args = worktree_remove_args(target_arg.as_str(), false, include_dirty, target_has_submodules);
        crate::utils::git::run(options.base, &remove_args)?;
        crate::utils::git::run(options.base, &["worktree", "prune"])?;
        Ok::<(), CliError>(())
    })();
    if let Err(error) = &result {
        let _ = transition_scratch_state(
            gctx,
            options.home,
            options.base,
            &target,
            Availability::Quarantined,
            None,
            Some(format!("destruction did not complete: {}", error.message)),
        );
    } else {
        remove_scratch_state(gctx, options.home, options.base, &target)?;
    }
    drop(reservation);
    result?;
    if tmux {
        crate::utils::tmux::kill_session(gctx, &session_name)?;
    }
    gctx.shell().note(format!("destroyed quarantined scratch worktree {}", target.display()));
    Ok(())
}

fn resolve_recovery_target(options: &RecoveryOptions<'_>, target: &str) -> Result<PathBuf, CliError> {
    let registered = worktrees(options.base)?;
    let remove_options = RemoveOptions {
        base: options.base,
        current: options.cwd,
        cwd: options.cwd,
        home: options.home,
        name: target,
        force: false,
        delete_branch: false,
        tmux: false,
        dry_run: false,
        include_dirty: false,
        include_in_use: false,
        allow_state_transition: false,
        session_name: session_name_for(options.base, target),
    };
    let resolved = resolve_remove_target(&remove_options, &registered)?;
    if !resolved.scratch {
        return Err(CliError::from(format!("recovery target {target} is a named worktree; named worktrees are never managed by scratch recovery")));
    }
    Ok(resolved.path)
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
                    current: options.base,
                    cwd: options.base,
                    home: options.base,
                    name: &name,
                    force: options.force,
                    delete_branch: true,
                    tmux: options.tmux,
                    dry_run: false,
                    include_dirty: false,
                    include_in_use: false,
                    allow_state_transition: false,
                    session_name: session_name_for(options.base, &name),
                },
            )?;
        }
    }

    Ok(())
}

fn resolve_worktree_rows(
    base: &Path,
    current: &Path,
    default_branch: &str,
    worktrees: &[Worktree],
    path_root: &Path,
    verbose: bool,
) -> Result<Vec<WorktreeRow>, CliError> {
    let remote_branches = crate::utils::git::remote_branches(base, "origin").unwrap_or_default();
    let tmux_sessions: HashSet<_> = crate::utils::tmux::sessions().unwrap_or_default().into_iter().collect();
    let mut rows: Vec<_> = worktrees.iter().map(|worktree| WorktreeRow::from_worktree(base, current, worktree, path_root, verbose)).collect();
    let handles: Vec<_> = worktrees
        .iter()
        .enumerate()
        .map(|(index, worktree)| {
            let path = worktree.path.clone();
            thread::spawn(move || {
                let status = status_summary(&path).unwrap_or_default();
                let has_submodules = has_submodules(&path);
                let processes = crate::utils::process::processes_in_worktree(&path).unwrap_or_default();
                (index, status, has_submodules, processes)
            })
        })
        .collect();

    for handle in handles {
        let (index, status, has_submodules, processes) = handle.join().map_err(|_| CliError::from("worktree status lookup panicked"))?;
        if let Some(row) = rows.get_mut(index) {
            row.apply_status(status);
            row.has_submodules = Some(has_submodules);
            row.apply_processes(processes);
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
    current: &Path,
    default_branch: &str,
    worktrees: Vec<Worktree>,
    path_root: &Path,
    verbose: bool,
    pr_lookup: bool,
) -> CliResult {
    let rows: Vec<_> = worktrees.iter().map(|worktree| WorktreeRow::from_worktree(base, current, worktree, path_root, verbose)).collect();
    let (tx, rx) = mpsc::channel();
    let mut pending = 0;

    for (index, worktree) in worktrees.iter().enumerate() {
        let tx = tx.clone();
        let path = worktree.path.clone();
        thread::spawn(move || {
            let status = status_summary(&path).unwrap_or_default();
            let has_submodules = has_submodules(&path);
            let processes = crate::utils::process::processes_in_worktree(&path).unwrap_or_default();
            let _ = tx.send(WorktreeUpdate::Status { index, status, has_submodules, processes });
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
            let sessions = crate::utils::in_use::active_sessions();
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
    current: bool,
    verbose: bool,
    status: Option<StatusSummary>,
    remote_gone: Option<bool>,
    remote_exists: Option<bool>,
    has_submodules: Option<bool>,
    in_use: Option<crate::utils::in_use::InUseInspection>,
    tmux_session: Option<String>,
}

impl WorktreeRow {
    fn from_worktree(base: &Path, current: &Path, worktree: &Worktree, path_root: &Path, verbose: bool) -> Self {
        let branch = worktree.branch.as_deref().unwrap_or("detached");
        Self {
            path: display_path(&worktree.path, path_root),
            branch: branch.to_string(),
            head: worktree.head.as_deref().and_then(|head| head.get(..7)).unwrap_or("").to_string(),
            source_path: worktree.path.clone(),
            source_base: base.to_path_buf(),
            base: same_worktree_path(&worktree.path, base),
            current: same_worktree_path(&worktree.path, current),
            verbose,
            status: None,
            remote_gone: None,
            remote_exists: None,
            has_submodules: None,
            in_use: None,
            tmux_session: None,
        }
    }

    fn apply_status(&mut self, status: StatusSummary) {
        self.status = Some(status);
    }

    fn apply_processes(&mut self, processes: Vec<crate::utils::process::ProcessInfo>) {
        self.in_use.get_or_insert_with(Default::default).processes = processes;
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
        self.in_use.get_or_insert_with(Default::default).active_sessions = self.tmux_session.clone().into_iter().collect();
    }

    fn in_use_reasons(&self) -> Vec<String> {
        self.in_use.as_ref().map(crate::utils::in_use::InUseInspection::reasons).unwrap_or_default()
    }

    fn is_dirty(&self) -> Option<bool> {
        self.status.as_ref().map(StatusSummary::is_dirty)
    }

    fn badges(&self, pr: DynamicBadge, spinner: Option<char>) -> WorktreeBadges {
        let (mut state, mut state_color) = match &self.status {
            Some(status) if status.is_dirty() => {
                let details = status.tracked_badge();
                let cleanliness = if details.is_empty() { "dirty".to_string() } else { format!("dirty ({details})") };
                (cleanliness, BadgeColor::YellowBold)
            }
            Some(_) => ("clean".to_string(), BadgeColor::Green),
            None => (loading_badge("status", spinner), BadgeColor::Black),
        };

        if self.base {
            state = format!("base,{state}");
        }
        if self.current {
            state = format!("{state},current");
        }

        let in_use_reasons = self.in_use_reasons();
        if !in_use_reasons.is_empty() {
            state = format!("{state},in-use:{}", in_use_reasons.join(","));
            state_color = BadgeColor::YellowBold;
        }

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
    Status { index: usize, status: StatusSummary, has_submodules: bool, processes: Vec<crate::utils::process::ProcessInfo> },
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
            WorktreeUpdate::Status { index, status, has_submodules, processes } => {
                if let Some(row) = self.rows.get_mut(index) {
                    row.apply_status(status);
                    row.has_submodules = Some(has_submodules);
                    row.apply_processes(processes);
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

fn same_worktree_path(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

fn status_summary(repo: &Path) -> Result<StatusSummary, CliError> {
    let output = crate::utils::git::output(repo, &["status", "--porcelain", "--untracked-files=all"])?;
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

struct LinkStore {
    config: RepoConfig,
    current: PathBuf,
}

impl LinkStore {
    fn new(base: impl Into<PathBuf>, current: impl Into<PathBuf>) -> Self {
        Self { config: RepoConfig::new(base), current: current.into() }
    }

    fn track(&self, paths: &[&str], worktrees: &[PathBuf], force: bool) -> Result<Vec<PathBuf>, CliError> {
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

    fn linked_paths(&self) -> Result<Vec<PathBuf>, CliError> {
        Ok(LinkConfig::read(self)?.paths)
    }

    fn sync(&self, worktrees: &[PathBuf], force: bool) -> CliResult {
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

fn has_submodules(repo: &Path) -> bool {
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

fn scratch_dir(home: &Path) -> PathBuf {
    home.join(".bp/worktrees")
}

fn scratch_registered_worktrees(base: &Path, root: &Path) -> Result<Vec<Registered>, CliError> {
    let Some(root) = root.canonicalize().ok() else {
        return Ok(Vec::new());
    };
    Ok(worktrees(base)?
        .into_iter()
        .filter_map(|worktree| {
            if worktree.branch.is_some() {
                return None;
            }
            let path = worktree.path.canonicalize().ok()?;
            let name = path.file_name()?.to_str()?.to_string();
            if !name.starts_with("scratch-") || path.parent() != Some(root.as_path()) {
                return None;
            }
            Some(Registered { name, path })
        })
        .collect())
}

fn scratch_store(home: &Path) -> Store {
    Store::new(&scratch_dir(home))
}

fn scratch_store_from_root(root: &Path) -> Store {
    Store::new(root)
}

fn report_state_notices(gctx: &mut GlobalContext, notices: Vec<String>) {
    for notice in notices {
        gctx.shell().warn(notice);
    }
}

fn state_snapshot(gctx: &mut GlobalContext, home: &Path, base: &Path) -> Result<State, CliError> {
    let root = scratch_dir(home);
    let registered = scratch_registered_worktrees(base, &root)?;
    let (state, notices) = scratch_store(home).read(&registered)?;
    report_state_notices(gctx, notices);
    Ok(state)
}

fn transition_scratch_state(
    gctx: &mut GlobalContext,
    home: &Path,
    base: &Path,
    target: &Path,
    availability: Availability,
    owner: Option<String>,
    reason: Option<String>,
) -> CliResult {
    let root = scratch_dir(home);
    let name =
        target.file_name().and_then(|value| value.to_str()).ok_or_else(|| CliError::from("scratch worktree path has no valid name"))?.to_string();
    let registered = scratch_registered_worktrees(base, &root)?;
    let ((), notices) = scratch_store(home).transaction(&registered, |state| {
        let entry = state.entries.get_mut(&name).ok_or_else(|| {
            CliError::from(format!("scratch worktree {} has no durable state; use `bp worktree recover inspect`", target.display()))
        })?;
        entry.availability = availability;
        entry.owner = owner;
        entry.reason = reason;
        entry.last_used = Some(scratch_state::now_seconds());
        Ok(())
    })?;
    report_state_notices(gctx, notices);
    Ok(())
}

fn remove_scratch_state(gctx: &mut GlobalContext, home: &Path, base: &Path, target: &Path) -> CliResult {
    let name =
        target.file_name().and_then(|value| value.to_str()).ok_or_else(|| CliError::from("scratch worktree path has no valid name"))?.to_string();
    let registered = scratch_registered_worktrees(base, &scratch_dir(home))?;
    let ((), notices) = scratch_store(home).transaction(&registered, |state| {
        state.entries.remove(&name);
        Ok(())
    })?;
    report_state_notices(gctx, notices);
    Ok(())
}

fn directory_size(path: &Path) -> Result<u64, CliError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Ok(metadata.len());
    }

    let mut size: u64 = 0;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        size = size
            .checked_add(directory_size(&entry.path())?)
            .ok_or_else(|| CliError::from(format!("disk usage is too large to measure: {}", path.display())))?;
    }
    Ok(size)
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {} ({bytes} B)", UNITS[unit])
    }
}

fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs();
    let (value, unit) = if seconds % (7 * 24 * 60 * 60) == 0 {
        (seconds / (7 * 24 * 60 * 60), "w")
    } else if seconds % (24 * 60 * 60) == 0 {
        (seconds / (24 * 60 * 60), "d")
    } else if seconds % (60 * 60) == 0 {
        (seconds / (60 * 60), "h")
    } else if seconds % 60 == 0 {
        (seconds / 60, "m")
    } else {
        (seconds, "s")
    };
    format!("{value}{unit}")
}

struct ScratchReservation {
    lock: fs::File,
}

impl Drop for ScratchReservation {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.lock);
    }
}

fn format_scratch_prefix() -> String {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    format!("scratch-{timestamp}-{}", std::process::id())
}

fn reserve_scratch_target_in(root: &Path, prefix: &str) -> Result<PathBuf, CliError> {
    for attempt in 0..u64::MAX {
        let name = if attempt == 0 { prefix.to_string() } else { format!("{prefix}-{attempt}") };
        let target = root.join(name);
        match fs::create_dir(&target) {
            Ok(()) => return Ok(target),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
    }

    Err(CliError::from("could not allocate a scratch worktree name"))
}

fn enter_worktree_shell(target: &Path) -> CliResult {
    let shell = std::env::var_os("SHELL").or_else(|| std::env::var_os("COMSPEC")).unwrap_or_else(|| "sh".into());
    let status = std::process::Command::new(&shell).current_dir(target).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(CliError::from(format!("worktree shell exited unsuccessfully: {}", shell.to_string_lossy())))
    }
}

fn find_reusable_scratch(
    gctx: &mut GlobalContext,
    base: &Path,
    root: &Path,
    worktrees: &[Worktree],
) -> Result<Option<(PathBuf, ScratchReservation)>, CliError> {
    let root_path = root.to_path_buf();
    let root = root.canonicalize()?;
    let mut candidates: Vec<_> = worktrees.iter().collect();
    candidates.sort_by(|left, right| left.path.cmp(&right.path));
    let registered_state = scratch_registered_worktrees(base, &root)?;
    let (durable_state, notices) = scratch_store_from_root(&root).read(&registered_state)?;
    report_state_notices(gctx, notices);

    for worktree in candidates {
        let registered = &worktree.path;
        let Ok(candidate) = registered.canonicalize() else {
            if registered.starts_with(&root_path) {
                warn_scratch_skip(gctx, registered, "registered path is missing");
            }
            continue;
        };
        if candidate.parent() != Some(root.as_path()) {
            continue;
        }
        let Some(name) = candidate.file_name().and_then(|name| name.to_str()) else {
            warn_scratch_skip(gctx, &candidate, "path has no valid name");
            continue;
        };
        if !name.starts_with("scratch-") {
            warn_scratch_skip(gctx, &candidate, "registered worktree is not a scratch worktree");
            continue;
        }
        if worktree.branch.is_some() {
            warn_scratch_skip(gctx, &candidate, "worktree is named and not detached");
            continue;
        }
        let Some(entry) = durable_state.entries.get(name) else {
            warn_scratch_skip(gctx, &candidate, "state metadata is missing; worktree is quarantined");
            continue;
        };
        if entry.availability != Availability::Available {
            warn_scratch_skip(gctx, &candidate, format!("state is {}; worktree is not reusable", entry.availability.as_str()));
            continue;
        }

        let Some(reservation) = try_reserve_scratch(&root, &candidate)? else {
            warn_scratch_skip(gctx, &candidate, "short-lived acquisition reservation is held");
            continue;
        };
        let (claimed, notices) = scratch_store_from_root(&root).transaction(&registered_state, |state| {
            let Some(entry) = state.entries.get_mut(name) else {
                return Ok(false);
            };
            if entry.availability != Availability::Available {
                return Ok(false);
            }
            if let Err(reason) = reusable_scratch_check(base, &candidate) {
                entry.availability = Availability::Quarantined;
                entry.owner = None;
                entry.reason = Some(reason.clone());
                warn_scratch_skip(gctx, &candidate, format!("{reason}; worktree is quarantined"));
                return Ok(false);
            }
            entry.availability = Availability::Leased;
            entry.owner = Some(scratch_state::owner_token());
            entry.last_used = Some(scratch_state::now_seconds());
            entry.reason = None;
            Ok(true)
        })?;
        report_state_notices(gctx, notices);
        if claimed {
            return Ok(Some((candidate, reservation)));
        }
        drop(reservation);
    }

    Ok(None)
}

fn reusable_scratch_check(base: &Path, candidate: &Path) -> Result<(), String> {
    let status = status_summary(candidate).map_err(|error| format!("could not inspect status: {}", error.message))?;
    if status.is_dirty() {
        return Err(format!("worktree is dirty: {}", dirty_status_reason(&status)));
    }

    let session = candidate.file_name().and_then(|name| name.to_str()).map(|name| session_name_for(base, name));
    let mut in_use =
        crate::utils::in_use::inspect(candidate, session.as_deref()).map_err(|error| format!("could not inspect use: {}", error.message))?;
    in_use.processes.retain(|process| process.pid != std::process::id());
    if in_use.is_in_use() {
        return Err(format!("worktree is in use: {}", in_use.reasons().join(", ")));
    }

    Ok(())
}

fn dirty_status_reason(status: &StatusSummary) -> String {
    let mut reasons = Vec::new();
    if status.staged > 0 {
        reasons.push(format!("staged:{}", status.staged));
    }
    if status.modified > 0 {
        reasons.push(format!("modified:{}", status.modified));
    }
    if status.deleted > 0 {
        reasons.push(format!("deleted:{}", status.deleted));
    }
    if status.untracked > 0 {
        reasons.push(format!("untracked:{}", status.untracked));
    }
    if status.submodule > 0 {
        reasons.push(format!("submodule-dirty:{}", status.submodule));
    }
    reasons.join(", ")
}

fn warn_scratch_skip(gctx: &mut GlobalContext, path: &Path, reason: impl std::fmt::Display) {
    gctx.shell().warn(format!("skipping scratch worktree {}: {reason}", path.display()));
}

fn try_reserve_scratch(root: &Path, target: &Path) -> Result<Option<ScratchReservation>, CliError> {
    let name = target.file_name().and_then(|name| name.to_str()).ok_or_else(|| CliError::from("scratch worktree path has no valid name"))?;
    let marker = root.join(format!(".{name}.reservation"));
    let lock = OpenOptions::new().create(true).truncate(false).read(true).write(true).open(marker)?;
    match lock.try_lock_exclusive() {
        Ok(()) => Ok(Some(ScratchReservation { lock })),
        Err(error) if error.kind() == io::ErrorKind::WouldBlock => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn resolve_return_target(options: &ReturnOptions<'_>, worktrees: &[Worktree]) -> Result<PathBuf, CliError> {
    let candidate = match options.target {
        None => options.current.to_path_buf(),
        Some(value) => return_target_candidate(options, value, worktrees),
    };
    let candidate =
        candidate.canonicalize().map_err(|error| CliError::from(format!("worktree path is not available: {} ({error})", candidate.display())))?;
    let Some(worktree) = worktrees.iter().find(|worktree| same_worktree_path(&worktree.path, &candidate)) else {
        return Err(CliError::from(format!("not a registered Git worktree: {}", candidate.display())));
    };

    if same_worktree_path(&worktree.path, options.base) {
        return Err(CliError::from(format!(
            "refusing to return the base worktree {}; return accepts detached scratch worktrees only",
            candidate.display()
        )));
    }
    if worktree.branch.is_some() {
        return Err(CliError::from(format!(
            "refusing to return named worktree {}; return accepts detached scratch worktrees only",
            candidate.display()
        )));
    }

    let root = scratch_dir(options.home)
        .canonicalize()
        .map_err(|error| CliError::from(format!("scratch root is unavailable: {} ({error})", scratch_dir(options.home).display())))?;
    let is_direct_child = candidate.parent() == Some(root.as_path());
    let is_scratch_name = candidate.file_name().and_then(|name| name.to_str()).is_some_and(|name| name.starts_with("scratch-"));
    if !is_direct_child || !is_scratch_name {
        return Err(CliError::from(format!(
            "refusing to return {}; target must be a detached scratch worktree directly under {}",
            candidate.display(),
            root.display()
        )));
    }

    Ok(candidate)
}

fn return_target_candidate(options: &ReturnOptions<'_>, value: &str, worktrees: &[Worktree]) -> PathBuf {
    let value_path = Path::new(value);
    if value_path.is_absolute() || value_path.components().count() > 1 || value_path.starts_with(".") {
        return if value_path.is_absolute() { value_path.to_path_buf() } else { options.current.join(value_path) };
    }

    let scratch = scratch_dir(options.home).join(value_path);
    if worktrees.iter().any(|worktree| same_worktree_path(&worktree.path, &scratch)) {
        return scratch;
    }

    let named = target_dir(options.base, value);
    if worktrees.iter().any(|worktree| same_worktree_path(&worktree.path, &named)) {
        return named;
    }

    if crate::utils::git::current_branch(options.base).ok().flatten().as_deref() == Some(value) {
        return options.base.to_path_buf();
    }

    scratch
}

fn reset_for_return(target: &Path, base_head: &str) -> CliResult {
    crate::utils::git::run(target, &["reset", "--hard", base_head])?;
    crate::utils::git::run(target, &["clean", "-fd"])?;

    if has_submodules(target) {
        crate::utils::git::run(target, &["submodule", "foreach", "--recursive", "git", "reset", "--hard"])?;
        crate::utils::git::run(target, &["submodule", "foreach", "--recursive", "git", "clean", "-fd"])?;
    }

    Ok(())
}

fn worktrees(repo: &Path) -> Result<Vec<Worktree>, CliError> {
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

fn session_name_for(base: &Path, name: &str) -> String {
    let repo_name = base.file_name().and_then(|name| name.to_str()).unwrap_or("worktree");
    format!("{repo_name}-{name}")
}

fn name_from_worktree_path(base: &Path, path: &Path) -> Option<String> {
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
    fn scratch_target_uses_a_dedicated_user_directory() {
        assert_eq!(scratch_dir(Path::new("/tmp/home")), PathBuf::from("/tmp/home/.bp/worktrees"));
    }

    #[test]
    fn remove_resolves_only_an_exact_registered_detached_scratch_target() {
        let home = std::env::temp_dir().join(format!("branp-remove-target-{}", std::process::id()));
        let scratch = scratch_dir(&home).join("scratch-test");
        fs::create_dir_all(&scratch).unwrap();
        let options = RemoveOptions {
            base: Path::new("/tmp/repo"),
            current: Path::new("/tmp/repo"),
            cwd: Path::new("/tmp/repo"),
            home: &home,
            name: "scratch-test",
            force: false,
            delete_branch: true,
            tmux: false,
            dry_run: false,
            include_dirty: false,
            include_in_use: false,
            allow_state_transition: false,
            session_name: "repo-scratch-test".to_string(),
        };
        let registered = vec![Worktree { path: scratch.clone(), head: None, branch: None }];

        let resolved = resolve_remove_target(&options, &registered).unwrap();
        assert!(resolved.scratch);
        assert_eq!(resolved.path, scratch);

        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn enter_resolves_base_named_and_nested_scratch_targets() {
        let root = std::env::temp_dir().join(format!("branp-enter-targets-{}", std::process::id()));
        let base = root.join("repo");
        let named = root.join("repo-feature");
        let home = root.join("nested/home");
        let scratch = scratch_dir(&home).join("scratch-enter");
        fs::create_dir_all(&base).unwrap();
        fs::create_dir_all(&named).unwrap();
        fs::create_dir_all(&scratch).unwrap();

        let registered = vec![
            Worktree { path: base.clone(), head: Some("base".to_string()), branch: Some("mega".to_string()) },
            Worktree { path: named.clone(), head: Some("named".to_string()), branch: Some("feature".to_string()) },
            Worktree { path: scratch.clone(), head: Some("scratch".to_string()), branch: None },
        ];
        let options = EnterOptions { base: &base, current: &base, cwd: &base, home: &home, default_branch: "mega", target: "mega" };

        assert_eq!(resolve_enter_target(&options, &registered).unwrap(), base);
        let options = EnterOptions { target: "feature", ..options };
        assert_eq!(resolve_enter_target(&options, &registered).unwrap(), named);
        let options = EnterOptions { target: "scratch-enter", ..options };
        assert_eq!(resolve_enter_target(&options, &registered).unwrap(), scratch);
        let scratch_path = scratch.to_str().unwrap();
        let options = EnterOptions { target: scratch_path, ..options };
        assert_eq!(resolve_enter_target(&options, &registered).unwrap(), scratch);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn enter_refuses_ambiguous_unregistered_and_missing_targets() {
        let root = std::env::temp_dir().join(format!("branp-enter-refusals-{}", std::process::id()));
        let base = root.join("repo");
        let named = root.join("repo-scratch-collision");
        let home = root.join("nested/home");
        let scratch = scratch_dir(&home).join("scratch-collision");
        let missing = root.join("repo-missing");
        fs::create_dir_all(&base).unwrap();
        fs::create_dir_all(&named).unwrap();
        fs::create_dir_all(&scratch).unwrap();

        let registered = vec![
            Worktree { path: base.clone(), head: Some("base".to_string()), branch: Some("mega".to_string()) },
            Worktree { path: named.clone(), head: Some("named".to_string()), branch: Some("scratch-collision".to_string()) },
            Worktree { path: scratch.clone(), head: Some("scratch".to_string()), branch: None },
            Worktree { path: missing.clone(), head: Some("missing".to_string()), branch: Some("missing".to_string()) },
        ];
        let options = EnterOptions { base: &base, current: &base, cwd: &base, home: &home, default_branch: "mega", target: "scratch-collision" };
        let error = resolve_enter_target(&options, &registered).unwrap_err();
        assert!(error.message.contains("ambiguous"));
        assert!(error.message.contains(&named.display().to_string()));
        assert!(error.message.contains(&scratch.display().to_string()));

        let options = EnterOptions { target: "does-not-exist", ..options };
        let error = resolve_enter_target(&options, &registered).unwrap_err();
        assert!(error.message.contains("not a registered worktree"));

        let missing_path = missing.to_str().unwrap();
        let options = EnterOptions { target: missing_path, ..options };
        let error = resolve_enter_target(&options, &registered).unwrap_err();
        assert!(error.message.contains("is missing"));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn scratch_target_reservation_skips_collisions() {
        let root = std::env::temp_dir().join(format!("branp-scratch-reservation-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("scratch-test")).unwrap();

        let target = reserve_scratch_target_in(&root, "scratch-test").unwrap();
        assert_eq!(target, root.join("scratch-test-1"));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn scratch_reservation_is_exclusive_and_releases_after_drop() {
        let root = std::env::temp_dir().join(format!("branp-scratch-lock-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let target = root.join("scratch-test");

        let first = try_reserve_scratch(&root, &target).unwrap().unwrap();
        assert!(try_reserve_scratch(&root, &target).unwrap().is_none());
        drop(first);
        let third = try_reserve_scratch(&root, &target).unwrap().unwrap();
        drop(third);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn return_target_candidate_resolves_scratch_name_before_named_name() {
        let home = std::env::temp_dir().join(format!("branp-return-target-{}", std::process::id()));
        let scratch = scratch_dir(&home).join("scratch-test");
        fs::create_dir_all(&scratch).unwrap();
        let options = ReturnOptions {
            home: &home,
            base: Path::new("/tmp/repo"),
            current: Path::new("/tmp/repo"),
            target: Some("scratch-test"),
            discard_changes: false,
        };
        let worktrees = vec![Worktree { path: scratch.clone(), head: None, branch: None }];

        assert_eq!(return_target_candidate(&options, "scratch-test", &worktrees), scratch);
        fs::remove_dir_all(home).unwrap();
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
