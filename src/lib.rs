pub mod player;
pub mod schedule;

use player::Player;
use std::io::{self, Write};

pub fn add_players() -> Vec<Player> {
    let mut players = Vec::new();
    
    println!("{}Add players to the tournament:{}", "\x1b[1;32m", "\x1b[0m");
    println!("Format: name[:rating] (rating is optional, defaults to 1200)");
    println!("Press ENTER with no input to finish.\n");
    
    loop {
        print!("> ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");
        
        let input = input.trim();
        
        if input.is_empty() {
            break;
        }
        
        let parts: Vec<&str> = input.split(':').collect();
        let name = parts[0].trim().to_string();
        
        if name.is_empty() {
            println!("{}Name cannot be empty.{}", "\x1b[1;31m", "\x1b[0m");
            continue;
        }
        
        let rating = if parts.len() > 1 {
            parts[1].trim().parse::<u32>().unwrap_or(1200)
        } else {
            1200
        };

        players.push(Player::new(name, rating));
    }
    
    println!("\nAdded {} players to the tournament.\n", players.len());
    players
}
