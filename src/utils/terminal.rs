use std::io::{self, IsTerminal, Write};
use std::sync::mpsc;
use std::time::Duration;

pub fn hyperlink(text: &str, url: &str) -> String {
    format!("\x1b]8;;{url}\x1b\\\x1b[4m{text}\x1b[24m\x1b]8;;\x1b\\")
}

pub fn supports_dynamic_lines() -> bool {
    io::stdout().is_terminal()
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
