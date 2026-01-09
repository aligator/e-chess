package me.aligator.e_chess.service

import android.content.Context
import android.content.SharedPreferences
import androidx.core.os.LocaleListCompat
import java.util.Locale
import androidx.core.content.edit

private const val PREF_NAME = "app_configuration"
private const val KEY_LICHESS_TOKEN = "lichess_token"
private const val KEY_LAST_GAME = "last_loaded_game"
private const val KEY_LANGUAGE = "language"

/**
 * Central configuration storage for the app.
 * Stores:
 * - Lichess API token
 * - Last loaded game key
 * - User language preference
 */
class ConfigurationStore(private val context: Context) {
    private val prefs: SharedPreferences =
        context.getSharedPreferences(PREF_NAME, Context.MODE_PRIVATE)

    // Lichess Token
    fun getLichessToken(): String? = prefs.getString(KEY_LICHESS_TOKEN, null)?.takeIf { it.isNotBlank() }

    fun saveLichessToken(token: String?) {
        prefs.edit {
            if (token.isNullOrBlank()) {
                remove(KEY_LICHESS_TOKEN)
            } else {
                putString(KEY_LICHESS_TOKEN, token.trim())
            }
        }
    }

    // Last Loaded Game
    fun getLastLoadedGame(): String? = prefs.getString(KEY_LAST_GAME, null)?.takeIf { it.isNotBlank() }

    fun saveLastLoadedGame(gameKey: String?) {
        prefs.edit {
            if (gameKey.isNullOrBlank()) {
                remove(KEY_LAST_GAME)
            } else {
                putString(KEY_LAST_GAME, gameKey.trim())
            }
        }
    }

    // Language
    companion object {
        const val LANGUAGE_SYSTEM_DEFAULT = "system"
    }

    /**
     * Get the user's language preference.
     * Returns null if system default should be used.
     * Returns a language code (e.g., "en", "de") if a specific language is set.
     */
    fun getLanguage(): String? {
        val saved = prefs.getString(KEY_LANGUAGE, LANGUAGE_SYSTEM_DEFAULT)
        return if (saved == LANGUAGE_SYSTEM_DEFAULT) null else saved
    }

    /**
     * Save the user's language preference.
     * Pass null or LANGUAGE_SYSTEM_DEFAULT to use system language.
     */
    fun saveLanguage(languageCode: String?) {
        prefs.edit {
            if (languageCode.isNullOrBlank() || languageCode == LANGUAGE_SYSTEM_DEFAULT) {
                putString(KEY_LANGUAGE, LANGUAGE_SYSTEM_DEFAULT)
            } else {
                putString(KEY_LANGUAGE, languageCode)
            }
        }
    }


}
