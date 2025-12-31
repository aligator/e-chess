package me.aligator.e_chess

import java.util.Locale

enum class AppLanguage(val locale: Locale, val flag: String) {
    DE(Locale.GERMAN, "🇩🇪"),
    EN(Locale.ENGLISH, "🇬🇧"),
    NO(Locale("no"), "🇳🇴");
}
