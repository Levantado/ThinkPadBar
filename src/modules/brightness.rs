use std::fs;
use std::process::Command;

pub fn get_brightness() -> String {
    let mut percentage = 0;

    if let Ok(entries) = fs::read_dir("/sys/class/backlight") {
        for entry in entries.flatten() {
            let path = entry.path();
            if let (Ok(max), Ok(current)) = (
                fs::read_to_string(path.join("max_brightness")),
                fs::read_to_string(path.join("brightness")),
            ) {
                if let (Ok(m), Ok(c)) = (max.trim().parse::<u32>(), current.trim().parse::<u32>()) {
                    if m > 0 {
                        percentage = (c * 100) / m;
                        break;
                    }
                }
            }
        }
    }
    format!("󰃠 {}%", percentage)
}

pub fn set_brightness(val: u32) {
    let _ = Command::new("brightnessctl")
        .arg("s")
        .arg(format!("{}%", val))
        .output();
}

