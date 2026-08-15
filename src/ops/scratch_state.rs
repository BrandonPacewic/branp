use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use fs2::FileExt;
use toml_edit::{value, DocumentMut, Item, Table};

use crate::errors::CliError;

const STATE_VERSION: i64 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Availability {
    Available,
    Leased,
    Provisioning,
    Cleaning,
    Destroying,
    Quarantined,
}

impl Availability {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Leased => "leased",
            Self::Provisioning => "provisioning",
            Self::Cleaning => "cleaning",
            Self::Destroying => "destroying",
            Self::Quarantined => "quarantined",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "available" => Some(Self::Available),
            "leased" => Some(Self::Leased),
            "provisioning" => Some(Self::Provisioning),
            "cleaning" => Some(Self::Cleaning),
            "destroying" => Some(Self::Destroying),
            "quarantined" => Some(Self::Quarantined),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Entry {
    pub path: PathBuf,
    pub owner: Option<String>,
    pub availability: Availability,
    pub last_used: Option<u64>,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Registered {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct State {
    pub entries: BTreeMap<String, Entry>,
}

pub struct Store {
    root: PathBuf,
    state_path: PathBuf,
    lock_path: PathBuf,
}

impl Store {
    pub fn new(root: &Path) -> Self {
        Self { root: root.to_path_buf(), state_path: root.join("state.toml"), lock_path: root.join("state.toml.lock") }
    }

    pub fn transaction<F, R>(&self, registered: &[Registered], operation: F) -> Result<(R, Vec<String>), CliError>
    where F: FnOnce(&mut State) -> Result<R, CliError> {
        fs::create_dir_all(&self.root)?;
        let lock = OpenOptions::new().create(true).truncate(false).read(true).write(true).open(&self.lock_path)?;
        lock.lock_exclusive()?;

        let (mut state, mut changed, mut notices) = self.load_locked()?;
        let original = state.clone();
        changed |= reconcile(&mut state, registered, &mut notices);
        let result = operation(&mut state);
        changed |= state != original;

        let write_result = if changed { self.write_locked(&state) } else { Ok(()) };
        let _ = FileExt::unlock(&lock);

        match (result, write_result) {
            (Ok(value), Ok(())) => Ok((value, notices)),
            (Err(error), Ok(())) => Err(error),
            (_, Err(error)) => Err(error),
        }
    }

    pub fn read(&self, registered: &[Registered]) -> Result<(State, Vec<String>), CliError> {
        self.transaction(registered, |state| Ok(state.clone()))
    }

    pub fn state_path(&self) -> &Path {
        &self.state_path
    }

    fn load_locked(&self) -> Result<(State, bool, Vec<String>), CliError> {
        let mut contents = String::new();
        let mut notices = Vec::new();
        let state = match File::open(&self.state_path) {
            Ok(mut file) => {
                file.read_to_string(&mut contents)?;
                match parse_state(&contents) {
                    Ok(state) => state,
                    Err(reason) => {
                        let backup = self.backup_corrupt_state()?;
                        notices.push(format!(
                            "scratch state is corrupt ({reason}); quarantining registered scratch worktrees (preserved as {})",
                            backup.display()
                        ));
                        State::default()
                    }
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                notices.push("scratch state is missing; existing scratch worktrees are quarantined until recovered".to_string());
                State::default()
            }
            Err(error) => return Err(error.into()),
        };

        let changed = contents.is_empty() || !notices.is_empty();
        Ok((state, changed, notices))
    }

    fn backup_corrupt_state(&self) -> Result<PathBuf, CliError> {
        let timestamp = now_nanos();
        let backup = self.root.join(format!("state.toml.corrupt.{timestamp}"));
        fs::rename(&self.state_path, &backup)?;
        Ok(backup)
    }

    fn write_locked(&self, state: &State) -> Result<(), CliError> {
        let temporary = self.root.join(format!("state.toml.tmp.{}.{}", std::process::id(), now_nanos()));
        let mut file = OpenOptions::new().create_new(true).write(true).open(&temporary)?;
        let contents = render_state(state);
        if let Err(error) = file.write_all(contents.as_bytes()).and_then(|_| file.sync_all()) {
            let _ = fs::remove_file(&temporary);
            return Err(error.into());
        }
        drop(file);
        if let Err(error) = fs::rename(&temporary, &self.state_path) {
            let _ = fs::remove_file(&temporary);
            return Err(error.into());
        }
        if let Ok(directory) = File::open(&self.root) {
            let _ = directory.sync_all();
        }
        Ok(())
    }
}

pub fn owner_token() -> String {
    format!("pid:{}:{}", std::process::id(), now_nanos())
}

pub fn now_seconds() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

fn now_nanos() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos()
}

fn reconcile(state: &mut State, registered: &[Registered], notices: &mut Vec<String>) -> bool {
    let mut changed = false;
    for worktree in registered {
        let path = canonical_or_original(&worktree.path);
        let Some(entry) = state.entries.get_mut(&worktree.name) else {
            state.entries.insert(
                worktree.name.clone(),
                Entry {
                    path,
                    owner: None,
                    availability: Availability::Quarantined,
                    last_used: None,
                    reason: Some("state metadata is missing; use `bp worktree recover release` after inspection".to_string()),
                },
            );
            notices.push(format!("quarantined scratch worktree {}: state metadata is missing", worktree.path.display()));
            changed = true;
            continue;
        };

        if entry.path != path {
            entry.path = path;
            entry.owner = None;
            entry.availability = Availability::Quarantined;
            entry.reason = Some("state path does not match Git worktree metadata".to_string());
            notices.push(format!("quarantined scratch worktree {}: state path does not match", worktree.path.display()));
            changed = true;
        }
    }
    changed
}

fn canonical_or_original(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn parse_state(contents: &str) -> Result<State, String> {
    let document = contents.parse::<DocumentMut>().map_err(|error| error.to_string())?;
    let version = document.get("version").and_then(Item::as_integer).ok_or_else(|| "missing integer version".to_string())?;
    if version != STATE_VERSION {
        return Err(format!("unsupported version {version}, expected {STATE_VERSION}"));
    }

    let mut entries = BTreeMap::new();
    let Some(scratch) = document.get("scratch").and_then(Item::as_table) else {
        return Ok(State { entries });
    };
    for (name, item) in scratch.iter() {
        let table = item.as_table().ok_or_else(|| format!("scratch.{name} is not a table"))?;
        let path =
            table.get("path").and_then(Item::as_str).map(PathBuf::from).ok_or_else(|| format!("scratch.{name}.path is missing or not a string"))?;
        let availability = table
            .get("availability")
            .and_then(Item::as_str)
            .and_then(Availability::parse)
            .ok_or_else(|| format!("scratch.{name}.availability is missing or invalid"))?;
        let owner = table.get("owner").and_then(Item::as_str).map(str::to_string).filter(|owner| owner != "none");
        let last_used = table.get("last_used").and_then(Item::as_integer).map(|value| value as u64);
        let reason = table.get("reason").and_then(Item::as_str).map(str::to_string);
        entries.insert(name.to_string(), Entry { path, owner, availability, last_used, reason });
    }
    Ok(State { entries })
}

fn render_state(state: &State) -> String {
    let mut document = DocumentMut::new();
    document["version"] = value(STATE_VERSION);
    if !state.entries.is_empty() {
        document["scratch"] = Item::Table(Table::new());
    }
    for (name, entry) in &state.entries {
        let table = document["scratch"][name].or_insert(Item::Table(Table::new()));
        table["path"] = value(entry.path.display().to_string());
        table["owner"] = value(entry.owner.as_deref().unwrap_or("none"));
        table["availability"] = value(entry.availability.as_str());
        if let Some(last_used) = entry.last_used {
            table["last_used"] = value(last_used as i64);
        }
        if let Some(reason) = &entry.reason {
            table["reason"] = value(reason.as_str());
        }
    }
    document.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_round_trips_human_inspectable_metadata() {
        let state = State {
            entries: BTreeMap::from([(
                "scratch-one".to_string(),
                Entry {
                    path: PathBuf::from("/tmp/scratch-one"),
                    owner: Some("pid:42:123".to_string()),
                    availability: Availability::Leased,
                    last_used: Some(7),
                    reason: None,
                },
            )]),
        };
        let parsed = parse_state(&render_state(&state)).unwrap();
        assert_eq!(parsed, state);
        assert!(render_state(&state).contains("version = 1"));
        assert!(render_state(&state).contains("availability = \"leased\""));
    }

    #[test]
    fn invalid_state_is_rejected() {
        assert!(parse_state("version = 99").unwrap_err().contains("unsupported version"));
        assert!(parse_state("version = 1\n[scratch.bad]\navailability = \"unknown\"\npath = \"/tmp/bad\"\n").is_err());
    }

    #[test]
    fn missing_state_is_quarantined_then_can_be_released_atomically() {
        let root = std::env::temp_dir().join(format!("branp-state-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let scratch = root.join("scratch-one");
        fs::create_dir_all(&scratch).unwrap();
        let registered = [Registered { name: "scratch-one".to_string(), path: scratch.clone() }];
        let store = Store::new(&root);

        let (state, notices) = store.read(&registered).unwrap();
        assert_eq!(state.entries["scratch-one"].availability, Availability::Quarantined);
        assert!(!notices.is_empty());
        assert!(store.state_path().is_file());

        store
            .transaction(&registered, |state| {
                let entry = state.entries.get_mut("scratch-one").unwrap();
                entry.availability = Availability::Available;
                entry.reason = None;
                Ok(())
            })
            .unwrap();
        let contents = fs::read_to_string(store.state_path()).unwrap();
        assert!(contents.contains("version = 1"));
        assert!(contents.contains("availability = \"available\""));

        fs::remove_dir_all(root).unwrap();
    }
}
