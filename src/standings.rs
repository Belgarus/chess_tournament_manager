//! Derive standings from the stored game history.
//!
//! Nothing here is cached, so correcting a result anywhere updates the table,
//! the tiebreaks and the colour balance together.

use std::cmp::Ordering;
use std::collections::HashMap;

use serde::Serialize;

use crate::model::{Color, PlayerId, Tournament};
use crate::tiebreak::Tiebreak;

/// A bye in a round-robin is worth a full point, matching FIDE rules for
/// round-robin events. With an odd field every player receives exactly one, so
/// it never changes the ranking; it only keeps scores comparable to a normal
/// club event.
pub const BYE_HALF_POINTS: u32 = 2;

#[derive(Debug, Clone, Serialize)]
pub struct StandingRow {
    pub rank: usize,
    pub player: PlayerId,
    pub name: String,
    pub rating: u32,
    pub score: f32,
    pub wins: u32,
    pub draws: u32,
    pub losses: u32,
    pub whites: u32,
    pub blacks: u32,
    pub byes: u32,
    pub buchholz: f32,
    pub buchholz_cut1: f32,
    pub sonneborn_berger: f32,
    pub direct_encounter: f32,
}

struct Played {
    opponent: PlayerId,
    /// `Some(half points)` once a result exists.
    halfs: Option<u32>,
}

fn halves(score: f32) -> u32 {
    (score * 2.0).round() as u32
}

fn descending(a: f32, b: f32) -> Ordering {
    b.partial_cmp(&a).unwrap_or(Ordering::Equal)
}

pub fn rank(tournament: &Tournament, chain: &[Tiebreak]) -> Vec<StandingRow> {
    // Pass 1: final scores, because Buchholz and Sonneborn-Berger need the
    // opponents' scores before they can be computed.
    let mut totals: HashMap<PlayerId, u32> = tournament
        .players
        .iter()
        .map(|player| (player.id, 0))
        .collect();

    for round in &tournament.rounds {
        for game in &round.games {
            if let Some(outcome) = game.result {
                *totals.entry(game.white).or_insert(0) += halves(outcome.score_for(Color::White));
                *totals.entry(game.black).or_insert(0) += halves(outcome.score_for(Color::Black));
            }
        }
        if let Some(bye) = round.bye {
            *totals.entry(bye).or_insert(0) += BYE_HALF_POINTS;
        }
    }

    // Pass 2: per-player derived values.
    let mut rows = Vec::with_capacity(tournament.players.len());
    for player in &tournament.players {
        let mut played: Vec<Played> = Vec::new();
        let mut byes = 0;
        let mut whites = 0;
        let mut blacks = 0;

        for round in &tournament.rounds {
            if round.bye == Some(player.id) {
                byes += 1;
            }
            for game in &round.games {
                if let Some((color, opponent)) = game.player(player.id) {
                    match color {
                        Color::White => whites += 1,
                        Color::Black => blacks += 1,
                    }
                    played.push(Played {
                        opponent,
                        halfs: game
                            .result
                            .map(|outcome| halves(outcome.score_for(color))),
                    });
                }
            }
        }

        let mut contribs: Vec<f32> = Vec::new();
        let mut sonneborn_berger = 0.0;
        for game in &played {
            let Some(halfs) = game.halfs else { continue };
            let opponent_score = totals.get(&game.opponent).copied().unwrap_or(0) as f32 / 2.0;
            contribs.push(opponent_score);
            sonneborn_berger += match halfs {
                2 => opponent_score,
                1 => opponent_score * 0.5,
                _ => 0.0,
            };
        }

        // A bye is scored against a virtual opponent with the player's own
        // score, which keeps the number of Buchholz contributions equal.
        let own_score = totals.get(&player.id).copied().unwrap_or(0) as f32 / 2.0;
        for _ in 0..byes {
            contribs.push(own_score);
        }

        let buchholz: f32 = contribs.iter().sum();
        let buchholz_cut1 = if contribs.len() > 1 {
            buchholz - contribs.iter().copied().fold(f32::INFINITY, f32::min)
        } else {
            0.0
        };

        rows.push(StandingRow {
            rank: 0,
            player: player.id,
            name: player.name.clone(),
            rating: player.rating,
            score: totals.get(&player.id).copied().unwrap_or(0) as f32 / 2.0,
            wins: played
                .iter()
                .filter(|game| game.halfs == Some(2))
                .count() as u32,
            draws: played
                .iter()
                .filter(|game| game.halfs == Some(1))
                .count() as u32,
            losses: played
                .iter()
                .filter(|game| game.halfs == Some(0))
                .count() as u32,
            whites,
            blacks,
            byes: byes as u32,
            buchholz,
            buchholz_cut1,
            sonneborn_berger,
            direct_encounter: 0.0,
        });
    }

    // Direct encounter only counts games between players who end up on the
    // same score, so it has to be computed after the totals are known.
    if chain.contains(&Tiebreak::DirectEncounter) {
        let mut direct: HashMap<PlayerId, f32> = HashMap::new();
        for round in &tournament.rounds {
            for game in &round.games {
                let Some(outcome) = game.result else { continue };
                if totals.get(&game.white) != totals.get(&game.black) {
                    continue;
                }
                *direct.entry(game.white).or_insert(0.0) += outcome.score_for(Color::White);
                *direct.entry(game.black).or_insert(0.0) += outcome.score_for(Color::Black);
            }
        }
        for row in &mut rows {
            row.direct_encounter = direct.get(&row.player).copied().unwrap_or(0.0);
        }
    }

    rows.sort_by(|a, b| {
        let mut ordering = descending(a.score, b.score);
        for criterion in chain {
            if ordering != Ordering::Equal {
                break;
            }
            ordering = match criterion {
                Tiebreak::Buchholz => descending(a.buchholz, b.buchholz),
                Tiebreak::BuchholzCut1 => descending(a.buchholz_cut1, b.buchholz_cut1),
                Tiebreak::SonnebornBerger => descending(a.sonneborn_berger, b.sonneborn_berger),
                Tiebreak::DirectEncounter => descending(a.direct_encounter, b.direct_encounter),
                Tiebreak::Wins => b.wins.cmp(&a.wins),
            };
        }
        if ordering == Ordering::Equal {
            ordering = b.rating.cmp(&a.rating);
        }
        if ordering == Ordering::Equal {
            ordering = a.name.cmp(&b.name);
        }
        ordering
    });

    for (index, row) in rows.iter_mut().enumerate() {
        row.rank = index + 1;
    }

    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Outcome, Player, Tournament};

    fn tournament(n: usize) -> Tournament {
        let players = (0..n)
            .map(|i| Player::new(i, format!("P{i}"), 1200))
            .collect();
        Tournament::new("T".into(), players, false)
    }

    fn play_all(t: &mut Tournament, outcome: Outcome) {
        while t.rounds.len() < t.total_rounds() {
            t.pair_next().unwrap();
            let number = t.rounds.len();
            let boards: Vec<usize> = t.rounds[number - 1]
                .games
                .iter()
                .map(|game| game.board)
                .collect();
            for board in boards {
                t.set_result(number, board, outcome).unwrap();
            }
        }
    }

    #[test]
    fn winner_of_a_single_game_ranks_first() {
        let mut t = tournament(2);
        t.pair_next().unwrap();
        t.set_result(1, 1, Outcome::WhiteWin).unwrap();

        let chain = Tiebreak::default_chain();
        let rows = rank(&t, &chain);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].score, 1.0);
        assert_eq!(rows[1].score, 0.0);
        assert_eq!(rows[0].rank, 1);
        assert_eq!(rows[1].rank, 2);
    }

    #[test]
    fn bye_is_worth_one_point_in_every_round_robin() {
        let mut t = tournament(3);
        while t.rounds.len() < t.total_rounds() {
            t.pair_next().unwrap();
        }

        let rows = rank(&t, &Tiebreak::default_chain());
        assert_eq!(rows.len(), 3);
        for row in rows {
            assert_eq!(row.score, 1.0, "{} should have one bye point", row.name);
        }
    }

    #[test]
    fn draws_are_half_points() {
        let mut t = tournament(2);
        t.pair_next().unwrap();
        t.set_result(1, 1, Outcome::Draw).unwrap();
        let rows = rank(&t, &Tiebreak::default_chain());
        assert!(rows.iter().all(|row| row.score == 0.5));
    }

    #[test]
    fn buchholz_is_the_sum_of_opponents_scores() {
        let mut t = tournament(4);
        play_all(&mut t, Outcome::WhiteWin);
        let rows = rank(&t, &Tiebreak::default_chain());

        for row in &rows {
            let mut expected = 0.0;
            for round in &t.rounds {
                for game in &round.games {
                    if game.result.is_none() {
                        continue;
                    }
                    let opponent = if game.white == row.player {
                        game.black
                    } else if game.black == row.player {
                        game.white
                    } else {
                        continue;
                    };
                    let opponent_row = rows
                        .iter()
                        .find(|candidate| candidate.player == opponent)
                        .expect("every player has a row");
                    expected += opponent_row.score;
                }
            }
            assert_eq!(row.buchholz, expected, "{}", row.name);
        }
    }

    #[test]
    fn colour_counts_are_derived() {
        let mut t = tournament(4);
        play_all(&mut t, Outcome::Draw);
        for row in rank(&t, &Tiebreak::default_chain()) {
            assert_eq!(row.whites + row.blacks, 3);
        }
    }
}
