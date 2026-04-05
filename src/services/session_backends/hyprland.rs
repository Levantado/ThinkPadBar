use crate::services::capabilities::CapabilityMode;
use super::PowerSessionBackend;

pub struct HyprlandSessionBackend;

impl PowerSessionBackend for HyprlandSessionBackend {
    fn backend_name(&self) -> &'static str {
        "hyprland"
    }

    fn capability_mode(&self) -> CapabilityMode {
        CapabilityMode::Native
    }

    fn lock(&self) {
        self.spawn("hyprlock", &[]);
    }

    fn logout(&self) {
        self.spawn("hyprctl", &["dispatch", "exit"]);
    }

    fn suspend(&self) {
        self.spawn("systemctl", &["suspend"]);
    }

    fn hibernate(&self) {
        self.spawn("systemctl", &["hibernate"]);
    }

    fn reboot(&self) {
        self.spawn("systemctl", &["reboot"]);
    }

    fn shutdown(&self) {
        self.spawn("systemctl", &["poweroff"]);
    }
}

impl HyprlandSessionBackend {
    fn spawn(&self, bin: &str, args: &[&str]) {
        if let Ok(mut child) = std::process::Command::new(bin)
            .args(args)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            std::mem::drop(std::thread::spawn(move || {
                let _ = child.wait();
            }));
        }
    }
}
