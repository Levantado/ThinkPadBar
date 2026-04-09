use super::PowerSessionBackend;
use crate::services::capabilities::CapabilityMode;

#[allow(dead_code)]
pub struct SystemdPowerBackend;

impl PowerSessionBackend for SystemdPowerBackend {
    fn backend_name(&self) -> &'static str {
        "systemd"
    }

    fn capability_mode(&self) -> CapabilityMode {
        CapabilityMode::Native
    }

    fn lock(&self) {
        self.spawn("loginctl", &["lock-session"]);
    }

    fn logout(&self) {
        self.spawn("loginctl", &["terminate-session"]);
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

impl SystemdPowerBackend {
    #[allow(dead_code)]
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
