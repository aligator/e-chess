# AI Agent Guide

This document defines how AI agents should work on the E‑Chess UI.

## Purpose
Build a clean, maintainable UI without overengineering. Keep functionality intact and improve UX only when it is clearly better.

## Architecture & Structure
- Feature‑first packages: `feature/<name>` (e.g., `feature/ble`, `feature/settings`).
- Platform code lives in `platform/<area>` (e.g., `platform/ble`).
- Data/storage lives in `data/` and `data/model/`.
- UI only talks to ViewModels; ViewModels talk to platform/data.

## UI State Rules
- Single `UiState` per screen, source of truth for the UI.
- Prefer sealed state models for clear flows: `Requirement`, `Ready`, `Loading`, `Error`.
- No business logic inside Composables; keep them deterministic.

## UX Principles
- Always show a clear status for BLE (ready, scanning, connected).
- Use a simple step flow for BLE: Scan → Connect → Load.
- Provide explicit empty states and actionable guidance.
- Favor snackbars for user actions and feedback; avoid silent failures.

## Copy & Strings
- All text must be in `strings.xml` (including `values-de` and `values-nb`).
- Keep text short, clear, and action‑oriented.
- Naming: use consistent prefixes (`ble_*`, `settings_*`, `ota_*`).

## Components
- Reusable components go in `feature/<name>/components`.
- Use cards to group related settings instead of dividers.
- Avoid overly clever UI patterns; clarity first.

## Lifecycle & Performance
- Stop BLE scanning on `ON_PAUSE`.
- Avoid collecting flows without cancellation.
- Use `remember` and `LaunchedEffect` only when needed.

## Quality Bar
- Keep changes small, consistent, and testable.
- If a UX change is not clearly better, ask first.

