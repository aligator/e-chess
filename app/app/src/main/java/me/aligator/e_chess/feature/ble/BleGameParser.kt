package me.aligator.e_chess.feature.ble

import me.aligator.e_chess.data.model.GameOption
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.contentOrNull

private val jsonParser = Json { ignoreUnknownKeys = true }

internal fun parseOngoingGames(json: String): List<GameOption> {
    val element = jsonParser.parseToJsonElement(json)
    val array = element as? JsonArray ?: return emptyList()
    val games = mutableListOf<GameOption>()

    for (item in array) {
        val game = item as? JsonObject ?: continue
        val gameId = game["game_id"]?.jsonPrimitive?.contentOrNull ?: continue
        val opponent = game["opponent"] as? JsonObject
        val opponentName = opponent?.get("username")?.jsonPrimitive?.contentOrNull ?: "Unknown"

        games.add(
            GameOption(
                id = gameId,
                displayName = "vs $opponentName ($gameId)"
            )
        )
    }

    return games
}
