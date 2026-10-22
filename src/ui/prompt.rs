use std::io::{self, Write};

use crate::error::{AppError, Result};
use crate::model::Outcome;

use super::Painter;

/// Read one line. `Ok(None)` means end of input, which callers must treat as
/// "save and stop" rather than as an empty answer — that distinction is what
/// used to turn Ctrl-D into an infinite loop.
pub fn read_line(prompt: &str) -> Result<Option<String>> {
    print!("{prompt}");
    io::stdout()
        .flush()
        .map_err(|error| AppError::io("<stdout>", error))?;

    let mut buffer = String::new();
    let read = io::stdin()
        .read_line(&mut buffer)
        .map_err(|error| AppError::io("<stdin>", error))?;

    if read == 0 {
        Ok(None)
    } else {
        Ok(Some(buffer.trim().to_string()))
    }
}

/// Prompt for a result until the input parses. `None` means the user asked to
/// stop (`q`, `quit`, `exit`) or input ended.
pub fn read_outcome(prompt: &str, painter: &Painter) -> Result<Option<Outcome>> {
    loop {
        let Some(line) = read_line(prompt)? else {
            return Ok(None);
        };

        if matches!(line.to_ascii_lowercase().as_str(), "q" | "quit" | "exit") {
            return Ok(None);
        }

        if let Some(outcome) = Outcome::parse(&line) {
            return Ok(Some(outcome));
        }

        println!(
            "{}",
            painter.yellow("  Enter 1-0, 0-1 or 1/2-1/2 (legacy: 1, 0, -1), or q to save and quit.")
        );
    }
}
