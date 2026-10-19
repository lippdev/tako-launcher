# STREAM-001 Sunshine Integration Implementation Plan

> **For agentic workers:** Implement this plan task-by-task, in order, checking off each step as it passes. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Detect Sunshine/Apollo locally and let users preview, confirm, and safely export Tako games while documenting Moonlight as the client.

**Architecture:** Keep OS process/install detection and JSON file merging in `tako-platform`; keep preview, confirmation, persisted sync preference, and library mapping in `tako-ui`. Export merges only entries carrying the Tako marker, creates a backup first, and refuses invalid JSON.

**Tech Stack:** Rust 2024, serde_json, Slint UI patterns, existing SettingsStore.

**Spec:** `docs/PRD.md` STREAM-001; `docs/adr/0034-integracao-sunshine-moonlight.md`.

## Global Constraints

- `CARGO_BUILD_JOBS=3`.
- No network calls; all integration stays on the local machine.
- New UI text exists in pt-BR and en-US.
- Desktop follows `LinhaDeConfiguracao` in tako-design.
- Back up `apps.json` before writing; preserve user entries; refuse malformed JSON.
- Capture settings at 1600x900 and 960x600 outside the repository.
- Run crate tests during work; full workspace tests and clippy once before push.

## Review Focus

- Malformed Sunshine JSON: fail before writing; covered by platform export tests.
- User entries resembling Tako entries: preserve unless both marker and URI identify Tako; covered by merge fixture tests.
- Paths and URI arguments with spaces/reserved characters: quote and JSON escape correctly; covered by command construction tests.
- Missing config file: preview still works and confirmed export creates a valid file; covered by export tests.
- Sync enabled during rescan: only after explicit first confirmation; covered by settings tests.

### Task 1: Safe Sunshine file integration

**Files:** `crates/tako-platform/src/sunshine.rs`, `crates/tako-platform/src/lib.rs`, `crates/tako-platform/tests/fixtures/sunshine_apps.json`, `crates/tako-platform/tests/fixtures/sunshine_malformed.json`.

- [x] Detect installation and running status on Windows, Linux, and macOS.
- [x] Construct `cmd` entries that launch the current Tako executable with `--open-url tako://play/<id>`.
- [x] Preserve non-Tako data, back up before writing, and refuse malformed input.
- [x] Test preservation, idempotency, argument/path quoting, backup, and malformed input.

### Task 2: Settings consent and optional synchronization

**Files:** `crates/tako-ui/src/shell/settings_sunshine.rs`, `crates/tako-ui/src/shell/settings.rs`, `crates/tako-ui/src/shell/ctx.rs`, `crates/tako-ui/src/app.rs`, `crates/tako-ui/src/shell/settings_tests.rs`, `crates/tako-ui/src/ui_tests.rs`, `crates/tako-core/locales/{pt-BR,en-US}/settings.toml`.

- [x] Add a Settings › Integrations category with detection state, export preview, confirmation, Moonlight guidance, and sync toggle off by default.
- [x] Persist explicit sync consent and update exports after a library rescan only when enabled.
- [x] Verify UI labels exist in both locales and enabling sync first opens the preview.

### Task 3: Decision, docs, and captures

**Files:** `docs/adr/0034-integracao-sunshine-moonlight.md`, `docs/adr/README.md`, `docs/PRD.md`, `docs/testing/stream-001-sunshine.md`.

- [x] Record the file-based integration and Moonlight client scope.
- [x] Mark STREAM-001 complete and document manual verification on all three OSes.
- [x] Capture and compare the desktop screen at 1600x900 and 960x600 using `LinhaDeConfiguracao`.
