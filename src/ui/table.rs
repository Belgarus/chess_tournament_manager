use crate::model::{Round, Tournament};
use crate::standings::StandingRow;

use super::Painter;

const NAME_WIDTH: usize = 20;

/// Truncate to `width` characters and pad with spaces. Rust pads by characters,
/// so a long name can no longer push the following columns out of line.
fn fit(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        format!("{text:<width$}")
    } else {
        let mut shortened: String = text.chars().take(width.saturating_sub(1)).collect();
        shortened.push('…');
        shortened
    }
}

pub fn round_view(tournament: &Tournament, round: &Round, painter: &Painter) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{}\n",
        painter.bold(format!("Round {}", round.number))
    ));

    if round.games.is_empty() {
        out.push_str("  no games this round\n");
    }

    for game in &round.games {
        let white = fit(tournament.player_name(game.white), NAME_WIDTH);
        let black = fit(tournament.player_name(game.black), NAME_WIDTH);
        let result = match game.result {
            Some(outcome) => painter.cyan(outcome.pgn()),
            None => painter.dim("pending"),
        };
        out.push_str(&format!(
            " Board {:>2}: {} (W) vs {} (B)  {}\n",
            game.board,
            painter.green(white),
            painter.orange(black),
            result
        ));
    }

    if let Some(bye) = round.bye {
        out.push_str(&format!(
            " Bye:      {} gains 1 point (no game)\n",
            painter.yellow(fit(tournament.player_name(bye), NAME_WIDTH))
        ));
    }

    out
}

pub fn standings_table(tournament: &Tournament, rows: &[StandingRow], painter: &Painter) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{}\n",
        painter.bold(format!(
            "{} — {} players, {} rounds",
            tournament.name,
            tournament.players.len(),
            tournament.total_rounds()
        ))
    ));

    let header = format!(
        " {:>4} | {:<20} | {:>6} | {:>5} | {:>8} | {:>6} | {:<7} | {:>3}",
        "Rank", "Player", "Rating", "Score", "Buchholz", "SB", "W-D-L", "Bye"
    );
    out.push_str(&header);
    out.push('\n');
    out.push_str(
        &header
            .chars()
            .map(|character| if character == '|' { '+' } else { '-' })
            .collect::<String>(),
    );
    out.push('\n');

    for row in rows {
        let rank = match row.rank {
            1 => painter.bold(painter.yellow(format!("{:>4}", row.rank))),
            2 => painter.bold(format!("{:>4}", row.rank)),
            3 => painter.orange(format!("{:>4}", row.rank)),
            _ => format!("{:>4}", row.rank),
        };
        out.push_str(&format!(
            " {} | {} | {:>6} | {:>5.1} | {:>8.1} | {:>6.2} | {:<7} | {:>3}\n",
            rank,
            fit(&row.name, NAME_WIDTH),
            row.rating,
            row.score,
            row.buchholz,
            row.sonneborn_berger,
            format!("{}-{}-{}", row.wins, row.draws, row.losses),
            row.byes
        ));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Player, Tournament};

    #[test]
    fn fit_keeps_the_column_width() {
        assert_eq!(fit("Alice", 10).chars().count(), 10);
        assert_eq!(fit("A very long name indeed", 10).chars().count(), 10);
        assert!(fit("A very long name indeed", 10).ends_with('…'));
    }

    #[test]
    fn table_rows_have_equal_width_without_colour() {
        let players = vec![
            Player::new(0, "Alice", 1700),
            Player::new(1, "Bob", 1500),
        ];
        let mut tournament = Tournament::new("T".into(), players, false);
        tournament.pair_next().unwrap();
        let rows = tournament.standings(&crate::tiebreak::Tiebreak::default_chain());
        let table = standings_table(&tournament, &rows, &Painter::plain());
        let widths: Vec<usize> = table
            .lines()
            .filter(|line| line.contains('|'))
            .map(|line| line.chars().count())
            .collect();
        assert!(widths.windows(2).all(|pair| pair[0] == pair[1]), "{widths:?}");
    }
}
