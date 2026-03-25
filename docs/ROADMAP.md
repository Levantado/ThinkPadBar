# ThinkPadBar Roadmap

## Scope
This roadmap defines the next evolution steps from a reliable single-product bar to a reusable Wayland platform, while preserving the project priorities:
- efficiency first,
- low runtime overhead,
- strict quality gate,
- predictable UX.

## Baseline (as of 2026-03-25)
- Current release line: `0.6.x`
- Validation gate: `fmt/check/clippy(-D warnings)/test` is mandatory
- Open high-priority tech debt: `TD-TRAY-001` (tray right-click stability)

## Milestone v0.7.0 — Product Hardening
Goal: improve product maturity and observability without architecture bloat.

### Deliverables
1. Stable internal module API
- Define module capability model (what a module can do, publish, and consume).
- Standardize module lifecycle hooks and event contracts.
- Eliminate ad-hoc cross-module calls where feasible.

2. Observability subsystem
- Structured logs for critical interaction paths (tray/wifi/bluetooth/workspace).
- Perf counters:
  - update frequency per module,
  - average and p95 handler latency,
  - popup open/close latency.
- Optional debug panel or compact debug overlay.

3. Declarative layout + theme layer
- Externalize visual tokens (spacing, radius, colors, typography).
- Support declarative layout blocks for top bar and popup sections.
- Keep backward compatibility with existing config defaults.

### Definition of Done
- No regressions in mandatory quality gate.
- Runtime diagnostics can explain at least 90% of interaction failures without code changes.
- Users can adjust theme/layout basics from config without recompilation.

### Target Metrics
- Idle CPU: `<= 0.7%` on target hardware profile.
- Idle RSS: `<= 35 MiB`.
- UI action latency (common controls): p95 `<= 120 ms`.

## Milestone v0.8.0 — Platform Preparation
Goal: prepare the codebase for ecosystem growth and distribution-grade operations.

### Deliverables
1. Plugin boundary (phase 1)
- Introduce clear internal ABI/trait boundary for built-in modules.
- Define safe data contracts for events, commands, and state snapshots.
- Add versioned compatibility markers for module interfaces.

2. Distribution-grade release contour
- Config migration pipeline between minor versions.
- Versioned changelog discipline + release checklist automation.
- Packaging strategy (e.g. cargo install + distro package docs + checksum artifacts).

3. Compatibility policy
- Document supported config keys and deprecation cycle.
- Add startup warnings for deprecated/invalid config with actionable guidance.

### Definition of Done
- New module can be integrated via module boundary without touching unrelated core code.
- Config migration works across at least two consecutive minor versions.
- Release process is reproducible from a single documented checklist.

### Target Metrics
- Backward config compatibility success rate: `100%` for supported versions.
- Upgrade failure rate in smoke tests: `0%`.

## Milestone v0.9.0 — Multi-Compositor Backend
Goal: abstract compositor-specific behavior while keeping performance predictable.

### Deliverables
1. Backend trait abstraction
- Introduce compositor backend trait for:
  - workspace model,
  - active window,
  - layout switching,
  - cursor position,
  - event stream.
- Keep Hyprland backend as reference implementation.

2. Additional compositor backends
- Implement at least one more backend (`Sway` or `Niri`) with parity for core features.
- Add backend selection/config mechanism.

3. Cross-backend validation
- Backend conformance tests for common behavior.
- Runtime fallback behavior for unsupported capabilities.

### Definition of Done
- Core bar behavior runs on at least two compositors.
- Missing backend capabilities degrade gracefully with explicit user-facing status.
- No critical increase in idle resource usage versus Hyprland-only baseline.

### Target Metrics
- Cross-backend core feature parity: `>= 85%`.
- Idle overhead delta vs Hyprland backend: `<= +15%` CPU/RAM.

## Cross-Milestone Constraints
1. Keep `Cargo.lock` managed only by cargo commands.
2. Treat warnings as errors.
3. Every bug fix includes regression coverage unless explicitly waived.
4. Version bump + changelog update required for release-level changes.

## Suggested Execution Order (short-term)
1. Close `TD-TRAY-001` using new observability hooks.
2. Implement module capability schema draft.
3. Introduce declarative theme tokens and map current style to them.
4. Add perf counters and expose a minimal debug panel.
