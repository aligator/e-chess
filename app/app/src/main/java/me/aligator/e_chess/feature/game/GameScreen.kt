package me.aligator.e_chess.feature.game

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import me.aligator.e_chess.R
import me.aligator.e_chess.feature.ble.BleViewModel
import me.aligator.e_chess.ui.AppTopBar

@Composable
fun GameScreen(
    viewModel: BleViewModel,
    onBack: () -> Unit,
    modifier: Modifier = Modifier
) {
    val uiState by viewModel.uiState.collectAsState()

    Scaffold(
        modifier = modifier.fillMaxSize(),
        topBar = {
            AppTopBar(
                title = stringResource(R.string.game_title),
                onBack = onBack
            )
        }
    ) { innerPadding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
        ) {
            val fen = uiState.boardFen
            if (fen != null) {
                ChessBoardView(
                    fen = fen,
                    modifier = Modifier
                        .padding(horizontal = 16.dp)
                        .fillMaxWidth()
                )
            } else {
                Text(
                    text = stringResource(R.string.no_board_state),
                    modifier = Modifier.padding(16.dp)
                )
            }
        }
    }
}
