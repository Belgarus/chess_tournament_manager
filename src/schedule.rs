use crate::player::Player;
use std::io::{self, Write};

// ANSI color constants (kept small and readable)
const BOLD: &str = "\x1b[1m";
const GREEN: &str = "\x1b[32m";
const ORANGE: &str = "\x1b[38;5;208m";
const YELLOW: &str = "\x1b[33m";
const RESET: &str = "\x1b[0m";
const BYE: &str = "<bye>";

/// Generate a Berger-style round-robin, prompt for results, then show standings.
///
/// This implementation is compact and avoids unnecessary clones and allocations.
pub fn generate_round_robin(mut players: Vec<Player>) {
    let original_count = players.len();
    let odd = original_count % 2 == 1;

    if odd {
        players.push(Player::new(BYE.to_string()));
    }

    let n = players.len();

    // Track opponents by index; use usize::MAX to mark a bye opponent
    let mut opponents: Vec<Vec<usize>> = vec![Vec::new(); n];

    println!("{}Enter results: 1=white, 0=draw, -1=black{}\n", BOLD, RESET);

    for round in 0..n - 1 {
        println!("{}{}Round {}:{}", BOLD, ORANGE, round + 1, RESET);
        let mut board = 1;
        let mut round_pairs: Vec<(usize, usize)> = Vec::new();

        for i in 0..(n / 2) {
            let a = i;
            let b = n - 1 - i;

            if players[a].name == BYE {
                println!("{}{}{} gets a break", YELLOW, players[b].name, RESET);
                opponents[b].push(usize::MAX);
                continue;
            }
            if players[b].name == BYE {
                println!("{}{}{} gets a break", YELLOW, players[a].name, RESET);
                opponents[a].push(usize::MAX);
                continue;
            }

            let (w, bl) = assign_colors(&players, a, b);
            println!(
                "{}Board {}:{} {}{} (W){} vs {}{} (B){}",
                GREEN,
                board,
                RESET,
                BOLD,
                players[w].name,
                RESET,
                ORANGE,
                players[bl].name,
                RESET
            );

            // record color history
            players[w].whites += 1;
            players[bl].blacks += 1;

            // record opponents for buchholz calculation later
            opponents[w].push(bl);
            opponents[bl].push(w);

            round_pairs.push((w, bl));
            board += 1;
        }

        // Prompt for results for this round's matches
        for &(w, bl) in &round_pairs {
            prompt_and_record_result(&mut players, w, bl);
        }

        // rotate (circle method)
        if round < n - 2 {
            let last = players.pop().unwrap();
            players.insert(1, last);
            // keep opponents vector in sync with players rotation
            let last_ops = opponents.pop().unwrap();
            opponents.insert(1, last_ops);
        }
    }

    // Calculate Buchholz: sum of opponents' final points (bye counts as 1)
    for i in 0..n {
        if players[i].name == BYE { continue; }
        let sum: i32 = opponents[i]
          .iter()
          .map(|&opp| if opp == usize::MAX {1} else { players[opp].points() as i32 })
          .sum();
        players[i].buchholz = sum as f32;
    }

    display_scoreboard(&players, original_count);
}
  
fn assign_colors(players: &[Player], i: usize, j: usize) -> (usize, usize) {
    // Prefer the player with fewer whites to take white; tie-break by fewer blacks
    let p1 = &players[i];
    let p2 = &players[j];
    if p1.whites < p2.whites { (i, j) }
    else if p2.whites < p1.whites { (j, i) }
    else if p1.blacks < p2.blacks { (i, j) }
    else { (j, i) }
}

fn prompt_and_record_result(players: &mut [Player], white: usize, black: usize) {
    print!("Result for {} vs {} (1/0/-1): ", players[white].name, players[black].name);
    io::stdout().flush().ok();

    let mut buf = String::new();
    io::stdin().read_line(&mut buf).ok();
    match buf.trim().parse::<i32>() {
        Ok(1) => {
            players[white].wins += 1;
            players[black].losses += 1;
        }
        Ok(-1) => {
            players[black].wins += 1;
            players[white].losses += 1;
        }
        _ => {
            players[white].draws += 1;
            players[black].draws += 1;
        }
    }
}

fn display_scoreboard(players: &[Player], original_count: usize) {
    let mut list: Vec<&Player> = players.iter().filter(|p| p.name != BYE).collect();
    list.sort_by(|a, b| {
        b.points()
            .partial_cmp(&a.points()).unwrap()
            .then_with(|| b.buchholz.partial_cmp(&a.buchholz).unwrap())
            .then_with(|| b.wins.cmp(&a.wins))
    });

    let games = original_count - 1;
    println!("\n{}{}{}", BOLD, GREEN, "\n=== FINAL STANDINGS ===");
    println!("{}({} games per player - Round Robin)", RESET, games);

    // Column widths are chosen to match the row formatting below
    println!("Rank | {:<20} | {:>6} | {:>7} | {}", "Player", "Points", "Buchholz", "W D L");
    println!("{}", "─".repeat(72));
    for (i, p) in list.iter().enumerate() {
        println!(
            "{:2}. | {:<20} | {:6.1} | {:7.1} | {} {} {}",
            i + 1,
            p.name,
            p.points(),
            p.buchholz,
            p.wins,
            p.draws,
            p.losses
        );
    }
    println!("{}", "─".repeat(72));
}
