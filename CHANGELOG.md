# Changelog

All notable changes to this project are documented in this file.

## [0.6.33] - 2026-03-24

### Fixed
- Center title special highlight now uses Hyprland monitor state (`j/monitors -> specialWorkspace`) instead of workspace-name heuristics.
- This avoids false positives/negatives when special workspace naming/listing differs from focus window title behavior.

### Quality
- Added regression tests for parsing visible/absent special workspace state from monitor JSON.
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.32] - 2026-03-24

### Fixed
- Special-focus center highlight now works when Hyprland reports `activeworkspace` as special but omits that entry from `j/workspaces`.
- Added synthetic active special workspace entry in that edge case so UI state remains consistent.

### Quality
- Added regression test covering synthetic special workspace insertion path.
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.31] - 2026-03-24

### Fixed
- Center focused-window title highlight no longer stays permanently enabled when special workspaces are merely present in workspace list.
- Active special detection is now tied to Hyprland `activeworkspace` identity using both id and name matching, so highlight appears only while special is actually active.

### Quality
- Added regression test for workspace active-state resolution by id/name fallback.
- Updated app-level regression test to enforce active-only special highlight logic.
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.30] - 2026-03-24

### Fixed
- Center focused-window title highlight for special workspaces now triggers when a `special` workspace is present in Hyprland workspace list, not only when it is marked `active`.
- This fixes cases where special windows are visible but Hyprland does not report the special workspace as active.

### Quality
- Updated regression test to cover visible special workspace detection.
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.29] - 2026-03-24

### Changed
- Center focused-window title pill now switches to a light-red background when an active Hyprland `special` workspace is present.

### Quality
- Added regression test for active special workspace detection.
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.28] - 2026-03-24

### Fixed
- Clicking Hyprland `special` workspace pills now works correctly:
  - regular workspaces use `dispatch workspace <id>`;
  - `special` / `special:*` use `dispatch togglespecialworkspace [name]`.
- Removed optimistic local active-state flip on workspace click; UI now reflects actual state from Hyprland refresh.

### Quality
- Added regression tests for workspace dispatch command mapping (normal vs special workspace names).
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.27] - 2026-03-24

### Changed
- Workspace pill for Hyprland `special` workspace (`special` / `special:*`) now uses orange styling in both active and inactive states.

### Quality
- Added regression test for special workspace name detection.
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.26] - 2026-03-24

### Fixed
- Launcher process lifecycle hardened: after spawning `rofi`, thinkpadbar now explicitly waits for child exit in a background reaper thread.
- This prevents accumulation of zombie `rofi` processes after manual launcher close.

### Quality
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.25] - 2026-03-24

### Fixed
- Launcher command now uses `rofi -replace -show drun` to force reopening even when a stale/background `rofi` process remains after manual close.

### Quality
- Updated launcher regression test to validate `-replace` argument in command mapping.
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.24] - 2026-03-24

### Changed
- Launcher button behavior simplified: it now always runs `rofi -show drun` and no longer attempts toggle/kill logic.
- Removed launcher process discovery/kill path (`pgrep/pkill`) to avoid false-positive "running" states that blocked reopening after manual close.

### Quality
- Removed obsolete launcher toggle command tests and kept launcher command mapping regression test.
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.23] - 2026-03-24

### Fixed
- Launcher toggle now uses process discovery (`pgrep -x rofi`) and close (`pkill -x rofi`) instead of tracking spawned child PID, which fixes reopen-after-manual-close regressions.
- Suppressed launcher command stdout/stderr in toggle path to prevent PID spam in terminal logs.

### Quality
- Added regression tests for launcher check/close command mappings (`pgrep -x rofi`, `pkill -x rofi`).
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.22] - 2026-03-24

### Fixed
- Launcher button now toggles `rofi`: second click closes launcher (`pkill -x rofi`) instead of spawning additional instances.
- Launcher button icon changed from emoji rocket to Nerd Font glyph (``) to avoid missing-glyph square rendering on systems without emoji font support.

### Quality
- Added regression test for launcher close command mapping (`pkill -x rofi`).
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.21] - 2026-03-24

### Added
- Added launcher button (`🚀`) to the left of workspace buttons in the main bar.
- Added launcher action that opens system launch manager via `rofi -show drun`.
- Added regression test verifying launcher command mapping (`rofi`, `-show`, `drun`).

### Quality
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.20] - 2026-03-24

### Fixed
- Popup layer-surface handling now explicitly enforces `exclusive_zone = 0`:
  - at popup surface creation,
  - on popup show,
  - on popup hide.
- This hardens against compositor-side stale input-region behavior where hidden popup surfaces could still interfere with pointer input.

### Quality
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.19] - 2026-03-24

### Fixed
- Bluetooth toggle path hardened to reduce permission-failure regressions:
  - tries `bluetoothctl power on|off`,
  - falls back to `rfkill block|unblock bluetooth`,
  - then to direct sysfs write (`/sys/class/rfkill/.../state`).
- Reduced repeated Bluetooth permission error spam by stopping after first matching rfkill device write failure.
- Bluetooth state updates now refresh via `TickSlow` after toggle request instead of optimistic local state flip.

### Added
- Added Control Center action button to launch `overskride` (with `flatpak run io.github.kaii_lb.Overskride` fallback).
- Added parser tests for `bluetoothctl show` powered-state extraction.

### Quality
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.18] - 2026-03-24

### Changed
- Performance profile button in Control Center now displays effective live intervals directly:
  - format: `Perf <badge> <brightness>/<thermal>/<slow>`,
  - example: `Perf NRM 1/2/10`.
- This provides immediate visual feedback for runtime profile switches and applied interval values.

### Quality
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.17] - 2026-03-24

### Added
- Added runtime performance profile cycling in Control Center (`Perf NRM/LP/HR` button).
- Added `PerformanceConfig` helpers:
  - profile normalization,
  - compact profile badge label,
  - runtime cycling logic (`normal -> low_power -> high_responsiveness -> normal`).
- Added regression test for runtime profile cycle behavior.

### Changed
- Runtime profile cycle now applies profile defaults immediately by resetting explicit tick overrides to profile-driven mode.

### Quality
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.16] - 2026-03-24

### Added
- Added performance profile support in config:
  - `performance.profile = "normal" | "low_power" | "high_responsiveness"`.
- Added profile-based effective interval resolver with explicit per-field override support.
- Added tests for:
  - default profile behavior,
  - low_power profile mapping,
  - explicit interval override precedence.

### Changed
- App subscriptions now use profile-resolved effective intervals instead of raw values only.
- Updated config example and README with profile-driven tuning guidance.

### Quality
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.15] - 2026-03-24

### Added
- Added configurable refresh intervals in config:
  - `performance.tick_brightness_secs`
  - `performance.tick_thermal_secs`
  - `performance.tick_slow_secs`
- Added defaults for the new performance section with backward-compatible parsing when section is absent.
- Added config tests for default intervals and missing-section fallback parsing.

### Changed
- App subscriptions now use configured performance intervals (with safety clamp to at least 1 second).
- Updated `config.toml.example` and README with performance tuning examples.

### Quality
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.14] - 2026-03-24

### Added
- Added runtime metrics tool: `scripts/runtime-metrics.sh`.
  - Collects sampled `%CPU`, `RSS`, threads, FD count, and context-switch deltas.
  - Produces CSV output suitable for before/after optimization comparisons.
- Added documentation: `docs/validation/runtime-metrics.md`.
- Added README section with quick metrics examples.

### Quality
- Script syntax validated with `bash -n`.
- Validation passed: `cargo check`.

## [0.6.13] - 2026-03-24

### Changed
- Wi-Fi iwd path discovery now uses a short TTL cache (30s), reducing repeated `busctl tree` process launches in steady state.
- Wi-Fi scanning switched from fixed `sleep(1500ms)` to adaptive retry polling (`10 x 150ms` with early exit), improving perceived menu responsiveness when networks appear quickly.

### Quality
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.12] - 2026-03-24

### Changed
- Reduced redundant runtime polling:
  - `Tick` no longer force-refreshes audio/mic every second;
  - audio event subscription now sends dedicated `RefreshAudioMic` update instead of full `Tick`;
  - `UpdateWorkspaces` no longer triggers extra audio/mic probes.
- Added tray icon resolution cache (`icon_name -> image handle`) to avoid repeated filesystem icon lookups on recurring tray updates.
- Reduced `SysMonitor` copy overhead by replacing clone-at-start path with move-based update (`mem::take`) and a single clone on return.

### Quality
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

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
