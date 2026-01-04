package me.aligator.e_chess.service

import android.content.Context
import android.util.Log
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import okhttp3.OkHttpClient
import okhttp3.Request
import java.util.concurrent.TimeUnit

private const val LOG_TAG = "LichessApi"

@Serializable
data class LichessOpponent(
    val id: String? = null,
    val username: String? = null
)

@Serializable
data class LichessGame(
    val fullId: String,
    val opponent: LichessOpponent? = null
)

@Serializable
data class LichessPlayingResponse(
    val nowPlaying: List<LichessGame> = emptyList()
)

data class GameOption(
    val id: String,
    val displayName: String
)

class LichessApi(context: Context) {
    private val tokenStore = LichessTokenStore(context)
    private val json = Json {
        ignoreUnknownKeys = true
        encodeDefaults = true
    }

    private val httpClient = OkHttpClient.Builder()
        .connectTimeout(10, TimeUnit.SECONDS)
        .readTimeout(10, TimeUnit.SECONDS)
        .writeTimeout(10, TimeUnit.SECONDS)
        .build()

    suspend fun getOngoingGames(): List<GameOption> = withContext(Dispatchers.IO) {
        try {
            val token = tokenStore.getToken()
            if (token == null) {
                Log.d(LOG_TAG, "No token available")
                return@withContext emptyList()
            }

            val request = Request.Builder()
                .url("https://lichess.org/api/account/playing?nb=9")
                .header("Authorization", "Bearer $token")
                .get()
                .build()

            httpClient.newCall(request).execute().use { response ->
                if (!response.isSuccessful) {
                    Log.e(LOG_TAG, "Failed to fetch ongoing games: HTTP ${response.code}")
                    return@withContext emptyList()
                }

                val body = response.body?.string() ?: ""
                val playingResponse = json.decodeFromString<LichessPlayingResponse>(body)

                playingResponse.nowPlaying.map { game ->
                    val opponentName = game.opponent?.username ?: "Unknown"
                    GameOption(
                        id = game.fullId,
                        displayName = "vs $opponentName (${game.fullId})"
                    )
                }
            }
        } catch (e: Exception) {
            Log.e(LOG_TAG, "Failed to fetch ongoing games", e)
            emptyList()
        }
    }
}
