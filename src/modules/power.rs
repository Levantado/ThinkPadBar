use std::fs;
use std::path::Path;
use tracing::{info, warn};

pub fn get_profile() -> String {
    if Path::new("/run/tlp.pid").exists() {
        return "auto-tlp".to_string();
    }

    match fs::read_to_string("/sys/firmware/acpi/platform_profile") {
        Ok(profile) => profile.trim().to_string(),
        Err(e) => {
            warn!(
                "Could not read platform_profile: {}. Using 'balanced' as fallback.",
                e
            );
            "balanced".to_string()
        }
    }
}

pub async fn set_profile(profile: &str) {
    let profile_str = profile.to_string();
    info!("Requesting power profile change to: {}", profile_str);

    if profile_str == "auto-tlp" {
        info!("Starting TLP service via pkexec...");
        let _ = tokio::process::Command::new("pkexec")
            .arg("systemctl")
            .arg("start")
            .arg("tlp.service")
            .status()
            .await;
    } else {
        if Path::new("/run/tlp.pid").exists() {
            info!("Stopping TLP service via pkexec...");
            let _ = tokio::process::Command::new("pkexec")
                .arg("systemctl")
                .arg("stop")
                .arg("tlp.service")
                .status()
                .await;
        }

        info!("Attempting direct write to platform_profile");
        if let Err(e) = fs::write("/sys/firmware/acpi/platform_profile", &profile_str) {
            warn!("Direct write failed: {}. Falling back to pkexec.", e);
            let p_str = profile_str.clone();
            let _ = tokio::process::Command::new("pkexec")
                .arg("sh")
                .arg("-c")
                .arg(format!(
                    "echo '{}' > /sys/firmware/acpi/platform_profile",
                    p_str
                ))
                .status()
                .await;
        } else {
            info!("Power profile successfully set via direct write");
        }
    }
}
