use std::fs;
use std::process::{Command, Stdio};
use tracing::{info, warn};

#[derive(Debug, Default, Clone, Copy)]
pub struct SysfsBrightnessBackend;

impl super::BrightnessBackend for SysfsBrightnessBackend {
    fn backend_name(&self) -> &'static str {
        "sysfs+brightnessctl+light"
    }

    fn capability_mode(&self) -> crate::services::capabilities::CapabilityMode {
        crate::services::capabilities::CapabilityMode::Fallback
    }

    fn snapshot(&self) -> crate::services::controls::BrightnessSnapshot {
        crate::services::controls::BrightnessSnapshot::from_percent(
            read_backlight_percent().unwrap_or(0),
        )
    }

    fn set_brightness(&self, percent: u32) {
        let percent = percent.clamp(0, 100);
        info!("Attempting to set brightness to {}%", percent);

        if write_backlight_percent(percent) {
            return;
        }

        let percent_arg = format!("{}%", percent.clamp(1, 100));
        let brightnessctl_ok = Command::new("brightnessctl")
            .args(["-q", "s", &percent_arg])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
        if brightnessctl_ok {
            info!("Brightness set to {} via brightnessctl", percent_arg);
            return;
        }

        let light_ok = Command::new("light")
            .args(["-S", &percent.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
        if light_ok {
            info!("Brightness set to {} via light", percent);
        } else {
            warn!("Brightness update failed: direct write, brightnessctl and light backends unavailable or denied");
        }
    }
}

fn read_backlight_percent() -> Option<u32> {
    let entries = fs::read_dir("/sys/class/backlight").ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        let current = fs::read_to_string(path.join("brightness"))
            .ok()?
            .trim()
            .parse::<u32>()
            .ok()?;
        let max = fs::read_to_string(path.join("max_brightness"))
            .ok()?
            .trim()
            .parse::<u32>()
            .ok()?;
        if max > 0 {
            return Some(percent_from_raw(current, max));
        }
    }
    None
}

fn write_backlight_percent(percent: u32) -> bool {
    let Ok(entries) = fs::read_dir("/sys/class/backlight") else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let max = fs::read_to_string(path.join("max_brightness"))
            .ok()
            .and_then(|value| value.trim().parse::<u32>().ok())
            .unwrap_or(0);
        if max == 0 {
            continue;
        }
        let target = raw_from_percent(percent, max);
        let brightness_path = path.join("brightness");
        match fs::write(&brightness_path, target.to_string()) {
            Ok(_) => {
                info!(
                    "Brightness set to {} ({}%) via direct write",
                    target, percent
                );
                return true;
            }
            Err(error) => {
                warn!(
                    "Direct brightness write failed for {:?}: {}",
                    brightness_path, error
                );
            }
        }
    }
    false
}

pub(crate) fn percent_from_raw(current: u32, max: u32) -> u32 {
    if max == 0 {
        return 0;
    }
    (current.saturating_mul(100)) / max
}

pub(crate) fn raw_from_percent(percent: u32, max: u32) -> u32 {
    percent.clamp(0, 100).saturating_mul(max) / 100
}

#[cfg(test)]
mod tests {
    use super::{percent_from_raw, raw_from_percent};

    #[test]
    fn percent_from_raw_uses_integer_backlight_scale() {
        assert_eq!(percent_from_raw(50, 200), 25);
        assert_eq!(percent_from_raw(0, 0), 0);
    }

    #[test]
    fn raw_from_percent_scales_to_max_brightness() {
        assert_eq!(raw_from_percent(25, 200), 50);
        assert_eq!(raw_from_percent(130, 200), 200);
    }
}
