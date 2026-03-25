use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MicInfo {
    pub volume: u32,
    pub muted: bool,
}

pub fn update_led(muted: bool) {
    let brightness = if muted { "1" } else { "0" };
    let paths = [
        "/sys/class/leds/platform::micmute/brightness",
        "/sys/class/leds/tpacpi::micmute/brightness",
    ];

    for path_str in paths {
        let path = Path::new(path_str);
        if path.exists() {
            let _ = fs::write(path, brightness);
        }
    }
}
