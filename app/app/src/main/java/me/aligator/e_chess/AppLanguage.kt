package me.aligator.e_chess

import java.util.Locale

enum class AppLanguage(val code: String, val locale: Locale, val flag: String) {
    SYSTEM("system", Locale.getDefault(), "🌐"),
    DE("de", Locale.GERMAN, "🇩🇪"),
    EN("en", Locale.ENGLISH, "🇬🇧"),
    NO("nb", Locale.forLanguageTag("nb"), "🇳🇴");

    companion object {
        fun fromCode(code: String?): AppLanguage {
            return when (code) {
                null, "system" -> SYSTEM
                "de" -> DE
                "en" -> EN
                "nb", "no" -> NO
                else -> SYSTEM
            }
        }

        fun fromLocale(locale: Locale): AppLanguage {
            return when (locale.language) {
                "de" -> DE
                "en" -> EN
                "nb", "no" -> NO
                else -> EN
            }
        }
    }
}
