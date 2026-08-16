use std::io::{self, IsTerminal, Write};
use std::sync::mpsc;
use std::time::Duration;

pub const DEFAULT_DISPLAY_WIDTH: usize = 120;

pub fn hyperlink(text: &str, url: &str) -> String {
    format!("\x1b]8;;{url}\x1b\\\x1b[4m{text}\x1b[24m\x1b]8;;\x1b\\")
}

pub fn supports_dynamic_lines() -> bool {
    io::stdout().is_terminal()
}

/// Select the stable human-output width: an explicit `COLUMNS` value, the
/// connected terminal width, or 120 columns for non-TTY output.
pub fn display_width() -> usize {
    configured_display_width(std::env::var("COLUMNS").ok().as_deref(), terminal_width()).unwrap_or(DEFAULT_DISPLAY_WIDTH)
}

fn configured_display_width(columns: Option<&str>, terminal_width: Option<usize>) -> Option<usize> {
    columns.and_then(|columns| columns.parse::<usize>().ok()).filter(|columns| *columns > 0).or(terminal_width).filter(|width| *width > 0)
}

#[cfg(unix)]
fn terminal_width() -> Option<usize> {
    use std::os::fd::AsRawFd;

    if !supports_dynamic_lines() {
        return None;
    }

    let mut size = libc::winsize { ws_row: 0, ws_col: 0, ws_xpixel: 0, ws_ypixel: 0 };
    let result = unsafe { libc::ioctl(io::stdout().as_raw_fd(), libc::TIOCGWINSZ, &mut size) };
    (result == 0 && size.ws_col > 0).then_some(size.ws_col as usize)
}

#[cfg(not(unix))]
fn terminal_width() -> Option<usize> {
    None
}

pub struct DynamicLines {
    line_count: usize,
    enabled: bool,
}

impl DynamicLines {
    pub fn print(lines: &[String]) -> io::Result<Self> {
        let enabled = supports_dynamic_lines();
        write_lines(lines)?;
        Ok(Self { line_count: lines.len(), enabled })
    }

    pub fn replace(&mut self, lines: &[String]) -> io::Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let mut stdout = io::stdout();
        if self.line_count > 0 {
            write!(stdout, "\x1b[{}A", self.line_count)?;
        }
        for line in lines {
            write!(stdout, "\r\x1b[2K{line}\n")?;
        }
        stdout.flush()?;
        self.line_count = lines.len();
        Ok(())
    }
}

pub struct DynamicRenderLoop {
    frame: DynamicLines,
    spinner: Vec<char>,
    tick: usize,
    tick_interval: Duration,
}

impl DynamicRenderLoop {
    pub fn start(lines: &[String]) -> io::Result<Self> {
        Ok(Self { frame: DynamicLines::print(lines)?, spinner: vec!['-', '\\', '|', '/'], tick: 0, tick_interval: Duration::from_millis(120) })
    }

    pub fn tick_interval(&self) -> Duration {
        self.tick_interval
    }

    pub fn spinner(&self) -> char {
        self.spinner[self.tick % self.spinner.len()]
    }

    pub fn advance(&mut self, lines: &[String]) -> io::Result<()> {
        self.tick += 1;
        self.frame.replace(lines)
    }

    pub fn replace(&mut self, lines: &[String]) -> io::Result<()> {
        self.frame.replace(lines)
    }
}

pub fn render_dynamic_updates<State, Update>(
    state: &mut State,
    pending: usize,
    receiver: mpsc::Receiver<Update>,
    mut render: impl FnMut(&State, Option<char>) -> Vec<String>,
    mut apply: impl FnMut(&mut State, Update),
) -> io::Result<()> {
    let mut pending = pending;
    let mut frame = DynamicRenderLoop::start(&render(state, Some('-')))?;

    while pending > 0 {
        match receiver.recv_timeout(frame.tick_interval()) {
            Ok(update) => {
                apply(state, update);
                pending -= 1;

                while let Ok(update) = receiver.try_recv() {
                    apply(state, update);
                    pending -= 1;
                }

                frame.replace(&render(state, Some(frame.spinner())))?;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                frame.advance(&render(state, Some(frame.spinner())))?;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    frame.replace(&render(state, None))
}

fn write_lines(lines: &[String]) -> io::Result<()> {
    let mut stdout = io::stdout();
    for line in lines {
        writeln!(stdout, "{line}")?;
    }
    stdout.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configured_width_prefers_valid_columns_over_terminal_width() {
        assert_eq!(configured_display_width(Some("80"), Some(120)), Some(80));
        assert_eq!(configured_display_width(Some("0"), Some(120)), Some(120));
        assert_eq!(configured_display_width(Some("invalid"), Some(120)), Some(120));
    }

    #[test]
    fn configured_width_uses_stable_fallback_when_no_width_is_available() {
        assert_eq!(configured_display_width(None, None).unwrap_or(DEFAULT_DISPLAY_WIDTH), 120);
    }
}
