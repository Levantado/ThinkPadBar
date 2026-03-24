use std::fs;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatteryInfo {
    pub capacity: u8,
    pub status: String,
    pub time_remaining: Option<String>,
}

pub fn get_battery_info() -> BatteryInfo {
    let read_val = |name: &str| -> u64 {
        fs::read_to_string(format!("/sys/class/power_supply/BAT0/{}", name))
            .unwrap_or_else(|_| "0\n".to_string())
            .trim()
            .parse::<u64>()
            .unwrap_or(0)
    };

    let capacity = read_val("capacity") as u8;
    let status = fs::read_to_string("/sys/class/power_supply/BAT0/status")
        .unwrap_or_else(|_| "Unknown\n".to_string())
        .trim()
        .to_string();

    let energy_now = read_val("energy_now");
    let energy_full = read_val("energy_full");
    let power_now = read_val("power_now");

    let mut time_remaining = None;

    if power_now > 0 {
        if status == "Discharging" {
            let hours = energy_now as f64 / power_now as f64;
            time_remaining = Some(format_time(hours, "remaining"));
        } else if status == "Charging" {
            let hours = (energy_full.saturating_sub(energy_now)) as f64 / power_now as f64;
            time_remaining = Some(format_time(hours, "until full"));
        }
    }

    BatteryInfo {
        capacity,
        status,
        time_remaining,
    }
}

fn format_time(hours: f64, suffix: &str) -> String {
    let total_minutes = (hours * 60.0).round() as u32;
    let h = total_minutes / 60;
    let m = total_minutes % 60;
    if h > 0 {
        format!("{}h {}m {}", h, m, suffix)
    } else {
        format!("{}m {}", m, suffix)
    }
}
