package me.aligator.e_chess.di

import me.aligator.e_chess.repository.BleRepository
import me.aligator.e_chess.repository.GamesRepository
import me.aligator.e_chess.repository.SettingsRepository
import me.aligator.e_chess.service.ConfigurationStore
import me.aligator.e_chess.service.LichessApi
import me.aligator.e_chess.ui.BleViewModel
import me.aligator.e_chess.ui.ConfigViewModel
import org.koin.android.ext.koin.androidContext
import org.koin.androidx.viewmodel.dsl.viewModel
import org.koin.dsl.module

val appModule = module {
    // Services
    single { ConfigurationStore(androidContext()) }
    single { LichessApi() }

    // Repositories (Service-scoped)
    single { BleRepository() }
    single { GamesRepository(get()) }
    single { SettingsRepository(get()) }

    // ViewModels (Activity-scoped)
    viewModel { BleViewModel(get(), get()) }
    viewModel { ConfigViewModel(get()) }
}
