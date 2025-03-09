let colored_symbol = if let Some(last_move) = last_move {
    let last_move_square = last_move.get_dest();
    if square == last_move_square {
        format!(" {} ", symbol).on_green()
    } else {
        colored_symbol
    }
} else {
    colored_symbol
}; 