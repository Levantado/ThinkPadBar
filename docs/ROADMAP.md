# ThinkPadBar Roadmap

## Scope
This roadmap defines the next evolution steps from a reliable single-product bar to a reusable Wayland platform, while preserving the project priorities:
- efficiency first,
- low runtime overhead,
- strict quality gate,
- predictable UX.

## Baseline (as of 2026-04-10)
- **Current release: `1.0.69`**
- **Major Milestone Completed**: S8 Integrated MPRIS Media Module.
- **UI Maturity**: Unified popup scale, standardized pill alignment, Golden Ratio layouts.
- **Observability**: Perf counters and diagnostics for D-Bus, PipeWire, and Wayland runtimes.

## Milestone v1.1.0 — Customization & Hot Reload
Goal: Allow users to define the bar structure and appearance without recompilation.

### Deliverables
1. **User Visual Config (S7)**
- Move hardcoded bar segments into a declarative `config.toml` structure.
- Support `left`, `center`, `right` alignment groups for modules.
- Implement Hot Reload: apply theme and layout changes instantly on config save.

2. **Advanced Theme Engine**
- Externalize all visual tokens (colors, margins, radius) to a dedicated theme section.
- Support multiple built-in color profiles (Tokyo Night, Gruvbox, Nord).

3. **Dynamic Media Control**
- Auto-hide media pill when no players are active.
- Support for multiple active players with a switching mechanism in the popup.

### Definition of Done
- Users can reorder or disable any module (Wifi, Battery, Media) via config.
- Bar updates visual appearance within 500ms of config change without restart.
- No increase in idle resource usage due to config monitoring.

## Milestone v1.2.0 — Extended Ecosystem
Goal: Expand functionality to cover typical desktop environments needs.

### Deliverables
1. **Notification Center**
- Integrated D-Bus notification server (`org.freedesktop.Notifications`).
- History log in a dedicated popup.
- Actionable notifications (buttons, quick replies).

2. **Weather & Geolocation**
- Local weather module with auto-refresh based on IP or manual location.
- Support for multiple providers (OpenWeatherMap, PirateWeather).

3. **NetworkManager Backend**
- Full parity with IWD backend for users on standard distributions.
- VPN and Mobile Data (WWAN) status and control.

## Cross-Milestone Constraints
1. Keep `Cargo.lock` managed only by cargo commands.
2. Treat warnings as errors.
3. Every bug fix includes regression coverage.
4. Version bump + changelog update required for every release.

## Execution History
- **2026-04-10**: Released **1.0.69**. Finalized Media Module with Marquee, Seek, and Golden Ratio popup. Fixed visualizer freeze.
- **2026-04-09**: Released **1.0.62-1.0.66**. Global UI unification and hierarchical tray menu.
- **2026-03-22**: Initial core architecture and basic modules (Wifi, Battery, Power).
