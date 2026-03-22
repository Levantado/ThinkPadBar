use std::process::Command;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct HyprKeyboard {
    #[serde(default)]
    layout: String,
    #[serde(default)]
    active_layout_index: usize,
    #[serde(default)]
    main: bool,
}

#[derive(Deserialize, Debug)]
struct HyprDevices {
    keyboards: Vec<HyprKeyboard>,
}

pub fn get_layout() -> String {
    if let Ok(out) = Command::new("hyprctl").args(["devices", "-j"]).output() {
        if let Ok(s) = String::from_utf8(out.stdout) {
            if let Ok(devices) = serde_json::from_str::<HyprDevices>(&s) {
                if let Some(kb) = devices.keyboards.into_iter().find(|k| k.main) {
                    let layouts: Vec<&str> = kb.layout.split(',').map(|s| s.trim()).collect();
                    if let Some(layout) = layouts.get(kb.active_layout_index) {
                        return layout.to_uppercase();
                    }
                }
            }
        }
    }
    "UNKNOWN".to_string()
}

pub fn next_layout() {
    let _ = Command::new("hyprctl")
        .args(["switchxkblayout", "all", "next"])
        .output();
}
