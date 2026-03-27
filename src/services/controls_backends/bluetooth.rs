use std::fs;
use std::process::{Command, Stdio};
use tracing::{info, warn};

#[derive(Debug, Default, Clone, Copy)]
pub struct BluetoothCtlBackend;

impl super::BluetoothBackend for BluetoothCtlBackend {
    fn backend_name(&self) -> &'static str {
        "bluetoothctl+rfkill+sysfs"
    }

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

    fn device_summary(&self) -> crate::services::controls::BluetoothDeviceSummary {
        crate::services::controls::BluetoothDeviceSummary {
            connected_devices: connected_devices(),
        }
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

fn connected_devices() -> Vec<String> {
    Command::new("bluetoothctl")
        .args(["devices", "Connected"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|stdout| parse_connected_devices(&stdout))
        .unwrap_or_default()
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

pub(crate) fn parse_connected_devices(output: &str) -> Vec<String> {
    output
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            let rest = trimmed.strip_prefix("Device ")?;
            let (_addr, name) = rest.split_once(' ')?;
            let name = name.trim();
            (!name.is_empty()).then(|| name.to_string())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{parse_connected_devices, parse_powered_from_bluetoothctl};

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

    #[test]
    fn parse_bluetoothctl_connected_devices_extracts_names() {
        let sample = "Device AA:BB:CC:DD:EE:FF WH-1000XM5\nDevice 11:22:33:44:55:66 MX Master 3S\n";
        assert_eq!(
            parse_connected_devices(sample),
            vec!["WH-1000XM5".to_string(), "MX Master 3S".to_string()]
        );
    }
}
