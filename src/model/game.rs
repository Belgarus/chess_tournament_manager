use serde::{Deserialize, Serialize};

use super::PlayerId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub fn opposite(self) -> Self {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }

    pub fn symbol(self) -> char {
        match self {
            Color::White => 'W',
            Color::Black => 'B',
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    #[serde(rename = "1-0")]
    WhiteWin,
    #[serde(rename = "0-1")]
    BlackWin,
    #[serde(rename = "1/2-1/2")]
    Draw,
}

impl Outcome {
    /// Accepts chess notation (`1-0`, `0-1`, `1/2-1/2`, `½-½`) and the legacy
    /// three-key input (`1`, `0`, `-1`). `0` means draw there, which is why it
    /// is matched before any prefix logic.
    pub fn parse(input: &str) -> Option<Self> {
        match input.trim().to_ascii_lowercase().as_str() {
            "1-0" | "1" => Some(Outcome::WhiteWin),
            "0-1" | "-1" => Some(Outcome::BlackWin),
            "1/2-1/2" | "½-½" | "0.5-0.5" | "0" | "d" | "draw" => Some(Outcome::Draw),
            _ => None,
        }
    }

    pub fn pgn(self) -> &'static str {
        match self {
            Outcome::WhiteWin => "1-0",
            Outcome::BlackWin => "0-1",
            Outcome::Draw => "1/2-1/2",
        }
    }

    pub fn score_for(self, color: Color) -> f32 {
        match (self, color) {
            (Outcome::Draw, _) => 0.5,
            (Outcome::WhiteWin, Color::White) | (Outcome::BlackWin, Color::Black) => 1.0,
            _ => 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub board: usize,
    pub white: PlayerId,
    pub black: PlayerId,
    pub result: Option<Outcome>,
}

impl Game {
    pub fn player(&self, id: PlayerId) -> Option<(Color, PlayerId)> {
        if self.white == id {
            Some((Color::White, self.black))
        } else if self.black == id {
            Some((Color::Black, self.white))
        } else {
            None
        }
    }

    pub fn score_of(&self, id: PlayerId) -> Option<f32> {
        let (color, _) = self.player(id)?;
        self.result.map(|outcome| outcome.score_for(color))
    }
}

/// How often and how recently a player had each colour.
///
/// `streak` counts consecutive games with `last`, so colour allocation can
/// refuse a third identical colour in a row instead of only alternating.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ColorHistory {
    pub whites: u32,
    pub blacks: u32,
    pub last: Option<Color>,
    pub streak: u32,
}

impl ColorHistory {
    pub fn record(&mut self, color: Color) {
        match color {
            Color::White => self.whites += 1,
            Color::Black => self.blacks += 1,
        }
        if self.last == Some(color) {
            self.streak += 1;
        } else {
            self.streak = 1;
        }
        self.last = Some(color);
    }

    pub fn imbalance(&self) -> i32 {
        self.whites as i32 - self.blacks as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_both_notations() {
        assert_eq!(Outcome::parse("1-0"), Some(Outcome::WhiteWin));
        assert_eq!(Outcome::parse("1"), Some(Outcome::WhiteWin));
        assert_eq!(Outcome::parse("0-1"), Some(Outcome::BlackWin));
        assert_eq!(Outcome::parse("-1"), Some(Outcome::BlackWin));
        assert_eq!(Outcome::parse("1/2-1/2"), Some(Outcome::Draw));
        assert_eq!(Outcome::parse("½-½"), Some(Outcome::Draw));
        assert_eq!(Outcome::parse("0"), Some(Outcome::Draw));
        assert_eq!(Outcome::parse("2"), None);
    }

    #[test]
    fn legacy_zero_is_a_draw_not_a_black_win() {
        assert_eq!(Outcome::parse("0"), Some(Outcome::Draw));
    }

    #[test]
    fn tracks_streaks() {
        let mut h = ColorHistory::default();
        h.record(Color::White);
        h.record(Color::White);
        assert_eq!(h.streak, 2);
        h.record(Color::Black);
        assert_eq!(h.streak, 1);
        assert_eq!(h.imbalance(), 1);
    }
}
