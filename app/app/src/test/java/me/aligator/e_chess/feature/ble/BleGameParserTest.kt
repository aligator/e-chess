package me.aligator.e_chess.feature.ble

import me.aligator.e_chess.data.model.GameOption
import org.junit.Assert.assertEquals
import org.junit.Test

class BleGameParserTest {
    @Test
    fun parsesOngoingGames() {
        val json = """
            [
              {"game_id": "abc123", "opponent": {"username": "Magnus"}},
              {"game_id": "def456", "opponent": {"username": "Hikaru"}},
              {"game_id": "zzz999"}
            ]
        """.trimIndent()

        val games = parseOngoingGames(json)

        assertEquals(
            listOf(
                GameOption("abc123", "vs Magnus (abc123)"),
                GameOption("def456", "vs Hikaru (def456)"),
                GameOption("zzz999", "vs Unknown (zzz999)")
            ),
            games
        )
    }
}
