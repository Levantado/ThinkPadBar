use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MicInfo {
    pub volume: u32,
    pub muted: bool,
}

pub fn update_led(muted: bool) {
    let brightness = if muted { "1" } else { "0" };

    // Standard ThinkPad LED paths
    let paths = [
        "/sys/class/leds/platform::micmute/brightness",
        "/sys/class/leds/tpacpi::micmute/brightness",
    ];

    for path_str in paths {
        let path = Path::new(path_str);
        if path.exists() {
            // Attempt to write. If it fails (Permission Denied), we just skip it.
            // To make this work without spam, the user should add a udev rule.
            let _ = fs::write(path, brightness);
        }
    }
}

pub fn get_info() -> MicInfo {
    if let Ok(output) = Command::new("wpctl")
        .args(["get-volume", "@DEFAULT_AUDIO_SOURCE@"])
        .output()
    {
        if let Ok(s) = String::from_utf8(output.stdout) {
            let s = s.trim();
            let muted = s.contains("[MUTED]");

            if let Some(vol_part) = s.split_whitespace().nth(1) {
                if let Ok(vol) = vol_part.parse::<f32>() {
                    return MicInfo {
                        volume: (vol * 100.0).round() as u32,
                        muted,
                    };
                }
            }
        }
    }
    MicInfo {
        volume: 0,
        muted: false,
    }
}

pub async fn set_volume(percent: u32) {
    let vol_str = format!("{:.2}", percent as f32 / 100.0);
    let _ = tokio::process::Command::new("wpctl")
        .args(["set-volume", "@DEFAULT_AUDIO_SOURCE@", &vol_str])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await;
}

pub async fn toggle_mute() {
    let _ = tokio::process::Command::new("wpctl")
        .args(["set-mute", "@DEFAULT_AUDIO_SOURCE@", "toggle"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await;
}
