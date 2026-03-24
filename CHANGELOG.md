# Changelog

All notable changes to this project are documented in this file.

## [0.6.11] - 2026-03-24

### Fixed
- Reverted popup keyboard interactivity from `Exclusive` to `OnDemand` to eliminate compositor-wide input lock risk.
- Removed global mouse-press dismiss hook that could interact badly with layer-surface focus routing.
- Added safe popup auto-close on active-window change (`WorkspacesUpdated` title change), avoiding global input capture.

### Quality
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.10] - 2026-03-24

### Fixed
- Popup dismiss logic on Wayland hardened:
  - popup layer now requests `KeyboardInteractivity::Exclusive` while open to reliably receive focus-loss events;
  - popup also closes on mouse press routed to the main bar window while any popup is open.
- This restores practical outside-dismiss behavior without reintroducing a fullscreen click-capture surface.

### Quality
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.9] - 2026-03-24

### Fixed
- Restored popup auto-dismiss behavior on outside click/focus loss: when popup window receives `Unfocused`, open popup is now closed and hidden immediately.
- Dismiss is scoped to the popup window only; no fullscreen click-capture overlay was reintroduced.

### Quality
- Added app-level regression tests for popup close condition on `window::Event::Unfocused`.
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.8] - 2026-03-24

### Fixed
- iwd path handling is now discovery-first: runtime-discovered adapter/station paths are prioritized before config values.
- Removed residual hardcoded station path (`/net/connman/iwd/0/wlan0`) from Wi-Fi power toggle flow.
- Improved station-path discovery for iwd layouts like `/net/connman/iwd/0/5` to avoid selecting network-level object paths.

### Quality
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.7] - 2026-03-24

### Fixed
- Wi-Fi network list fallback parser now strips ANSI escape sequences from `iwctl` output, preventing control-code garbage in popup UI.
- Wi-Fi fallback list now filters table header lines (`Network name / Security / Signal`) and separator rows, showing only actual SSIDs.

### Quality
- Added regression test with ANSI-colored `iwctl` sample output to ensure clean parsing.
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.6] - 2026-03-24

### Fixed
- Wi-Fi popup no longer blocks scanning/toggling when `config.toml` iwd paths are stale; runtime path discovery is now used instead of hard UI rejection.
- Connected SSID detection became more resilient with fallback parsing from `iw dev` when D-Bus station network name is unavailable.
- Network scan fallback added via `iwctl station <iface> get-networks` when iwd D-Bus ordered network list is empty.

### Quality
- Added Wi-Fi parser regression tests for `iw dev` SSID extraction and `iwctl` network-list parsing.
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.5] - 2026-03-24

### Fixed
- Keyboard layout click switching no longer prints `ok` in terminal output.
- Center active-window pill remains strictly centered but is now hard-limited in width and text length to prevent overlap with neighboring bars.

### Changed
- Layout switching fallback path improved with device-targeted `hyprctl switchxkblayout` attempts.
- Keyboard layout labels normalized for compact display (`Russian -> RU`, `English -> US`).

### Quality
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.4] - 2026-03-24

### Fixed
- Popup surfaces are now strictly content-bounded for all popup types (calendar/system/control center) and no longer capture the full top screen area.
- Removed fullscreen popup click-capture behavior; popups now avoid blocking clicks/scroll outside their visible bounds.

### Changed
- Added explicit Wi-Fi status messages in control center for operational states and failures:
  - invalid iwd config paths,
  - D-Bus unavailable,
  - scan progress and empty scan result,
  - connect progress and success/failure result.
- Wi-Fi path validation helper exposed for app-level diagnostics.

### Quality
- Fixed task execution flow for Wi-Fi/Bluetooth toggles by returning `Task::perform` instead of dropping tasks.
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.3] - 2026-03-24

### Changed
- Split metric refresh contours for responsiveness under load:
  - `Tick` keeps fast UI/clock/audio-mic updates.
  - `TickThermal` (2s) updates temperature and fan state for compile-time visibility.
  - `TickSlow` keeps non-interactive heavier refreshes.
- Added `read_temperature_celsius()` in system module and reused it for thermal reads.

### Fixed
- Removed panic-prone runtime paths:
  - Wi-Fi D-Bus path builders no longer use `unwrap()` and now validate object paths.
  - Audio subscription handles `pactl` spawn/stdout failures gracefully with retry.
  - Tray initialization handles poisoned lock without panic.
  - Safer command/date handling in app flow without `unwrap()` at runtime boundaries.

### Quality
- Added Wi-Fi object path validation unit tests.
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.2] - 2026-03-24

### Fixed
- Popup surface now starts hidden (`1px`) to avoid accidental input interception in the top screen area when popup is closed.
- Added stricter popup surface hide/show handling in app flow, including explicit hide on power-action close path.
- Improved brightness rendering consistency: module now returns percentage-only string, and UI renders icon + value with tighter alignment.

### Quality
- Resolved strict clippy warnings in `src/modules/system.rs` (removed unnecessary casts).
- Added unit tests for memory parsing helper in `src/modules/system.rs`.
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.
