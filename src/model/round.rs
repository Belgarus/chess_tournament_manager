use serde::{Deserialize, Serialize};

use super::{Game, PlayerId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Round {
    pub number: usize,
    pub games: Vec<Game>,
    /// `None` for an even field. In a round-robin with an odd field every
    /// player sits out exactly once, so this is a property of the round rather
    /// than a fake player.
    pub bye: Option<PlayerId>,
}

impl Round {
    pub fn is_complete(&self) -> bool {
        self.games.iter().all(|game| game.result.is_some())
    }

    pub fn pending_boards(&self) -> Vec<usize> {
        self.games
            .iter()
            .filter(|game| game.result.is_none())
            .map(|game| game.board)
            .collect()
    }

    pub fn game(&self, board: usize) -> Option<&Game> {
        self.games.iter().find(|game| game.board == board)
    }

    pub fn involves(&self, id: PlayerId) -> bool {
        self.bye == Some(id) || self.games.iter().any(|game| game.player(id).is_some())
    }
}
