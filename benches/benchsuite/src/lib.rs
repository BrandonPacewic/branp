use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Fixtures {
    target_tmpdir: PathBuf,
}

impl Fixtures {
    pub fn new(target_tmpdir: &str) -> Self {
        let fixtures = Self { target_tmpdir: PathBuf::from(target_tmpdir) };
        fixtures.unpack_archives();
        fixtures
    }

    fn root(&self) -> PathBuf {
        self.target_tmpdir.join("bench")
    }

    fn workspace_root(&self) -> PathBuf {
        self.root().join("workspaces")
    }

    fn archive_root(&self) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("workspaces")
    }

    fn metadata_path(&self) -> PathBuf {
        self.root().join("cloc-inputs.json")
    }

    fn unpack_archives(&self) {
        let archive_root = self.archive_root();
        let Ok(entries) = fs::read_dir(&archive_root) else {
            return;
        };

        fs::create_dir_all(self.workspace_root()).unwrap();

        for archive_path in
            entries.filter_map(Result::ok).map(|entry| entry.path()).filter(|path| path.extension().is_some_and(|extension| extension == "tgz"))
        {
            let name = archive_path.file_stem().unwrap();
            let dest = self.workspace_root().join(name);
            if dest.exists() {
                fs::remove_dir_all(&dest).unwrap();
            }

            let file = fs::File::open(&archive_path).unwrap();
            let decoder = flate2::read::GzDecoder::new(file);
            let mut archive = tar::Archive::new(decoder);
            archive.unpack(self.workspace_root()).unwrap();
        }
    }

    pub fn cloc_inputs(&self) -> Vec<ClocInput> {
        let mut inputs = fs::read_dir(self.workspace_root())
            .ok()
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .map(|path| {
                let name = path.file_name().unwrap().to_str().unwrap().to_owned();
                ClocInput::new(name, path)
            })
            .collect::<Vec<_>>();

        if let Some(path) = external_repo_path() {
            if !path.is_dir() {
                panic!("BP_CLOC_BENCH_REPO is not a directory: {}", path.display());
            }
            inputs.push(ClocInput::new("large-repo".to_owned(), path));
        }

        inputs.sort_by(|left, right| left.name.cmp(&right.name));
        self.write_metadata(&inputs);
        inputs
    }

    fn write_metadata(&self, inputs: &[ClocInput]) {
        fs::create_dir_all(self.root()).unwrap();
        let metadata = render_metadata(inputs);
        fs::write(self.metadata_path(), metadata).unwrap();
    }
}

pub struct ClocInput {
    pub name: String,
    pub path: PathBuf,
    git: GitInfo,
}

impl ClocInput {
    fn new(name: String, path: PathBuf) -> Self {
        let git = GitInfo::for_path(&path);
        Self { name, path, git }
    }
}

#[derive(Default)]
struct GitInfo {
    head: Option<String>,
    branch: Option<String>,
    remote: Option<String>,
    dirty: Option<bool>,
}

#[macro_export]
macro_rules! fixtures {
    () => {
        $crate::Fixtures::new(env!("CARGO_TARGET_TMPDIR"))
    };
}

impl GitInfo {
    fn for_path(path: &Path) -> Self {
        let Some(root) = git_output(path, &["rev-parse", "--show-toplevel"]) else {
            return Self::default();
        };
        let Ok(root) = fs::canonicalize(root) else {
            return Self::default();
        };
        let Ok(path) = fs::canonicalize(path) else {
            return Self::default();
        };
        if root != path {
            return Self::default();
        }

        Self {
            head: git_output(&path, &["rev-parse", "HEAD"]),
            branch: git_output(&path, &["branch", "--show-current"]),
            remote: git_output(&path, &["remote", "get-url", "origin"]),
            dirty: git_dirty(&path),
        }
    }
}

fn external_repo_path() -> Option<PathBuf> {
    std::env::var_os("BP_CLOC_BENCH_REPO").or_else(|| env_file_value("BP_CLOC_BENCH_REPO")).map(PathBuf::from)
}

fn env_file_value(key: &str) -> Option<std::ffi::OsString> {
    let env_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".env");
    let contents = fs::read_to_string(env_path).ok()?;
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((candidate, value)) = line.split_once('=') else {
            continue;
        };
        if candidate.trim() == key {
            return Some(strip_quotes(value.trim()).into());
        }
    }
    None
}

fn strip_quotes(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| value.strip_prefix('\'').and_then(|value| value.strip_suffix('\'')))
        .unwrap_or(value)
}

fn git_output(path: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git").current_dir(path).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8(output.stdout).ok()?;
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

fn git_dirty(path: &Path) -> Option<bool> {
    let status = Command::new("git").current_dir(path).args(["diff-index", "--quiet", "HEAD", "--"]).status().ok()?;
    Some(!status.success())
}

fn render_metadata(inputs: &[ClocInput]) -> String {
    let mut output = String::from("[\n");
    for (index, input) in inputs.iter().enumerate() {
        if index > 0 {
            output.push_str(",\n");
        }

        output.push_str("  {\n");
        output.push_str(&format!("    \"name\": {},\n", json_string(&input.name)));
        output.push_str(&format!("    \"path\": {},\n", json_string(&input.path.display().to_string())));
        output.push_str(&format!("    \"git_head\": {},\n", json_optional_string(input.git.head.as_deref())));
        output.push_str(&format!("    \"git_branch\": {},\n", json_optional_string(input.git.branch.as_deref())));
        output.push_str(&format!("    \"git_remote\": {},\n", json_optional_string(input.git.remote.as_deref())));
        output.push_str(&format!("    \"git_dirty\": {}\n", json_optional_bool(input.git.dirty)));
        output.push_str("  }");
    }
    output.push_str("\n]\n");
    output
}

fn json_optional_string(value: Option<&str>) -> String {
    value.map(json_string).unwrap_or_else(|| "null".to_owned())
}

fn json_optional_bool(value: Option<bool>) -> String {
    value.map(|value| value.to_string()).unwrap_or_else(|| "null".to_owned())
}

fn json_string(value: &str) -> String {
    let mut output = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            ch if ch.is_control() => output.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => output.push(ch),
        }
    }
    output.push('"');
    output
}
