# Rust Chess game

This project contains the base handling of a chess game.  
It can be run directly for testing by providing a cli prompt simulating the physical chess board.

As input it works with a single bit board that tells which square is occupied.
At any time this `physical` representation should only differ from the last observed state by one single part.
Otherwise it is not possible to distinguish what should happen in this step.

If the board gets in an invalid state it will stop doing anything.

If it is only one pice (either removed or added), it will be validated:
* based on the current game state (e.g. which player)
* if the last action is not complete yet, check if the new state completes it 
(e.g. moving a piece - 1. take it 2. place it on the new spot)
* is the move a valid chess move

To simulate the board for development there are only two comments to simulate the physical behavior:
* take b5
* put b5

after each action the cli prints out the new board state as ASCII.
The background color encodes the real chess color with black and grey for visualization.

But if a piece is wrong it is color coded this way:
* Blue - physical piece missing
* Red - physical piece should be here but it isn't
