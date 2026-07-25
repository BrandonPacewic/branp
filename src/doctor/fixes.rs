use std::fs;
use std::path::PathBuf;

use crate::errors::{CliError, CliResult};

pub enum Fix {
    MarkerBlock {
        path: PathBuf,
        marker: &'static str,
        lines: Vec<String>,
        description: String,
    },
    Manual {
        description: String,
        instructions: Vec<String>,
    },
}

pub struct ApplyOutcome {
    pub description: String,
    pub warnings: Vec<String>,
}

impl Fix {
    pub fn description(&self) -> &str {
        match self {
            Fix::MarkerBlock { description, .. } | Fix::Manual { description, .. } => description,
        }
    }

    fn key(&self) -> String {
        match self {
            Fix::MarkerBlock { path, marker, .. } => {
                format!("marker:{}:{marker}", path.display())
            }
            Fix::Manual { description, .. } => format!("manual:{description}"),
        }
    }
}

pub fn dedupe_fixes(fixes: Vec<Fix>) -> Vec<Fix> {
    let mut deduped = Vec::new();
    for fix in fixes {
        let key = fix.key();
        if !deduped.iter().any(|existing: &Fix| existing.key() == key) {
            deduped.push(fix);
        }
    }
    deduped
}

pub fn apply_fix(fix: Fix) -> Result<ApplyOutcome, CliError> {
    match fix {
        Fix::MarkerBlock {
            path,
            marker,
            lines,
            description,
        } => {
            upsert_marker_block(&path, marker, &lines)?;
            Ok(ApplyOutcome {
                description: format!("{description} ({})", path.display()),
                warnings: Vec::new(),
            })
        }
        Fix::Manual {
            description,
            instructions,
        } => Ok(ApplyOutcome {
            description,
            warnings: instructions,
        }),
    }
}

fn upsert_marker_block(path: &PathBuf, marker: &str, lines: &[String]) -> CliResult {
    let begin = format!("# >>> {marker}");
    let end = format!("# <<< {marker}");
    let block = format!("{begin}\n{}\n{end}", lines.join("\n"));
    let existing = fs::read_to_string(path).unwrap_or_default();

    let updated = if let Some(start) = existing.find(&begin) {
        let Some(relative_end) = existing[start..].find(&end) else {
            return Err(CliError::from(format!(
                "found `{begin}` in {}, but missing `{end}`",
                path.display()
            )));
        };
        let end_index = start + relative_end + end.len();
        format!("{}{}{}", &existing[..start], block, &existing[end_index..])
    } else if existing.trim().is_empty() {
        format!("{block}\n")
    } else {
        format!("{}\n{block}\n", existing.trim_end())
    };

    fs::write(path, updated)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn marker_block_is_inserted_and_replaced() {
        let path = temp_path("marker-block");
        fs::write(&path, "set -g mouse on\n").unwrap();

        assert!(upsert_marker_block(&path, "bp doctor: test", &["first".to_string()]).is_ok());
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "set -g mouse on\n# >>> bp doctor: test\nfirst\n# <<< bp doctor: test\n"
        );

        assert!(upsert_marker_block(&path, "bp doctor: test", &["second".to_string()]).is_ok());
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "set -g mouse on\n# >>> bp doctor: test\nsecond\n# <<< bp doctor: test\n"
        );

        let _ = fs::remove_file(path);
    }

    fn temp_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("bp-doctor-{name}-{nanos}"))
    }
}
