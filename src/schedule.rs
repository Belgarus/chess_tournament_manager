use crate::player::{Color, Player};
use std::io::{self, Write};

const BOLD: &str = "\x1b[1m";
const GREEN: &str = "\x1b[32m";
const ORANGE: &str = "\x1b[38;5;208m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const RESET: &str = "\x1b[0m";
const BYE: &str = "<bye>";

pub fn generate_round_robin(mut players: Vec<Player>) {
    let original_count = players.len();
    if players.is_empty() {
        println!("No players added. Exiting.");
        return;
    }

    let odd = original_count % 2 == 1;
    if odd {
        players.push(Player::new(BYE.to_string(), 0));
    }

    let n = players.len();
    let mut indices: Vec<usize> = (0..n).collect();
    
    let mut opponent_indices: Vec<Vec<usize>> = vec![Vec::new(); n];

    println!("{}Enter results: 1 = White Wins, 0 = Draw, -1 = Black Wins{}\n", BOLD, RESET);

    for round in 0..n - 1 {
        println!("{}{}Round {}:{}", BOLD, CYAN, round + 1, RESET);
        println!("{}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━{}", CYAN, RESET);
        
        let mut board = 1;
        let mut round_matches: Vec<(usize, usize)> = Vec::new();

        for i in 0..(n / 2) {
            let p1_idx = indices[i];
            let p2_idx = indices[n - 1 - i];

            if players[p1_idx].name == BYE {
                println!("  {}{:<20}{} gets a break", YELLOW, players[p2_idx].name, RESET);
                continue;
            }
            if players[p2_idx].name == BYE {
                println!("  {}{:<20}{} gets a break", YELLOW, players[p1_idx].name, RESET);
                continue;
            }

            let (w_idx, b_idx) = assign_colors(&players, p1_idx, p2_idx);
            
            let w_padded = format!("{:<20}", players[w_idx].name);
            let b_padded = format!("{:>20}", players[b_idx].name);

            println!(
                "  {}Board {}:{} {}{}{} (W) vs {}{}{} (B)",
                BOLD, board, RESET,
                GREEN, w_padded, RESET,
                ORANGE, b_padded, RESET
            );

            players[w_idx].whites += 1;
            players[w_idx].last_color = Some(Color::White);
            players[b_idx].blacks += 1;
            players[b_idx].last_color = Some(Color::Black);
            opponent_indices[w_idx].push(b_idx);
            opponent_indices[b_idx].push(w_idx);

            round_matches.push((w_idx, b_idx));
            board += 1;
        }

        for (w_idx, b_idx) in round_matches {
            prompt_and_record_result(&mut players, w_idx, b_idx);
        }
        println!();

        let last = indices.pop().unwrap();
        indices.insert(1, last);
    }

    for i in 0..n {
        if players[i].name == BYE { continue; }
        let mut sum = 0.0;
        for &opp_idx in &opponent_indices[i] {
            sum += players[opp_idx].points();
        }
        players[i].buchholz = sum;
    }

    display_scoreboard(&players);
}

fn assign_colors(players: &[Player], i: usize, j: usize) -> (usize, usize) {
    let pref_i = players[i].last_color.map(Color::opposite);
    let pref_j = players[j].last_color.map(Color::opposite);

    match (pref_i, pref_j) {
        (Some(Color::White), Some(Color::Black)) => (i, j),
        (Some(Color::Black), Some(Color::White)) => (j, i),
        (Some(Color::White), None) => (i, j),
        (None, Some(Color::White)) => (j, i),
        (Some(Color::Black), None) => (j, i),
        (None, Some(Color::Black)) => (i, j),
        _ => assign_colors_by_balance(players, i, j),
    }
}

fn assign_colors_by_balance(players: &[Player], i: usize, j: usize) -> (usize, usize) {
    let p1 = &players[i];
    let p2 = &players[j];

    let imbalance1 = p1.whites as i32 - p1.blacks as i32;
    let imbalance2 = p2.whites as i32 - p2.blacks as i32;

    if imbalance1 < imbalance2 {
        (i, j)
    } else if imbalance2 < imbalance1 {
        (j, i)
    } else if p1.whites < p2.whites {
        (i, j)
    } else if p2.whites < p1.whites {
        (j, i)
    } else if p1.rating >= p2.rating {
        (i, j)
    } else {
        (j, i)
    }
}

fn prompt_and_record_result(players: &mut [Player], white: usize, black: usize) {
    loop {
        print!("  Result for {} vs {} (1/0/-1): ", players[white].name, players[black].name);
        io::stdout().flush().ok();

        let mut buf = String::new();
        io::stdin().read_line(&mut buf).ok();
        match buf.trim() {
            "1" => {
                players[white].wins += 1;
                players[black].losses += 1;
                break;
            }
            "0" => {
                players[white].draws += 1;
                players[black].draws += 1;
                break;
            }
            "-1" => {
                players[black].wins += 1;
                players[white].losses += 1;
                break;
            }
            _ => {
                println!("  {}Invalid input. Please enter 1, 0, or -1.{}", YELLOW, RESET);
            }
        }
    }
}

fn display_scoreboard(players: &[Player]) {
    let mut list: Vec<&Player> = players.iter().filter(|p| p.name != BYE).collect();
    
    list.sort_by(|a, b| {
        b.points().partial_cmp(&a.points()).unwrap()
            .then_with(|| b.wins.cmp(&a.wins))
            .then_with(|| b.buchholz.partial_cmp(&a.buchholz).unwrap())
            .then_with(|| b.rating.cmp(&a.rating))
    });

    println!("\n{}{}┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓{}", BOLD, GREEN, RESET);
    println!("{}{}┃                           FINAL STANDINGS                           ┃{}", BOLD, GREEN, RESET);
    println!("{}{}┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛{}", BOLD, GREEN, RESET);

    println!(
        " {}Rank | {:<20} | {:>6} | {:>5} | {:>8} | {:>5}{}",
        BOLD, "Player", "Rating", "Score", "Buchholz", "W-D-L", RESET
    );
    println!(" ─────┼──────────────────────┼────────┼───────┼──────────┼───────");

    for (i, p) in list.iter().enumerate() {
        let color = match i {
            0 => "\x1b[1;33m", 
            1 => "\x1b[1;37m", 
            2 => "\x1b[1;38;5;130m", 
            _ => "",
        };

        println!(
            " {:4} | {}{:<20}{} | {:>6} | {:>5.1} | {:>8.1} | {}-{}-{}",
            i + 1,
            color, p.name, RESET,
            p.rating,
            p.points(),
            p.buchholz,
            p.wins, p.draws, p.losses
        );
    }
    println!(" ─────┴──────────────────────┴────────┴───────┴──────────┴───────\n");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::Color;

    fn player_with_colors(
        name: &str,
        whites: u32,
        blacks: u32,
        last: Option<Color>,
    ) -> Player {
        let mut p = Player::new(name.to_string(), 1200);
        p.whites = whites;
        p.blacks = blacks;
        p.last_color = last;
        p
    }

    #[test]
    fn alternates_white_to_black() {
        let players = vec![
            player_with_colors("Alice", 1, 0, Some(Color::White)),
            player_with_colors("Bob", 0, 1, Some(Color::Black)),
        ];
        assert_eq!(assign_colors(&players, 0, 1), (1, 0));
    }

    #[test]
    fn alternates_black_to_white() {
        let players = vec![
            player_with_colors("Alice", 0, 1, Some(Color::Black)),
            player_with_colors("Bob", 1, 0, Some(Color::White)),
        ];
        assert_eq!(assign_colors(&players, 0, 1), (0, 1));
    }

    #[test]
    fn first_round_uses_balance() {
        let players = vec![
            player_with_colors("Alice", 0, 0, None),
            player_with_colors("Bob", 1, 0, None),
        ];
        assert_eq!(assign_colors(&players, 0, 1), (0, 1));
    }

    #[test]
    fn conflict_prefers_equal_distribution() {
        let players = vec![
            player_with_colors("Alice", 2, 0, Some(Color::White)),
            player_with_colors("Bob", 0, 2, Some(Color::White)),
        ];
        assert_eq!(assign_colors(&players, 0, 1), (1, 0));
    }

    #[test]
    fn honors_alternation_when_opponent_has_no_history() {
        let players = vec![
            player_with_colors("Alice", 1, 0, Some(Color::White)),
            player_with_colors("Bob", 0, 0, None),
        ];
        assert_eq!(assign_colors(&players, 0, 1), (1, 0));
    }
}
