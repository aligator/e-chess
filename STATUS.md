# Android App Refactoring - Status

**Start**: 2026-01-11
**Aktueller Stand**: Phase 4 - BLE Layer

## ✅ Phase 1: Foundation (ABGESCHLOSSEN)
- ✅ Koin Dependencies hinzugefügt (v3.5.6)
- ✅ App.kt mit Koin initialization erstellt
- ✅ AppError.kt sealed class hierarchy erstellt
  - BleError (ConnectionFailed, BondingFailed, DeviceNotFound, ScanFailed, etc.)
  - ApiError (NetworkError, AuthError, Timeout, ServerError, etc.)
  - OtaError (UploadFailed, InvalidFile, DeviceNotConnected, etc.)
- ✅ AppModule.kt Koin DI module erstellt (basic structure)
- ✅ AndroidManifest.xml aktualisiert (App class registriert)

## ✅ Phase 2: Settings Layer (ABGESCHLOSSEN)
- ✅ SettingsRepository erstellt (~120 LOC)
  - ConfigurationStore wrapper
  - OtaAction integration via setOtaAction()
  - Error handling mit AppError
  - StateFlows für lichessToken, language, otaState, error
- ✅ ConfigViewModel refactored (~90 LOC)
  - Pure delegation zu SettingsRepository
  - Removed direct OtaAction dependency
  - Added saveLichessToken(), saveLanguage(), clearError()
- ✅ ConfigScreen updated
  - koinViewModel() injection
  - Removed direct otaAction und bleService parameters
  - Error Snackbar hinzugefügt
  - Language selector uses ViewModel
- ✅ BluetoothService updated
  - Koin injection für SettingsRepository
  - setOtaAction() called in onCreate()

## ✅ Phase 3: Games Layer (ABGESCHLOSSEN)
- ✅ GamesRepository erstellt (~160 LOC)
  - LichessApi wrapper
  - Game loading logic with loadAvailableGames()
  - ChessBoardDeviceAction integration via setChessBoardAction()
  - StateFlows für availableGames, isLoadingGames, isLoadingGame, selectedGameKey, error
  - Methods: loadAvailableGames(), selectGame(), loadOpenGamesOnDevice(), clearError(), reset()
- ✅ BluetoothService updated
  - Koin injection für GamesRepository
  - setChessBoardAction() called in onCreate()
- ✅ AppModule.kt updated
  - LichessApi(androidContext()) fixed (requires Context parameter)
  - GamesRepository registered in Koin DI

## ✅ Phase 4: BLE Layer (ABGESCHLOSSEN)
- ✅ BleRepository erstellt (~190 LOC)
  - Ble service wrapper
  - BLE state propagation via StateFlow collection
  - Device scanning: startScan(), stopScan()
  - Connection management: connect(), disconnect(), checkBluetooth()
  - PIN/Bonding handling: submitPin(), cancelPinDialog() with onPinRequested callback
  - StateFlows für bleState, showPinDialog, error
  - Methods: setBle(), startScan(), stopScan(), connect(), disconnect(), checkBluetooth(), submitPin(), cancelPinDialog(), clearError(), reset()
- ✅ BluetoothService updated
  - BleRepository via Koin injected
  - setBle() bei onCreate aufgerufen
  - Imports cleaned up (removed fully qualified names)

## Phase 5: ViewModels (TODO)

### Nächste Schritte
1. BleViewModel erstellen
   - BleRepository delegation
   - StateFlows propagation
2. GamesViewModel erstellen
   - GamesRepository delegation
   - Game selection logic
3. Existing ViewModels updaten falls nötig

---
*Letzte Aktualisierung: 2026-01-11*
