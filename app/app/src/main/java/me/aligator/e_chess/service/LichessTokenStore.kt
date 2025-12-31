package me.aligator.e_chess.service

import android.content.Context
import android.content.SharedPreferences

private const val PREF_NAME = "lichess_prefs"
private const val KEY_TOKEN = "lichess_token"

class LichessTokenStore(context: Context) {
    private val prefs: SharedPreferences =
        context.getSharedPreferences(PREF_NAME, Context.MODE_PRIVATE)

    fun getToken(): String? = prefs.getString(KEY_TOKEN, null)?.takeIf { it.isNotBlank() }

    fun saveToken(token: String?) {
        prefs.edit().apply {
            if (token.isNullOrBlank()) {
                remove(KEY_TOKEN)
            } else {
                putString(KEY_TOKEN, token.trim())
            }
        }.apply()
    }
}
