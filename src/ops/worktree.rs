use std::fs;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

use toml_edit::{value, Array, DocumentMut, Item};

use crate::config::RepoConfig;
use crate::context::GlobalContext;
use crate::errors::{CliError, CliResult};

pub struct PathOptions<'a> {
    pub base: &'a Path,
    pub default_branch: &'a str,
    pub name: &'a str,
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

pub fn path(gctx: &mut GlobalContext, options: &PathOptions<'_>) -> CliResult {
    let path = if options.name == options.default_branch { options.base.to_path_buf() } else { target_dir(options.base, options.name) };
    gctx.shell().note(path.display());
    Ok(())
}

pub fn prune(repo: &Path) -> CliResult {
    crate::utils::git::run(repo, &["worktree", "prune"])?;
    Ok(())
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
