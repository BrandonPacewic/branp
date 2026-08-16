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
    lines: Vec<String>,
    line_count: usize,
    enabled: bool,
}

impl DynamicLines {
    pub fn print(lines: &[String]) -> io::Result<Self> {
        let enabled = supports_dynamic_lines();
        write_lines(lines)?;
        Ok(Self { lines: lines.to_vec(), line_count: lines.len(), enabled })
    }

    pub fn replace(&mut self, lines: &[String]) -> io::Result<()> {
        self.replace_if_changed(lines).map(|_| ())
    }

    fn replace_if_changed(&mut self, lines: &[String]) -> io::Result<bool> {
        if self.lines == lines {
            return Ok(false);
        }

        if !self.enabled {
            self.lines = lines.to_vec();
            self.line_count = lines.len();
            return Ok(true);
        }

        let mut stdout = io::stdout();
        write_replacement(&mut stdout, self.line_count, lines)?;
        stdout.flush()?;
        self.lines = lines.to_vec();
        self.line_count = lines.len();
        Ok(true)
    }
}

fn write_replacement(writer: &mut impl Write, previous_line_count: usize, lines: &[String]) -> io::Result<()> {
    if previous_line_count > 0 {
        write!(writer, "\x1b[{previous_line_count}A")?;
    }

    let line_count = previous_line_count.max(lines.len());
    for index in 0..line_count {
        let line = lines.get(index).map(String::as_str).unwrap_or_default();
        write!(writer, "\r\x1b[2K{line}\n")?;
    }

    if lines.len() < previous_line_count {
        let cleared_lines = previous_line_count - lines.len();
        write!(writer, "\x1b[{cleared_lines}A")?;
    }

    Ok(())
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

                frame.replace(&render(state, Some('-')))?;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
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
    fn replacement_clears_trailing_lines_and_restores_cursor_position() {
        let mut output = Vec::new();
        write_replacement(&mut output, 3, &["updated".to_string()]).unwrap();

        assert_eq!(String::from_utf8(output).unwrap(), "\x1b[3A\r\x1b[2Kupdated\n\r\x1b[2K\n\r\x1b[2K\n\x1b[2A");
    }

    #[test]
    fn replacement_preserves_all_lines_when_frame_grows() {
        let mut output = Vec::new();
        write_replacement(&mut output, 1, &["first".to_string(), "second".to_string()]).unwrap();

        assert_eq!(String::from_utf8(output).unwrap(), "\x1b[1A\r\x1b[2Kfirst\n\r\x1b[2Ksecond\n");
    }

    #[test]
    fn identical_frames_are_not_replacements() {
        let lines = vec!["same".to_string()];
        let mut frame = DynamicLines { lines: lines.clone(), line_count: lines.len(), enabled: false };

        assert!(!frame.replace_if_changed(&lines).unwrap());
        assert!(frame.replace_if_changed(&["changed".to_string()]).unwrap());

        assert_eq!(frame.lines, vec!["changed".to_string()]);
    }

    #[test]
    fn dynamic_updates_do_not_render_timeout_only_frames() {
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(250));
            sender.send(()).unwrap();
        });

        let mut state = 0;
        let mut renders = 0;
        render_dynamic_updates(
            &mut state,
            1,
            receiver,
            |state, _spinner| {
                renders += 1;
                vec![state.to_string()]
            },
            |state, ()| *state += 1,
        )
        .unwrap();

        assert_eq!(state, 1);
        assert_eq!(renders, 3, "initial, changed update, and final render only");
    }

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
