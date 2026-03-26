use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatteryInfo {
    pub capacity: u8,
    pub status: String,
    pub time_remaining: Option<String>,
    pub ac_online: Option<bool>,
    pub health_percent: Option<u8>,
    pub power_rate_mw: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BatteryReadings {
    capacity: u8,
    status: String,
    stored_now: Option<u64>,
    stored_full: Option<u64>,
    stored_full_design: Option<u64>,
    drain_rate: Option<u64>,
    power_now_uw: Option<u64>,
    current_now_ua: Option<u64>,
    voltage_now_uv: Option<u64>,
    ac_online: Option<bool>,
}

pub fn get_battery_info() -> BatteryInfo {
    let Some(battery_path) = find_power_supply_by_type("Battery") else {
        return BatteryInfo {
            capacity: 0,
            status: "Unknown".to_string(),
            time_remaining: None,
            ac_online: None,
            health_percent: None,
            power_rate_mw: None,
        };
    };

    build_battery_info(BatteryReadings {
        capacity: read_u64_field(&battery_path, "capacity")
            .unwrap_or(0)
            .min(u8::MAX as u64) as u8,
        status: read_string_field(&battery_path, "status").unwrap_or_else(|| "Unknown".to_string()),
        stored_now: read_u64_field(&battery_path, "energy_now")
            .or_else(|| read_u64_field(&battery_path, "charge_now")),
        stored_full: read_u64_field(&battery_path, "energy_full")
            .or_else(|| read_u64_field(&battery_path, "charge_full")),
        stored_full_design: read_u64_field(&battery_path, "energy_full_design")
            .or_else(|| read_u64_field(&battery_path, "charge_full_design")),
        drain_rate: read_u64_field(&battery_path, "power_now")
            .or_else(|| read_u64_field(&battery_path, "current_now")),
        power_now_uw: read_u64_field(&battery_path, "power_now"),
        current_now_ua: read_u64_field(&battery_path, "current_now"),
        voltage_now_uv: read_u64_field(&battery_path, "voltage_now"),
        ac_online: find_power_supply_by_type("Mains")
            .and_then(|path| read_u64_field(&path, "online"))
            .map(|value| value > 0),
    })
}

fn find_power_supply_by_type(kind: &str) -> Option<PathBuf> {
    let entries = fs::read_dir("/sys/class/power_supply").ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if read_string_field(&path, "type").as_deref() == Some(kind) {
            return Some(path);
        }
    }
    None
}

fn read_string_field(path: &Path, name: &str) -> Option<String> {
    fs::read_to_string(path.join(name))
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn read_u64_field(path: &Path, name: &str) -> Option<u64> {
    read_string_field(path, name)?.parse::<u64>().ok()
}

fn build_battery_info(readings: BatteryReadings) -> BatteryInfo {
    let drain_rate = readings.drain_rate.filter(|value| *value > 0);
    let time_remaining = match (
        drain_rate,
        readings.stored_now,
        readings.stored_full,
        readings.status.as_str(),
    ) {
        (Some(rate), Some(now), _, "Discharging") => {
            Some(format_time(now as f64 / rate as f64, "remaining"))
        }
        (Some(rate), Some(now), Some(full), "Charging") => Some(format_time(
            full.saturating_sub(now) as f64 / rate as f64,
            "until full",
        )),
        _ => None,
    };

    let health_percent = match (readings.stored_full, readings.stored_full_design) {
        (Some(full), Some(design)) if design > 0 => {
            let percent = ((full as f64 / design as f64) * 100.0).round() as u64;
            Some(percent.min(100) as u8)
        }
        _ => None,
    };

    let power_rate_mw = readings
        .power_now_uw
        .or_else(
            || match (readings.current_now_ua, readings.voltage_now_uv) {
                (Some(current), Some(voltage)) => Some(current.saturating_mul(voltage) / 1_000_000),
                _ => None,
            },
        )
        .map(|uw| (uw / 1_000) as u32)
        .filter(|mw| *mw > 0);

    BatteryInfo {
        capacity: readings.capacity,
        status: readings.status,
        time_remaining,
        ac_online: readings.ac_online,
        health_percent,
        power_rate_mw,
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

#[cfg(test)]
mod tests {
    use super::{build_battery_info, BatteryReadings};

    #[test]
    fn build_battery_info_reports_discharging_runtime_health_and_power() {
        let info = build_battery_info(BatteryReadings {
            capacity: 64,
            status: "Discharging".to_string(),
            stored_now: Some(32_000_000),
            stored_full: Some(48_000_000),
            stored_full_design: Some(52_000_000),
            drain_rate: Some(16_000_000),
            power_now_uw: Some(12_400_000),
            current_now_ua: None,
            voltage_now_uv: None,
            ac_online: Some(false),
        });

        assert_eq!(info.time_remaining.as_deref(), Some("2h 0m remaining"));
        assert_eq!(info.health_percent, Some(92));
        assert_eq!(info.power_rate_mw, Some(12_400));
        assert_eq!(info.ac_online, Some(false));
    }

    #[test]
    fn build_battery_info_reports_charging_time_until_full() {
        let info = build_battery_info(BatteryReadings {
            capacity: 70,
            status: "Charging".to_string(),
            stored_now: Some(35_000_000),
            stored_full: Some(50_000_000),
            stored_full_design: None,
            drain_rate: Some(10_000_000),
            power_now_uw: Some(20_000_000),
            current_now_ua: None,
            voltage_now_uv: None,
            ac_online: Some(true),
        });

        assert_eq!(info.time_remaining.as_deref(), Some("1h 30m until full"));
        assert_eq!(info.ac_online, Some(true));
    }

    #[test]
    fn build_battery_info_can_derive_power_from_current_and_voltage() {
        let info = build_battery_info(BatteryReadings {
            capacity: 88,
            status: "Charging".to_string(),
            stored_now: None,
            stored_full: None,
            stored_full_design: None,
            drain_rate: Some(2_000_000),
            power_now_uw: None,
            current_now_ua: Some(2_000_000),
            voltage_now_uv: Some(20_000_000),
            ac_online: None,
        });

        assert_eq!(info.power_rate_mw, Some(40_000));
        assert_eq!(info.health_percent, None);
        assert_eq!(info.time_remaining, None);
    }
}
