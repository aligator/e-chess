package me.aligator.e_chess.feature.game

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Scaffold
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import me.aligator.e_chess.R
import me.aligator.e_chess.feature.ble.BleViewModel
import me.aligator.e_chess.feature.ble.components.GameLoader
import me.aligator.e_chess.ui.AppTopBar

@Composable
fun SelectGameScreen(
    viewModel: BleViewModel,
    onBack: () -> Unit,
    onGameLoaded: () -> Unit,
    modifier: Modifier = Modifier
) {
    val uiState by viewModel.uiState.collectAsState()

    Scaffold(
        modifier = modifier.fillMaxSize(),
        topBar = {
            AppTopBar(
                title = stringResource(R.string.select_game_title),
                onBack = onBack
            )
        }
    ) { innerPadding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
        ) {
            GameLoader(
                availableGames = uiState.availableGames,
                selectedGameKey = uiState.selectedGameKey,
                onGameKeyChanged = viewModel::setSelectedGameKey,
                onLoadGame = {
                    viewModel.loadGame(it)
                    onGameLoaded()
                },
                onFetchGames = viewModel::fetchGames,
                isLoadingGames = uiState.isLoadingGames,
                isLoadingGame = uiState.isLoadingGame
            )

        }
    }
}
