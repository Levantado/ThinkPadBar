use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

#[derive(Debug, Default, Clone, Copy)]
pub struct PlatformProfilePowerBackend;

impl super::PowerBackend for PlatformProfilePowerBackend {
    fn backend_name(&self) -> &'static str {
        "platform_profile+tlp"
    }

    fn diagnostics_summary(&self) -> Option<String> {
        Some(power_runtime_summary(
            tlp_active(),
            Path::new("/sys/firmware/acpi/platform_profile").exists(),
            battery_threshold_paths().is_some(),
        ))
    }

    fn profile(&self) -> String {
        current_profile()
    }

    fn set_profile(&self, profile: String) -> super::BackendFuture<'_, ()> {
        Box::pin(async move {
            info!("Requesting power profile change to: {}", profile);

            if profile == "auto-tlp" {
                let _ = tokio::process::Command::new("pkexec")
                    .arg("systemctl")
                    .arg("start")
                    .arg("tlp.service")
                    .status()
                    .await;
                return;
            }

            if tlp_active() {
                let _ = tokio::process::Command::new("pkexec")
                    .arg("systemctl")
                    .arg("stop")
                    .arg("tlp.service")
                    .status()
                    .await;
            }

            if let Err(error) = fs::write("/sys/firmware/acpi/platform_profile", &profile) {
                warn!("Direct write failed: {}. Falling back to pkexec.", error);
                let _ = tokio::process::Command::new("pkexec")
                    .arg("sh")
                    .arg("-c")
                    .arg(format!(
                        "echo '{}' > /sys/firmware/acpi/platform_profile",
                        profile
                    ))
                    .status()
                    .await;
            } else {
                info!("Power profile successfully set via direct write");
            }
        })
    }

    fn set_battery_thresholds(
        &self,
        thresholds: crate::services::controls::BatteryThresholds,
    ) -> super::BackendFuture<'_, ()> {
        Box::pin(async move {
            let Some((start_path, end_path)) = battery_threshold_paths() else {
                warn!("Battery threshold files are unavailable; skipping threshold write");
                return;
            };

            let thresholds =
                crate::services::controls::BatteryThresholds::new(thresholds.start, thresholds.end);

            if let Err(error) = write_battery_thresholds(&start_path, &end_path, thresholds) {
                warn!(
                    "Direct battery threshold write failed: {}. Falling back to pkexec.",
                    error
                );
                let _ = tokio::process::Command::new("pkexec")
                    .arg("sh")
                    .arg("-c")
                    .arg(battery_threshold_script(&start_path, &end_path, thresholds))
                    .status()
                    .await;
            } else {
                info!(
                    "Battery thresholds successfully set via direct write: {} -> {}",
                    thresholds.start, thresholds.end
                );
            }
        })
    }
}

fn tlp_active() -> bool {
    Path::new("/run/tlp.pid").exists()
}

pub(crate) fn current_profile() -> String {
    resolve_current_profile(
        tlp_active(),
        fs::read_to_string("/sys/firmware/acpi/platform_profile"),
    )
}

fn resolve_current_profile(
    tlp_active: bool,
    profile_result: Result<String, std::io::Error>,
) -> String {
    if tlp_active {
        return "auto-tlp".to_string();
    }

    match profile_result {
        Ok(profile) => profile.trim().to_string(),
        Err(error) => {
            warn!(
                "Could not read platform_profile: {}. Using 'balanced' as fallback.",
                error
            );
            "balanced".to_string()
        }
    }
}

fn battery_threshold_paths() -> Option<(PathBuf, PathBuf)> {
    let battery_path = find_power_supply_by_type("Battery")?;
    let start_path = battery_path.join("charge_control_start_threshold");
    let end_path = battery_path.join("charge_control_end_threshold");
    (start_path.exists() && end_path.exists()).then_some((start_path, end_path))
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

fn write_battery_thresholds(
    start_path: &Path,
    end_path: &Path,
    thresholds: crate::services::controls::BatteryThresholds,
) -> Result<(), std::io::Error> {
    fs::write(start_path, thresholds.start.to_string())?;
    fs::write(end_path, thresholds.end.to_string())?;
    Ok(())
}

fn battery_threshold_script(
    start_path: &Path,
    end_path: &Path,
    thresholds: crate::services::controls::BatteryThresholds,
) -> String {
    format!(
        "printf '%u' {} > {} && printf '%u' {} > {}",
        thresholds.start,
        start_path.display(),
        thresholds.end,
        end_path.display()
    )
}

fn power_runtime_summary(
    tlp_active: bool,
    profile_path_exists: bool,
    threshold_paths_exist: bool,
) -> String {
    format!(
        "tlp:{} platform_profile:{} thresholds:{}",
        tlp_active, profile_path_exists, threshold_paths_exist
    )
}

#[cfg(test)]
mod tests {
    use super::{battery_threshold_script, power_runtime_summary, resolve_current_profile};
    use crate::services::controls::BatteryThresholds;
    use std::path::Path;

    #[test]
    fn resolve_current_profile_prefers_tlp_when_active() {
        assert_eq!(
            resolve_current_profile(true, Ok("performance\n".to_string())),
            "auto-tlp"
        );
    }

    #[test]
    fn resolve_current_profile_trims_profile_contents() {
        assert_eq!(
            resolve_current_profile(false, Ok("balanced\n".to_string())),
            "balanced"
        );
    }

    #[test]
    fn resolve_current_profile_falls_back_to_balanced_on_read_error() {
        assert_eq!(
            resolve_current_profile(
                false,
                Err(std::io::Error::new(std::io::ErrorKind::NotFound, "missing"))
            ),
            "balanced"
        );
    }

    #[test]
    fn power_runtime_summary_reports_tlp_and_profile_path_state() {
        assert_eq!(
            power_runtime_summary(true, false, true),
            "tlp:true platform_profile:false thresholds:true"
        );
    }

    #[test]
    fn battery_threshold_script_writes_both_threshold_files() {
        assert_eq!(
            battery_threshold_script(
                Path::new("/sys/class/power_supply/BAT0/charge_control_start_threshold"),
                Path::new("/sys/class/power_supply/BAT0/charge_control_end_threshold"),
                BatteryThresholds::new(40, 80),
            ),
            "printf '%u' 40 > /sys/class/power_supply/BAT0/charge_control_start_threshold && printf '%u' 80 > /sys/class/power_supply/BAT0/charge_control_end_threshold"
        );
    }
}
