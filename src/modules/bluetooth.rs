use std::fs;
use std::process::{Command, Stdio};
use tracing::{info, warn};

fn parse_powered_from_bluetoothctl(output: &str) -> Option<bool> {
    for line in output.lines() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("Powered:") {
            return Some(value.trim().eq_ignore_ascii_case("yes"));
        }
    }
    None
}

pub fn get_bluetooth_info() -> bool {
    if let Ok(output) = Command::new("bluetoothctl")
        .arg("show")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
    {
        if output.status.success() {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                if let Some(powered) = parse_powered_from_bluetoothctl(&stdout) {
                    return powered;
                }
            }
        }
    }

    if let Ok(entries) = fs::read_dir("/sys/class/rfkill") {
        for entry in entries.flatten() {
            if let Ok(rf_type) = fs::read_to_string(entry.path().join("type")) {
                if rf_type.trim() == "bluetooth" {
                    if let Ok(state) = fs::read_to_string(entry.path().join("state")) {
                        return state.trim() == "1";
                    }
                }
            }
        }
    }
    false
}

pub fn toggle_bluetooth(enable: bool) -> bool {
    let state = if enable { "on" } else { "off" };
    info!("Attempting to toggle bluetooth to state: {}", state);

    let btctl_ok = Command::new("bluetoothctl")
        .args(["power", state])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if btctl_ok {
        return true;
    }

    let rfkill_args: &[&str] = if enable {
        &["unblock", "bluetooth"]
    } else {
        &["block", "bluetooth"]
    };
    let rfkill_ok = Command::new("rfkill")
        .args(rfkill_args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if rfkill_ok {
        return true;
    }

    let target = if enable { "1" } else { "0" };
    if let Ok(entries) = fs::read_dir("/sys/class/rfkill") {
        for entry in entries.flatten() {
            if let Ok(rf_type) = fs::read_to_string(entry.path().join("type")) {
                if rf_type.trim() == "bluetooth" {
                    let path = entry.path().join("state");
                    match fs::write(&path, target) {
                        Ok(_) => {
                            info!("Bluetooth state successfully changed via sysfs");
                            return true;
                        }
                        Err(e) => {
                            warn!(
                                "Bluetooth sysfs write failed for {:?}: {}. Ensure udev/polkit permissions.",
                                path, e
                            );
                            return false;
                        }
                    }
                }
            }
        }
    }
    false
}

pub fn open_overskride() -> bool {
    let direct = Command::new("overskride")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .is_ok();
    if direct {
        return true;
    }

    Command::new("flatpak")
        .args(["run", "io.github.kaii_lb.Overskride"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::parse_powered_from_bluetoothctl;

    #[test]
    fn parse_bluetoothctl_powered_yes() {
        let sample = "Controller XX:XX:XX\n\tPowered: yes\n";
        assert_eq!(parse_powered_from_bluetoothctl(sample), Some(true));
    }

    #[test]
    fn parse_bluetoothctl_powered_no() {
        let sample = "Controller XX:XX:XX\n\tPowered: no\n";
        assert_eq!(parse_powered_from_bluetoothctl(sample), Some(false));
    }
}
