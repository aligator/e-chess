package me.aligator.e_chess.feature.ble.components

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.MenuAnchorType
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import me.aligator.e_chess.R
import me.aligator.e_chess.data.model.GameOption

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun GameLoader(
    availableGames: List<GameOption>,
    selectedGameKey: String,
    onGameKeyChanged: (String) -> Unit,
    onLoadGame: (String) -> Unit,
    onFetchGames: () -> Unit,
    isLoadingGames: Boolean = false,
    isLoadingGame: Boolean = false,
    modifier: Modifier = Modifier
) {
    var expanded by rememberSaveable { mutableStateOf(false) }

    // Fetch games when component becomes visible
    LaunchedEffect(Unit) {
        onFetchGames()
    }

    Card(
        modifier = modifier
            .fillMaxWidth()
            .padding(16.dp),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.secondaryContainer
        )
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            Row {
                Text(
                    text = stringResource(R.string.load_game_title),
                    style = MaterialTheme.typography.titleMedium,
                    modifier = Modifier.weight(1f)
                )
                if (isLoadingGames) {
                    CircularProgressIndicator(
                        modifier = Modifier.size(20.dp),
                        strokeWidth = 2.dp
                    )
                    Spacer(modifier = Modifier.width(4.dp))
                }
                IconButton(
                    onClick = onFetchGames,
                    enabled = !isLoadingGames,
                    modifier = Modifier.size(32.dp)
                ) {
                    Icon(
                        imageVector = Icons.Default.Refresh,
                        contentDescription = stringResource(R.string.refresh_games),
                        modifier = Modifier.size(20.dp)
                    )
                }
            }

            Spacer(modifier = Modifier.height(12.dp))

            ExposedDropdownMenuBox(
                expanded = expanded,
                onExpandedChange = { expanded = it },
                modifier = Modifier.fillMaxWidth()
            ) {
                OutlinedTextField(
                    value = selectedGameKey,
                    onValueChange = onGameKeyChanged,
                    label = { Text(stringResource(R.string.game_key_label)) },
                    placeholder = { Text(stringResource(R.string.game_key_placeholder)) },
                    singleLine = true,
                    trailingIcon = {
                        ExposedDropdownMenuDefaults.TrailingIcon(expanded = expanded)
                    },
                    modifier = Modifier
                        .menuAnchor(MenuAnchorType.PrimaryNotEditable)
                        .fillMaxWidth()
                )
                ExposedDropdownMenu(
                    expanded = expanded,
                    onDismissRequest = { expanded = false }
                ) {
                    // Standard game option for local play
                    DropdownMenuItem(
                        text = { Text(stringResource(R.string.standard_game_option)) },
                        onClick = {
                            onGameKeyChanged("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
                            expanded = false
                        }
                    )

                    // Available Lichess games
                    if (availableGames.isEmpty() && !isLoadingGames) {
                        DropdownMenuItem(
                            text = { Text(stringResource(R.string.no_games_available)) },
                            onClick = { expanded = false },
                            enabled = false
                        )
                    } else {
                        availableGames.forEach { game ->
                            if (game.id == "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1") {
                                return@forEach
                            }
                            DropdownMenuItem(
                                text = { Text(game.displayName) },
                                onClick = {
                                    onGameKeyChanged(game.id)
                                    expanded = false
                                }
                            )
                        }
                    }
                }
            }

            Button(
                onClick = { onLoadGame(selectedGameKey.trim()) },
                enabled = selectedGameKey.isNotBlank() && !isLoadingGame,
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(top = 12.dp)
            ) {
                if (isLoadingGame) {
                    CircularProgressIndicator(
                        modifier = Modifier.size(20.dp),
                        strokeWidth = 2.dp,
                        color = MaterialTheme.colorScheme.onPrimary
                    )
                    Spacer(modifier = Modifier.width(8.dp))
                }
                Text(stringResource(R.string.load_game_button))
            }
        }
    }
}
