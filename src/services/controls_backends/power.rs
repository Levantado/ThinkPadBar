use std::{fs, path::Path, process::Command, process::Stdio, time::Duration};

use iced::futures::{SinkExt, StreamExt};
use tracing::{info, warn};

const POWER_PROFILES_DESTINATION: &str = "net.hadess.PowerProfiles";
const POWER_PROFILES_PATH: &str = "/net/hadess/PowerProfiles";
const POWER_PROFILES_INTERFACE: &str = "net.hadess.PowerProfiles";
const PLATFORM_PROFILE_PATH: &str = "/sys/firmware/acpi/platform_profile";
const POWER_PROFILE_RETRY_DELAY: Duration = Duration::from_secs(2);

#[derive(Debug, Default, Clone, Copy)]
pub struct PowerProfilesDaemonBackend;

impl super::PowerBackend for PowerProfilesDaemonBackend {
    fn backend_name(&self) -> &'static str {
        "ppd+platform_profile"
    }

    fn capability_mode(&self) -> crate::services::capabilities::CapabilityMode {
        crate::services::capabilities::CapabilityMode::Hybrid
    }

    fn diagnostics_summary(&self) -> Option<String> {
        Some(power_runtime_summary(
            ppd_cli_available(),
            Path::new(PLATFORM_PROFILE_PATH).exists(),
        ))
    }

    fn profile(&self) -> String {
        current_profile()
    }

    fn set_profile(&self, profile: String) -> super::BackendFuture<'_, ()> {
        Box::pin(async move {
            let canonical = canonical_profile_value(&profile);
            info!("Requesting power profile change to: {}", canonical);

            if let Err(error) = set_profile_via_dbus(&canonical).await {
                warn!(
                    "Power profile update via D-Bus failed: {}. Falling back to platform_profile.",
                    error
                );
                if let Err(fallback_error) = set_profile_via_platform_profile(&canonical).await {
                    warn!(
                        "Power profile update via platform_profile fallback failed: {}",
                        fallback_error
                    );
                }
            }
        })
    }

    fn subscription(&self) -> iced::Subscription<crate::services::controls::ControlsEvent> {
        struct PowerProfileListener;

        iced::Subscription::run_with_id(
            std::any::TypeId::of::<PowerProfileListener>(),
            iced::stream::channel(1, move |mut output| async move {
                loop {
                    let connection = match zbus::Connection::system().await {
                        Ok(connection) => connection,
                        Err(error) => {
                            warn!(
                                "Power profile event listener failed to connect to system bus: {}",
                                error
                            );
                            tokio::time::sleep(POWER_PROFILE_RETRY_DELAY).await;
                            continue;
                        }
                    };

                    let proxy = match zbus::Proxy::new(
                        &connection,
                        POWER_PROFILES_DESTINATION,
                        POWER_PROFILES_PATH,
                        POWER_PROFILES_INTERFACE,
                    )
                    .await
                    {
                        Ok(proxy) => proxy,
                        Err(error) => {
                            warn!(
                                "Power profile event listener failed to create proxy: {}",
                                error
                            );
                            tokio::time::sleep(POWER_PROFILE_RETRY_DELAY).await;
                            continue;
                        }
                    };

                    let mut changes = proxy
                        .receive_property_changed::<String>("ActiveProfile")
                        .await;

                    while let Some(change) = changes.next().await {
                        match change.get().await {
                            Ok(_profile) => {
                                if output
                                    .send(crate::services::controls::ControlsEvent::PowerProfileChanged)
                                    .await
                                    .is_err()
                                {
                                    return;
                                }
                            }
                            Err(error) => {
                                warn!(
                                    "Power profile event listener failed to decode ActiveProfile payload: {}",
                                    error
                                );
                                break;
                            }
                        }
                    }

                    tokio::time::sleep(POWER_PROFILE_RETRY_DELAY).await;
                }
            }),
        )
    }
}

pub(crate) fn current_profile() -> String {
    if let Some(profile) = read_profile_via_powerprofilesctl() {
        return profile;
    }

    match fs::read_to_string(PLATFORM_PROFILE_PATH) {
        Ok(profile) => canonical_profile_value(profile.trim()),
        Err(error) => {
            warn!("Could not read power profile from powerprofilesctl or {}: {}. Using 'balanced' fallback.", PLATFORM_PROFILE_PATH, error);
            "balanced".to_string()
        }
    }
}

fn ppd_cli_available() -> bool {
    Command::new("powerprofilesctl")
        .arg("get")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn read_profile_via_powerprofilesctl() -> Option<String> {
    let output = Command::new("powerprofilesctl")
        .arg("get")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8(output.stdout).ok()?;
    parse_powerprofilesctl_get_output(&stdout)
}

async fn set_profile_via_dbus(profile: &str) -> Result<(), String> {
    let connection = zbus::Connection::system()
        .await
        .map_err(|error| error.to_string())?;
    let proxy = zbus::Proxy::new(
        &connection,
        POWER_PROFILES_DESTINATION,
        POWER_PROFILES_PATH,
        POWER_PROFILES_INTERFACE,
    )
    .await
    .map_err(|error| error.to_string())?;
    proxy
        .set_property("ActiveProfile", daemon_profile_name(profile))
        .await
        .map_err(|error| error.to_string())
}

async fn set_profile_via_platform_profile(profile: &str) -> Result<(), String> {
    if !Path::new(PLATFORM_PROFILE_PATH).exists() {
        return Err(format!("{PLATFORM_PROFILE_PATH} is unavailable"));
    }

    let platform_profile = platform_profile_name(profile);
    if let Err(error) = fs::write(PLATFORM_PROFILE_PATH, platform_profile) {
        warn!(
            "Direct platform_profile write failed: {}. Falling back to pkexec.",
            error
        );
        return write_profile_via_pkexec(platform_profile).await;
    }

    Ok(())
}

async fn write_profile_via_pkexec(profile: &str) -> Result<(), String> {
    let script = format!("printf '%s' '{}' > {}", profile, PLATFORM_PROFILE_PATH);
    let output = tokio::process::Command::new("pkexec")
        .arg("sh")
        .arg("-c")
        .arg(script)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|error| error.to_string())?;

    if output.status.success() {
        return Ok(());
    }

    let summary = process_stderr_summary(&output.stderr)
        .unwrap_or_else(|| "pkexec command failed without stderr output".to_string());
    Err(summary)
}

fn canonical_profile_value(raw: &str) -> String {
    canonical_profile_name(raw).to_string()
}

fn canonical_profile_name(raw: &str) -> &'static str {
    match raw.trim().to_ascii_lowercase().as_str() {
        "power-saver" | "powersaver" | "powersave" | "low-power" | "low" | "quiet" => "low-power",
        "performance" | "perf" | "high" => "performance",
        "balanced" | "balance" | "auto" => "balanced",
        _ => "balanced",
    }
}

fn daemon_profile_name(profile: &str) -> &'static str {
    match canonical_profile_name(profile) {
        "low-power" => "power-saver",
        "performance" => "performance",
        _ => "balanced",
    }
}

fn platform_profile_name(profile: &str) -> &'static str {
    match canonical_profile_name(profile) {
        "low-power" => "low-power",
        "performance" => "performance",
        _ => "balanced",
    }
}

fn process_stderr_summary(stderr: &[u8]) -> Option<String> {
    String::from_utf8_lossy(stderr)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(ToOwned::to_owned)
}

fn parse_powerprofilesctl_get_output(raw: &str) -> Option<String> {
    let value = raw.lines().next()?.trim();
    if value.is_empty() {
        None
    } else {
        Some(canonical_profile_value(value))
    }
}

fn power_runtime_summary(ppd_cli_available: bool, platform_profile_exists: bool) -> String {
    format!(
        "ppd-cli:{} platform_profile:{}",
        ppd_cli_available, platform_profile_exists
    )
}

#[cfg(test)]
mod tests {
    use super::{
        canonical_profile_name, daemon_profile_name, parse_powerprofilesctl_get_output,
        platform_profile_name, power_runtime_summary, process_stderr_summary,
    };

    #[test]
    fn canonical_profile_name_maps_supported_aliases() {
        assert_eq!(canonical_profile_name("power-saver"), "low-power");
        assert_eq!(canonical_profile_name("powersave"), "low-power");
        assert_eq!(canonical_profile_name("balanced"), "balanced");
        assert_eq!(canonical_profile_name("auto"), "balanced");
        assert_eq!(canonical_profile_name("performance"), "performance");
        assert_eq!(canonical_profile_name("high"), "performance");
        assert_eq!(canonical_profile_name("unknown"), "balanced");
    }

    #[test]
    fn daemon_profile_name_uses_power_profiles_daemon_labels() {
        assert_eq!(daemon_profile_name("low-power"), "power-saver");
        assert_eq!(daemon_profile_name("balanced"), "balanced");
        assert_eq!(daemon_profile_name("performance"), "performance");
    }

    #[test]
    fn platform_profile_name_uses_sysfs_labels() {
        assert_eq!(platform_profile_name("low-power"), "low-power");
        assert_eq!(platform_profile_name("balanced"), "balanced");
        assert_eq!(platform_profile_name("performance"), "performance");
    }

    #[test]
    fn process_stderr_summary_returns_first_non_empty_line() {
        assert_eq!(
            process_stderr_summary(b"\nPermission denied\nsecond line\n"),
            Some("Permission denied".to_string())
        );
        assert_eq!(process_stderr_summary(b"\n\n"), None);
    }

    #[test]
    fn power_runtime_summary_reports_backend_presence() {
        assert_eq!(
            power_runtime_summary(true, false),
            "ppd-cli:true platform_profile:false"
        );
    }

    #[test]
    fn parse_powerprofilesctl_get_output_normalizes_supported_values() {
        assert_eq!(
            parse_powerprofilesctl_get_output("power-saver\n"),
            Some("low-power".to_string())
        );
        assert_eq!(
            parse_powerprofilesctl_get_output("balanced\n"),
            Some("balanced".to_string())
        );
        assert_eq!(
            parse_powerprofilesctl_get_output("performance\n"),
            Some("performance".to_string())
        );
        assert_eq!(parse_powerprofilesctl_get_output("\n"), None);
    }

    #[test]
    fn current_profile_is_safe_inside_tokio_runtime() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let result = runtime.block_on(async { std::panic::catch_unwind(super::current_profile) });
        assert!(result.is_ok());
    }
}
