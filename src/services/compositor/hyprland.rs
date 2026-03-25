use iced::Subscription;
use serde::Deserialize;
use serde_json::Value;
use std::future::Future;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream as SyncUnixStream;
use std::pin::Pin;
use std::process::{Command, Stdio};
use std::time::Duration;

use super::types::{CompositorEvent, WorkspaceInfo};

#[derive(Deserialize)]
struct HyprWorkspace {
    id: i32,
    name: String,
}

#[derive(Deserialize)]
struct HyprActiveWorkspace {
    id: i32,
    name: String,
}

#[derive(Deserialize)]
struct HyprWindow {
    title: String,
    class: String,
}

#[derive(Deserialize)]
struct HyprClient {
    workspace: HyprWorkspaceId,
    #[serde(rename = "initialClass")]
    initial_class: String,
    title: String,
}

#[derive(Deserialize)]
struct HyprWorkspaceId {
    id: i32,
}

#[derive(Deserialize)]
struct HyprMonitor {
    #[serde(rename = "specialWorkspace")]
    special_workspace: Option<HyprSpecialWorkspace>,
}

#[derive(Deserialize)]
struct HyprSpecialWorkspace {
    id: i32,
    name: String,
}

#[derive(Deserialize, Debug)]
struct HyprKeyboard {
    #[serde(default)]
    name: String,
    #[serde(default)]
    layout: String,
    #[serde(default)]
    active_layout_index: usize,
    #[serde(default)]
    active_keymap: String,
    #[serde(default)]
    main: bool,
}

#[derive(Deserialize, Debug)]
struct HyprDevices {
    keyboards: Vec<HyprKeyboard>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppLocateResult {
    NotFound,
    FoundSameWorkspace,
    SwitchedWorkspace,
}

#[derive(Debug, Clone)]
pub struct HyprlandBackend {
    command_socket_path: Option<String>,
    event_socket_path: Option<String>,
}

impl Default for HyprlandBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl HyprlandBackend {
    pub fn new() -> Self {
        let signature = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").ok();
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").ok();
        let command_socket_path = match (&signature, &runtime_dir) {
            (Some(signature), Some(runtime_dir)) => {
                Some(format!("{}/hypr/{}/.socket.sock", runtime_dir, signature))
            }
            _ => None,
        };
        let event_socket_path = match (&signature, &runtime_dir) {
            (Some(signature), Some(runtime_dir)) => {
                Some(format!("{}/hypr/{}/.socket2.sock", runtime_dir, signature))
            }
            _ => None,
        };

        Self {
            command_socket_path,
            event_socket_path,
        }
    }

    fn command(&self, cmd: &str) -> Option<String> {
        let socket_path = self.command_socket_path.as_ref()?;
        let mut stream = SyncUnixStream::connect(socket_path).ok()?;
        stream.write_all(cmd.as_bytes()).ok()?;

        let mut response = String::new();
        stream.read_to_string(&mut response).ok()?;
        Some(response)
    }

    fn parse_cursor_pos(raw: &str) -> Option<(i32, i32)> {
        let value = serde_json::from_str::<Value>(raw).ok()?;
        let x = value.get("x")?.as_f64()?.round() as i32;
        let y = value.get("y")?.as_f64()?.round() as i32;
        Some((x, y))
    }

    fn parse_special_workspace_visible_from_monitors_json(raw: &str) -> bool {
        if let Ok(monitors) = serde_json::from_str::<Vec<HyprMonitor>>(raw) {
            return monitors.iter().any(|monitor| {
                if let Some(special_workspace) = &monitor.special_workspace {
                    let name = special_workspace.name.to_ascii_lowercase();
                    special_workspace.id != 0 || (!name.is_empty() && name.starts_with("special"))
                } else {
                    false
                }
            });
        }
        false
    }

    fn workspace_dispatch_command(id: i32, name: &str) -> String {
        let lower = name.to_ascii_lowercase();
        if lower == "special" {
            return "dispatch togglespecialworkspace".to_string();
        }
        if lower.starts_with("special:") {
            let target = name
                .split_once(':')
                .map(|(_, rhs)| rhs.trim())
                .unwrap_or("");
            if target.is_empty() {
                return "dispatch togglespecialworkspace".to_string();
            }
            return format!("dispatch togglespecialworkspace {}", target);
        }
        format!("dispatch workspace {}", id)
    }

    fn workspace_is_active(id: i32, name: &str, active_id: i32, active_name: &str) -> bool {
        id == active_id || name == active_name
    }

    fn normalize_match_text(input: &str) -> String {
        input
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() {
                    character.to_ascii_lowercase()
                } else {
                    ' '
                }
            })
            .collect::<String>()
    }

    fn client_matches_query(query: &str, initial_class: &str, title: &str) -> bool {
        let normalized_query = Self::normalize_match_text(query);
        let query = normalized_query.trim();
        if query.len() < 2 {
            return false;
        }

        let class_normalized = Self::normalize_match_text(initial_class);
        let title_normalized = Self::normalize_match_text(title);
        if class_normalized.contains(query) || title_normalized.contains(query) {
            return true;
        }

        query
            .split_whitespace()
            .filter(|token| token.len() >= 3)
            .any(|token| class_normalized.contains(token) || title_normalized.contains(token))
    }

    fn normalize_layout_label(raw: &str) -> String {
        let value = raw.trim().to_lowercase();
        if value.is_empty() {
            return "UNKNOWN".to_string();
        }
        if value.contains("russian") || value == "ru" || value.starts_with("ru_") {
            return "RU".to_string();
        }
        if value.contains("english")
            || value.contains("us")
            || value == "en"
            || value.starts_with("en_")
        {
            return "US".to_string();
        }
        value.to_uppercase()
    }

    fn dispatch_succeeded(raw: &str) -> bool {
        raw.trim().eq_ignore_ascii_case("ok")
    }

    fn switch_layout_with_dispatch(&self, target: &str) -> bool {
        let cmd = format!("dispatch switchxkblayout {} next", target);
        self.command(&cmd)
            .map(|out| Self::dispatch_succeeded(&out))
            .unwrap_or(false)
    }

    fn switch_layout_with_hyprctl(target: &str) -> bool {
        Command::new("hyprctl")
            .args(["switchxkblayout", target, "next"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    fn find_and_switch_to_app_detailed(&self, name: String) -> AppLocateResult {
        let active_workspace_id = if let Some(raw) = self.command("j/activeworkspace") {
            serde_json::from_str::<HyprActiveWorkspace>(&raw)
                .map(|workspace| workspace.id)
                .unwrap_or(0)
        } else {
            0
        };

        if let Some(raw) = self.command("j/clients") {
            if let Ok(clients) = serde_json::from_str::<Vec<HyprClient>>(&raw) {
                for client in clients {
                    if Self::client_matches_query(&name, &client.initial_class, &client.title) {
                        if client.workspace.id != active_workspace_id {
                            self.switch_workspace(client.workspace.id, "");
                            return AppLocateResult::SwitchedWorkspace;
                        }
                        return AppLocateResult::FoundSameWorkspace;
                    }
                }
            }
        }

        AppLocateResult::NotFound
    }

    pub fn get_workspaces(&self) -> Vec<WorkspaceInfo> {
        let mut infos = Vec::new();

        let (active_id, active_name) = if let Some(raw) = self.command("j/activeworkspace") {
            serde_json::from_str::<HyprActiveWorkspace>(&raw)
                .map(|workspace| (workspace.id, workspace.name))
                .unwrap_or((0, String::new()))
        } else {
            (0, String::new())
        };

        if let Some(raw) = self.command("j/workspaces") {
            if let Ok(mut workspaces) = serde_json::from_str::<Vec<HyprWorkspace>>(&raw) {
                workspaces.sort_by_key(|workspace| workspace.id);
                for workspace in workspaces {
                    let is_active = Self::workspace_is_active(
                        workspace.id,
                        &workspace.name,
                        active_id,
                        &active_name,
                    );
                    infos.push(WorkspaceInfo {
                        id: workspace.id,
                        name: workspace.name,
                        active: is_active,
                    });
                }
            }
        }

        if infos.is_empty() {
            infos.push(WorkspaceInfo {
                id: 1,
                name: "1".to_string(),
                active: true,
            });
        }

        let active_is_special = active_name == "special" || active_name.starts_with("special:");
        if active_is_special
            && !infos
                .iter()
                .any(|workspace| workspace.id == active_id || workspace.name == active_name)
        {
            infos.push(WorkspaceInfo {
                id: active_id,
                name: active_name,
                active: true,
            });
        }

        infos
    }

    pub fn is_special_workspace_visible(&self) -> bool {
        self.command("j/monitors")
            .map(|raw| Self::parse_special_workspace_visible_from_monitors_json(&raw))
            .unwrap_or(false)
    }

    pub fn get_active_window_title(&self) -> String {
        if let Some(raw) = self.command("j/activewindow") {
            if let Ok(window) = serde_json::from_str::<HyprWindow>(&raw) {
                if !window.title.is_empty() {
                    return window.title;
                }
                return window.class;
            }
        }
        String::new()
    }

    pub fn get_keyboard_layout(&self) -> String {
        if let Some(raw) = self.command("j/devices") {
            if let Ok(devices) = serde_json::from_str::<HyprDevices>(&raw) {
                if let Some(keyboard) = devices.keyboards.into_iter().find(|keyboard| keyboard.main)
                {
                    if !keyboard.active_keymap.trim().is_empty() {
                        return Self::normalize_layout_label(&keyboard.active_keymap);
                    }
                    let layouts: Vec<&str> = keyboard.layout.split(',').map(|s| s.trim()).collect();
                    if let Some(layout) = layouts.get(keyboard.active_layout_index) {
                        return Self::normalize_layout_label(layout);
                    }
                }
            }
        }
        "UNKNOWN".to_string()
    }

    pub fn switch_workspace(&self, id: i32, name: &str) {
        let _ = self.command(&Self::workspace_dispatch_command(id, name));
    }

    pub fn next_keyboard_layout(&self) {
        if let Some(raw) = self.command("j/devices") {
            if let Ok(devices) = serde_json::from_str::<HyprDevices>(&raw) {
                if let Some(main_keyboard) = devices.keyboards.iter().find(|keyboard| keyboard.main)
                {
                    if !main_keyboard.name.is_empty()
                        && (Self::switch_layout_with_hyprctl(&main_keyboard.name)
                            || self.switch_layout_with_dispatch(&main_keyboard.name))
                    {
                        return;
                    }
                }

                for keyboard in &devices.keyboards {
                    if !keyboard.name.is_empty()
                        && (Self::switch_layout_with_hyprctl(&keyboard.name)
                            || self.switch_layout_with_dispatch(&keyboard.name))
                    {
                        return;
                    }
                }
            }
        }

        if Self::switch_layout_with_hyprctl("all") || self.switch_layout_with_dispatch("all") {
            return;
        }
        let _ = self.command("dispatch switchxkblayout all next");
    }

    pub fn subscription(&self) -> Subscription<CompositorEvent> {
        struct HyprlandListener;

        let socket_path = self.event_socket_path.clone();
        Subscription::run_with_id(
            std::any::TypeId::of::<HyprlandListener>(),
            iced::stream::channel(1, move |mut output| async move {
                if let Some(socket_path) = socket_path {
                    loop {
                        use tokio::io::{AsyncBufReadExt, BufReader};
                        use tokio::net::UnixStream;

                        if let Ok(stream) = UnixStream::connect(&socket_path).await {
                            let mut reader = BufReader::new(stream);
                            let mut line = String::new();

                            loop {
                                line.clear();
                                match reader.read_line(&mut line).await {
                                    Ok(0) => break,
                                    Ok(_) => {
                                        if line.starts_with("workspace>>")
                                            || line.starts_with("activewindow>>")
                                            || line.starts_with("focusedmon>>")
                                            || line.starts_with("createworkspace>>")
                                            || line.starts_with("destroyworkspace>>")
                                            || line.starts_with("movewindow>>")
                                            || line.starts_with("activelayout>>")
                                            || line.starts_with("urgent>>")
                                            || line.starts_with("configreloaded>>")
                                        {
                                            let _ = output.try_send(CompositorEvent::StateChanged);
                                        }
                                    }
                                    Err(_) => break,
                                }
                            }
                        }
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                } else {
                    loop {
                        tokio::time::sleep(Duration::from_secs(3600)).await;
                    }
                }
            }),
        )
    }

    pub fn cursor_position(&self) -> Option<(i32, i32)> {
        self.command("j/cursorpos")
            .and_then(|raw| Self::parse_cursor_pos(&raw))
    }

    pub fn find_and_switch_to_app(
        &self,
        name: String,
    ) -> Pin<Box<dyn Future<Output = bool> + Send>> {
        let backend = self.clone();
        Box::pin(async move {
            !matches!(
                backend.find_and_switch_to_app_detailed(name),
                AppLocateResult::NotFound
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::HyprlandBackend;
    use crate::services::compositor::types::WorkspaceInfo;

    #[test]
    fn parse_special_workspace_visible_handles_prefix() {
        let raw = r#"[{"specialWorkspace":{"id":0,"name":"special:term"}}]"#;
        assert!(HyprlandBackend::parse_special_workspace_visible_from_monitors_json(raw));
    }

    #[test]
    fn workspace_dispatch_uses_regular_workspace_for_normal_name() {
        assert_eq!(
            HyprlandBackend::workspace_dispatch_command(2, "2"),
            "dispatch workspace 2"
        );
        assert_eq!(
            HyprlandBackend::workspace_dispatch_command(5, "dev"),
            "dispatch workspace 5"
        );
    }

    #[test]
    fn workspace_dispatch_uses_special_toggle_for_special_workspace() {
        assert_eq!(
            HyprlandBackend::workspace_dispatch_command(0, "special"),
            "dispatch togglespecialworkspace"
        );
        assert_eq!(
            HyprlandBackend::workspace_dispatch_command(0, "special:term"),
            "dispatch togglespecialworkspace term"
        );
        assert_eq!(
            HyprlandBackend::workspace_dispatch_command(0, "SPECIAL:notes"),
            "dispatch togglespecialworkspace notes"
        );
        assert_eq!(
            HyprlandBackend::workspace_dispatch_command(0, "special:"),
            "dispatch togglespecialworkspace"
        );
    }

    #[test]
    fn workspace_active_detection_matches_by_id_or_name() {
        assert!(HyprlandBackend::workspace_is_active(3, "3", 3, "3"));
        assert!(HyprlandBackend::workspace_is_active(
            -99,
            "special:term",
            4,
            "special:term"
        ));
        assert!(!HyprlandBackend::workspace_is_active(2, "2", 1, "1"));
    }

    #[test]
    fn synthetic_active_special_workspace_is_added_when_missing() {
        let mut infos = vec![WorkspaceInfo {
            id: 1,
            name: "1".to_string(),
            active: true,
        }];
        let active_id = -99;
        let active_name = "special:term".to_string();
        let active_is_special = active_name == "special" || active_name.starts_with("special:");
        if active_is_special
            && !infos
                .iter()
                .any(|workspace| workspace.id == active_id || workspace.name == active_name)
        {
            infos.push(WorkspaceInfo {
                id: active_id,
                name: active_name,
                active: true,
            });
        }

        assert_eq!(infos.len(), 2);
        assert_eq!(infos[1].name, "special:term");
        assert!(infos[1].active);
    }

    #[test]
    fn client_matching_handles_punctuation_and_tokenized_queries() {
        assert!(HyprlandBackend::client_matches_query(
            "telegram",
            "org.telegram.desktop",
            "Telegram"
        ));
        assert!(HyprlandBackend::client_matches_query(
            "google chrome",
            "google-chrome",
            "Google Chrome"
        ));
        assert!(HyprlandBackend::client_matches_query(
            "jet brains",
            "jetbrains-rustrover",
            "RustRover"
        ));
        assert!(!HyprlandBackend::client_matches_query("x", "code", "Code"));
    }

    #[test]
    fn parse_cursor_pos_extracts_coordinates() {
        assert_eq!(
            HyprlandBackend::parse_cursor_pos(r#"{"x": 101.2, "y": 44.8}"#),
            Some((101, 45))
        );
    }

    #[test]
    fn normalize_layout_label_maps_common_values() {
        assert_eq!(
            HyprlandBackend::normalize_layout_label("English (US)"),
            "US"
        );
        assert_eq!(HyprlandBackend::normalize_layout_label("Russian"), "RU");
        assert_eq!(HyprlandBackend::normalize_layout_label("de"), "DE");
    }

    #[test]
    fn dispatch_succeeded_accepts_ok() {
        assert!(HyprlandBackend::dispatch_succeeded("ok"));
        assert!(HyprlandBackend::dispatch_succeeded("ok\n"));
        assert!(HyprlandBackend::dispatch_succeeded("OK"));
    }

    #[test]
    fn dispatch_succeeded_rejects_errors() {
        assert!(!HyprlandBackend::dispatch_succeeded(""));
        assert!(!HyprlandBackend::dispatch_succeeded("invalid dispatcher"));
        assert!(!HyprlandBackend::dispatch_succeeded("unknown layout"));
    }
}
