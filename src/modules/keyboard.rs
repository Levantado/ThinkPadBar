use crate::modules::workspaces::hyprland_command;
use serde::Deserialize;
use std::process::{Command, Stdio};

#[derive(Deserialize, Debug)]
struct HyprKeyboard {
    #[serde(default)]
    name: String,
    #[serde(default)]
    layout: String,
    #[serde(default)]
    active_layout_index: usize,
    #[serde(default)]
    active_keymap: String,
    #[serde(default)]
    main: bool,
}

#[derive(Deserialize, Debug)]
struct HyprDevices {
    keyboards: Vec<HyprKeyboard>,
}

pub fn get_layout() -> String {
    fn normalize_layout_label(raw: &str) -> String {
        let value = raw.trim().to_lowercase();
        if value.is_empty() {
            return "UNKNOWN".to_string();
        }
        if value.contains("russian") || value == "ru" || value.starts_with("ru_") {
            return "RU".to_string();
        }
        if value.contains("english")
            || value.contains("us")
            || value == "en"
            || value.starts_with("en_")
        {
            return "US".to_string();
        }
        value.to_uppercase()
    }

    if let Some(s) = hyprland_command("j/devices") {
        if let Ok(devices) = serde_json::from_str::<HyprDevices>(&s) {
            if let Some(kb) = devices.keyboards.into_iter().find(|k| k.main) {
                if !kb.active_keymap.trim().is_empty() {
                    return normalize_layout_label(&kb.active_keymap);
                }
                let layouts: Vec<&str> = kb.layout.split(',').map(|s| s.trim()).collect();
                if let Some(layout) = layouts.get(kb.active_layout_index) {
                    return normalize_layout_label(layout);
                }
            }
        }
    }
    "UNKNOWN".to_string()
}

fn switch_layout_with_dispatch(target: &str) -> bool {
    let cmd = format!("dispatch switchxkblayout {} next", target);
    hyprland_command(&cmd)
        .map(|out| dispatch_succeeded(&out))
        .unwrap_or(false)
}

fn dispatch_succeeded(raw: &str) -> bool {
    raw.trim().eq_ignore_ascii_case("ok")
}

fn switch_layout_with_hyprctl(target: &str) -> bool {
    Command::new("hyprctl")
        .args(["switchxkblayout", target, "next"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn next_layout() {
    if let Some(s) = hyprland_command("j/devices") {
        if let Ok(devices) = serde_json::from_str::<HyprDevices>(&s) {
            if let Some(main_kb) = devices.keyboards.iter().find(|k| k.main) {
                if !main_kb.name.is_empty()
                    && (switch_layout_with_hyprctl(&main_kb.name)
                        || switch_layout_with_dispatch(&main_kb.name))
                {
                    return;
                }
            }

            for kb in &devices.keyboards {
                if !kb.name.is_empty()
                    && (switch_layout_with_hyprctl(&kb.name)
                        || switch_layout_with_dispatch(&kb.name))
                {
                    return;
                }
            }
        }
    }

    if switch_layout_with_hyprctl("all") || switch_layout_with_dispatch("all") {
        return;
    }
    let _ = hyprland_command("dispatch switchxkblayout all next");
}

#[cfg(test)]
mod tests {
    use super::dispatch_succeeded;

    #[test]
    fn dispatch_succeeded_accepts_ok() {
        assert!(dispatch_succeeded("ok"));
        assert!(dispatch_succeeded("ok\n"));
        assert!(dispatch_succeeded("OK"));
    }

    #[test]
    fn dispatch_succeeded_rejects_errors() {
        assert!(!dispatch_succeeded(""));
        assert!(!dispatch_succeeded("invalid dispatcher"));
        assert!(!dispatch_succeeded("unknown layout"));
    }
}
