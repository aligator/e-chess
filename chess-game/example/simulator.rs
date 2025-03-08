// This example requires the lichess-api feature
// From the examples-crate directory, run:
// cargo run --example lichess_game --features lichess-api

use chess_game::game::ChessGame;
use chess_game::request;
use std::io::{self, Write};

#[tokio::main]
async fn main() {
    println!("Chess board simulator - Lichess Game");
    println!("------------------------------------");

    let api_key = std::env::var("LICHESS_API_KEY").unwrap_or_else(|_| {
        let args: Vec<String> = std::env::args().collect();
        if args.len() < 2 {
            eprintln!(
                "Please provide API key as argument or set LICHESS_API_KEY environment variable"
            );
            std::process::exit(1);
        }
        args[1].clone()
    });

    let id = std::env::var("GAME_ID").unwrap_or_else(|_| {
        let args: Vec<String> = std::env::args().collect();
        if args.len() < 3 {
            String::new()
        } else {
            args[2].clone()
        }
    });

    // Use the factory function from the request module
    let mut game = ChessGame::new(request::create_lichess_connector(api_key), &id).unwrap();

    let mut physical_board = game.expected_physical();
    // Start with all set correctly.
    game.physical = physical_board;

    println!("{:?}", game);

    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim().to_lowercase();

        // Parse commands
        if input.starts_with("take ")
            || input.starts_with("put ")
            || input.starts_with("t ")
            || input.starts_with("p ")
        {
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
                    "take" => physical_board.0 &= !mask, // Clear the bit
                    "t" => physical_board.0 &= !mask,    // Clear the bit
                    "put" => physical_board.0 |= mask,   // Set the bit
                    "p" => physical_board.0 |= mask,     // Set the bit
                    _ => unreachable!(),
                }

                // Use try-catch pattern to gracefully handle errors
                match game.tick(physical_board) {
                    Ok(new_board) => {
                        physical_board = new_board;
                        // Print the game state
                        println!("{:?}", game);
                    }
                    Err(e) => {
                        println!("Error updating game state: {:?}", e);
                        println!(
                            "This may be due to an API connection issue or invalid game state."
                        );
                        // Keep the physical board unchanged
                    }
                }
            } else {
                println!("Invalid square notation. Use a1-h8");
            }
        } else if input == "quit" || input == "exit" {
            break;
        } else if input == "r" || input == "refresh" {
            // Refresh command - tick the game with the current physical board state
            println!("Refreshing board...");
            match game.tick(physical_board) {
                Ok(new_board) => {
                    physical_board = new_board;
                    println!("{:?}", game);
                }
                Err(e) => {
                    println!("Error refreshing game state: {:?}", e);
                }
            }
        } else {
            println!(
                "Unknown command. Valid commands are: 'take <square>', 'put <square>', 'refresh'/'r', 'quit'"
            );
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
