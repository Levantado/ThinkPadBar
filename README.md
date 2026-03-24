# ThinkPadBar

A lightweight, high-performance status bar and control center for ThinkPads on Hyprland (Wayland), written in Rust.

## 🚀 Installation

### 1. Build from source
Ensure you have Rust and Cargo installed. Then run:
```bash
cargo build --release
```

### 2. Install the binary
You can copy the binary to your local bin directory:
```bash
mkdir -p ~/.local/bin
cp target/release/thinkpadbar ~/.local/bin/
```
*Alternatively, use cargo:*
```bash
cargo install --path .
```

### 3. Autostart in Hyprland
Add the following line to your `~/.config/hypr/hyprland.conf`:
```text
exec-once = thinkpadbar
```

## 🗑️ Uninstallation

If you copied it to `~/.local/bin`:
```bash
rm ~/.local/bin/thinkpadbar
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
