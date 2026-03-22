use std::process::Command;

pub struct AudioInfo {
    pub volume: u32,
    pub muted: bool,
}

pub fn get_info() -> AudioInfo {
    if let Ok(output) = Command::new("wpctl")
        .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
        .output()
    {
        if let Ok(s) = String::from_utf8(output.stdout) {
            let s = s.trim();
            let muted = s.contains("[MUTED]");
            
            if let Some(vol_part) = s.split_whitespace().nth(1) {
                if let Ok(vol) = vol_part.parse::<f32>() {
                    return AudioInfo {
                        volume: (vol * 100.0).round() as u32,
                        muted,
                    };
                }
            }
        }
    }
    AudioInfo { volume: 0, muted: false }
}

pub fn set_volume(percent: u32) {
    let vol_str = format!("{:.2}", percent as f32 / 100.0);
    let _ = Command::new("wpctl")
        .args(["set-volume", "@DEFAULT_AUDIO_SINK@", &vol_str])
        .output();
}

pub fn toggle_mute() {
    let _ = Command::new("wpctl")
        .args(["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"])
        .output();
}
