use std::fs;

pub fn get_profile() -> String {
    let tlp_status = std::process::Command::new("systemctl")
        .arg("is-active")
        .arg("tlp.service")
        .output();
    
    if let Ok(output) = tlp_status {
        let status = String::from_utf8_lossy(&output.stdout);
        if status.trim() == "active" {
            return "auto-tlp".to_string();
        }
    }

    let profile = fs::read_to_string("/sys/firmware/acpi/platform_profile")
        .unwrap_or_else(|_| "balanced\n".to_string());
    profile.trim().to_string()
}

pub fn set_profile(profile: &str) {
    let profile_str = profile.to_string();
    tokio::spawn(async move {
        if profile_str == "auto-tlp" {
            let _ = std::process::Command::new("pkexec")
                .arg("systemctl")
                .arg("start")
                .arg("tlp.service")
                .spawn();
        } else {
            if let Err(e) = tokio::fs::write("/sys/firmware/acpi/platform_profile", &profile_str).await {
                println!("Direct write to platform_profile failed: {:?}. Trying pkexec.", e);
                let _ = std::process::Command::new("pkexec")
                    .arg("sh")
                    .arg("-c")
                    .arg(format!("systemctl stop tlp.service && echo '{}' > /sys/firmware/acpi/platform_profile", profile_str))
                    .spawn();
            } else {
                let _ = std::process::Command::new("pkexec")
                    .arg("systemctl")
                    .arg("stop")
                    .arg("tlp.service")
                    .spawn();
            }
        }
    });
}
