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

trait SessionRunner: Send + Sync {
    fn spawn(&self, bin: &str, args: &[&str]);
}

#[derive(Debug, Default)]
struct ProcessSessionRunner;

impl SessionRunner for ProcessSessionRunner {
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

#[derive(Clone)]
pub struct SessionService {
    snapshot: SessionSnapshot,
    runner: Arc<dyn SessionRunner>,
}

impl SessionService {
    pub fn new() -> Self {
        Self {
            snapshot: SessionSnapshot::default(),
            runner: Arc::new(ProcessSessionRunner),
        }
    }

    pub fn snapshot(&self) -> SessionSnapshot {
        self.snapshot
    }

    pub fn capability_status(&self) -> crate::services::capabilities::CapabilityStatus {
        crate::services::capabilities::CapabilityStatus {
            key: "ses",
            label: "Session Actions",
            mode: crate::services::capabilities::CapabilityMode::Fallback,
            provider: "rofi+systemctl+hyprctl".to_string(),
            detail: Some("process-spawn providers".to_string()),
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
                let (bin, args) = Self::launcher_command();
                self.runner.spawn(bin, args);
                SessionFollowUp::None
            }
            SessionCommand::Lock => {
                self.spawn_shell_command("hyprlock");
                SessionFollowUp::RefreshCompositor
            }
            SessionCommand::Sleep => {
                self.spawn_shell_command("systemctl suspend");
                SessionFollowUp::RefreshCompositor
            }
            SessionCommand::Hibernate => {
                self.spawn_shell_command("systemctl hibernate");
                SessionFollowUp::RefreshCompositor
            }
            SessionCommand::Restart => {
                self.spawn_shell_command("systemctl reboot");
                SessionFollowUp::RefreshCompositor
            }
            SessionCommand::Shutdown => {
                self.spawn_shell_command("systemctl poweroff");
                SessionFollowUp::RefreshCompositor
            }
            SessionCommand::Logout => {
                self.spawn_shell_command("hyprctl dispatch exit");
                SessionFollowUp::RefreshCompositor
            }
        }
    }

    pub fn launcher_command() -> (&'static str, &'static [&'static str]) {
        ("rofi", &["-replace", "-show", "drun"])
    }

    fn spawn_shell_command(&self, raw: &str) {
        let mut args = raw.split_whitespace();
        let Some(bin) = args.next() else {
            return;
        };
        let args: Vec<&str> = args.collect();
        self.runner.spawn(bin, &args);
    }
}

impl Default for SessionService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::{SessionCommand, SessionFollowUp, SessionService};

    type RecordedCalls = Arc<Mutex<Vec<(String, Vec<String>)>>>;

    #[derive(Default)]
    struct RecordingRunner {
        calls: RecordedCalls,
    }

    impl super::SessionRunner for RecordingRunner {
        fn spawn(&self, bin: &str, args: &[&str]) {
            self.calls.lock().unwrap().push((
                bin.to_string(),
                args.iter().map(|arg| (*arg).to_string()).collect(),
            ));
        }
    }

    impl SessionService {
        fn with_runner(runner: Arc<dyn super::SessionRunner>) -> Self {
            Self {
                snapshot: super::SessionSnapshot::default(),
                runner,
            }
        }
    }

    #[test]
    fn launcher_command_points_to_rofi_replace_drun() {
        let (bin, args) = SessionService::launcher_command();
        assert_eq!(bin, "rofi");
        assert_eq!(args, &["-replace", "-show", "drun"]);
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
    async fn launcher_returns_no_follow_up_without_spawning_real_rofi() {
        let runner = RecordingRunner::default();
        let calls = runner.calls.clone();
        let service = SessionService::with_runner(Arc::new(runner));
        assert_eq!(
            service.execute(SessionCommand::OpenLauncher).await,
            SessionFollowUp::None
        );
        assert_eq!(
            &*calls.lock().unwrap(),
            &[(
                "rofi".to_string(),
                vec![
                    "-replace".to_string(),
                    "-show".to_string(),
                    "drun".to_string()
                ],
            )]
        );
    }

    #[tokio::test]
    async fn lock_command_uses_runner_and_requests_refresh() {
        let runner = RecordingRunner::default();
        let calls = runner.calls.clone();
        let service = SessionService::with_runner(Arc::new(runner));

        assert_eq!(
            service.execute(SessionCommand::Lock).await,
            SessionFollowUp::RefreshCompositor
        );
        assert_eq!(
            &*calls.lock().unwrap(),
            &[("hyprlock".to_string(), Vec::<String>::new())]
        );
    }

    #[test]
    fn capability_status_reports_process_runner_fallback() {
        let status = SessionService::new().capability_status();

        assert_eq!(status.key, "ses");
        assert_eq!(
            status.mode,
            crate::services::capabilities::CapabilityMode::Fallback
        );
        assert_eq!(status.provider, "rofi+systemctl+hyprctl");
        assert_eq!(status.detail.as_deref(), Some("process-spawn providers"));
    }
}
