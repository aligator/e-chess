# Android App Refactoring - Status

**Start**: 2026-01-11
**Aktueller Stand**: Phase 2 - Settings Layer

## ✅ Phase 1: Foundation (ABGESCHLOSSEN)
- ✅ Koin Dependencies hinzugefügt (v3.5.6)
- ✅ App.kt mit Koin initialization erstellt
- ✅ AppError.kt sealed class hierarchy erstellt
  - BleError (ConnectionFailed, BondingFailed, DeviceNotFound, ScanFailed, etc.)
  - ApiError (NetworkError, AuthError, Timeout, ServerError, etc.)
  - OtaError (UploadFailed, InvalidFile, DeviceNotConnected, etc.)
- ✅ AppModule.kt Koin DI module erstellt (basic structure)
- ✅ AndroidManifest.xml aktualisiert (App class registriert)

## Phase 2: Settings Layer (IN PROGRESS)

### Nächste Schritte
1. SettingsRepository erstellen (~160 LOC)
   - ConfigurationStore wrapper
   - OtaAction integration
   - Error handling mit AppError
   - StateFlows für lichessToken, language, otaState
2. ConfigViewModel refactoren
3. ConfigScreen updaten
4. BluetoothService updaten

### Fortschritt
- [ ] SettingsRepository
- [ ] ConfigViewModel refactored
- [ ] ConfigScreen updated
- [ ] BluetoothService updated

---
*Letzte Aktualisierung: 2026-01-11*
