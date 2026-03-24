use std::fs;
use std::process::{Command, Stdio};
use tracing::{info, warn};

pub fn get_brightness() -> String {
    let mut percentage = 0;

    if let Ok(entries) = fs::read_dir("/sys/class/backlight") {
        for entry in entries.flatten() {
            let path = entry.path();
            let cur = fs::read_to_string(path.join("brightness"))
                .unwrap_or_default()
                .trim()
                .parse::<u32>()
                .unwrap_or(0);
            let max = fs::read_to_string(path.join("max_brightness"))
                .unwrap_or_default()
                .trim()
                .parse::<u32>()
                .unwrap_or(100);

            if max > 0 {
                percentage = (cur * 100) / max;
                break;
            }
        }
    }
    format!("{}%", percentage)
}

pub fn set_brightness(val: u32) {
    info!("Attempting to set brightness to {}%", val);
    let mut direct_write_ok = false;

    if let Ok(entries) = fs::read_dir("/sys/class/backlight") {
        for entry in entries.flatten() {
            let path = entry.path();
            let max = fs::read_to_string(path.join("max_brightness"))
                .unwrap_or_default()
                .trim()
                .parse::<u32>()
                .unwrap_or(0);

            if max > 0 {
                let target = (val * max) / 100;
                let b_path = path.join("brightness");

                if let Err(e) = fs::write(&b_path, target.to_string()) {
                    warn!("Direct brightness write failed for {:?}: {}", b_path, e);
                } else {
                    info!("Brightness set to {} ({}%) via direct write", target, val);
                    direct_write_ok = true;
                    break;
                }
            }
        }
    }

    if direct_write_ok {
        return;
    }

    // Non-interactive user-space fallbacks (no password prompts).
    let percent_arg = format!("{}%", val.clamp(1, 100));
    let brightnessctl_ok = Command::new("brightnessctl")
        .args(["-q", "s", &percent_arg])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if brightnessctl_ok {
        info!("Brightness set to {} via brightnessctl", percent_arg);
        return;
    }

    let light_ok = Command::new("light")
        .args(["-S", &val.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if light_ok {
        info!("Brightness set to {} via light", val);
    } else {
        warn!("Brightness update failed: direct write, brightnessctl and light backends unavailable or denied");
    }
}
