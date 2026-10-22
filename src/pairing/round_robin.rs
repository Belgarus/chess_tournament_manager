//! Round-robin pairing with the circle method.
//!
//! One slot is fixed and the others rotate, which guarantees that every pair
//! meets exactly once per cycle. An odd field gets a `None` slot instead of a
//! fake player, so a bye never leaks into the player list, the standings, or a
//! name lookup.

use std::collections::HashMap;

use crate::error::{AppError, Result};
use crate::model::{Color, ColorHistory, Game, PlayerId, Round, Tournament};

pub fn pair_round(tournament: &Tournament) -> Result<Round> {
    let number = tournament.rounds.len() + 1;
    if number > tournament.total_rounds() {
        return Err(AppError::state("all rounds have already been paired"));
    }

    let cycle = tournament.rounds_per_cycle();
    if tournament.double_round_robin && number > cycle {
        return mirror_round(tournament, number, cycle);
    }

    let slots = rotated_slots(tournament, number - 1);
    let width = slots.len();
    let mut pairs: Vec<(PlayerId, PlayerId)> = Vec::new();
    let mut bye = None;

    for index in 0..width / 2 {
        match (slots[index], slots[width - 1 - index]) {
            (Some(a), Some(b)) => pairs.push((a, b)),
            (Some(a), None) => bye = Some(a),
            (None, Some(b)) => bye = Some(b),
            (None, None) => {}
        }
    }

    let histories = color_histories(tournament);
    let games = pairs
        .into_iter()
        .enumerate()
        .map(|(index, (a, b))| {
            let (white, black) = choose_colors(tournament, &histories, a, b);
            Game {
                board: index + 1,
                white,
                black,
                result: None,
            }
        })
        .collect();

    Ok(Round {
        number,
        games,
        bye,
    })
}

/// Second cycle of a double round-robin: reuse the first cycle's pairings and
/// swap the colours, which is both correct and perfectly balanced.
fn mirror_round(tournament: &Tournament, number: usize, cycle: usize) -> Result<Round> {
    let source_number = number - cycle;
    let source = tournament
        .rounds
        .get(source_number - 1)
        .ok_or_else(|| AppError::state("cannot mirror a round that has not been paired yet"))?;

    let games = source
        .games
        .iter()
        .enumerate()
        .map(|(index, game)| Game {
            board: index + 1,
            white: game.black,
            black: game.white,
            result: None,
        })
        .collect();

    Ok(Round {
        number,
        games,
        bye: source.bye,
    })
}

fn rotated_slots(tournament: &Tournament, rotations: usize) -> Vec<Option<PlayerId>> {
    let mut slots: Vec<Option<PlayerId>> =
        tournament.players.iter().map(|player| Some(player.id)).collect();
    if slots.len() % 2 == 1 {
        slots.push(None);
    }

    let period = slots.len().saturating_sub(1);
    if period == 0 {
        return slots;
    }
    for _ in 0..(rotations % period) {
        let last = slots.pop().flatten();
        slots.insert(1, last);
    }
    slots
}

fn color_histories(tournament: &Tournament) -> HashMap<PlayerId, ColorHistory> {
    let mut histories: HashMap<PlayerId, ColorHistory> = tournament
        .players
        .iter()
        .map(|player| (player.id, ColorHistory::default()))
        .collect();

    for round in &tournament.rounds {
        for game in &round.games {
            if let Some(history) = histories.get_mut(&game.white) {
                history.record(Color::White);
            }
            if let Some(history) = histories.get_mut(&game.black) {
                history.record(Color::Black);
            }
        }
    }
    histories
}

fn choose_colors(
    tournament: &Tournament,
    histories: &HashMap<PlayerId, ColorHistory>,
    a: PlayerId,
    b: PlayerId,
) -> (PlayerId, PlayerId) {
    let ha = histories.get(&a).copied().unwrap_or_default();
    let hb = histories.get(&b).copied().unwrap_or_default();

    let a_white = badness(&ha, Color::White) + badness(&hb, Color::Black);
    let b_white = badness(&hb, Color::White) + badness(&ha, Color::Black);

    match a_white.cmp(&b_white) {
        std::cmp::Ordering::Less => (a, b),
        std::cmp::Ordering::Greater => (b, a),
        std::cmp::Ordering::Equal => {
            let (ra, rb) = (tournament.rating_of(a), tournament.rating_of(b));
            match ra.cmp(&rb) {
                std::cmp::Ordering::Greater => (a, b),
                std::cmp::Ordering::Less => (b, a),
                std::cmp::Ordering::Equal if a.0 <= b.0 => (a, b),
                std::cmp::Ordering::Equal => (b, a),
            }
        }
    }
}

/// Lower is better. A third identical colour in a row is close to forbidden;
/// otherwise the white/black difference dominates, so players hover around
/// even instead of drifting to a colour they keep getting.
fn badness(history: &ColorHistory, color: Color) -> i32 {
    let (whites, blacks) = match color {
        Color::White => (history.whites + 1, history.blacks),
        Color::Black => (history.whites, history.blacks + 1),
    };
    let difference = whites as i32 - blacks as i32;

    let mut cost = difference * difference * 20;

    if history.last == Some(color) {
        cost += 40;
        if history.streak >= 2 {
            cost += 100_000;
        }
    }

    if difference.abs() > 2 {
        cost += 10_000;
    }

    cost
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Player, Tournament};
    use std::collections::{HashMap, HashSet};

    fn tournament(n: usize, double: bool) -> Tournament {
        let players = (0..n)
            .map(|i| Player::new(i, format!("P{i}"), 1200 + i as u32 * 10))
            .collect();
        Tournament::new("T".into(), players, double)
    }

    fn pair_key(a: PlayerId, b: PlayerId) -> (usize, usize) {
        if a.0 <= b.0 {
            (a.0, b.0)
        } else {
            (b.0, a.0)
        }
    }

    #[test]
    fn every_pair_meets_exactly_once() {
        for n in 2..=9 {
            let mut t = tournament(n, false);
            while t.rounds.len() < t.total_rounds() {
                t.pair_next().unwrap();
            }

            let mut seen: HashSet<(usize, usize)> = HashSet::new();
            for round in &t.rounds {
                for game in &round.games {
                    assert!(
                        seen.insert(pair_key(game.white, game.black)),
                        "n={n}: pair repeated in round {}",
                        round.number
                    );
                }
            }
            assert_eq!(
                seen.len(),
                n * (n - 1) / 2,
                "n={n}: wrong number of distinct pairs"
            );
        }
    }

    #[test]
    fn everyone_plays_at_most_once_per_round() {
        for n in 2..=9 {
            let mut t = tournament(n, false);
            while t.rounds.len() < t.total_rounds() {
                t.pair_next().unwrap();
            }

            for round in &t.rounds {
                let mut players: HashSet<PlayerId> = HashSet::new();
                for game in &round.games {
                    assert!(players.insert(game.white), "n={n}: {:?} twice", game.white);
                    assert!(players.insert(game.black), "n={n}: {:?} twice", game.black);
                }
                if let Some(bye) = round.bye {
                    assert!(players.insert(bye), "n={n}: bye player also played");
                }
                assert_eq!(players.len(), n, "n={n}: round {}", round.number);
            }
        }
    }

    #[test]
    fn odd_field_gives_everyone_exactly_one_bye() {
        for n in (3..=9).step_by(2) {
            let mut t = tournament(n, false);
            while t.rounds.len() < t.total_rounds() {
                t.pair_next().unwrap();
            }
            let mut byes: HashMap<PlayerId, usize> = HashMap::new();
            for round in &t.rounds {
                if let Some(bye) = round.bye {
                    *byes.entry(bye).or_insert(0) += 1;
                }
            }
            assert_eq!(byes.len(), n, "n={n}: not every player had a bye");
            assert!(byes.values().all(|&count| count == 1), "n={n}: {byes:?}");
        }
    }

    #[test]
    fn even_field_has_no_byes() {
        let mut t = tournament(6, false);
        while t.rounds.len() < t.total_rounds() {
            t.pair_next().unwrap();
        }
        assert!(t.rounds.iter().all(|round| round.bye.is_none()));
    }

    #[test]
    fn colors_respect_the_two_rules() {
        for n in 2..=9 {
            let mut t = tournament(n, false);
            while t.rounds.len() < t.total_rounds() {
                t.pair_next().unwrap();
            }

            let mut histories: HashMap<PlayerId, ColorHistory> = t
                .players
                .iter()
                .map(|p| (p.id, ColorHistory::default()))
                .collect();
            for round in &t.rounds {
                for game in &round.games {
                    histories.get_mut(&game.white).unwrap().record(Color::White);
                    histories.get_mut(&game.black).unwrap().record(Color::Black);
                }
            }

            for (id, history) in &histories {
                assert!(
                    history.imbalance().abs() <= 2,
                    "n={n}: {:?} has whites={} blacks={}",
                    id,
                    history.whites,
                    history.blacks
                );
                assert!(
                    history.streak <= 2,
                    "n={n}: {:?} had {} identical colours in a row",
                    id,
                    history.streak
                );
            }
        }
    }

    #[test]
    fn double_cycle_reverses_every_colour() {
        let mut t = tournament(4, true);
        while t.rounds.len() < t.total_rounds() {
            t.pair_next().unwrap();
        }
        let cycle = t.rounds_per_cycle();
        for index in 0..cycle {
            let first = &t.rounds[index];
            let second = &t.rounds[index + cycle];
            for (a, b) in first.games.iter().zip(second.games.iter()) {
                assert_eq!(a.white, b.black);
                assert_eq!(a.black, b.white);
            }
        }
    }

    #[test]
    fn colours_balance_after_a_double_cycle() {
        for n in 3..=8 {
            let mut t = tournament(n, true);
            while t.rounds.len() < t.total_rounds() {
                t.pair_next().unwrap();
            }
            let mut histories: HashMap<PlayerId, ColorHistory> = t
                .players
                .iter()
                .map(|p| (p.id, ColorHistory::default()))
                .collect();
            for round in &t.rounds {
                for game in &round.games {
                    histories.get_mut(&game.white).unwrap().record(Color::White);
                    histories.get_mut(&game.black).unwrap().record(Color::Black);
                }
            }
            for (id, history) in &histories {
                assert!(
                    history.imbalance() == 0,
                    "n={n}: {:?} ended at {} whites / {} blacks",
                    id,
                    history.whites,
                    history.blacks
                );
            }
        }
    }
}
