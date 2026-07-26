use std::io::{self, IsTerminal, Write};

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
        Ok(Self {
            line_count: lines.len(),
            enabled,
        })
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

fn write_lines(lines: &[String]) -> io::Result<()> {
    let mut stdout = io::stdout();
    for line in lines {
        writeln!(stdout, "{line}")?;
    }
    stdout.flush()
}
