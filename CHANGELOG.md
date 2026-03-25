# Changelog

All notable changes to this project are documented in this file.

## [0.6.80] - 2026-03-26

### Changed
- Extended `P2 Coalescing Foundation` from slider command bursts to refresh/request paths:
  - added a reusable `RequestCoalescer<K>` primitive in `services/coalescing.rs` for per-key in-flight deduplication with a single queued rerun;
  - wired `RefreshControls(...)` through the new coalescing gate, so repeated brightness, fan, slow, and audio refresh requests no longer spawn parallel duplicate backend tasks for the same refresh kind;
  - made `ControlsRefreshed` carry its originating refresh kind, allowing the app to deterministically rerun exactly one queued refresh after an in-flight request completes.

### Quality
- Added regression tests for:
  - request coalescer restart-on-completion behavior,
  - independent coalescing state for distinct refresh keys,
  - app-level refresh deduplication for repeated brightness refreshes,
  - queued follow-up refresh behavior while the same control refresh is already in flight.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [0.6.79] - 2026-03-26

### Changed
- Started `P2 Coalescing Foundation` with a minimal reusable control-path coalescer:
  - added `services/coalescing.rs` with a small generic `ValueCoalescer<T>` primitive for latest-value wins semantics;
  - wired control-center slider bursts through coalesced flushes for `volume`, `mic volume`, and `brightness`, so dragging sliders no longer spawns a backend command for every intermediate value;
  - kept UI preview immediate while deferring the actual backend command to a short coalesced flush window.

### Quality
- Added regression tests for:
  - stale coalescing generations not consuming the latest pending value,
  - current generations consuming a value only once,
  - per-kind coalescing independence for `volume` and `brightness`,
  - latest-generation wins behavior in the `app` control coalescing path.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [0.6.78] - 2026-03-26

### Changed
- Started `P1 Hermetic Test Sweep` and removed the remaining real runtime probes from the most visible test paths:
  - `app` tests now use hermetic test-only constructors for compositor, controls, idle inhibitor, and system info instead of touching live Wayland or host runtime state;
  - `CompositorService` now has a hermetic test constructor and pure backend-kind resolution test path, so compositor tests no longer depend on Hyprland environment variables or sockets;
  - `SystemInfoService` thermal logic now has a pure helper/test seam, and system-info tests no longer read live `/proc` or `/sys` state;
  - `ControlsService` now has a test-only snapshot constructor with noop backends, removing host audio/brightness/bluetooth/power reads from `app` test setup;
  - power backend tests now validate pure profile-resolution logic instead of reading the real platform-profile sysfs path.

### Quality
- Added and updated regression tests for:
  - hermetic compositor snapshot and backend-kind resolution,
  - hermetic system-info thermal updates,
  - pure power-profile resolution behavior,
  - `app` coalescing test setup without live runtime probes.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [0.6.77] - 2026-03-25

### Fixed
- Removed the last GUI side effect from the session-service unit tests:
  - `SessionService` now owns a runner seam for launcher/session commands, so unit tests can assert the requested command without actually spawning `rofi`, `hyprlock`, or other real processes;
  - `cargo test` no longer flashes the launcher because `launcher_returns_no_follow_up` now uses a recording runner instead of executing `rofi -replace -show drun`.

### Quality
- Added regression tests for:
  - launcher command execution returning `None` without spawning a real process,
  - lock command execution routing through the runner and requesting compositor refresh.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [0.6.76] - 2026-03-25

### Changed
- Added a practical systems-polish slice across native Wayland diagnostics, ThinkPad hardware visibility, and backend observability:
  - `IdleInhibitorService` now exposes typed runtime diagnostics for the native Wayland backend, including backend name, requested state, surface binding, and compositor/idle-inhibit protocol versions;
  - System Info now includes a `ThinkPad Hardware` block with battery runtime, active power profile, fan runtime, and idle inhibitor status;
  - debug-only `Observability` now reports controls backend identities plus the live idle-inhibitor runtime summary.

### Quality
- Added regression tests for:
  - controls backend diagnostics exposure,
  - idle inhibitor diagnostics/debug summary,
  - ThinkPad hardware summary formatting in `app.rs`.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [0.6.75] - 2026-03-25

### Changed
- Completed `M4 Controls Backend Split`:
  - introduced explicit control backend seams in `services/controls_backends/` for `audio`, `brightness`, `bluetooth`, and `power`;
  - `ControlsService` now owns backend instances and routes refresh/command execution through service-owned backend traits instead of directly calling `modules/{audio,brightness,bluetooth,power}.rs`;
  - moved audio sink/source runtime access into a single `WpctlAudioBackend`, including the `pactl subscribe` event subscription path used by the control-center refresh flow;
  - removed legacy compiled control modules for `audio`, `brightness`, `bluetooth`, and `power`, leaving the lower-level system access split under the service layer;
  - kept ThinkPad-specific `fan`, `battery`, and `mic LED` flows pragmatic and focused, instead of over-generalizing them prematurely.

### Quality
- Added regression tests for:
  - backend-specific parsing in audio, brightness, bluetooth, and power backends,
  - `ControlsService` refresh delegation through migrated backends,
  - `ControlsService` command execution delegation through migrated backends.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [0.6.74] - 2026-03-25

### Changed
- Closed `TD-TRAY-002` on the post-`M3` service-owned tray architecture:
  - `OwnedTrayMenu` now keeps per-action prefetch ancestry, so nested `DBusMenu` activations prefetch `root -> submenu ancestors -> selected item` instead of blindly dispatching `root + selected`;
  - submenu container items (`children-display=submenu`) are now treated as non-activatable in the local flattened tray menu, which prevents dead local clicks on branch nodes that only exist to reveal children;
  - tray popup sizing is now menu-aware instead of using a fixed `420px` height, which keeps the local tray menu anchored closer to the click target for short and medium menus.

### Quality
- Added regression tests for:
  - nested tray-menu selection preserving submenu ancestry in the dispatch prefetch path,
  - submenu-header selections closing the popup without dispatching a dead action,
  - ancestor-aware tray prefetch sequencing without duplicates,
  - menu-aware tray popup height planning.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [0.6.73] - 2026-03-25

### Changed
- Completed the remaining `M3 Tray Domain Cleanup` step:
  - `TrayUiService` now owns the current open tray-menu id in addition to menu cursor state, so `app.rs` no longer carries tray-specific popup identity state;
  - `Popup::TrayMenu` is now a plain popup kind without embedded item id, and tray menu selection dispatch uses the current service-owned open menu context;
  - tray runtime removal and repeated secondary-click behavior now close/open the current tray menu through service-owned state transitions instead of `app`-level string matching.

### Quality
- Added regression tests for:
  - repeated secondary-click toggling the same tray menu open/closed,
  - runtime tray-item removal closing an open tray menu,
  - menu item selection using the current service-owned open menu id.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features` (`89` tests passed).

## [0.6.72] - 2026-03-25

### Changed
- Completed `M2 Compositor Runtime` in practical terms:
  - `CompositorService` is now the sole owner of compositor snapshot state, and `app.rs` no longer stores a parallel `CompositorSnapshot` copy;
  - compositor refreshes now apply back into service-owned state through `CompositorService::apply_refresh(...)`, so the app consumes the service snapshot boundary directly;
  - the compositor runtime remains intentionally `Hyprland-only` for this stage, while configured `niri` still degrades explicitly to active `Hyprland`.
- Completed the service-layer cleanup slice of `M3 Tray Domain Cleanup`:
  - moved the remaining tray domain model/runtime helpers from `modules/tray.rs` into `services/tray_model.rs`;
  - updated `services/tray.rs` and `services/tray_ui.rs` to use the new service-layer tray model path;
  - removed `modules/tray.rs` from the compiled module graph, so tray runtime/model ownership now lives entirely under `services/*`.

### Quality
- Preserved and relocated tray regression coverage under the service layer:
  - menu diff propagation,
  - icon candidate resolution,
  - address parsing and watcher lookup behavior,
  - secondary-click routing/fallback behavior,
  - tray UI selection/candidate generation.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features` (`86` tests passed).

## [0.6.71] - 2026-03-25

### Changed
- Completed the practical `M1 Network Runtime` cut:
  - `services/network/iwd.rs` now owns IWD path discovery, runtime path caching, D-Bus proxy calls, scan/connect/toggle operations, and CLI fallbacks instead of delegating those flows back into `modules/wifi.rs`;
  - `modules/wifi.rs` has been removed from the compiled module graph, so the Wi-Fi domain no longer hides a second runtime owner outside `NetworkService`;
  - preserved regression coverage for object-path validation, IWD path-shape parsing, ANSI-stripped `iwctl` network parsing, SSID fallback parsing, and `iwctl` passphrase argument handling.
- Started and completed the practical `M2 Compositor Runtime` first slice in the same release:
  - introduced `services/compositor/types.rs` and `services/compositor/hyprland.rs`, moving Hyprland IPC, event subscription, workspace queries, active-window lookup, keyboard-layout handling, cursor lookup, and app-location switching into the compositor service layer;
  - `CompositorService` is now backed by a real `HyprlandBackend` runtime owner instead of delegating into `modules/workspaces.rs` and `modules/keyboard.rs`;
  - removed `modules/workspaces.rs` and `modules/keyboard.rs` from the compiled module graph, keeping the app on a service-owned compositor boundary.

### Quality
- Added or preserved regression tests for:
  - Hyprland workspace dispatch rules, special-workspace visibility parsing, cursor-position parsing, client matching, layout-label normalization, and keyboard dispatch success detection;
  - IWD runtime parsing and fallback argument generation after the service-layer migration.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features` (`86` tests passed).

## [0.6.70] - 2026-03-25

### Changed
- Started `M1 Network Runtime` in the new simplified migration form:
  - removed the split `ConnectivityService` + `WifiFlowService` path and merged Wi-Fi state ownership into a single `NetworkService`;
  - introduced typed network-domain types in `services/network/types.rs` (`NetworkSnapshot`, `NetworkStatus`, `NetworkCommand`, `NetworkEvent`, `NetworkFollowUp`);
  - added a real `IwdBackend` in `services/network/iwd.rs` as the first and only runtime backend for this milestone;
  - kept `networkmanager` as a config-level compatibility marker that still falls back to the IWD runtime instead of pretending to provide full parity.
- Moved app-level Wi-Fi orchestration onto the typed network domain:
  - `app.rs` now drives Wi-Fi popup actions through `Message::NetworkCommand(...)` and async results through `Message::NetworkEvent(...)`,
  - periodic D-Bus Wi-Fi refresh now feeds `NetworkEvent::WifiInfoSynced(...)` instead of mutating a separate flow service.
- Reduced architectural duplication:
  - deleted legacy `src/services/connectivity.rs`,
  - deleted legacy `src/services/wifi_flow.rs`,
  - moved `WifiInfo` and `WifiNetwork` into the network service layer and made `modules/wifi.rs` use the service-owned types.

### Quality
- Added regression tests for:
  - secure-network password gating,
  - submit-password connect follow-up,
  - successful connect result status,
  - transient `Connecting(...)` status surviving background Wi-Fi sync,
  - backend fallback behavior for `networkmanager` configuration.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [0.6.69] - 2026-03-25

### Changed
- Reworked native Wayland idle inhibition to follow the `ashell` service pattern instead of relying on `iced` layer-surface startup events:
  - `services/idle_inhibitor.rs` now opens its own Wayland connection, binds `wl_compositor` and `zwp_idle_inhibit_manager_v1`, and creates a dedicated `wl_surface` for inhibition;
  - this removes the fragile dependency on `iced::event::wayland::Event::Layer(...)`, which did not provide a reliable startup bind path for the main bar surface and left the toggle stuck at `N/A`.
- Simplified app integration:
  - removed the `IdleInhibitorSurface` message path and the related Wayland event listener from `app.rs`,
  - the idle-inhibitor toggle is now fully service-owned and available as soon as the compositor exposes the protocol.

### Quality
- Added regression tests for:
  - available backend snapshot starting in `Off`,
  - successful enable transition on an available backend,
  - no-backend toggle behavior remaining inactive.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [0.6.68] - 2026-03-25

### Changed
- Added native Wayland idle inhibition using `zwp_idle_inhibit_manager_v1`:
  - introduced `services/idle_inhibitor.rs`, which binds the compositor's idle-inhibit manager from the existing main bar `wl_surface` backend instead of using external commands or compositor-specific shell integration;
  - the inhibitor is attached to the main layer-surface window, so the bar can keep the session awake while remaining inside the application's existing Wayland lifecycle.
- Extended the control popup with an idle-inhibitor toggle button:
  - the new button lives alongside the Wi-Fi and Bluetooth toggles,
  - it reflects `On` / `Off` / `N/A` based on real Wayland capability availability.
- Added a live idle-inhibitor status icon to the main pill:
  - when the inhibitor is active, an extra `coffee` icon is shown next to the existing connectivity/battery indicators.

### Quality
- Added regression tests for:
  - idle-inhibitor snapshot defaults,
  - idle-inhibitor label mapping,
  - no-support toggle behavior remaining inactive.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features` (`84` tests passed).

## [0.6.67] - 2026-03-25

### Changed
- Completed the remaining app-to-service migration seam for connectivity and tray runtime flows:
  - added `services/connectivity.rs` so Wi-Fi/network runtime ownership now sits behind a single `ConnectivityService` that wraps `NetworkService` and `WifiFlowService`;
  - `app.rs` no longer stores separate `network_service` and `wifi_flow` fields or dispatches their commands independently;
  - tray runtime subscription is now exposed through `TrayUiService::subscription()` and `TrayRuntimeEvent`, so `app.rs` no longer consumes the lower-level tray service runtime directly.
- Reduced `app.rs` further into typed orchestration-only behavior:
  - Wi-Fi scan/connect/toggle flows now route through `ConnectivityRequest` emitted by `ConnectivityService`;
  - tray runtime handling now stays behind `TrayUiService` boundaries instead of mixing service/runtime details into app message flow.

### Quality
- Added regression tests for:
  - connectivity service backend exposure,
  - Wi-Fi menu-open scan intent,
  - secure-network selection remaining local until password submission.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features` (`81` tests passed).

## [0.6.66] - 2026-03-25

### Changed
- Continued the service migration for remaining direct `app.rs` flows:
  - added `services/system_info.rs` so system-monitor state now lives behind `SystemInfoService` instead of direct `SysMonitor`/`SysData` ownership in app state;
  - added `services/session.rs` so launcher and power actions now execute through typed `SessionCommand` / `SessionFollowUp` boundaries;
  - added `services/tray_ui.rs` so tray UI state, menu cursor tracking, candidate generation, and local tray-menu selection validation are no longer implemented ad hoc in `app.rs`.
- Reduced `app.rs` further into orchestration-only logic:
  - removed direct ownership of `SysMonitor`, `SysData`, `show_power_menu`, tray state, and tray-menu cursor from app state;
  - popup close/open paths now clear session/tray transient state through service APIs;
  - tray primary/secondary/menu-item interaction paths now go through `TrayUiService`.

### Quality
- Added regression tests for:
  - `SystemInfoService` snapshot/thermal refresh behavior,
  - `SessionService` launcher command and power-menu state,
  - `TrayUiService` tray search candidate generation and invalid menu selection handling.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features` (`78` tests passed).

## [0.6.65] - 2026-03-25

### Changed
- Completed the next app-orchestration reduction step from plan item 4:
  - added `services/controls.rs` with owned `ControlsSnapshot`, typed `ControlsRefreshKind`, `ControlsEvent`, `ControlsCommand`, and `ControlsFollowUp`;
  - moved control-center runtime orchestration in `app.rs` from direct module calls to `ControlsService` refresh/command boundaries;
  - removed direct app ownership of audio/mic/brightness/fan/battery/power/bluetooth runtime fields in favor of typed service snapshot state.
- Deepened compositor service ownership:
  - `CompositorSnapshot` now includes keyboard layout,
  - `CompositorService` now owns typed compositor subscriptions/events and keyboard-layout actions,
  - `app.rs` now refreshes compositor state through `CompositorEvent`/`CompositorRefreshed` instead of module-level workspace messages.
- Lower layers no longer depend on `app::Message` for these flows:
  - `modules/audio.rs` subscription now emits typed controls events,
  - `modules/workspaces.rs` subscription now emits typed compositor events.

### Quality
- Added regression tests for:
  - controls service brightness parsing,
  - controls snapshot preview/apply flows,
  - compositor snapshot still honoring backend fallback semantics after typed event migration.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features` (`73` tests passed).

## [0.6.64] - 2026-03-25

### Changed
- Implemented plan items 1-3 around service ownership and app decoupling:
  - added `WifiFlowService` to own Wi-Fi transient UI flow state, scan/connect/toggle intents, and typed `WifiFlowSnapshot`/`WifiFlowCommand` transitions;
  - added `PopupAnchorService` to own popup surface sizing and anchor policy instead of keeping layout heuristics in `app.rs`;
  - moved `app.rs` to typed service-driven state for compositor refresh results and Wi-Fi flow snapshots instead of ad-hoc local orchestration fields.
- Extended service configuration with backend skeletons:
  - `compositor.backend = "hyprland" | "niri"`
  - `network.backend = "iwd" | "networkmanager"`
- Added graceful backend fallback semantics for not-yet-implemented backends:
  - `niri` currently reports configured `Niri` but runs on `Hyprland`,
  - `networkmanager` currently reports configured `NetworkManager` but runs on `Iwd`.
- Exposed service backend information in debug/system info so runtime state shows configured versus active backend.

### Quality
- Added regression tests for:
  - Wi-Fi flow state transitions and command emission,
  - popup anchor planning,
  - backend skeleton fallback behavior in compositor and network services.
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.63] - 2026-03-25

### Changed
- Continued architecture work for plan items 3/4 (stateful services + app decoupling):
  - `services/compositor` is now stateful via `CompositorService`:
    - owns backend access plus deterministic `WorkspaceRefreshCoalescer`,
    - exposes typed snapshots (`CompositorSnapshot`, `RefreshResult`) and refresh flow.
  - `services/network` is now stateful via `NetworkService`:
    - owns network paths/config state and backend kind (`Iwd`),
    - app Wi-Fi flows now call service instance methods (`get_wifi_info`, `scan_networks`, `connect_network`, `toggle_wifi`).
- `app.rs` orchestration was reduced:
  - workspace refresh coalescing moved from app-local flags to `CompositorService`,
  - compositor and network interactions now go through owned service instances in app state.

### Quality
- Added regression tests for compositor coalescer/service snapshot and network service config ownership.
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.62] - 2026-03-25

### Fixed
- Addressed critical tray consistency issues from runtime/code-review findings:
  - `UpdateEvent::MenuDiff` is now applied to local tray menu state instead of being ignored, and owned menu model is rebuilt after diffs.
  - tray activation runtime caches (`resolved_item_addresses`, per-item preferred secondary actions, context connection) are now reset on each client reconnect to avoid stale `:1.xxx` routing state reuse.
- Added regression tests for:
  - nested `MenuDiff` propagation into owned menu model,
  - reconnect cache reset behavior in tray service runtime state.

### Quality
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.61] - 2026-03-25

### Changed
- Continued `services/*` migration (plan p.2):
  - added `src/services/network/mod.rs` with typed `NetworkBackend` abstraction and `IwdBackend` implementation,
  - switched app Wi-Fi flows (`get info`, `scan`, `connect`, `toggle`) to `services::network` API.
- Deepened tray owned menu model (plan p.3):
  - added `src/services/tray_menu.rs` with typed `OwnedTrayMenu`/`OwnedTrayMenuNode`,
  - tray state now stores and serves owned menu model per item for popup rendering and ID validation.
- Implemented `TD-TRAY-002` runtime mitigations (plan p.1):
  - tray popup now anchors near cursor for tray menu opens (`TOP|LEFT` + dynamic margin) instead of fixed far `TOP|RIGHT` position,
  - menu item activation path now prefetches dbusmenu state with deterministic sequence (`about_to_show(0)` + selected id) before click dispatch.

### Quality
- Added regression tests for:
  - tray popup margin/anchor computation,
  - tray menu prefetch sequencing,
  - owned tray menu flattening/depth mapping.
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.60] - 2026-03-25

### Changed
- Added new high-priority tech debt item `TD-TRAY-002` for remaining tray menu production issues:
  - local tray menu item actions do not execute reliably for all indicators,
  - tray menu popup anchor/position can appear far from the click target.
- Started next architecture step from roadmap: introduced `services/compositor` layer with `CompositorBackend` abstraction and `HyprlandBackend` implementation.
- Moved app workspace/compositor interactions to service API:
  - workspace list, active window title, special workspace visibility,
  - workspace switch command,
  - compositor event subscription,
  - app-locate/switch lookup for tray click flow.
- Tray cursor lookup now uses `services::compositor::cursor_position()` instead of direct module call.

### Quality
- Added service-level regression test for compositor cursor-position API path.
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.59] - 2026-03-25

### Changed
- Step 2 of tray hardening: introduced owned tray menu model in app/module flow.
- `modules/tray` now stores per-item `menu_layout` state (`TrayMenu`) from tray events.
- Right-click behavior is now deterministic:
  - if local tray menu model exists, open local tray menu popup and execute selected menu item via `DBusMenu` command path,
  - otherwise keep existing secondary-click activation strategy fallback chain.
- Introduced typed tray command channel (`TrayCommand`) between module state and service runtime.

### Quality
- Added regression coverage for menu-entry availability helper and existing tray fallback paths.
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.58] - 2026-03-25

### Changed
- Step 1 of `services/*` migration:
  - introduced `src/services/mod.rs` and `src/services/tray.rs`,
  - moved tray runtime subscription/event loop ownership from `modules/tray` to `services/tray`,
  - `app` now subscribes via `crate::services::tray::subscription()`.
- `modules/tray` now acts as module state/model + activation backend helpers, while service layer owns integration loop wiring.

### Quality
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.57] - 2026-03-25

### Fixed
- Extended tray right-click fallback chain for SNI edge-cases:
  - `ContextMenu` -> `SecondaryActivate` -> `DefaultActivate`,
  - if all SNI activation methods fail and item has a menu path, execute `DBusMenu` root activation (`MenuItem` with `submenu_id=0`) as final fallback.
- This targets indicators that expose `com.canonical.dbusmenu` but do not implement `ContextMenu`/`SecondaryActivate`.

### Quality
- Added regression test for `menu_root_activate` fallback path after default activation failure.
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.56] - 2026-03-25

### Fixed
- Tray secondary-click now resolves full SNI address (`bus + object path`) via `org.kde.StatusNotifierWatcher.RegisteredStatusNotifierItems` before calling `ContextMenu`/`SecondaryActivate`.
- This fixes runtime cases where tray id is destination-only (e.g. `:1.533`) and direct calls to default `/StatusNotifierItem` fail with `Object does not exist`.
- Added targeted regression tests for destination extraction and watcher-address selection with custom object paths.

### Quality
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.55] - 2026-03-25

### Changed
- Debug UI controls are now shown only in debug mode:
  - `DBG` pill in the main bar is hidden unless `RUST_LOG` enables `debug`/`trace` for `thinkpadbar`,
  - `Observability` and `Runtime Contract` sections in `System Info` are hidden in non-debug runs.
- `ToggleDebugOverlay` now has a guard: in non-debug mode it does not enable overlay state.

### Quality
- Added regression test for `RUST_LOG` parsing used by debug-UI gating.
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.54] - 2026-03-25

### Fixed
- Adjusted per-item tray right-click pinning after runtime validation:
  - successful method is pinned only when the primary route succeeds,
  - fallback success no longer gets pinned as preferred action,
  - fallback/failure clears stale preference for the item.
- This prevents post-first-click degradation where a one-off fallback could lock subsequent clicks into a non-deterministic/ineffective path.

### Quality
- Updated regression test for preference lifecycle (`primary` pin + fallback/failure clear).
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.53] - 2026-03-25

### Fixed
- Closed `TD-TRAY-001`: tray right-click strategy is now deterministic per item.
- Secondary click route now uses adaptive per-item pinning:
  - initial route is capability-based (`item_is_menu`/`menu`),
  - after first successful click, the successful action is pinned for that item and reused on next clicks,
  - only one bounded fallback is allowed per click path (no fallback cycles).
- Extended tray diagnostics for every secondary click:
  - selected primary/fallback route,
  - previously pinned action,
  - cursor coordinates,
  - activation result and latency.

### Quality
- Added regression tests for:
  - preferred-action route selection,
  - single-fallback execution behavior,
  - successful-action pinning and failure cleanup in per-item cache,
  - mock/stub integration-style click execution path.
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.52] - 2026-03-25

### Added
- `v0.7` module-runtime contract draft in [src/modules/runtime.rs]:
  - lifecycle hooks (`on_start`, `on_stop`),
  - event handler (`on_event`),
  - command/event model for future module isolation.
- Debug overlay toggle in main bar (`DBG` pill) with live runtime counters.
- Extended System Monitor observability block:
  - workspace refresh requested/coalesced/latency,
  - D-Bus connect success/fail,
  - capability/runtime contract summary.

### Changed
- App state now tracks observability counters via explicit `PerfCounters`.
- Workspace refresh pipeline reports measured elapsed time into perf counters.

### Quality
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.51] - 2026-03-25

### Added
- `v0.7` foundation: built-in module capability schema draft in [src/modules/capabilities.rs].
- Runtime observability counters in app state:
  - workspace refresh requested/coalesced/completed,
  - workspace refresh last/avg latency (ms),
  - D-Bus connect attempts/success/fail.
- System Monitor popup now shows observability block and module capability summary.

### Changed
- Workspace refresh update pipeline now records end-to-end refresh latency for each `UpdateWorkspaces` task.

### Quality
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.50] - 2026-03-25

### Fixed
- Tray right-click (`TD-TRAY-001`) routing is now capability-aware:
  - `item_is_menu` / `menu` capabilities are preserved in local tray state,
  - menu-like items prefer `ContextMenu`, other items prefer `SecondaryActivate`.
- Right-click fallback is now deterministic by strategy:
  - each route has explicit primary/fallback order (`ContextMenu -> Secondary` or `Secondary -> ContextMenu`).
- Context-menu path now uses a cached session D-Bus connection and explicit timeout control.
- Cursor position fallback is stabilized:
  - parse failures no longer force `(0,0)` immediately; last known cursor coordinates are reused.
- Added per-click tray diagnostics (`tracing`) for route, capabilities, cursor coordinates, result, and latency.

### Quality
- Added tray regression tests for route selection and cursor position parsing.
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.49] - 2026-03-25

### Fixed
- Wi‑Fi connect flow now actually uses user-entered passphrase:
  - when D-Bus `Network.Connect` fails or no matching network is found, fallback `iwctl` connect now passes `--passphrase <...>` for secured networks.
- Removed immediate self-trigger D-Bus reconnect loop:
  - failed `zbus::Connection::system()` attempts no longer emit instant `TickSlow` recursion.
- Tray startup resilience improved:
  - replaced one-shot init + 1h sleep with bounded exponential retry (`1s .. 30s`) for `Client::new()` failures.
- Audio/mic interactive actions are now non-blocking on async runtime:
  - volume/mute mutations switched from sync `Command::output()` to async `tokio::process::Command`.

### Quality
- Added regression tests for Wi‑Fi `iwctl` connect argument construction (with/without passphrase).
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.48] - 2026-03-25

### Fixed
- Keyboard layout click switching restored with reliability-first path:
  - primary path uses `hyprctl switchxkblayout <device> next`,
  - Hyprland IPC `dispatch switchxkblayout ...` kept as fallback.
- This fixes the regression where IPC-only switching could report success but not actually change layout on some runtime setups.

### Quality
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.47] - 2026-03-25

### Fixed
- Keyboard layout click-switch regression fixed:
  - `switchxkblayout` dispatch is now treated as successful only when Hyprland returns explicit `ok`,
  - non-`ok` responses no longer short-circuit fallback targets, so switching continues through next candidates.
- This restores actual layout switching on click instead of only refreshing and re-showing current layout.

### Quality
- Added regression tests for dispatch success parsing (`ok`/error responses).
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.46] - 2026-03-24

### Changed
- Workspace refresh path now uses coalescing in app state:
  - only one `UpdateWorkspaces` task can be in-flight,
  - burst events are queued into one follow-up refresh instead of spawning overlapping tasks.
- Hyprland workspace event listener no longer sleeps per event after emit; refresh pacing is now handled by app-side coalescing.

### Optimized
- Reduced process spawning in interaction paths:
  - tray cursor position now reads via Hyprland IPC socket (`j/cursorpos`) instead of spawning `hyprctl`,
  - keyboard layout switching now uses Hyprland IPC dispatch directly instead of spawning `hyprctl`.
- Reduced string allocations in hot updates:
  - system module now rewrites metric strings in-place (`cpu/mem/swap/temp/net/disk`) instead of repeated `format!` allocations,
  - app module now updates `audio_str`, `battery_str`, and brightness percent string in-place and only on value changes where applicable.

### Quality
- Added regression test for workspace-refresh coalescing state transitions.
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.45] - 2026-03-24

### Fixed
- Tray right-click now prioritizes explicit D-Bus `org.kde.StatusNotifierItem.ContextMenu` on the target tray item.
- If `ContextMenu` is not available/fails, a single fallback to `SecondaryActivate` is used.
- This removes app-dependent right-click randomness where some items ignored `SecondaryActivate` for menu opening.

### Quality
- Added regression tests for tray activation-channel parsing and status-notifier address/path parsing.
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.44] - 2026-03-24

### Fixed
- Tray right-click path simplified to a single deterministic action:
  - only `SecondaryActivate` is used (with real cursor coordinates),
  - removed mixed `ContextMenu + retry` sequence that caused inconsistent/chaotic behavior.

### Quality
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.43] - 2026-03-24

### Fixed
- Tray right-click flow simplified for deterministic UX:
  - removed right-click workspace search/switch sequencing from app layer,
  - right click now directly triggers tray secondary/context action at current cursor position.
- This removes random right-click latency and cross-workspace sequencing jitter.

### Quality
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.42] - 2026-03-24

### Fixed
- Reduced right-click latency by removing unconditional delay: delay is now applied only when tray fallback actually switched to another workspace.
- Improved right-click reliability by retrying `SecondaryActivate` once when direct `ContextMenu` call is unavailable.
- Tray fallback now exposes detailed switch outcome (same workspace vs switched) so click sequencing can adapt without UX penalty.

### Quality
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.41] - 2026-03-24

### Fixed
- Tray interaction no longer warps cursor as part of generic fallback:
  - removed forced `focuswindow` step from tray fallback path.
- Tray activation/context menu now use actual current cursor coordinates (`hyprctl -j cursorpos`) instead of `(0, 0)`:
  - `Activate(Default/Secondary)` uses real pointer position,
  - right-click `ContextMenu` call uses real pointer position.
- This keeps pointer where user left it and opens context menus near the pointer instead of screen center.

### Quality
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.40] - 2026-03-24

### Fixed
- Tray app matching for click fallback is now more robust and app-agnostic:
  - matching normalizes punctuation/case in query, window class, and title;
  - tokenized matching improves detection when tray identifiers and window classes differ in format.
- Right-click tray flow now waits briefly after focus/switch fallback before requesting context menu, improving cross-workspace context-menu opening reliability.

### Quality
- Added regression test for normalized/tokenized tray-to-window matching.
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.39] - 2026-03-24

### Fixed
- Tray click sequencing adjusted for cross-workspace reliability:
  - Left click now first attempts generic focus/switch fallback, and only sends `Activate(Default)` if no window match is found.
  - Right click now first attempts generic focus/switch fallback, then sends context-menu activation.
- This avoids left-click minimize/restore toggling when a direct window focus path is available and improves behavior from other workspaces.

### Quality
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.38] - 2026-03-24

### Fixed
- Tray right-click now explicitly attempts `org.kde.StatusNotifierItem.ContextMenu` via user D-Bus (`busctl`) using parsed status-notifier address/path.
- If `ContextMenu` is not available, right-click falls back to `SecondaryActivate`.
- Left-click fallback remains generic and now reliably focuses matched windows after workspace switch.

### Quality
- Added regression test for status-notifier address/path parsing.
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.37] - 2026-03-24

### Fixed
- Tray left-click fallback is now stronger and app-agnostic:
  - after StatusNotifier `Activate(Default)`, fallback searches windows by normalized candidates and now both:
    - switches to the app workspace when needed,
    - focuses the matched window by Hyprland window address.
- This ensures visible reaction on left click even when tray activation handlers are inconsistent.

### Quality
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.36] - 2026-03-24

### Fixed
- Tray mouse behavior aligned with StatusNotifier expectations:
  - left click triggers `Activate(Default)`,
  - right click triggers `Activate(Secondary)` to open app context menus when supported.
- Left-click flow keeps generic workspace fallback switching to bring app windows into view if normal activation does not raise them.

### Quality
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.35] - 2026-03-24

### Changed
- Tray click interaction is now uniformly generic:
  - always attempts StatusNotifier `Activate`,
  - then runs a generic fallback search over normalized candidates from tray title/icon/id,
  - and switches to the workspace containing the matched app window when needed.

### Fixed
- Tray icon lookup now uses generalized icon-name normalization and themed subdirectory probing (`apps/panel/status` across common sizes), without app-specific hardcoding.

### Quality
- Added regression tests for generic tray candidate generation and icon-name normalization.
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

## [0.6.34] - 2026-03-24

### Fixed
- Tray icon resolution now normalizes icon names before lookup:
  - handles `file://` and quoted paths,
  - extracts file basename from paths,
  - tries icon name without `.svg/.png/.xpm` extension.
- This improves resolution for icon names like `godot-mono.svg` and reduces noisy missing-icon fallbacks.

### Quality
- Added regression tests for icon-name candidate normalization.
- Validation passed: `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

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
