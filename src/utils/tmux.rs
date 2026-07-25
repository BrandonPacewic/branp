use std::path::Path;

use crate::context::GlobalContext;
use crate::errors::{CliError, CliResult};

pub fn has_session(session: &str) -> Result<bool, CliError> {
    crate::utils::command::status("tmux", &["has-session", "-t", session], None)
}

pub fn ensure_session(gctx: &mut GlobalContext, session: &str, cwd: &Path) -> CliResult {
    if has_session(session)? {
        return Ok(());
    }

    let cwd = path_arg(cwd);
    crate::utils::command::run(
        "tmux",
        &["new-session", "-d", "-s", session, "-c", cwd.as_str()],
        None,
    )?;
    gctx.shell().note(format!("tmux: started `{session}`"));
    Ok(())
}

pub fn kill_session(gctx: &mut GlobalContext, session: &str) -> CliResult {
    if has_session(session)? {
        crate::utils::command::run("tmux", &["kill-session", "-t", session], None)?;
        gctx.shell().note(format!("tmux: killed `{session}`"));
    }
    Ok(())
}

fn path_arg(path: &Path) -> String {
    path.to_string_lossy().to_string()
}
