use crate::player::Player;
use std::io::{self, Write};

// ANSI color constants
const BOLD: &str = "\x1b[1m";
const GREEN: &str = "\x1b[32m";
const ORANGE: &str = "\x1b[38;5;208m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const RESET: &str = "\x1b[0m";
const BYE: &str = "<bye>";

/// Generate a Berger-style round-robin, prompt for results, then show standings.
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
    // Indices for rotation (Circle Method)
    let mut indices: Vec<usize> = (0..n).collect();
    
    // Track opponents by original index
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
            players[b_idx].blacks += 1;
            opponent_indices[w_idx].push(b_idx);
            opponent_indices[b_idx].push(w_idx);

            round_matches.push((w_idx, b_idx));
            board += 1;
        }

        // Collect results for the round
        for (w_idx, b_idx) in round_matches {
            prompt_and_record_result(&mut players, w_idx, b_idx);
        }
        println!();

        // Rotate indices (keep 0 fixed, rotate the rest)
        let last = indices.pop().unwrap();
        indices.insert(1, last);
    }

    // Calculate Buchholz (sum of opponents' final points)
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
    let p1 = &players[i];
    let p2 = &players[j];
    
    // Balance colors: choose the one who has played fewer games as white
    if p1.whites < p2.whites { (i, j) }
    else if p2.whites < p1.whites { (j, i) }
    // Tie-break: choose the one who has played more games as black
    else if p1.blacks > p2.blacks { (i, j) }
    else { (j, i) }
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
    
    // Sort by points, then wins, then Buchholz, then rating
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
            0 => "\x1b[1;33m", // Gold
            1 => "\x1b[1;37m", // Silver
            2 => "\x1b[1;38;5;130m", // Bronze
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
