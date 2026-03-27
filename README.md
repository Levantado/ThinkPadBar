# ThinkPadBar

A lightweight, high-performance status bar and control center for ThinkPads on Hyprland (Wayland), written in Rust.

## 🚀 Installation

### 1. Build from source
Ensure you have Rust and Cargo installed. Then run:
```bash
cargo build --release --locked
```

### 2. Install the binary
Recommended local install path:
```bash
./scripts/install-local.sh
```

If you prefer `cargo install`, keep it locked to the repository dependency graph:
```bash
cargo install --path . --locked --force
```

Do not use plain `cargo install --path .` here. It resolves a fresh dependency graph, which can pull newer incompatible git revisions of transitive dependencies like `cosmic-text` instead of the pinned versions in `Cargo.lock`.

### 3. Autostart in Hyprland
Add the following line to your `~/.config/hypr/hyprland.conf`:
```text
exec-once = thinkpadbar
```

You can preview the local installer without writing files:
```bash
./scripts/install-local.sh --dry-run
```

## 🗑️ Uninstallation

If you used the local installer:
```bash
./scripts/uninstall-local.sh
```

If you used `cargo install`:
```bash
cargo uninstall thinkpadbar
```

Don't forget to remove the `exec-once` line from your Hyprland configuration.

## 📈 Runtime Metrics

You can collect runtime CPU/RSS/threads/fd metrics with:

```bash
./scripts/runtime-metrics.sh
```

Examples:

```bash
./scripts/runtime-metrics.sh --pid 1576 --duration 120 --interval 2
./scripts/runtime-metrics.sh --output /tmp/thinkpadbar-idle.csv
```

Detailed notes: `docs/validation/runtime-metrics.md`.

## ⚙️ Performance Tuning

You can tune periodic refresh intervals in `~/.config/thinkpadbar/config.toml`:

```toml
[performance]
# profile: normal | low_power | high_responsiveness
profile = "normal"

# 0 means "use profile default"
tick_brightness_secs = 1
tick_thermal_secs = 2
tick_slow_secs = 10
```
