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
    pub pack_voltage_mv: Option<u32>,
    pub cycle_count: Option<u32>,
    pub full_charge_mwh: Option<u32>,
    pub design_capacity_mwh: Option<u32>,
    pub charge_start_threshold: Option<u8>,
    pub charge_end_threshold: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BatteryReadings {
    capacity: u8,
    status: String,
    energy_now_uwh: Option<u64>,
    energy_full_uwh: Option<u64>,
    energy_full_design_uwh: Option<u64>,
    charge_now_uah: Option<u64>,
    charge_full_uah: Option<u64>,
    charge_full_design_uah: Option<u64>,
    power_now_uw: Option<u64>,
    current_now_ua: Option<u64>,
    voltage_now_uv: Option<u64>,
    voltage_min_design_uv: Option<u64>,
    ac_online: Option<bool>,
    cycle_count: Option<u32>,
    charge_start_threshold: Option<u8>,
    charge_end_threshold: Option<u8>,
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
            pack_voltage_mv: None,
            cycle_count: None,
            full_charge_mwh: None,
            design_capacity_mwh: None,
            charge_start_threshold: None,
            charge_end_threshold: None,
        };
    };

    build_battery_info(BatteryReadings {
        capacity: read_u64_field(&battery_path, "capacity")
            .unwrap_or(0)
            .min(u8::MAX as u64) as u8,
        status: read_string_field(&battery_path, "status").unwrap_or_else(|| "Unknown".to_string()),
        energy_now_uwh: read_u64_field(&battery_path, "energy_now"),
        energy_full_uwh: read_u64_field(&battery_path, "energy_full"),
        energy_full_design_uwh: read_u64_field(&battery_path, "energy_full_design"),
        charge_now_uah: read_u64_field(&battery_path, "charge_now"),
        charge_full_uah: read_u64_field(&battery_path, "charge_full"),
        charge_full_design_uah: read_u64_field(&battery_path, "charge_full_design"),
        power_now_uw: read_u64_field(&battery_path, "power_now"),
        current_now_ua: read_u64_field(&battery_path, "current_now"),
        voltage_now_uv: read_u64_field(&battery_path, "voltage_now"),
        voltage_min_design_uv: read_u64_field(&battery_path, "voltage_min_design"),
        ac_online: find_power_supply_by_type("Mains")
            .and_then(|path| read_u64_field(&path, "online"))
            .map(|value| value > 0),
        cycle_count: read_u64_field(&battery_path, "cycle_count").map(|value| value as u32),
        charge_start_threshold: read_percent_field(&battery_path, "charge_control_start_threshold"),
        charge_end_threshold: read_percent_field(&battery_path, "charge_control_end_threshold"),
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

fn read_percent_field(path: &Path, name: &str) -> Option<u8> {
    read_u64_field(path, name).map(|value| value.min(100) as u8)
}

fn build_battery_info(readings: BatteryReadings) -> BatteryInfo {
    let stored_now = readings.energy_now_uwh.or(readings.charge_now_uah);
    let stored_full = readings.energy_full_uwh.or(readings.charge_full_uah);
    let stored_full_design = readings
        .energy_full_design_uwh
        .or(readings.charge_full_design_uah);
    let drain_rate = readings
        .power_now_uw
        .or(readings.current_now_ua)
        .filter(|value| *value > 0);
    let time_remaining = match (
        drain_rate,
        stored_now,
        stored_full,
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

    let health_percent = match (stored_full, stored_full_design) {
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
    let pack_voltage_mv = readings
        .voltage_now_uv
        .map(|uv| (uv / 1_000) as u32)
        .filter(|mv| *mv > 0);

    let conversion_voltage_uv = readings.voltage_min_design_uv.or(readings.voltage_now_uv);
    let full_charge_mwh = readings.energy_full_uwh.map(uwh_to_mwh).or_else(|| {
        readings
            .charge_full_uah
            .zip(conversion_voltage_uv)
            .map(|(charge, voltage)| charge_to_mwh(charge, voltage))
    });
    let design_capacity_mwh = readings.energy_full_design_uwh.map(uwh_to_mwh).or_else(|| {
        readings
            .charge_full_design_uah
            .zip(conversion_voltage_uv)
            .map(|(charge, voltage)| charge_to_mwh(charge, voltage))
    });

    BatteryInfo {
        capacity: readings.capacity,
        status: readings.status,
        time_remaining,
        ac_online: readings.ac_online,
        health_percent,
        power_rate_mw,
        pack_voltage_mv,
        cycle_count: readings.cycle_count,
        full_charge_mwh,
        design_capacity_mwh,
        charge_start_threshold: readings.charge_start_threshold,
        charge_end_threshold: readings.charge_end_threshold,
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

fn uwh_to_mwh(value: u64) -> u32 {
    (value / 1_000) as u32
}

fn charge_to_mwh(charge_uah: u64, voltage_uv: u64) -> u32 {
    charge_uah
        .saturating_mul(voltage_uv)
        .saturating_div(1_000_000_000) as u32
}

#[cfg(test)]
mod tests {
    use super::{build_battery_info, charge_to_mwh, BatteryReadings};

    #[test]
    fn build_battery_info_reports_discharging_runtime_health_and_power() {
        let info = build_battery_info(BatteryReadings {
            capacity: 64,
            status: "Discharging".to_string(),
            energy_now_uwh: Some(32_000_000),
            energy_full_uwh: Some(48_000_000),
            energy_full_design_uwh: Some(52_000_000),
            charge_now_uah: None,
            charge_full_uah: None,
            charge_full_design_uah: None,
            power_now_uw: Some(12_400_000),
            current_now_ua: None,
            voltage_now_uv: None,
            voltage_min_design_uv: None,
            ac_online: Some(false),
            cycle_count: Some(187),
            charge_start_threshold: None,
            charge_end_threshold: None,
        });

        assert_eq!(info.time_remaining.as_deref(), Some("2h 35m remaining"));
        assert_eq!(info.health_percent, Some(92));
        assert_eq!(info.power_rate_mw, Some(12_400));
        assert_eq!(info.ac_online, Some(false));
        assert_eq!(info.full_charge_mwh, Some(48_000));
        assert_eq!(info.design_capacity_mwh, Some(52_000));
        assert_eq!(info.cycle_count, Some(187));
    }

    #[test]
    fn build_battery_info_reports_charging_time_until_full() {
        let info = build_battery_info(BatteryReadings {
            capacity: 70,
            status: "Charging".to_string(),
            energy_now_uwh: Some(35_000_000),
            energy_full_uwh: Some(50_000_000),
            energy_full_design_uwh: None,
            charge_now_uah: None,
            charge_full_uah: None,
            charge_full_design_uah: None,
            power_now_uw: Some(20_000_000),
            current_now_ua: None,
            voltage_now_uv: None,
            voltage_min_design_uv: None,
            ac_online: Some(true),
            cycle_count: None,
            charge_start_threshold: None,
            charge_end_threshold: None,
        });

        assert_eq!(info.time_remaining.as_deref(), Some("45m until full"));
        assert_eq!(info.ac_online, Some(true));
    }

    #[test]
    fn build_battery_info_can_derive_power_from_current_and_voltage() {
        let info = build_battery_info(BatteryReadings {
            capacity: 88,
            status: "Charging".to_string(),
            energy_now_uwh: None,
            energy_full_uwh: None,
            energy_full_design_uwh: None,
            charge_now_uah: None,
            charge_full_uah: None,
            charge_full_design_uah: None,
            power_now_uw: None,
            current_now_ua: Some(2_000_000),
            voltage_now_uv: Some(20_000_000),
            voltage_min_design_uv: None,
            ac_online: None,
            cycle_count: None,
            charge_start_threshold: None,
            charge_end_threshold: None,
        });

        assert_eq!(info.power_rate_mw, Some(40_000));
        assert_eq!(info.health_percent, None);
        assert_eq!(info.time_remaining, None);
    }

    #[test]
    fn build_battery_info_can_derive_pack_capacity_from_charge_and_voltage() {
        let info = build_battery_info(BatteryReadings {
            capacity: 82,
            status: "Discharging".to_string(),
            energy_now_uwh: None,
            energy_full_uwh: None,
            energy_full_design_uwh: None,
            charge_now_uah: Some(4_200_000),
            charge_full_uah: Some(4_800_000),
            charge_full_design_uah: Some(5_100_000),
            power_now_uw: None,
            current_now_ua: Some(1_600_000),
            voltage_now_uv: Some(12_000_000),
            voltage_min_design_uv: Some(11_400_000),
            ac_online: Some(false),
            cycle_count: Some(312),
            charge_start_threshold: Some(40),
            charge_end_threshold: Some(80),
        });

        assert_eq!(
            info.full_charge_mwh,
            Some(charge_to_mwh(4_800_000, 11_400_000))
        );
        assert_eq!(
            info.design_capacity_mwh,
            Some(charge_to_mwh(5_100_000, 11_400_000))
        );
        assert_eq!(info.cycle_count, Some(312));
        assert_eq!(info.pack_voltage_mv, Some(12_000));
        assert_eq!(info.charge_start_threshold, Some(40));
        assert_eq!(info.charge_end_threshold, Some(80));
    }
}
