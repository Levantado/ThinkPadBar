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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SessionService {
    snapshot: SessionSnapshot,
}

impl SessionService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> SessionSnapshot {
        self.snapshot
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
                Self::spawn_command_and_reap(bin, args);
                SessionFollowUp::None
            }
            SessionCommand::Lock => {
                Self::spawn_shell_command("hyprlock");
                SessionFollowUp::RefreshCompositor
            }
            SessionCommand::Sleep => {
                Self::spawn_shell_command("systemctl suspend");
                SessionFollowUp::RefreshCompositor
            }
            SessionCommand::Hibernate => {
                Self::spawn_shell_command("systemctl hibernate");
                SessionFollowUp::RefreshCompositor
            }
            SessionCommand::Restart => {
                Self::spawn_shell_command("systemctl reboot");
                SessionFollowUp::RefreshCompositor
            }
            SessionCommand::Shutdown => {
                Self::spawn_shell_command("systemctl poweroff");
                SessionFollowUp::RefreshCompositor
            }
            SessionCommand::Logout => {
                Self::spawn_shell_command("hyprctl dispatch exit");
                SessionFollowUp::RefreshCompositor
            }
        }
    }

    pub fn launcher_command() -> (&'static str, &'static [&'static str]) {
        ("rofi", &["-replace", "-show", "drun"])
    }

    fn spawn_shell_command(raw: &str) {
        let mut args = raw.split_whitespace();
        let Some(bin) = args.next() else {
            return;
        };
        let args: Vec<&str> = args.collect();
        Self::spawn_command_and_reap(bin, &args);
    }

    fn spawn_command_and_reap(bin: &str, args: &[&str]) {
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

#[cfg(test)]
mod tests {
    use super::{SessionCommand, SessionFollowUp, SessionService};

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
    async fn launcher_returns_no_follow_up() {
        let service = SessionService::new();
        assert_eq!(
            service.execute(SessionCommand::OpenLauncher).await,
            SessionFollowUp::None
        );
    }
}
