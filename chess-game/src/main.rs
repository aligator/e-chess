use game::ChessGame;
use std::io::{self, Write};
mod game;

fn main() {
    println!("Chess board simulator");

    let mut game = ChessGame::new();
    println!("Initial game state: {:?}", game);

    let mut physical_board = game.physical();
    
    loop {
        print!("> ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim().to_lowercase();

        // Parse commands
        if input.starts_with("take ") || input.starts_with("put ") {
            let parts: Vec<&str> = input.split_whitespace().collect();
            if parts.len() != 2 {
                println!("Invalid command format. Use 'take b5' or 'put b5'");
                continue;
            }

            let square = parts[1];
            // Convert algebraic notation (e.g. "b5") to board position
            if let Some(pos) = parse_square(square) {
                let mask = 1u64 << pos;
                
                match parts[0] {
                    "take" => physical_board &= !mask, // Clear the bit
                    "put" => physical_board |= mask,   // Set the bit
                    _ => unreachable!()
                }

                // Update the game state based on the physical board
                physical_board = game.tick(physical_board);

                // Print the game state
                println!("Game state: {:?}", game);
            } else {
                println!("Invalid square notation. Use a1-h8");
            }
        } else if input == "quit" || input == "exit" {
            break;
        } else {
            println!("Unknown command. Valid commands are: 'take <square>', 'put <square>', 'quit'");
        }
    }
}

/// Converts algebraic notation (e.g. "b5") to a board position (0-63)
fn parse_square(square: &str) -> Option<u32> {
    if square.len() != 2 {
        return None;
    }

    let file = square.chars().next().unwrap();
    let rank = square.chars().nth(1).unwrap();

    if !('a'..='h').contains(&file) || !('1'..='8').contains(&rank) {
        return None;
    }

    let file_idx = (file as u8 - b'a') as u32;
    let rank_idx = (rank as u8 - b'1') as u32;

    Some(rank_idx * 8 + file_idx)
}
