use std::fs;

pub fn get_battery_info() -> (u8, String) {
    let capacity_str = fs::read_to_string("/sys/class/power_supply/BAT0/capacity")
        .unwrap_or_else(|_| "0\n".to_string());
    let status_str = fs::read_to_string("/sys/class/power_supply/BAT0/status")
        .unwrap_or_else(|_| "Unknown\n".to_string());
    
    let capacity = capacity_str.trim().parse::<u8>().unwrap_or(0);
    let status = status_str.trim().to_string();
    
    (capacity, status)
}
