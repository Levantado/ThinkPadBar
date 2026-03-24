use std::fs;
use tracing::{error, info};

pub fn get_bluetooth_info() -> bool {
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

pub fn toggle_bluetooth(enable: bool) {
    let state = if enable { "1" } else { "0" };
    info!("Attempting to toggle bluetooth to state: {}", state);
    if let Ok(entries) = fs::read_dir("/sys/class/rfkill") {
        for entry in entries.flatten() {
            if let Ok(rf_type) = fs::read_to_string(entry.path().join("type")) {
                if rf_type.trim() == "bluetooth" {
                    if let Err(e) = fs::write(entry.path().join("state"), state) {
                        error!(
                            "Failed to toggle bluetooth: {}. Ensure udev rules are applied.",
                            e
                        );
                    } else {
                        info!("Bluetooth state successfully changed to: {}", state);
                    }
                }
            }
        }
    }
}
