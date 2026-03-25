use std::fs;
use std::path::Path;
use tracing::{info, warn};

#[derive(Debug, Default, Clone, Copy)]
pub struct PlatformProfilePowerBackend;

impl super::PowerBackend for PlatformProfilePowerBackend {
    fn backend_name(&self) -> &'static str {
        "platform_profile+tlp"
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

#[cfg(test)]
mod tests {
    use super::resolve_current_profile;

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
}
