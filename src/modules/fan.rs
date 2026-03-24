use std::fs;
use tracing::{error, info, warn};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FanInfo {
    pub speed: String,
    pub level: String,
}

pub fn get_fan_info() -> FanInfo {
    let mut speed = String::from("---");
    let mut level = String::from("auto");

    if let Ok(content) = fs::read_to_string("/proc/acpi/ibm/fan") {
        for line in content.lines() {
            if let Some(s) = line.strip_prefix("speed:") {
                speed = s.trim().to_string();
            } else if let Some(l) = line.strip_prefix("level:") {
                level = l.trim().to_string();
            }
        }
    }

    FanInfo { speed, level }
}

pub fn set_fan_level(level: &str) {
    let cmd = format!("level {}", level);
    info!("Attempting to set fan level to: {}", level);

    if let Err(e) = fs::write("/proc/acpi/ibm/fan", &cmd) {
        warn!("Direct write failed: {}. Falling back to pkexec.", e);

        let cmd_clone = cmd.clone();
        std::thread::spawn(move || {
            let status = std::process::Command::new("pkexec")
                .arg("sh")
                .arg("-c")
                .arg(format!("echo '{}' > /proc/acpi/ibm/fan", cmd_clone))
                .status();

            match status {
                Ok(s) if s.success() => info!("Fan level set via pkexec"),
                _ => error!("Failed to set fan level even with pkexec"),
            }
        });
    } else {
        info!("Fan level successfully set via direct write");
    }
}
