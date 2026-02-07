package me.aligator.e_chess.feature.game

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import me.aligator.e_chess.feature.game.parseFenBoard
import androidx.compose.ui.res.stringResource
import me.aligator.e_chess.R
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.sp
import androidx.compose.ui.unit.Dp
import androidx.compose.foundation.layout.offset

@Composable
fun ChessBoardView(
    fen: String,
    modifier: Modifier = Modifier
) {
    val board = remember(fen) { parseFenBoard(fen) }
    if (board.size != 8) return

    val lightSquare = Color(0xFFF2E3C6)
    val darkSquare = Color(0xFFB07A4A)
    val labelColor = Color(0xFF6B4A2C)
    val whitePiece = Color(0xFFBCA288)
    val whitePieceOutline = Color(0xFF6E5946)
    val blackPiece = Color(0xFF1E1A18)
    val squareSize = 36.dp

    Column(modifier = modifier, horizontalAlignment = Alignment.CenterHorizontally) {
        Text(
            text = stringResource(R.string.board_title),
            style = MaterialTheme.typography.titleMedium,
            modifier = Modifier.padding(bottom = 8.dp)
        )
        Column {
            // Top files
            Row(
                modifier = Modifier.padding(start = squareSize + 6.dp, bottom = 4.dp)
            ) {
                val files = listOf("A", "B", "C", "D", "E", "F", "G", "H")
                files.forEach { file ->
                    Box(
                        modifier = Modifier.size(squareSize),
                        contentAlignment = Alignment.Center
                    ) {
                        Text(
                            text = file,
                            color = labelColor,
                            fontSize = 11.sp,
                            fontWeight = FontWeight.SemiBold,
                            letterSpacing = 0.5.sp
                        )
                    }
                }
            }
            Row {
                // Left ranks
                Column(
                    modifier = Modifier.padding(end = 6.dp)
                ) {
                    board.forEachIndexed { rowIndex, _ ->
                        Box(
                            modifier = Modifier.size(squareSize),
                            contentAlignment = Alignment.Center
                        ) {
                            Text(
                                text = (8 - rowIndex).toString(),
                                color = labelColor,
                                fontSize = 11.sp,
                                fontWeight = FontWeight.SemiBold,
                                letterSpacing = 0.5.sp
                            )
                        }
                    }
                }
                // Board
                Column(
                    modifier = Modifier.clip(RoundedCornerShape(8.dp))
                ) {
                    board.forEachIndexed { rowIndex, row ->
                        Row {
                            row.forEachIndexed { colIndex, piece ->
                                val isLight = (rowIndex + colIndex) % 2 == 0
                                val squareColor = if (isLight) lightSquare else darkSquare
                                BoxSquare(
                                    piece = piece,
                                    background = squareColor,
                                    pieceColor = if (piece?.isUpperCase() == true) whitePiece else blackPiece,
                                    whitePieceOutline = whitePieceOutline,
                                    squareSize = squareSize
                                )
                            }
                        }
                    }
                }
                // Right ranks
                Column(
                    modifier = Modifier.padding(start = 6.dp)
                ) {
                    board.forEachIndexed { rowIndex, _ ->
                        Box(
                            modifier = Modifier.size(squareSize),
                            contentAlignment = Alignment.Center
                        ) {
                            Text(
                                text = (8 - rowIndex).toString(),
                                color = labelColor,
                                fontSize = 11.sp,
                                fontWeight = FontWeight.SemiBold,
                                letterSpacing = 0.5.sp
                            )
                        }
                    }
                }
            }
            // Bottom files
            Row(
                modifier = Modifier.padding(start = squareSize + 6.dp, top = 4.dp)
            ) {
                val files = listOf("A", "B", "C", "D", "E", "F", "G", "H")
                files.forEach { file ->
                    Box(
                        modifier = Modifier.size(squareSize),
                        contentAlignment = Alignment.Center
                    ) {
                        Text(
                            text = file,
                            color = labelColor,
                            fontSize = 11.sp,
                            fontWeight = FontWeight.SemiBold,
                            letterSpacing = 0.5.sp
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun BoxSquare(
    piece: Char?,
    background: Color,
    pieceColor: Color,
    whitePieceOutline: Color,
    squareSize: androidx.compose.ui.unit.Dp,
    modifier: Modifier = Modifier
) {
    val outlineOffset: Dp = 1.0.dp
    androidx.compose.foundation.layout.Box(
        modifier = modifier
            .size(squareSize)
            .background(background),
        contentAlignment = Alignment.Center
    ) {
        if (piece != null) {
            val isWhite = piece.isUpperCase()
            val outlineColor = if (isWhite) whitePieceOutline else Color.Transparent
            if (isWhite) {
                // Simple outline via 4-direction offsets
                Text(
                    text = pieceToUnicode(piece),
                    style = MaterialTheme.typography.bodyMedium,
                    fontFamily = FontFamily.Serif,
                    fontWeight = FontWeight.Medium,
                    fontSize = 22.sp,
                    color = outlineColor,
                    modifier = Modifier.offset(x = outlineOffset, y = 0.dp)
                )
                Text(
                    text = pieceToUnicode(piece),
                    style = MaterialTheme.typography.bodyMedium,
                    fontFamily = FontFamily.Serif,
                    fontWeight = FontWeight.Medium,
                    fontSize = 22.sp,
                    color = outlineColor,
                    modifier = Modifier.offset(x = -outlineOffset, y = 0.dp)
                )
                Text(
                    text = pieceToUnicode(piece),
                    style = MaterialTheme.typography.bodyMedium,
                    fontFamily = FontFamily.Serif,
                    fontWeight = FontWeight.Medium,
                    fontSize = 22.sp,
                    color = outlineColor,
                    modifier = Modifier.offset(x = 0.dp, y = outlineOffset)
                )
                Text(
                    text = pieceToUnicode(piece),
                    style = MaterialTheme.typography.bodyMedium,
                    fontFamily = FontFamily.Serif,
                    fontWeight = FontWeight.Medium,
                    fontSize = 22.sp,
                    color = outlineColor,
                    modifier = Modifier.offset(x = 0.dp, y = -outlineOffset)
                )
                Text(
                    text = pieceToUnicode(piece),
                    style = MaterialTheme.typography.bodyMedium,
                    fontFamily = FontFamily.Serif,
                    fontWeight = FontWeight.Medium,
                    fontSize = 22.sp,
                    color = outlineColor
                )
            }
            Text(
                text = pieceToUnicode(piece),
                style = MaterialTheme.typography.bodyMedium,
                fontFamily = FontFamily.Serif,
                fontWeight = FontWeight.Medium,
                fontSize = 22.sp,
                color = pieceColor
            )
        } else {
            Spacer(modifier = Modifier.size(0.dp))
        }
    }
}

private fun pieceToUnicode(piece: Char): String = when (piece) {
    'K' -> "♚"
    'Q' -> "♛"
    'R' -> "♜"
    'B' -> "♝"
    'N' -> "♞"
    'P' -> "♟"
    'k' -> "♚"
    'q' -> "♛"
    'r' -> "♜"
    'b' -> "♝"
    'n' -> "♞"
    'p' -> "♟"
    else -> piece.toString()
}
