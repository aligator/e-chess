package me.aligator.e_chess.feature.game

internal fun parseFenBoard(fen: String): List<List<Char?>> {
    val boardPart = fen.trim().split(" ").firstOrNull() ?: return emptyList()
    val rows = boardPart.split("/")
    if (rows.size != 8) return emptyList()

    return rows.map { row ->
        val squares = mutableListOf<Char?>()
        for (ch in row) {
            if (ch.isDigit()) {
                repeat(ch.digitToInt()) { squares.add(null) }
            } else {
                squares.add(ch)
            }
        }
        if (squares.size != 8) return emptyList()
        squares
    }
}
