use crate::services::session_backends::hyprland::HyprlandSessionBackend;
use crate::services::session_backends::rofi::RofiLauncherBackend;
use crate::services::session_backends::{LauncherBackend, PowerSessionBackend};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionCommand {
    OpenLauncher,
    Lock,
    Sleep,
    Hibernate,
    Restart,
    Shutdown,
    Logout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionFollowUp {
    None,
    RefreshCompositor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SessionSnapshot {
    pub power_menu_open: bool,
}

#[derive(Clone)]
pub struct SessionService {
    snapshot: SessionSnapshot,
    power: Arc<dyn PowerSessionBackend>,
    launcher: Arc<dyn LauncherBackend>,
}

impl SessionService {
    pub fn new() -> Self {
        Self {
            snapshot: SessionSnapshot::default(),
            power: Arc::new(HyprlandSessionBackend),
            launcher: Arc::new(RofiLauncherBackend),
        }
    }

    pub fn snapshot(&self) -> SessionSnapshot {
        self.snapshot
    }

    pub fn capability_status(&self) -> crate::services::capabilities::CapabilityStatus {
        crate::services::capabilities::CapabilityStatus {
            key: "ses",
            label: "Session Actions",
            mode: self.power.capability_mode(),
            provider: format!(
                "{}+{}",
                self.launcher.backend_name(),
                self.power.backend_name()
            ),
            detail: Some("provider-backed session service".to_string()),
        }
    }

    pub fn toggle_power_menu(&mut self) {
        self.snapshot.power_menu_open = !self.snapshot.power_menu_open;
    }

    pub fn close_transient_ui(&mut self) {
        self.snapshot.power_menu_open = false;
    }

    pub async fn execute(&self, command: SessionCommand) -> SessionFollowUp {
        match command {
            SessionCommand::OpenLauncher => {
                self.launcher.toggle_launcher();
                SessionFollowUp::None
            }
            SessionCommand::Lock => {
                self.power.lock();
                SessionFollowUp::RefreshCompositor
            }
            SessionCommand::Sleep => {
                self.power.suspend();
                SessionFollowUp::RefreshCompositor
            }
            SessionCommand::Hibernate => {
                self.power.hibernate();
                SessionFollowUp::RefreshCompositor
            }
            SessionCommand::Restart => {
                self.power.reboot();
                SessionFollowUp::RefreshCompositor
            }
            SessionCommand::Shutdown => {
                self.power.shutdown();
                SessionFollowUp::RefreshCompositor
            }
            SessionCommand::Logout => {
                self.power.logout();
                SessionFollowUp::RefreshCompositor
            }
        }
    }
}

impl Default for SessionService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{SessionCommand, SessionFollowUp, SessionService};
    use crate::services::capabilities::CapabilityMode;
    use crate::services::session_backends::{LauncherBackend, PowerSessionBackend};
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct MockPowerBackend {
        calls: Arc<Mutex<Vec<String>>>,
    }

    impl PowerSessionBackend for MockPowerBackend {
        fn backend_name(&self) -> &'static str {
            "mock-power"
        }
        fn capability_mode(&self) -> CapabilityMode {
            CapabilityMode::Native
        }
        fn lock(&self) {
            self.calls.lock().unwrap().push("lock".to_string());
        }
        fn logout(&self) {
            self.calls.lock().unwrap().push("logout".to_string());
        }
        fn suspend(&self) {
            self.calls.lock().unwrap().push("suspend".to_string());
        }
        fn hibernate(&self) {
            self.calls.lock().unwrap().push("hibernate".to_string());
        }
        fn reboot(&self) {
            self.calls.lock().unwrap().push("reboot".to_string());
        }
        fn shutdown(&self) {
            self.calls.lock().unwrap().push("shutdown".to_string());
        }
    }

    #[derive(Default)]
    struct MockLauncherBackend {
        calls: Arc<Mutex<Vec<String>>>,
    }

    impl LauncherBackend for MockLauncherBackend {
        fn backend_name(&self) -> &'static str {
            "mock-launcher"
        }
        fn toggle_launcher(&self) -> bool {
            self.calls.lock().unwrap().push("toggle".to_string());
            true
        }
    }

    impl SessionService {
        fn with_backends(
            power: Arc<dyn PowerSessionBackend>,
            launcher: Arc<dyn LauncherBackend>,
        ) -> Self {
            Self {
                snapshot: super::SessionSnapshot::default(),
                power,
                launcher,
            }
        }
    }

    #[test]
    fn power_menu_toggle_is_stateful() {
        let mut service = SessionService::new();
        assert!(!service.snapshot().power_menu_open);
        service.toggle_power_menu();
        assert!(service.snapshot().power_menu_open);
        service.close_transient_ui();
        assert!(!service.snapshot().power_menu_open);
    }

    #[tokio::test]
    async fn launcher_command_routes_to_launcher_backend() {
        let power_calls = Arc::new(Mutex::new(Vec::new()));
        let launcher_calls = Arc::new(Mutex::new(Vec::new()));

        let power = Arc::new(MockPowerBackend {
            calls: power_calls.clone(),
        });
        let launcher = Arc::new(MockLauncherBackend {
            calls: launcher_calls.clone(),
        });
        let service = SessionService::with_backends(power, launcher);

        assert_eq!(
            service.execute(SessionCommand::OpenLauncher).await,
            SessionFollowUp::None
        );
        assert_eq!(*launcher_calls.lock().unwrap(), vec!["toggle".to_string()]);
        assert!(power_calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn lock_command_routes_to_power_backend_and_requests_refresh() {
        let power_calls = Arc::new(Mutex::new(Vec::new()));
        let launcher_calls = Arc::new(Mutex::new(Vec::new()));

        let power = Arc::new(MockPowerBackend {
            calls: power_calls.clone(),
        });
        let launcher = Arc::new(MockLauncherBackend {
            calls: launcher_calls.clone(),
        });
        let service = SessionService::with_backends(power, launcher);

        assert_eq!(
            service.execute(SessionCommand::Lock).await,
            SessionFollowUp::RefreshCompositor
        );
        assert_eq!(*power_calls.lock().unwrap(), vec!["lock".to_string()]);
        assert!(launcher_calls.lock().unwrap().is_empty());
    }

    #[test]
    fn capability_status_reports_backend_names() {
        let status = SessionService::new().capability_status();

        assert_eq!(status.key, "ses");
        assert_eq!(
            status.mode,
            crate::services::capabilities::CapabilityMode::Native
        );
        assert_eq!(status.provider, "rofi+hyprland");
        assert_eq!(
            status.detail.as_deref(),
            Some("provider-backed session service")
        );
    }
}
