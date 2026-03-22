use std::fs;

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
    if let Err(_e) = fs::write("/proc/acpi/ibm/fan", &cmd) {
        // Fallback to pkexec to ask for password globally if write fails
        let _ = std::process::Command::new("pkexec")
            .arg("sh")
            .arg("-c")
            .arg(format!("echo '{}' > /proc/acpi/ibm/fan", cmd))
            .spawn();
    }
}
