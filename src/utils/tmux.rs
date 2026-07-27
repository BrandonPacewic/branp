use std::path::Path;

use crate::context::GlobalContext;
use crate::errors::{CliError, CliResult};

const HYPERLINK_MOUSE_BINDING: &str =
    "bind-key -n MouseDown1Pane select-pane -t = \\; if-shell -F \"#{mouse_hyperlink}\" \"run-shell 'open #{q:mouse_hyperlink}'\" \"send-keys -M\"";

pub fn is_inside_tmux() -> bool {
    std::env::var_os("TMUX").is_some()
}

pub fn has_session(session: &str) -> Result<bool, CliError> {
    crate::utils::command::status("tmux", &["has-session", "-t", session], None)
}

pub fn sessions() -> Result<Vec<String>, CliError> {
    Ok(crate::utils::command::output("tmux", &["list-sessions", "-F", "#{session_name}"], None)?.lines().map(str::to_string).collect())
}

pub fn ensure_session(gctx: &mut GlobalContext, session: &str, cwd: &Path) -> CliResult {
    if has_session(session)? {
        return Ok(());
    }

    let cwd = path_arg(cwd);
    crate::utils::command::run("tmux", &["new-session", "-d", "-s", session, "-c", cwd.as_str()], None)?;
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

pub fn version() -> Result<TmuxVersion, CliError> {
    Ok(TmuxVersion::parse(crate::utils::command::output("tmux", &["-V"], None)?.trim()))
}

pub fn client_features() -> Result<TmuxFeatures, CliError> {
    Ok(TmuxFeatures::parse(crate::utils::command::output("tmux", &["display-message", "-p", "#{client_termfeatures}"], None)?.trim()))
}

pub fn terminal_features() -> Result<TmuxFeatures, CliError> {
    Ok(TmuxFeatures::parse(&crate::utils::command::output("tmux", &["show-options", "-g", "terminal-features"], None)?))
}

pub fn mouse_mode() -> Result<bool, CliError> {
    let output = crate::utils::command::output("tmux", &["show-options", "-gqv", "mouse"], None)?;
    Ok(output.trim() == "on")
}

pub fn mouse_down1_pane_binding() -> Result<TmuxBinding, CliError> {
    Ok(TmuxBinding::parse(&crate::utils::command::output("tmux", &["list-keys", "MouseDown1Pane"], None)?))
}

pub fn hyperlink_config_lines() -> Vec<String> {
    vec![
        "set -as terminal-features ',*:hyperlinks'".to_string(),
        "set -as terminal-features ',xterm*:hyperlinks'".to_string(),
        HYPERLINK_MOUSE_BINDING.to_string(),
    ]
}

pub struct TmuxConfig {
    raw: String,
}

impl TmuxConfig {
    pub fn parse(raw: &str) -> Self {
        Self { raw: raw.to_string() }
    }

    pub fn has_hyperlink_terminal_features(&self) -> bool {
        self.active_lines().any(|line| {
            line.contains("terminal-features") && line.contains("hyperlinks") && (line.contains("*:hyperlinks") || line.contains("xterm*:hyperlinks"))
        })
    }

    pub fn has_mouse_hyperlink_binding(&self) -> bool {
        self.active_lines().any(|line| line.contains("MouseDown1Pane") && line.contains("mouse_hyperlink") && line.contains("open"))
    }

    fn active_lines(&self) -> impl Iterator<Item = &str> {
        self.raw.lines().map(str::trim).filter(|line| !line.is_empty() && !line.starts_with('#'))
    }
}

#[derive(Default)]
pub struct TmuxFeatures {
    raw: String,
}

impl TmuxFeatures {
    pub fn parse(raw: &str) -> Self {
        Self { raw: raw.trim().to_string() }
    }

    pub fn raw(&self) -> &str {
        &self.raw
    }

    pub fn has(&self, feature: &str) -> bool {
        self.raw.split(|c: char| c == ',' || c == ':' || c.is_whitespace()).any(|candidate| candidate == feature)
    }
}

pub struct TmuxVersion {
    raw: String,
    major: Option<u32>,
    minor: Option<u32>,
}

impl TmuxVersion {
    pub fn parse(raw: &str) -> Self {
        let numeric = raw.strip_prefix("tmux ").unwrap_or("").chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect::<String>();
        let mut parts = numeric.split('.');
        Self {
            raw: raw.to_string(),
            major: parts.next().and_then(|value| value.parse::<u32>().ok()),
            minor: parts.next().and_then(|value| value.parse::<u32>().ok()),
        }
    }

    pub fn raw(&self) -> &str {
        &self.raw
    }

    pub fn supports_hyperlinks(&self) -> bool {
        matches!((self.major, self.minor), (Some(major), Some(minor)) if major > 3 || (major == 3 && minor >= 4))
    }
}

#[derive(Default)]
pub struct TmuxBinding {
    raw: String,
}

impl TmuxBinding {
    pub fn parse(raw: &str) -> Self {
        Self { raw: raw.trim().to_string() }
    }

    pub fn opens_mouse_hyperlink(&self) -> bool {
        self.raw.contains("mouse_hyperlink")
    }
}

fn path_arg(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_tmux_hyperlink_versions() {
        assert!(TmuxVersion::parse("tmux 3.4").supports_hyperlinks());
        assert!(TmuxVersion::parse("tmux 3.6a").supports_hyperlinks());
        assert!(TmuxVersion::parse("tmux 4.0").supports_hyperlinks());
        assert!(!TmuxVersion::parse("tmux 3.3a").supports_hyperlinks());
        assert!(!TmuxVersion::parse("screen 4.9").supports_hyperlinks());
    }

    #[test]
    fn finds_features_in_tmux_option_output() {
        let features = TmuxFeatures::parse("terminal-features[0] xterm*:clipboard:ccolour:cstyle:focus:title\nterminal-features[1] *:hyperlinks");
        assert!(features.has("hyperlinks"));
        assert!(features.has("clipboard"));
        assert!(!features.has("sixel"));
    }

    #[test]
    fn config_detection_ignores_commented_lines() {
        let config = TmuxConfig::parse(
            r##"
            # set -as terminal-features ',*:hyperlinks'
            # bind-key -n MouseDown1Pane if-shell -F "#{mouse_hyperlink}" "run-shell 'open #{q:mouse_hyperlink}'" "send-keys -M"
            "##,
        );
        assert!(!config.has_hyperlink_terminal_features());
        assert!(!config.has_mouse_hyperlink_binding());
    }

    #[test]
    fn config_detection_finds_active_lines() {
        let config = TmuxConfig::parse(
            r##"
            set -as terminal-features ',xterm*:hyperlinks'
            bind-key -n MouseDown1Pane select-pane -t = \; if-shell -F "#{mouse_hyperlink}" "run-shell 'open #{q:mouse_hyperlink}'" "send-keys -M"
            "##,
        );
        assert!(config.has_hyperlink_terminal_features());
        assert!(config.has_mouse_hyperlink_binding());
    }
}
