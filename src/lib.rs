pub mod player;
pub mod schedule;

use player::Player;
use std::io::{self, Write};

pub fn add_players() -> Vec<Player> {
    let mut players = Vec::new();
    
    println!("Add players to the tournament:");
    println!("Format: name[:rating] (rating is optional)");
    println!("Press ENTER with no input to finish.\n");
    
    loop {
        print!("> ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");
        
        let input = input.trim();
        
        // Exit on empty input
        if input.is_empty() {
            break;
        }
        
        // Parse name and optional rating
        let parts: Vec<&str> = input.split(':').collect();
        let name = parts[0].to_string();
        let player = Player::new(name);
        players.push(player);
    }
    
    println!("\nAdded {} players to the tournament.\n", players.len());
    players
}
