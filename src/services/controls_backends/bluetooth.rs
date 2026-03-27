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
        connected_device_summary()
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

    fn connect_device(&self, address: String) -> super::BackendFuture<'_, bool> {
        Box::pin(async move {
            tokio::process::Command::new("bluetoothctl")
                .args(["connect", &address])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await
                .map(|status| status.success())
                .unwrap_or(false)
        })
    }

    fn disconnect_device(&self, address: String) -> super::BackendFuture<'_, bool> {
        Box::pin(async move {
            tokio::process::Command::new("bluetoothctl")
                .args(["disconnect", &address])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await
                .map(|status| status.success())
                .unwrap_or(false)
        })
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

fn connected_device_summary() -> crate::services::controls::BluetoothDeviceSummary {
    let device_briefs = Command::new("bluetoothctl")
        .arg("devices")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|stdout| parse_connected_device_briefs(&stdout))
        .unwrap_or_default();

    let device_details = device_briefs
        .iter()
        .map(|device| {
            let info_output = Command::new("bluetoothctl")
                .args(["info", &device.address])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .output()
                .ok()
                .and_then(|output| String::from_utf8(output.stdout).ok());

            let (connected, battery_percent, audio_profiles) = info_output
                .as_deref()
                .map(parse_bluetooth_device_info)
                .unwrap_or_default();

            crate::services::controls::BluetoothConnectedDevice {
                address: device.address.clone(),
                name: device.name.clone(),
                connected,
                battery_percent,
                audio_profiles,
            }
        })
        .collect::<Vec<_>>();

    let connected_devices = device_details
        .iter()
        .filter(|device| device.connected)
        .map(|device| device.name.clone())
        .collect();

    crate::services::controls::BluetoothDeviceSummary {
        connected_devices,
        device_details,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ConnectedDeviceBrief {
    address: String,
    name: String,
}

#[cfg(test)]
pub(crate) fn parse_connected_devices(output: &str) -> Vec<String> {
    parse_connected_device_briefs(output)
        .into_iter()
        .map(|device| device.name)
        .collect()
}

fn parse_connected_device_briefs(output: &str) -> Vec<ConnectedDeviceBrief> {
    output
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            let rest = trimmed.strip_prefix("Device ")?;
            let (address, name) = rest.split_once(' ')?;
            let address = address.trim();
            let name = name.trim();
            if address.is_empty() || name.is_empty() {
                return None;
            }
            Some(ConnectedDeviceBrief {
                address: address.to_string(),
                name: name.to_string(),
            })
        })
        .collect()
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

pub(crate) fn parse_bluetooth_device_info(output: &str) -> (bool, Option<u8>, Vec<String>) {
    let mut connected = false;
    let mut battery_percent = None;
    let mut audio_profiles = Vec::new();

    for line in output.lines() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("Connected:") {
            connected = value.trim().eq_ignore_ascii_case("yes");
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("Battery Percentage:") {
            battery_percent = parse_bluetooth_battery_percent(value);
            continue;
        }

        let Some(value) = trimmed.strip_prefix("UUID:") else {
            continue;
        };
        let value = value.trim();
        let Some((name, _uuid)) = value.rsplit_once('(') else {
            continue;
        };
        if let Some(profile) = normalize_audio_profile_name(name.trim()) {
            if !audio_profiles.iter().any(|existing| existing == profile) {
                audio_profiles.push(profile.to_string());
            }
        }
    }

    (connected, battery_percent, audio_profiles)
}

fn parse_bluetooth_battery_percent(value: &str) -> Option<u8> {
    if let Some((_, trailing)) = value.split_once('(') {
        let digits = trailing
            .chars()
            .take_while(|ch| ch.is_ascii_digit())
            .collect::<String>();
        if let Ok(percent) = digits.parse::<u8>() {
            return Some(percent);
        }
    }

    let digits = value
        .chars()
        .filter(|ch| ch.is_ascii_digit())
        .collect::<String>();
    digits.parse::<u8>().ok()
}

fn normalize_audio_profile_name(name: &str) -> Option<&'static str> {
    let lower = name.to_ascii_lowercase();
    if lower.contains("audio sink") {
        Some("A2DP")
    } else if lower.contains("handsfree") {
        Some("HFP")
    } else if lower.contains("headset") {
        Some("HSP")
    } else if lower.contains("a/v_remote control target") {
        Some("AVRCP Target")
    } else if lower.contains("a/v_remote control") {
        Some("AVRCP")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{
        parse_bluetooth_device_info, parse_connected_device_briefs, parse_connected_devices,
        parse_powered_from_bluetoothctl,
    };

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

    #[test]
    fn parse_bluetoothctl_connected_devices_extracts_addresses_and_names() {
        let sample = "Device AA:BB:CC:DD:EE:FF WH-1000XM5\n";
        let devices = parse_connected_device_briefs(sample);
        assert_eq!(devices[0].address, "AA:BB:CC:DD:EE:FF");
        assert_eq!(devices[0].name, "WH-1000XM5");
    }

    #[test]
    fn parse_bluetooth_device_info_extracts_battery_and_audio_profiles() {
        let sample = "Device AA:BB:CC:DD:EE:FF\n\
                      \tConnected: yes\n\
                      \tBattery Percentage: 0x5A (90)\n\
                      \tUUID: Audio Sink                (0000110b-0000-1000-8000-00805f9b34fb)\n\
                      \tUUID: Handsfree                 (0000111e-0000-1000-8000-00805f9b34fb)\n\
                      \tUUID: A/V_Remote Control        (0000110e-0000-1000-8000-00805f9b34fb)\n";
        let (connected, battery, profiles) = parse_bluetooth_device_info(sample);
        assert!(connected);
        assert_eq!(battery, Some(90));
        assert_eq!(
            profiles,
            vec!["A2DP".to_string(), "HFP".to_string(), "AVRCP".to_string()]
        );
    }
}
