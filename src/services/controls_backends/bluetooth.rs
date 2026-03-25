use std::fs;
use std::process::{Command, Stdio};
use tracing::{info, warn};

#[derive(Debug, Default, Clone, Copy)]
pub struct BluetoothCtlBackend;

impl super::BluetoothBackend for BluetoothCtlBackend {
    fn enabled(&self) -> bool {
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

        sysfs_bluetooth_state().unwrap_or(false)
    }

    fn toggle(&self, enable: bool) -> bool {
        let state = if enable { "on" } else { "off" };
        info!("Attempting to toggle bluetooth to state: {}", state);

        let btctl_ok = Command::new("bluetoothctl")
            .args(["power", state])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
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
            .map(|status| status.success())
            .unwrap_or(false);
        if rfkill_ok {
            return true;
        }

        let target = if enable { "1" } else { "0" };
        let Ok(entries) = fs::read_dir("/sys/class/rfkill") else {
            return false;
        };
        for entry in entries.flatten() {
            if let Ok(rf_type) = fs::read_to_string(entry.path().join("type")) {
                if rf_type.trim() == "bluetooth" {
                    let path = entry.path().join("state");
                    match fs::write(&path, target) {
                        Ok(_) => {
                            info!("Bluetooth state successfully changed via sysfs");
                            return true;
                        }
                        Err(error) => {
                            warn!(
                                "Bluetooth sysfs write failed for {:?}: {}. Ensure udev/polkit permissions.",
                                path, error
                            );
                            return false;
                        }
                    }
                }
            }
        }
        false
    }

    fn open_overskride(&self) -> bool {
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
}

fn sysfs_bluetooth_state() -> Option<bool> {
    let entries = fs::read_dir("/sys/class/rfkill").ok()?;
    for entry in entries.flatten() {
        if let Ok(rf_type) = fs::read_to_string(entry.path().join("type")) {
            if rf_type.trim() == "bluetooth" {
                if let Ok(state) = fs::read_to_string(entry.path().join("state")) {
                    return Some(state.trim() == "1");
                }
            }
        }
    }
    None
}

pub(crate) fn parse_powered_from_bluetoothctl(output: &str) -> Option<bool> {
    for line in output.lines() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("Powered:") {
            return Some(value.trim().eq_ignore_ascii_case("yes"));
        }
    }
    None
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
