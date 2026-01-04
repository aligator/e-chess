package me.aligator.e_chess.ui.components

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
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
import me.aligator.e_chess.service.GameOption

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun GameLoader(
    availableGames: List<GameOption>,
    selectedGameKey: String,
    onGameKeyChanged: (String) -> Unit,
    onLoadGame: (String) -> Unit,
    onFetchGames: () -> Unit,
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
            Text(
                text = stringResource(R.string.load_game_title),
                style = MaterialTheme.typography.titleMedium
            )

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
                            onGameKeyChanged("standard")
                            expanded = false
                        }
                    )

                    // Available Lichess games
                    availableGames.forEach { game ->
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

            Button(
                onClick = { onLoadGame(selectedGameKey.trim()) },
                enabled = selectedGameKey.isNotBlank(),
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(top = 12.dp)
            ) {
                Text(stringResource(R.string.load_game_button))
            }
        }
    }
}
