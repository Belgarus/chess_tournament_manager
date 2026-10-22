use crate::model::Tournament;
use crate::standings::StandingRow;

pub fn pgn(tournament: &Tournament) -> String {
    let mut out = String::new();

    for round in &tournament.rounds {
        for game in &round.games {
            let result = game.result.map(|outcome| outcome.pgn()).unwrap_or("*");
            out.push_str(&format!("[Event \"{}\"]\n", escape(&tournament.name)));
            out.push_str("[Site \"?\"]\n");
            out.push_str("[Date \"????.??.??\"]\n");
            out.push_str(&format!("[Round \"{}\"]\n", round.number));
            out.push_str(&format!("[Board \"{}\"]\n", game.board));
            out.push_str(&format!(
                "[White \"{}\"]\n",
                escape(tournament.player_name(game.white))
            ));
            out.push_str(&format!(
                "[Black \"{}\"]\n",
                escape(tournament.player_name(game.black))
            ));
            out.push_str(&format!("[Result \"{result}\"]\n\n{result}\n\n"));
        }
    }

    out
}

pub fn standings_csv(rows: &[StandingRow]) -> String {
    let mut out = String::from(
        "rank,player,rating,score,buchholz,buchholz_cut1,sonneborn_berger,direct_encounter,wins,draws,losses,whites,blacks\n",
    );

    for row in rows {
        out.push_str(&format!(
            "{},{},{},{:.1},{:.1},{:.1},{:.1},{:.1},{},{},{},{},{}\n",
            row.rank,
            csv_field(&row.name),
            row.rating,
            row.score,
            row.buchholz,
            row.buchholz_cut1,
            row.sonneborn_berger,
            row.direct_encounter,
            row.wins,
            row.draws,
            row.losses,
            row.whites,
            row.blacks
        ));
    }

    out
}

fn escape(value: &str) -> String {
    value.replace('"', "'").replace(['\n', '\r'], " ")
}

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Outcome, Player};
    use crate::tiebreak::Tiebreak;

    fn played_tournament() -> Tournament {
        let players = vec![
            Player::new(0, "Alice", 1700),
            Player::new(1, "Bob", 1500),
        ];
        let mut tournament = Tournament::new("Club \"Cup\"".into(), players, false);
        tournament.pair_next().unwrap();
        tournament.set_result(1, 1, Outcome::WhiteWin).unwrap();
        tournament
    }

    #[test]
    fn pgn_contains_headers_and_result() {
        let pgn = pgn(&played_tournament());
        assert!(pgn.contains("[White \"Alice\"]"), "{pgn}");
        assert!(pgn.contains("[Black \"Bob\"]"), "{pgn}");
        assert!(pgn.contains("[Result \"1-0\"]"), "{pgn}");
        assert!(pgn.contains("[Event \"Club 'Cup'\"]"), "{pgn}");
    }

    #[test]
    fn csv_has_one_row_per_player_plus_header() {
        let tournament = played_tournament();
        let rows = tournament.standings(&Tiebreak::default_chain());
        let csv = standings_csv(&rows);
        assert_eq!(csv.lines().count(), rows.len() + 1);
        assert!(csv.starts_with("rank,player,rating"), "{csv}");
    }
}
