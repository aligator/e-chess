package me.aligator.e_chess.model

sealed class AppError {
    abstract val message: String

    // BLE Errors
    sealed class BleError : AppError() {
        data class ConnectionFailed(override val message: String = "Connection failed") : BleError()
        data class BondingFailed(override val message: String = "Bonding failed") : BleError()
        data class DeviceNotFound(override val message: String = "Device not found") : BleError()
        data class ScanFailed(override val message: String = "Scan failed") : BleError()
        data class DisconnectFailed(override val message: String = "Disconnect failed") : BleError()
        data class Unknown(override val message: String = "Unknown BLE error") : BleError()
    }

    // API Errors
    sealed class ApiError : AppError() {
        data class NetworkError(override val message: String = "Network error") : ApiError()
        data class AuthError(override val message: String = "Authentication error") : ApiError()
        data class Timeout(override val message: String = "Request timeout") : ApiError()
        data class ServerError(override val message: String = "Server error") : ApiError()
        data class ParseError(override val message: String = "Failed to parse response") : ApiError()
        data class Unknown(override val message: String = "Unknown API error") : ApiError()
    }

    // OTA Errors
    sealed class OtaError : AppError() {
        data class UploadFailed(override val message: String = "Upload failed") : OtaError()
        data class InvalidFile(override val message: String = "Invalid file") : OtaError()
        data class DeviceNotConnected(override val message: String = "Device not connected") : OtaError()
        data class TransferError(override val message: String = "Transfer error") : OtaError()
        data class VerificationFailed(override val message: String = "Verification failed") : OtaError()
        data class Unknown(override val message: String = "Unknown OTA error") : OtaError()
    }

    // General Errors
    data class Generic(override val message: String) : AppError()
}
