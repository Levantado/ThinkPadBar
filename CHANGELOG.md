# Changelog

All notable changes to this project are documented in this file.

## [1.0.17] - 2026-03-27

### Changed
- Deepened the dedicated `Bluetooth Devices` popup from a one-way bounded scan button to a clearer transient pairing flow:
  - added an explicit `Stop Scan` action wired through a typed `ControlsCommand::StopBluetoothScan`;
  - scan progress now carries a visible countdown (`Scanning (5s left)` down to `Finishing scan...`) instead of a flat busy state;
  - completion still highlights newly discovered devices, but the in-flight state is now explicit and interruptible.
- Polished `Audio Routes` popup detail so Bluetooth and USB paths carry meaningful runtime semantics instead of only origin labels:
  - each route now surfaces a typed `profile` badge such as `A2DP`, `HFP`, `USB`, or `ANALOG`;
  - route detail text now explains the likely path behavior (`Higher-latency media path`, `Low-latency external path`, etc.);
  - active route summaries at the top of the popup now include both profile and latency interpretation.

### Quality
- Added regression coverage for:
  - bounded Bluetooth stop-scan arguments;
  - scan countdown and finishing-status rendering;
  - audio route popup item profile/detail shaping;
  - current-route summaries surfacing profile and latency interpretation.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [1.0.16] - 2026-03-27

### Changed
- Deepened the dedicated `Bluetooth Devices` popup from a bare scan button to a clearer lightweight pairing flow:
  - added a typed UI scan state with `Idle`, `Scanning`, and completed scan results;
  - the popup now surfaces explicit `Scan Status` feedback while a bounded Bluetooth discovery run is in progress and after it completes;
  - newly discovered device addresses are tracked across the post-scan refresh and highlighted with a `NEW` badge in the popup cards.
- Polished `Audio Routes` popup organization and active-device clarity:
  - route rows are now grouped by route family (`BT`, `USB`, `INTERNAL`, etc.) instead of a flat mixed list;
  - active route summaries are now framed as `Active Output Device` / `Active Input Device`, which reads better for Bluetooth headsets vs internal speakers/microphones;
  - route rows keep the new icon/status model and now present family grouping without adding polling or runtime overhead.

### Quality
- Added regression coverage for:
  - route-family grouping preserving stable order;
  - Bluetooth scan state transitions from `Scanning` to completed results;
  - `NEW` device emphasis derived from post-scan Bluetooth refresh data.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [1.0.15] - 2026-03-27

### Changed
- Deepened the dedicated `Bluetooth Devices` popup from managing only already-known devices to supporting lightweight discovery flow:
  - added a bounded `Scan 5s` action wired through a typed `ControlsCommand::ScanBluetoothDevices`;
  - the Bluetooth backend now runs a time-bounded `bluetoothctl --timeout 5 scan on` path and refreshes the typed device summary afterward;
  - this keeps pairing on the same dedicated surface without introducing a new background scan state machine.
- Polished `Audio Routes` popup readability so routes are distinguishable at a glance:
  - each route now surfaces an explicit origin icon for Bluetooth, USB, internal, HDMI/display, virtual, and unknown paths;
  - each route row now carries an explicit status label (`ACTIVE`, `AVAILABLE`, `UNAVAILABLE`) instead of relying only on the old default/unavailable badges;
  - route summaries remain typed and now combine with the origin icon/status model for clearer output and input selection.

### Quality
- Added regression coverage for:
  - bounded Bluetooth scan command arguments;
  - controls command routing for `ScanBluetoothDevices`;
  - route origin icon/status mapping in popup item generation.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [1.0.14] - 2026-03-27

### Changed
- Deepened the dedicated `Bluetooth Devices` popup from simple connect/disconnect controls to fuller device management:
  - device cards now surface `PAIRED` and `TRUSTED` state alongside the existing connection and battery badges;
  - per-device `Pair`, `Trust`, and `Remove` actions were added through typed `ControlsCommand` paths and the existing Bluetooth backend;
  - the local controls snapshot now previews Bluetooth trust/pair state and device removal before the backend refresh lands.
- Made the `Audio Routes` popup more informative instead of just listing raw route names:
  - current default output and input summaries are surfaced at the top of the popup;
  - each route now carries an explicit origin/type badge (`BT`, `USB`, `INTERNAL`, `HDMI`, `VIRTUAL`, `UNKNOWN`) in addition to its `SINK`/`SOURCE` capability badge;
  - route detail text is now typed from route origin classification instead of a generic availability label.

### Quality
- Added regression coverage for:
  - Bluetooth device info parsing with `Paired` and `Trusted` state;
  - audio route origin classification from `wpctl status`;
  - audio current-route summaries preferring typed route origin detail;
  - updated popup-item and device-card expectations for the richer UI state.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [1.0.13] - 2026-03-27

### Changed
- Deepened the daily-use device UX with a dedicated `Bluetooth Devices` popup:
  - control-center now exposes a focused device-management entry point instead of forcing all per-device actions into inline cards;
  - the popup surfaces adapter state, per-device address, connection state, battery badges, audio-profile details, and direct `Connect` / `Disconnect` actions;
  - `Bluetooth` refreshes are now requested when opening the dedicated popup so device state is not stale.
- Polished audio-route presentation around capability and availability:
  - the `Audio Routes` popup now labels entries explicitly as `SINK` or `SOURCE`;
  - missing output/input capabilities now render as explicit `UNAVAILABLE` rows instead of silently disabling the feature path;
  - the control-center route action now reflects route availability shape (`Audio Routes`, `Partial Routes`, `Routes Unavailable`) instead of a single generic label.

### Quality
- Added regression coverage for:
  - unavailable audio-route popup rows and capability labeling;
  - route-action labeling based on discovered output/input availability;
  - dedicated Bluetooth popup refresh behavior;
  - richer Bluetooth device card summaries with address and badges.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [1.0.12] - 2026-03-27

### Changed
- Replaced the last coarse audio-route UX with an explicit `Audio Routes` popup:
  - control-center now opens a dedicated route picker instead of cycling blindly through sinks and sources;
  - output and input routes are listed explicitly with `DEFAULT` badges and stable route ids;
  - selecting a route now drives a deterministic `wpctl set-default` path through `ControlsService`.
- Deepened the Bluetooth device surface from passive summaries to per-device actions:
  - the Bluetooth backend now inventories known devices from `bluetoothctl devices`, enriches them with `Connected`, battery, and audio-profile info from `bluetoothctl info`, and carries that typed state in `ControlsSnapshot`;
  - control-center Bluetooth cards now show connection state explicitly and expose `Connect` / `Disconnect` actions per device.
- Removed the now-dead route-cycling command path from the UI/controls command surface, keeping the product behavior aligned with the explicit popup model.

### Quality
- Added regression coverage for:
  - audio route popup item labeling and current-route selection;
  - Bluetooth device cards surfacing connection state, battery, and profiles;
  - `ControlsService` preview/dispatch for explicit route selection and Bluetooth connect/disconnect actions;
  - dedicated popup anchoring for the new audio-routes surface.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [1.0.11] - 2026-03-27

### Changed
- Made `Audio & Devices` interactive instead of read-only:
  - added typed audio route inventories to `ControlsSnapshot`;
  - added `Next Output` and `Next Input` actions in the control-center popup;
  - wired route switching through `wpctl set-default`, with local preview and post-command refresh.
- Deepened the Bluetooth daily-use surface from adapter state to connected device quality:
  - added typed connected-device details with address, optional battery percentage, and normalized audio profile badges;
  - enriched the control-center popup with per-device Bluetooth cards instead of only a flat device-name list.

### Quality
- Added regression coverage for:
  - `wpctl status` route inventory parsing and route cycling preview/dispatch;
  - `bluetoothctl info` parsing for battery and audio profiles;
  - control-center Bluetooth device cards surfacing battery/profile details;
  - `ControlsService` command routing for new audio route actions.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [1.0.10] - 2026-03-27

### Changed
- Deepened the `Audio & Devices` surface in the control-center popup from simple adapter state to typed device summaries:
  - audio now surfaces default output and input route names parsed from `wpctl status` alongside the existing mute/volume state;
  - Bluetooth now surfaces connected device names parsed from `bluetoothctl devices Connected` instead of showing only the adapter power state.
- Updated control-center refresh behavior so opening the popup refreshes `Bluetooth` alongside `AudioMic` and `BatteryPower`, keeping the daily-use device surface fresh.

### Quality
- Added regression coverage for:
  - `wpctl status` default route parsing;
  - `bluetoothctl devices Connected` parsing;
  - control-center device cards surfacing route/device details;
  - `ControlsService` carrying audio route and connected Bluetooth devices through refreshes;
  - control-center open behavior now requesting Bluetooth refresh too.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [1.0.9] - 2026-03-27

### Changed
- Deepened the `ThinkPad product depth` track in the control-center popup:
  - added a new `Audio & Devices` surface that exposes daily-use speaker, microphone, and Bluetooth adapter state as compact cards instead of forcing everything into top-row toggles;
  - added direct action buttons for output mute, microphone mute, and launching Overskride from the same surface.
- Made the `Displays` popup easier to scan:
  - replaced plain per-output detail strings with output cards that emphasize status through badges (`INTERNAL/EXTERNAL`, resolution, refresh, scale);
  - kept the existing hotplug-aware summary rows and quick navigation actions while making per-output state visually denser and faster to parse.
- Removed the now-redundant standalone `Overskride` button row from the control-center popup after folding that action into the new device surface.

### Quality
- Added regression coverage for:
  - control-center audio/microphone/Bluetooth device summaries;
  - display popup output cards and status badges.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [1.0.8] - 2026-03-27

### Changed
- Made the `ThinkPad Power` section in the control-center popup interactive:
  - added typed battery care presets (`CARE 40-80`, `BAL 60-90`, `FULL 0-100`) wired through `ControlsService` and `PowerBackend` instead of ad-hoc shell UI logic;
  - added a dedicated `BatteryPower` refresh path so threshold writes re-read battery and power-profile state immediately instead of waiting for a generic slow refresh;
  - refreshed both `AudioMic` and `BatteryPower` when opening the control center so the new daily-use power surface does not rely on stale slow-tick data.
- Expanded the `Displays` popup into a more useful daily surface:
  - replaced the single `Back` button with a quick action bar for `Controls`, `System Info`, and `Close`;
  - kept the hotplug-aware output detail cards and summary rows intact while making display navigation faster.
- Extended the platform power backend:
  - it now reports threshold-file support in diagnostics;
  - it now writes charge thresholds directly to sysfs and falls back to `pkexec` only when direct writes fail.

### Quality
- Added regression coverage for:
  - battery-threshold preset preview and backend execution routing;
  - threshold write script generation and power-runtime diagnostics;
  - active preset detection in the control center;
  - display popup navigation actions;
  - control-center open behavior requesting both audio and battery-power refreshes.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [1.0.7] - 2026-03-27

### Changed
- Started the `ThinkPad product depth` track with a new `ThinkPad Power` section in the control-center popup:
  - surfaces daily-use battery runtime, charge state, AC state, charge/draw power, and charge thresholds directly in the quick popup instead of keeping them only in `System Info`;
  - reuses the existing typed battery summaries, so no new backend or polling path was introduced.

### Quality
- Added regression coverage for the new control-center power items.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [1.0.6] - 2026-03-27

### Changed
- Extended `scripts/perf-smoke.sh` with baseline-aware profiles:
  - `--profile-file FILE` now loads key=value thresholds and baseline values from disk;
  - `--write-profile FILE` now writes the current measured run back as a reusable baseline profile;
  - baseline checks now enforce growth budgets for max RSS, average CPU, and max thread count.
- Added `docs/validation/perf-smoke.profile.example` and documented baseline workflows in `README.md` and `docs/validation/runtime-metrics.md`.

### Quality
- Added shell smoke coverage for profile-aware perf smoke flows:
  - `bash -n scripts/perf-smoke.sh`
  - dry-run validation for `--profile-file` and `--write-profile`
  - live profile write smoke using a short-lived process
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [1.0.5] - 2026-03-27

### Changed
- Added `scripts/perf-smoke.sh` as the first perf-tooling slice after the local deployment workflow:
  - launches `target/release/thinkpadbar` by default or uses `--installed`;
  - can measure an already-running PID with `--pid`;
  - reuses `scripts/runtime-metrics.sh` to collect CSV samples;
  - fails on simple max RSS / average CPU / max thread thresholds;
  - supports `--dry-run` for shell smoke validation.
- Documented the new perf smoke workflow in `README.md` and `docs/validation/runtime-metrics.md`.

### Quality
- Added shell smoke coverage via `bash -n scripts/perf-smoke.sh` and dry-run validation for both default and `--installed` flows.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [1.0.4] - 2026-03-27

### Changed
- Added a locked local installer workflow:
  - new `scripts/install-local.sh` builds ThinkPadBar with `cargo build --locked`, installs the built binary into a local bin directory, and supports `--dry-run`;
  - new `scripts/uninstall-local.sh` removes a locally installed binary and supports `--dry-run`.
- Started a small post-1.0 deployment ergonomics workstream by replacing manual README install/uninstall steps with the new helper scripts while keeping `cargo install --path . --locked --force` documented as the cargo-based alternative.

### Quality
- Added shell smoke coverage via `bash -n scripts/install-local.sh scripts/uninstall-local.sh` and dry-run validation for both scripts.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [1.0.3] - 2026-03-27

### Changed
- Fixed the local install workflow documentation for pinned git dependencies:
  - the recommended source build path now uses `cargo build --release --locked`;
  - the recommended local install path now copies the already-built binary with `install -Dm755 target/release/thinkpadbar ~/.local/bin/thinkpadbar`;
  - `cargo install` is now documented only as `cargo install --path . --locked --force`.
- Documented why plain `cargo install --path .` is unsafe for this project: it can resolve a fresh dependency graph and pull newer incompatible git revisions of transitive dependencies like `cosmic-text` instead of the versions pinned in `Cargo.lock`.

### Quality
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [1.0.2] - 2026-03-27

### Changed
- Reduced the heavy glyph path in `System Info` and debug observability surfaces:
  - replaced real emoji row markers with compact ASCII/Nerd-font-safe tags in `System Info`, display summaries, and warning rows;
  - kept the same information architecture while avoiding the most likely color-emoji/font-fallback allocation path that appears when opening the popup.

### Quality
- Added regression coverage to keep display summary rows on compact tags instead of emoji markers.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [1.0.1] - 2026-03-27

### Changed
- Continued `P7 Native Wayland Polish v2` after `1.0.0` with a final fast-access display surface:
  - the bar now exposes a hotplug-aware display pill that reflects the current `Display Mode` and opens the dedicated `Displays` popup directly;
  - the `Displays` popup now acts as a proper user-facing Wayland surface with detailed per-output mode, refresh, scale, and internal/external classification instead of only summary rows in existing popups.

### Quality
- Added regression coverage for:
  - display pill summary derivation,
  - popup output detail formatting.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [1.0.0] - 2026-03-27

### Changed
- Declared ThinkPadBar `1.0.0` to mark the end of the foundational migration phase and the start of a stable production-focused release line for the current Hyprland/ThinkPad scope.
- Continued `P7 Native Wayland Polish v2` with both requested next moves:
  - added a dedicated `Displays` popup with hotplug-aware output details, capability state, and a direct `Back` path into the control center;
  - added a new protocol-backed `Display Mode` feature that classifies current output state as `Laptop`, `Docked`, `Hybrid`, `Headless`, or `Wayland unavailable`;
  - the control-center `Displays` card now includes an explicit `Open` action instead of only showing passive summary rows.

### Quality
- Added regression coverage for:
  - display popup output detail formatting,
  - dedicated display popup surface policy,
  - display mode topology classification.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [0.6.102] - 2026-03-27

### Changed
- Continued `P7 Native Wayland Polish v2` with a compact, hotplug-aware display surface outside of `System Info`:
  - regular `System Info` now adds a derived `Display Mode` row (`Laptop`, `Docked`, `Hybrid`, `Headless`) on top of raw output listings;
  - the control-center popup now renders a dedicated `Displays` card with live `Display Mode`, `Display Topology`, `Display Scale`, and `Display Outputs` summaries sourced from `WaylandRuntimeService`;
  - display interpretation is now reused through shared app-level summary rows instead of being spread across multiple one-off render paths.

### Quality
- Added regression coverage for:
  - derived display mode classification across common output topologies,
  - app-level display summary row composition for hotplug-aware output state.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [0.6.101] - 2026-03-27

### Changed
- Continued `P7 Native Wayland Polish v2` with an event-driven output path and another user-visible Wayland feature:
  - `WaylandRuntimeService` now exposes a native `Subscription` and applies hotplug-aware snapshot updates from the Wayland socket instead of relying only on startup snapshots;
  - regular `System Info` now shows a dedicated `Display Scale` row derived from `wl_output` scale events.
- The Wayland output model is now rich enough to carry current mode, refresh, and scale while remaining event-driven and low-overhead.

### Quality
- Added regression coverage for:
  - per-output scale summaries,
  - app-level application of `WaylandRuntimeEvent::SnapshotUpdated`.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [0.6.100] - 2026-03-27

### Changed
- Continued `P7 Native Wayland Polish v2` by turning raw `wl_output` discovery into richer output interpretation:
  - `WaylandRuntimeService` now tracks current output mode and scale when `wl_output` advertises them;
  - regular `System Info` now shows `Display Topology` in addition to `Display Outputs`;
  - debug `System Info` now exposes `Wayland Outputs Detail` with connector/mode/scale details.

### Quality
- Added regression coverage for:
  - internal vs external output topology summaries,
  - detailed output formatting including mode and scale.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [0.6.99] - 2026-03-27

### Changed
- Continued `P7 Native Wayland Polish v2` in both planned directions:
  - added a dedicated `WaylandRuntimeService` that snapshots registry capabilities and outputs from a native Wayland connection without introducing a background loop;
  - `System Info` now exposes a user-visible `Display Outputs` row driven by `wl_output`;
  - debug `System Info` now uses the new service for `Wayland Runtime`, `Wayland Capabilities`, `Wayland Outputs`, and `Wayland Missing Caps`, expanding the capability surface beyond the idle-inhibit path alone.

### Quality
- Added regression coverage for:
  - named output summaries,
  - missing-capability reporting across compositor/shm/output/xdg/layer-shell/idle-inhibit protocols.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [0.6.98] - 2026-03-27

### Changed
- Continued `P7 Native Wayland Polish v2` with a clearer capability matrix in debug `System Info`:
  - added a dedicated `Wayland Capabilities` line that reports the state of `wl_compositor`, `zwp_idle_inhibit_manager_v1`, and `wl_surface`;
  - added an explicit `Wayland Missing Caps` warning row when one or more required protocol capabilities are absent.

### Quality
- Added regression coverage for:
  - capability-matrix formatting,
  - missing-capability reporting with explicit protocol names.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [0.6.97] - 2026-03-27

### Changed
- Finished a last focused `P6` polish by making common battery-threshold policies read naturally in `System Info`:
  - `0% -> 100%` is now summarized as `Full charge allowed`;
  - equal start/end thresholds are summarized as `Pinned at X%`.
- Started `P7 Native Wayland Polish v2` with a first capability-oriented diagnostics slice:
  - debug `System Info` now shows a dedicated `Wayland Runtime` line derived from native idle-inhibitor protocol state;
  - when Wayland capability initialization is unavailable, the debug view now surfaces an explicit `Wayland Unavailable` warning row instead of burying the reason inside a longer runtime summary.

### Quality
- Extended regression coverage for:
  - polished threshold-summary rendering,
  - protocol-version `Wayland Runtime` summaries.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [0.6.96] - 2026-03-27

### Changed
- Continued `P6 ThinkPad Hardware Refinement v2` with threshold-aware battery interpretation in `System Info`:
  - added a dedicated `Charge State` row that explains whether the pack is charging, discharging, holding at the ceiling, or idling inside the configured threshold window;
  - the new summary uses existing AC state, battery status, capacity, and charge-threshold data instead of adding more raw metrics.

### Quality
- Extended regression coverage for:
  - threshold-aware `Charge State` interpretation,
  - the expanded hardware row set including the new `Charge State` line.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [0.6.95] - 2026-03-27

### Changed
- Continued `P6 ThinkPad Hardware Refinement v2` with two ThinkPad-relevant battery details in `System Info`:
  - `BatteryInfo` now carries live pack voltage and optional `charge_control_{start,end}_threshold` values from `/sys/class/power_supply`;
  - `ThinkPad Hardware` now shows `Pack Voltage` and `Charge Thresholds`, so the battery block exposes both current electrical state and threshold policy without extra shell commands.

### Quality
- Extended regression coverage for:
  - deriving pack voltage and charge thresholds from battery readings,
  - hardware-summary formatting for the new `Pack Voltage` and `Charge Thresholds` rows.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [0.6.94] - 2026-03-27

### Changed
- Continued `P6 ThinkPad Hardware Refinement v2` with more actionable battery-health summaries in `System Info`:
  - added a dedicated `Battery Wear` row derived from battery health and, when available, the absolute lost pack capacity in Wh;
  - `Pack Capacity` now reports both current/design capacity and the absolute Wh loss when design capacity is known;
  - `Cycle Count` is now rendered as a human-readable `"N cycles"` summary instead of a raw integer.

### Quality
- Extended regression coverage for:
  - battery wear formatting,
  - pack-capacity loss formatting,
  - expanded hardware row set including the new `Battery Wear` line.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [0.6.93] - 2026-03-27

### Changed
- Refined the `System Info` / `ThinkPad Hardware` section after the first scrollable fix:
  - reserved an explicit right gutter inside the scrollable content so the vertical scrollbar no longer overlays value text;
  - continued `P6 ThinkPad Hardware Refinement v2` by extending `BatteryInfo` with cycle count plus full/design pack capacity and deriving those values from either `energy_*` fields or `charge_* x voltage`;
  - `ThinkPad Hardware` now shows `Pack Capacity` and `Cycle Count` in addition to the earlier AC/health/power/profile/fan/thermal lines.

### Quality
- Added regression coverage for:
  - deriving pack capacity from `charge_*` plus voltage,
  - hardware summary rows including the full expanded battery section,
  - battery detail summaries covering pack-capacity and cycle-count formatting.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [0.6.92] - 2026-03-27

### Changed
- Fixed a `System Info` layout regression introduced by the growing diagnostics/hardware sections:
  - the `System Info` popup is now wrapped in `scrollable(...)`, so lower sections such as `ThinkPad Hardware` are reachable instead of being clipped by the popup height;
  - hardware rows are now built through a dedicated helper, keeping the model stable as the section grows.

### Quality
- Added a regression test that asserts the complete `ThinkPad Hardware` row set is present and non-empty in the view model helper.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [0.6.91] - 2026-03-27

### Changed
- Hardened the remaining tray/icon memory-growth suspect without changing behavior:
  - `TrayItem` now stores a stable signature for the currently selected tray pixmap;
  - repeated `UpdateEvent::Icon` payloads with an identical best pixmap no longer rebuild a fresh `iced::image::Handle`;
  - fallback name/title resolution still runs when a pixmap disappears, but identical pixmap updates are now ignored.

### Quality
- Added regression tests for:
  - stable tray pixmap signatures,
  - deduplication of identical pixmap updates,
  - signature replacement when the underlying pixmap actually changes.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [0.6.90] - 2026-03-26

### Changed
- Finished the remaining practical `P5 Observability v2` gap by adding power-path diagnostics:
  - `PlatformProfilePowerBackend` now reports whether `tlp` is active and whether `/sys/firmware/acpi/platform_profile` is available;
  - `ControlsDiagnostics` now exposes `power_runtime`, and debug `System Info` shows a dedicated `Power Runtime` line.
- Started `P6 ThinkPad Hardware Refinement v2` with a richer hardware model and user-facing summaries:
  - `BatteryInfo` now carries AC adapter state, battery health versus design capacity, and charge/discharge power rate;
  - battery collection now resolves the first real `Battery` and `Mains` devices from `/sys/class/power_supply` instead of assuming a fixed `BAT0` path for all fields;
  - `ThinkPad Hardware` in `System Info` now shows `AC Adapter`, `Battery Health`, `Charge / Draw Power`, and `Thermal State` in addition to the previous runtime lines.

### Quality
- Added regression tests for:
  - battery health/runtime/power derivation and current+voltage power fallback,
  - power runtime diagnostics summary,
  - actionable battery detail summaries and thermal-state interpretation in the UI helpers.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [0.6.89] - 2026-03-26

### Changed
- Continued `P5 Observability v2` with runtime diagnostics for the remaining orchestration-heavy paths:
  - `CompositorService` now exposes structured runtime diagnostics with configured/runtime backend, refresh inflight/queued state, last refresh latency, and explicit backend-fallback reason;
  - `SystemInfoService` now tracks lightweight runtime diagnostics for the last refresh kind and whether the thermal sensor path is currently available;
  - app-level coalescing now exposes a dedicated runtime summary for pending slider flushes, pending slow ticks, control refresh queues, and background request queues.
- Debug `System Info` now includes dedicated `Compositor Runtime`, `Coalescing Runtime`, and `System Runtime` lines, plus a conditional `Compositor Unavailable` warning row when configured/runtime compositor backends diverge.

### Quality
- Added regression tests for:
  - compositor diagnostics refresh-state and last-latency reporting,
  - system-info diagnostics summary formatting,
  - app-level coalescing diagnostics reporting and summary stability.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [0.6.88] - 2026-03-26

### Changed
- Fixed the first concrete tray/icon memory-growth path:
  - `IconResolver` now uses a bounded cache (`256` entries) instead of an unbounded `HashMap`, so long-running icon lookups cannot grow process memory indefinitely;
  - tray title-based icon fallback now uses a `StoreHitsOnly` policy, which means volatile title misses no longer accumulate in the negative cache;
  - tray integration keeps normal cached `icon_name` lookups, but avoids persistent negative caching for changing title strings such as unread counters or track names.

### Quality
- Added regression tests for:
  - bounded icon resolver cache size,
  - title-hint misses not growing the negative cache,
  - tray title-only icon resolution not increasing resolver negative cache size.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [0.6.87] - 2026-03-26

### Changed
- Advanced `P5 Observability v2` across the remaining runtime-heavy domains:
  - `NetworkService` now exposes structured runtime diagnostics with configured/runtime backend, last fallback path, backend-unavailable reason, and last surfaced network error;
  - tray runtime now persists secondary-click/menu activation observations instead of only logging them, so debug `System Info` can show the last route, result, dispatch failure, and menu activation error;
  - `IdleInhibitorService` now reports explicit unavailable reasons in its runtime summary instead of a silent `N/A`.
- Debug `System Info` now includes dedicated `Network Runtime` and `Tray Runtime` lines plus conditional warning rows for network unavailability, network last error, tray dispatch failures, and tray menu activation errors.

### Quality
- Added regression tests for:
  - network runtime fallback/error diagnostics,
  - tray runtime diagnostics persistence and error clearing,
  - idle-inhibitor unavailable-reason reporting.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [0.6.86] - 2026-03-26

### Changed
- Deepened `P4 Audio Backend v2 (PipeWire Events)` and started the first practical `P5 Observability v2` slice:
  - the PipeWire audio runtime now reacts not only to route/property param events, but also to tracked audio node add/remove and relevant node info changes, so external device/default-audio transitions are picked up more reliably;
  - `WpctlAudioBackend` now keeps live listener diagnostics: running/stopped state, tracked audio node/metadata counts, event count, reconnect count, last event, and last error;
  - debug `System Info` now shows a dedicated `Audio Runtime` line sourced from backend diagnostics instead of exposing only the coarse backend name.

### Quality
- Added regression tests for:
  - node-info change-mask filtering,
  - deterministic audio event labels,
  - audio runtime diagnostics summary formatting,
  - controls diagnostics exposing the optional audio runtime summary.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [0.6.85] - 2026-03-26

### Changed
- Started `P4 Audio Backend v2 (PipeWire Events)` by replacing the `pactl subscribe` text-stream listener with a native `pipewire-rs` event runtime:
  - `WpctlAudioBackend` still uses `wpctl` for volume and mute commands, but now subscribes to PipeWire `default` metadata updates and audio device node parameter changes through a dedicated PipeWire main loop thread;
  - the listener filters to `Audio/Sink`, `Audio/Source`, and `Audio/Duplex` nodes plus `default.audio.sink` / `default.audio.source` metadata keys, so controls refreshes stay event-driven without subscribing to unrelated graph noise;
  - controls diagnostics now report the audio backend as `wpctl+pipewire` instead of `wpctl+pactl`.

### Quality
- Added regression tests for:
  - audio-node class filtering,
  - default metadata filtering,
  - audio metadata property filtering,
  - audio param filtering,
  - backend naming for the new PipeWire event runtime.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [0.6.84] - 2026-03-26

### Changed
- Extended `P3 Icon Resolver` with `.desktop` metadata and app-id alias lookup:
  - `IconResolver` now builds a lazy alias index from XDG and Flatpak `applications/` directories and can resolve icons through `.desktop` `Icon=`, `Name=`, `StartupWMClass`, desktop filename, and `Exec` basename aliases;
  - tray icon resolution now falls back to desktop/title aliases when a tray item has no usable `icon_name`, which improves hit rate for items that only expose a title or WM/app id;
  - icon-resolver diagnostics now track desktop entry count, desktop alias count, alias-hit count, and the last alias mapping used during icon resolution.

### Quality
- Added regression tests for:
  - desktop alias candidate generation priority,
  - `.desktop` alias lookup by app id and exec name,
  - `.desktop` alias lookup by title name,
  - tray runtime resolving an icon from title when `icon_name` is missing.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [0.6.83] - 2026-03-26

### Changed
- Extended `P3 Icon Resolver` with runtime diagnostics and better tray fallbacks:
  - `IconResolver` now reports cache/negative-cache counts, cache hit/miss totals, and last resolution outcome so icon lookup can be diagnosed from the app debug surface without enabling extra logs;
  - tray runtime now exposes `TrayDiagnostics`, including resolved/unresolved icon counts and the last unresolved tray item label;
  - unresolved tray items now render a deterministic fallback glyph derived from title or icon name, instead of falling back to raw `icon_name[0]` or `?`.

### Quality
- Added regression tests for:
  - icon-resolver diagnostics counters and last-result tracking,
  - tray diagnostics counting resolved vs unresolved icons,
  - deterministic fallback-label selection for tray items.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [0.6.82] - 2026-03-26

### Changed
- Started `P3 Icon Resolver` by extracting tray icon lookup into a dedicated service utility:
  - added `services/icon_resolver.rs` with a small cached `IconResolver` that resolves direct file icons, theme-name hints, and absolute theme roots with inherited theme fallback;
  - moved tray icon candidate expansion, theme search roots, themed icon paths, and pixmap fallback search out of `tray_model.rs`;
  - switched tray runtime to use the shared resolver instead of ad-hoc per-file lookup logic, preserving the existing tray UI but making icon resolution a reusable service boundary.

### Quality
- Added regression tests for:
  - theme-name root discovery across XDG and legacy icon locations,
  - inherited icon lookup from an absolute theme root via `index.theme`,
  - negative-cache stability for missing icons.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

## [0.6.81] - 2026-03-26

### Changed
- Completed the remaining `P2 Coalescing Foundation` slow/background slice:
  - added a `clear(...)` path to `RequestCoalescer<K>` so successful background requests can explicitly discard queued duplicates instead of keeping stale inflight state;
  - coalesced `TickSlow` behind a short delayed flush, so repeated slow ticks now collapse to the latest pending generation before running thermal/slow system-info refreshes;
  - coalesced background D-Bus reconnect attempts and background Wi-Fi info sync requests, so the bar no longer spawns parallel duplicate slow-path tasks when system bus availability or periodic network syncs flap.

### Quality
- Added regression tests for:
  - request-coalescer state clearing,
  - coalesced D-Bus reconnect retries after failure,
  - slow-tick reuse of a single background connect request when system bus is unavailable.
- Validation passed: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.

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
