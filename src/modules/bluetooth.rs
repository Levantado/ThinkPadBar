use std::process::Command;

pub fn get_bluetooth_info() -> bool {
    let mut enabled = false;
    if let Ok(output) = Command::new("rfkill").arg("list").arg("bluetooth").output() {
        let out_str = String::from_utf8_lossy(&output.stdout);
        if !out_str.contains("Soft blocked: yes") && !out_str.contains("Hard blocked: yes") && !out_str.is_empty() {
            enabled = true;
        }
    }
    enabled
}

pub fn toggle_bluetooth(enable: bool) {
    if enable {
        let _ = Command::new("rfkill").arg("unblock").arg("bluetooth").spawn();
    } else {
        let _ = Command::new("rfkill").arg("block").arg("bluetooth").spawn();
    }
}
