pub mod prompt;
pub mod table;

use std::fmt::Display;
use std::io::IsTerminal;

use clap::ValueEnum;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ColorChoice {
    Auto,
    Always,
    Never,
}

/// ANSI colouring that stays out of the way when output is piped or when the
/// user asks for no colour. Colour is a rendering decision, never part of the
/// data.
#[derive(Debug, Clone, Copy)]
pub struct Painter {
    enabled: bool,
}

impl Painter {
    pub fn detect(choice: ColorChoice) -> Self {
        let enabled = match choice {
            ColorChoice::Always => true,
            ColorChoice::Never => false,
            ColorChoice::Auto => {
                std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none()
            }
        };
        Self { enabled }
    }

    pub fn plain() -> Self {
        Self { enabled: false }
    }

    pub fn enabled(self) -> bool {
        self.enabled
    }

    fn paint(self, code: &str, text: &str) -> String {
        if self.enabled {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    fn of(self, code: &str, text: impl Display) -> String {
        self.paint(code, &text.to_string())
    }

    pub fn bold(self, text: impl Display) -> String {
        self.of("1", text)
    }

    pub fn dim(self, text: impl Display) -> String {
        self.of("2", text)
    }

    pub fn green(self, text: impl Display) -> String {
        self.of("32", text)
    }

    pub fn yellow(self, text: impl Display) -> String {
        self.of("33", text)
    }

    pub fn red(self, text: impl Display) -> String {
        self.of("31", text)
    }

    pub fn cyan(self, text: impl Display) -> String {
        self.of("36", text)
    }

    pub fn orange(self, text: impl Display) -> String {
        self.of("38;5;208", text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn always_paints() {
        let painter = Painter::detect(ColorChoice::Always);
        assert!(painter.bold("x").contains("\x1b[1m"));
    }

    #[test]
    fn never_stays_plain() {
        let painter = Painter::detect(ColorChoice::Never);
        assert_eq!(painter.bold("x"), "x");
        assert_eq!(painter.green(7), "7");
    }
}
