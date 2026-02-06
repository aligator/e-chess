package me.aligator.e_chess.ui

import androidx.annotation.StringRes

sealed interface UiMessage {
    data class Res(
        @StringRes val id: Int,
        val args: List<Any> = emptyList()
    ) : UiMessage

    data class Text(val value: String) : UiMessage
}

sealed interface UiEvent {
    data class Snackbar(val message: UiMessage) : UiEvent
}
