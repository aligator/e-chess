package me.aligator.e_chess

import android.content.Context
import android.content.res.Configuration
import android.os.Build
import android.os.Bundle
import android.os.LocaleList
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import me.aligator.e_chess.service.ConfigurationStore
import me.aligator.e_chess.ui.theme.EChessTheme
import java.util.Locale

class MainActivity : ComponentActivity() {
    private lateinit var configStore: ConfigurationStore
    private var currentLanguage by mutableStateOf<AppLanguage?>(null)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        configStore = ConfigurationStore(applicationContext)

        // Load language preference
        val languageCode = configStore.getLanguage()
        val appLanguage = AppLanguage.fromCode(languageCode)
        currentLanguage = appLanguage

        enableEdgeToEdge()
        setContent {
            EChessTheme {
                EChessApp()
            }
        }
    }

    override fun attachBaseContext(newBase: Context) {
        val configStore = ConfigurationStore(newBase)
        val languageCode = configStore.getLanguage()
        val appLanguage = AppLanguage.fromCode(languageCode)

        val context = if (appLanguage != AppLanguage.SYSTEM) {
            updateLocale(newBase, appLanguage.locale)
        } else {
            newBase
        }

        super.attachBaseContext(context)
    }

    private fun updateLocale(context: Context, locale: Locale): Context {
        Locale.setDefault(locale)

        val config = Configuration(context.resources.configuration)

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.N) {
            config.setLocale(locale)
            val localeList = LocaleList(locale)
            LocaleList.setDefault(localeList)
            config.setLocales(localeList)
        } else {
            @Suppress("DEPRECATION")
            config.locale = locale
        }

        return context.createConfigurationContext(config)
    }
}
