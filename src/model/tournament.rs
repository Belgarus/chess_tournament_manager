use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};
use crate::pairing::round_robin;
use crate::standings::{self, StandingRow};
use crate::tiebreak::Tiebreak;

use super::{Outcome, Player, PlayerId, Round, MAX_RATING};

/// A round-robin tournament.
///
/// Rounds are paired lazily, one at a time, and kept forever. Nothing is
/// cached: if a result is wrong, change it and every derived number follows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tournament {
    pub name: String,
    pub double_round_robin: bool,
    pub players: Vec<Player>,
    pub rounds: Vec<Round>,
}

impl Tournament {
    pub fn new(name: String, players: Vec<Player>, double_round_robin: bool) -> Self {
        Self {
            name,
            double_round_robin,
            players,
            rounds: Vec::new(),
        }
    }

    pub fn player(&self, id: PlayerId) -> Option<&Player> {
        self.players.get(id.0)
    }

    pub fn player_name(&self, id: PlayerId) -> &str {
        self.player(id).map(|p| p.name.as_str()).unwrap_or("?")
    }

    pub fn rating_of(&self, id: PlayerId) -> u32 {
        self.player(id).map(|p| p.rating).unwrap_or(0)
    }

    /// Number of rounds in one full cycle. With an odd field this equals the
    /// number of players, because the phantom slot still consumes a round.
    pub fn rounds_per_cycle(&self) -> usize {
        let n = self.players.len();
        match n {
            0 | 1 => 0,
            n if n % 2 == 0 => n - 1,
            n => n,
        }
    }

    pub fn total_rounds(&self) -> usize {
        let cycle = self.rounds_per_cycle();
        if self.double_round_robin {
            cycle * 2
        } else {
            cycle
        }
    }

    pub fn is_complete(&self) -> bool {
        self.total_rounds() > 0
            && self.rounds.len() >= self.total_rounds()
            && self.rounds.iter().all(Round::is_complete)
    }

    pub fn pair_next(&mut self) -> Result<()> {
        if self.rounds.len() >= self.total_rounds() {
            return Err(AppError::state("all rounds have already been paired"));
        }
        let round = round_robin::pair_round(self)?;
        self.rounds.push(round);
        Ok(())
    }

    pub fn set_result(&mut self, round: usize, board: usize, outcome: Outcome) -> Result<()> {
        self.game_mut(round, board)?.result = Some(outcome);
        Ok(())
    }

    pub fn clear_result(&mut self, round: usize, board: usize) -> Result<()> {
        self.game_mut(round, board)?.result = None;
        Ok(())
    }

    fn game_mut(&mut self, round: usize, board: usize) -> Result<&mut super::Game> {
        let round_number = round
            .checked_sub(1)
            .ok_or_else(|| AppError::usage("round numbers start at 1"))?;
        let stored = self
            .rounds
            .get_mut(round_number)
            .ok_or_else(|| AppError::usage(format!("round {round} does not exist")))?;
        stored
            .games
            .iter_mut()
            .find(|game| game.board == board)
            .ok_or_else(|| AppError::usage(format!("board {board} does not exist in round {round}")))
    }

    pub fn standings(&self, chain: &[Tiebreak]) -> Vec<StandingRow> {
        standings::rank(self, chain)
    }

    /// Check the invariants a save file must satisfy before it is trusted.
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(AppError::state("tournament name is empty"));
        }

        let mut names: HashSet<String> = HashSet::new();
        for (index, player) in self.players.iter().enumerate() {
            if player.id.0 != index {
                return Err(AppError::state(format!(
                    "player id {} does not match its position {index}",
                    player.id.0
                )));
            }
            if player.name.trim().is_empty() {
                return Err(AppError::state(format!("player {index} has an empty name")));
            }
            if player.rating > MAX_RATING {
                return Err(AppError::state(format!(
                    "player {} has an impossible rating {}",
                    player.name, player.rating
                )));
            }
            if !names.insert(player.name.to_lowercase()) {
                return Err(AppError::state(format!(
                    "duplicate player name: {}",
                    player.name
                )));
            }
        }

        if self.rounds.len() > self.total_rounds() {
            return Err(AppError::state(format!(
                "{} rounds are stored but only {} are possible",
                self.rounds.len(),
                self.total_rounds()
            )));
        }

        for (index, round) in self.rounds.iter().enumerate() {
            let expected = index + 1;
            if round.number != expected {
                return Err(AppError::state(format!(
                    "round number {} found where {expected} was expected",
                    round.number
                )));
            }
            if round.games.is_empty() && round.bye.is_none() {
                return Err(AppError::state(format!(
                    "round {expected} contains neither games nor a bye"
                )));
            }

            let mut boards: HashSet<usize> = HashSet::new();
            let mut seen: HashSet<PlayerId> = HashSet::new();
            for game in &round.games {
                if !boards.insert(game.board) {
                    return Err(AppError::state(format!(
                        "round {expected} has two board {} entries",
                        game.board
                    )));
                }
                if game.white == game.black {
                    return Err(AppError::state(format!(
                        "round {expected} board {} pairs a player with themselves",
                        game.board
                    )));
                }
                for id in [game.white, game.black] {
                    if self.player(id).is_none() {
                        return Err(AppError::state(format!(
                            "round {expected} references unknown player {}",
                            id.0
                        )));
                    }
                    if !seen.insert(id) {
                        return Err(AppError::state(format!(
                            "player {} appears twice in round {expected}",
                            self.player_name(id)
                        )));
                    }
                }
            }

            if let Some(bye) = round.bye {
                if self.player(bye).is_none() {
                    return Err(AppError::state(format!(
                        "round {expected} hands a bye to unknown player {}",
                        bye.0
                    )));
                }
                if !seen.insert(bye) {
                    return Err(AppError::state(format!(
                        "player {} has a bye and a game in round {expected}",
                        self.player_name(bye)
                    )));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Player;

    fn tournament(n: usize) -> Tournament {
        let players = (0..n)
            .map(|i| Player::new(i, format!("P{i}"), 1200))
            .collect();
        Tournament::new("Test".into(), players, false)
    }

    #[test]
    fn total_rounds_matches_field_size() {
        assert_eq!(tournament(2).total_rounds(), 1);
        assert_eq!(tournament(3).total_rounds(), 3);
        assert_eq!(tournament(4).total_rounds(), 3);
        assert_eq!(tournament(5).total_rounds(), 5);
    }

    #[test]
    fn double_round_robin_doubles() {
        let mut t = tournament(4);
        t.double_round_robin = true;
        assert_eq!(t.total_rounds(), 6);
    }

    #[test]
    fn result_can_be_corrected() {
        let mut t = tournament(4);
        t.pair_next().unwrap();
        t.set_result(1, 1, Outcome::WhiteWin).unwrap();
        t.clear_result(1, 1).unwrap();
        assert!(!t.rounds[0].is_complete());

        t.set_result(1, 1, Outcome::BlackWin).unwrap();
        assert!(!t.rounds[0].is_complete(), "other boards are still open");
        t.set_result(1, 2, Outcome::Draw).unwrap();
        assert!(t.rounds[0].is_complete());
    }

    #[test]
    fn rejects_result_for_missing_board() {
        let mut t = tournament(4);
        t.pair_next().unwrap();
        let err = t.set_result(1, 99, Outcome::Draw).unwrap_err();
        assert!(err.to_string().contains("board 99"), "{err}");
    }

    #[test]
    fn validates_player_ids() {
        let mut t = tournament(4);
        t.players[2].id = PlayerId(0);
        assert!(t.validate().is_err());
    }
}
