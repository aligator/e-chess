# Android App Refactoring Plan - MVVM + Repository Pattern

Dokumentiere den aktuellen implementierungsstatus immer in STATUS.md

Mache dazwischen immer ein git commit

## Ziel
Refactoring der e-chess Android App mit sauberem State Management ohne Over-Engineering.
Die App ist klein (~3.600 LOC) und soll clean und robust bleiben.

## Architektur-Entscheidungen

### Pattern: MVVM + Repository
- **ViewModels**: UI-Logik und State-Aggregation
- **Repositories**: Single Source of Truth, Business Logic
- **Dependency Injection**: Koin (lightweight, keine Use Cases mit nur einer Action)
- **State Management**: Kotlin Flow & StateFlow (reaktiv)
- **Error Handling**: Sealed Classes (strukturiert)

### Schichten
```
UI Layer (Composables)
    ↓
ViewModel Layer (UI State aggregation)
    ↓
Repository Layer (Business Logic, Single Source of Truth)
    ↓
Service/Data Layer (BLE, API, Storage)
```

## Implementation Plan

### ✅ Phase 1: Foundation
- [ ] Koin Dependencies hinzugefügt (v3.5.6)
- [ ] App.kt mit Koin initialization erstellt
- [ ] AppError.kt sealed class hierarchy erstellt
  - BleError (ConnectionFailed, BondingFailed, DeviceNotFound, etc.)
  - ApiError (NetworkError, AuthError, etc.)
  - OtaError (UploadFailed, InvalidFile, etc.)
- [ ] AppModule.kt Koin DI module erstellt

### ✅ Phase 2: Settings Layer
- [ ] SettingsRepository erstellt (~160 LOC)
  - ConfigurationStore wrapper
  - OtaAction integration
  - Error handling mit AppError
  - StateFlows für lichessToken, language, otaState
- [ ] ConfigViewModel refactored (75→68 LOC)
  - Delegation zu SettingsRepository
  - Removed WeakReference pattern
- [ ] ConfigScreen updated
  - koinViewModel() injection
  - Error Snackbar hinzugefügt
- [ ] BluetoothService updated
  - Koin injection für SettingsRepository
  - setOtaAction() bei onCreate

### ✅ Phase 3: Games Layer
- [ ] GamesRepository erstellt (~200 LOC)
  - LichessApi wrapper
  - Game loading logic
  - ChessBoardDeviceAction integration
  - StateFlows für availableGames, isLoadingGames, isLoadingGame, selectedGameKey

### ✅ Phase 4: BLE Layer
- [ ] BleRepository erstellt (~200 LOC)
  - Ble service wrapper
  - BLE state propagation (fixed: proper StateFlow collection)
  - Device scanning, connection, disconnection
  - PIN/Bonding handling
  - StateFlows für bleState, showPinDialog, error
- [ ] BluetoothService updated
  - Alle 3 Repositories via Koin injected
  - setBle(), setChessBoardAction(), setOtaAction() aufgerufen

### ✅ Phase 5: ViewModel Refactoring
- [ ] BleViewModel refactored (183→137 LOC, -25%)
  - Removed WeakReference pattern
  - Removed AndroidViewModel → regular ViewModel
  - Combined 8 StateFlows via nested combine() (Kotlin Flow limit: 5)
  - Pure delegation zu BleRepository und GamesRepository
  - PIN callback handling
- [ ] BleScreen updated
  - koinViewModel() injection
  - Removed setBluetoothService() call
  - Removed onGameKeyChanged callback (managed by repo)
  - Changed fetchGames → loadAvailableGames

### ✅ Phase 6: Bug Fixes & State Propagation
- [ ] **BleRepository.bleState Fix**
  - Problem: Getter returned empty BleState() when ble==null
  - Lösung: Eigenes StateFlow mit Launch-Collector in setBle()
- [ ] **SettingsRepository.otaState Fix**
  - Problem: Gleiches Problem wie bleState
  - Lösung: Analog zu BleRepository
- [ ] Alle Compilation Errors behoben
  - connectedDevice duplicate property
  - combine() 5-flow limit via nesting
  - Nullable String handling
  - BondingState.FAILED case
  - Method signature updates

### ✅ Phase 7: Cleanup
- [ ] Removed backward compatibility methods
  - `getBle()` aus BleRepository entfernt
  - `getOtaAction()` aus SettingsRepository & ConfigViewModel entfernt
- [ ] ConfigScreen refactored
  - Removed direct `otaAction` und `bleService` parameters
  - State jetzt via ConfigViewModel.otaState
  - OtaSection vereinfacht
- [ ] Unused imports entfernt

### 🚧 Phase 8: UI Polish & Final Integration (TODO)
- [ ] BleScreen Error Handling UI hinzufügen
  - Snackbar für bleRepo.error
  - Analog zu ConfigScreen error handling
- [ ] isDeviceConnected in ConfigScreen OtaSection
  - Via BleViewModel.connectedDevice != null
  - BleViewModel muss in ConfigScreen injected werden (für Connection Status)
  - Alternative: Eigenes connectedDeviceState in SettingsRepository
- [ ] Code Review & Optimization
  - Prüfen auf weitere direkte Service-Dependencies
  - WeakReferences entfernen (falls noch vorhanden)
  - Dead code cleanup
  - Alle TODOs/FIXMEs entfernen oder dokumentieren

#### Acceptance Criteria Phase 8
- ✅ BleScreen zeigt alle Errors in Snackbar (analog ConfigScreen)
- ✅ ConfigScreen OTA Section zeigt korrekten Connection Status
- ✅ Keine direkten Service-Dependencies mehr im UI Code
- ✅ Keine WeakReferences mehr im Code
- ✅ Alle obsoleten TODOs/FIXMEs removed

### 📋 Phase 9: Testing & Quality Assurance (TODO)
- [ ] Repository Unit Tests
  - BleRepository: Connection flow, error handling, state propagation
  - GamesRepository: API calls, game loading, error scenarios
  - SettingsRepository: OTA state transitions, token management
- [ ] ViewModel Tests
  - BleViewModel: StateFlow combination behavior (8 flows via nested combine)
  - ConfigViewModel: Settings updates, OTA state handling
  - Error propagation from Repositories to UI
- [ ] Manual Integration Tests
  - Happy path: Scan → Connect → Bond → Load Games → Play
  - Error scenarios:
    - Connection loss während game loading
    - Bonding failure (wrong PIN)
    - Network errors bei Lichess API
    - OTA upload failures
  - OTA upload flow: Select file → Upload → Verify → Reboot
- [ ] Performance Tests
  - StateFlow memory leaks prüfen
  - BLE scan performance
  - Game loading latency

#### Acceptance Criteria Phase 9
- ✅ Alle kritischen Flows manuell getestet
- ✅ Error handling funktioniert in allen Szenarien
- ✅ Keine Memory Leaks (LeakCanary optional)
- ✅ Performance akzeptabel (subjektiv, keine Regressionen)

## Architektur-Übersicht

### Repositories (3)
1. **BleRepository**
   - BLE device scanning, connection, pairing
   - State: bleState, showPinDialog, error

2. **GamesRepository**
   - Lichess API integration
   - Game loading & management
   - State: availableGames, selectedGameKey, isLoadingGames, isLoadingGame, error

3. **SettingsRepository**
   - App settings (token, language)
   - OTA firmware updates
   - State: lichessToken, language, otaState, error

### ViewModels (2)
1. **ConfigViewModel**
   - Settings & OTA screen
   - Delegates to SettingsRepository

2. **BleViewModel**
   - BLE & Chess screen
   - Combines BleRepository + GamesRepository
   - 8 StateFlows via nested combine()

### Key Files Modified
- `App.kt` - Koin initialization
- `AppModule.kt` - DI configuration
- `AppError.kt` - Sealed error classes
- `BleRepository.kt` - BLE state management
- `GamesRepository.kt` - Game state management
- `SettingsRepository.kt` - Settings state management
- `BleViewModel.kt` - Refactored, -25% LOC
- `ConfigViewModel.kt` - Refactored, -10% LOC
- `BleScreen.kt` - Updated to use repositories
- `ConfigScreen.kt` - Updated to use repositories
- `BluetoothService.kt` - Koin injection

## Metriken

### Lines of Code
- **BleViewModel**: 183 → 137 (-25%)
- **ConfigViewModel**: 75 → 68 (-10%)
- **BleRepository**: +200 LOC (neu)
- **GamesRepository**: +200 LOC (neu)
- **SettingsRepository**: +160 LOC (neu)

### Architektur
- **3 Repositories** (klare Separation of Concerns)
- **2 ViewModels** (pure delegation, keine Business Logic)
- **Koin DI** (lightweight, keine Over-Engineering)
- **Sealed Error Classes** (type-safe error handling)
- **StateFlow überall** (reactive, single source of truth)

## Lessons Learned

### Flow.combine() Limitation
Kotlin Flow's `combine()` unterstützt nur bis zu 5 Flows. Bei mehr Flows muss man nesting verwenden:
```kotlin
val group1 = combine(flow1, flow2, flow3, flow4) { ... }
val group2 = combine(flow5, flow6, flow7, flow8) { ... }
val result = combine(group1, group2) { ... }
```

### StateFlow Getter Anti-Pattern
❌ **Bad**: Getter der neue StateFlow erstellt
```kotlin
val state: StateFlow<State>
    get() = service?.state ?: MutableStateFlow(State()).asStateFlow()
```

✅ **Good**: Eigenes StateFlow mit Collection
```kotlin
private val _state = MutableStateFlow(State())
val state: StateFlow<State> = _state.asStateFlow()

fun setService(service: Service) {
    this.service = service
    scope.launch {
        service.state.collect { _state.value = it }
    }
}
```

### Repository Pattern ohne Over-Engineering
- ✅ Repositories für komplexe State & Business Logic
- ✅ ViewModels für UI State Aggregation
- ❌ KEINE Use Cases mit nur einer Action
- ❌ KEINE zusätzlichen Abstraktions-Layer

## Lifecycle & Memory Management

### Repository Scopes
- **Lifecycle**: Service-scoped (via Koin `single` - solange BluetoothService läuft)
- **CoroutineScope**: `CoroutineScope(SupervisorJob() + Dispatchers.Main)`
  - SupervisorJob: Child failures beeinflussen keine siblings
  - Dispatchers.Main: StateFlow updates auf Main Thread (UI-safe)
- **Cleanup**: Repositories werden mit BluetoothService zerstört

### ViewModel Scopes
- **Lifecycle**: Activity/Screen-scoped (via `koinViewModel()`)
- **CoroutineScope**: `viewModelScope` (automatisch von Android)
  - Wird bei ViewModel.onCleared() automatisch cancelled
- **Cleanup**: Automatisch beim Screen close

### StateFlow Collection
```kotlin
// Repository sammelt von Service
scope.launch {
    service.state.collect { _state.value = it }
}
```
- Collection wird mit Repository Scope cancelled
- Keine Memory Leaks: ViewModels referenzieren nur Repositories (keine Activity/Context)
- StateFlow ist cold: Subscription wird automatisch cancelled bei Scope cancellation

### Potential Issues ⚠️
1. **BluetoothService Restart**: Repositories behalten alten State
   - Lösung: Repositories sollten State bei setBle() resetten
2. **Long-lived Collections**: Repository lebt länger als einzelne ViewModels
   - Aktuell OK: Repositories sind lightweight State Holder
   - Watch out: Keine Activity/Fragment References in Repositories!

## Dependencies & Requirements

### Versions
- **Koin**: 3.5.6
  - Stable, lightweight DI
  - Breaking changes in 4.x: Migration bei Update nötig
- **Kotlin**: 1.9+ (für StateFlow API)
- **Kotlin Coroutines**: 1.7+
- **Android**: minSdk = ? (TODO: Check project config)

### Known Limitations
- **Flow.combine()**: Max 5 flows, nesting required für mehr
- **Koin**: Keine compile-time safety (vs. Hilt/Dagger)
- **StateFlow**: Hot flow - initial value required

### Risiken
- ❌ Koin 4.x Migration kann breaking changes haben
- ✅ Kotlin Flow API ist stabil
- ⚠️ Repository lifecycle muss mit Service lifecycle aligned sein
