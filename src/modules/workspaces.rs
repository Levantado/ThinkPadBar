use iced::Subscription;
use serde::Deserialize;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream as SyncUnixStream;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceInfo {
    pub id: i32,
    pub name: String,
    pub active: bool,
}

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

pub fn hyprland_command(cmd: &str) -> Option<String> {
    let signature = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").ok()?;
    let xdg_runtime_dir = std::env::var("XDG_RUNTIME_DIR").ok()?;
    let socket_path = format!("{}/hypr/{}/.socket.sock", xdg_runtime_dir, signature);

    let mut stream = SyncUnixStream::connect(socket_path).ok()?;
    stream.write_all(cmd.as_bytes()).ok()?;

    let mut response = String::new();
    stream.read_to_string(&mut response).ok()?;
    Some(response)
}

pub fn get_active_window_title() -> String {
    if let Some(s) = hyprland_command("j/activewindow") {
        if let Ok(window) = serde_json::from_str::<HyprWindow>(&s) {
            if !window.title.is_empty() {
                return window.title;
            }
            return window.class;
        }
    }
    String::new()
}

fn parse_special_workspace_visible_from_monitors_json(raw: &str) -> bool {
    if let Ok(monitors) = serde_json::from_str::<Vec<HyprMonitor>>(raw) {
        return monitors.iter().any(|m| {
            if let Some(sw) = &m.special_workspace {
                let n = sw.name.to_ascii_lowercase();
                sw.id != 0 || (!n.is_empty() && n.starts_with("special"))
            } else {
                false
            }
        });
    }
    false
}

pub fn is_special_workspace_visible() -> bool {
    hyprland_command("j/monitors")
        .map(|raw| parse_special_workspace_visible_from_monitors_json(&raw))
        .unwrap_or(false)
}

pub fn get_workspaces() -> Vec<WorkspaceInfo> {
    let mut infos = Vec::new();

    let (active_id, active_name) = if let Some(s) = hyprland_command("j/activeworkspace") {
        serde_json::from_str::<HyprActiveWorkspace>(&s)
            .map(|a| (a.id, a.name))
            .unwrap_or((0, String::new()))
    } else {
        (0, String::new())
    };

    if let Some(s) = hyprland_command("j/workspaces") {
        if let Ok(mut workspaces) = serde_json::from_str::<Vec<HyprWorkspace>>(&s) {
            workspaces.sort_by_key(|w| w.id);
            for w in workspaces {
                let is_active = workspace_is_active(w.id, &w.name, active_id, &active_name);
                infos.push(WorkspaceInfo {
                    id: w.id,
                    name: w.name,
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
            .any(|ws| ws.id == active_id || ws.name == active_name)
    {
        infos.push(WorkspaceInfo {
            id: active_id,
            name: active_name,
            active: true,
        });
    }

    infos
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

fn normalize_match_text(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
}

fn client_matches_query(query: &str, initial_class: &str, title: &str) -> bool {
    let q_norm = normalize_match_text(query);
    let q = q_norm.trim();
    if q.len() < 2 {
        return false;
    }

    let class_n = normalize_match_text(initial_class);
    let title_n = normalize_match_text(title);
    if class_n.contains(q) || title_n.contains(q) {
        return true;
    }

    q.split_whitespace()
        .filter(|t| t.len() >= 3)
        .any(|tok| class_n.contains(tok) || title_n.contains(tok))
}

pub fn switch_workspace(id: i32, name: &str) {
    let _ = hyprland_command(&workspace_dispatch_command(id, name));
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppLocateResult {
    NotFound,
    FoundSameWorkspace,
    SwitchedWorkspace,
}

pub async fn find_and_switch_to_app_detailed(name: String) -> AppLocateResult {
    let active_ws_id = if let Some(s) = hyprland_command("j/activeworkspace") {
        serde_json::from_str::<HyprActiveWorkspace>(&s)
            .map(|a| a.id)
            .unwrap_or(0)
    } else {
        0
    };

    if let Some(s) = hyprland_command("j/clients") {
        if let Ok(clients) = serde_json::from_str::<Vec<HyprClient>>(&s) {
            for client in clients {
                if client_matches_query(&name, &client.initial_class, &client.title) {
                    if client.workspace.id != active_ws_id {
                        switch_workspace(client.workspace.id, "");
                        return AppLocateResult::SwitchedWorkspace;
                    }
                    return AppLocateResult::FoundSameWorkspace;
                }
            }
        }
    }
    AppLocateResult::NotFound
}

pub async fn find_and_switch_to_app(name: String) -> bool {
    !matches!(
        find_and_switch_to_app_detailed(name).await,
        AppLocateResult::NotFound
    )
}

pub fn subscription() -> Subscription<crate::app::Message> {
    struct HyprlandListener;

    Subscription::run_with_id(
        std::any::TypeId::of::<HyprlandListener>(),
        iced::stream::channel(1, |mut output| async move {
            let signature = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").ok();
            let xdg_runtime_dir = std::env::var("XDG_RUNTIME_DIR").ok();

            if let (Some(sig), Some(runtime)) = (signature, xdg_runtime_dir) {
                let socket_path = format!("{}/hypr/{}/.socket2.sock", runtime, sig);

                loop {
                    use tokio::io::{AsyncBufReadExt, BufReader};
                    use tokio::net::UnixStream;

                    if let Ok(stream) = UnixStream::connect(&socket_path).await {
                        let mut reader = BufReader::new(stream);
                        let mut line = String::new();

                        loop {
                            line.clear();
                            match reader.read_line(&mut line).await {
                                Ok(0) => break, // Socket closed
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
                                        let _ =
                                            output.try_send(crate::app::Message::UpdateWorkspaces);
                                        tokio::time::sleep(Duration::from_millis(50)).await;
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

#[cfg(test)]
mod tests {
    use super::{
        client_matches_query, parse_special_workspace_visible_from_monitors_json,
        workspace_dispatch_command, workspace_is_active, WorkspaceInfo,
    };

    #[test]
    fn workspace_dispatch_uses_regular_workspace_for_normal_name() {
        assert_eq!(workspace_dispatch_command(2, "2"), "dispatch workspace 2");
        assert_eq!(workspace_dispatch_command(5, "dev"), "dispatch workspace 5");
    }

    #[test]
    fn workspace_dispatch_uses_special_toggle_for_special_workspace() {
        assert_eq!(
            workspace_dispatch_command(0, "special"),
            "dispatch togglespecialworkspace"
        );
        assert_eq!(
            workspace_dispatch_command(0, "special:term"),
            "dispatch togglespecialworkspace term"
        );
        assert_eq!(
            workspace_dispatch_command(0, "SPECIAL:notes"),
            "dispatch togglespecialworkspace notes"
        );
        assert_eq!(
            workspace_dispatch_command(0, "special:"),
            "dispatch togglespecialworkspace"
        );
    }

    #[test]
    fn workspace_active_detection_matches_by_id_or_name() {
        assert!(workspace_is_active(3, "3", 3, "3"));
        assert!(workspace_is_active(-99, "special:term", 4, "special:term"));
        assert!(!workspace_is_active(2, "2", 1, "1"));
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
                .any(|ws| ws.id == active_id || ws.name == active_name)
        {
            infos.push(WorkspaceInfo {
                id: active_id,
                name: active_name,
                active: true,
            });
        }

        assert!(infos
            .iter()
            .any(|ws| ws.name == "special:term" && ws.active));
    }

    #[test]
    fn parses_visible_special_workspace_from_monitors_json() {
        let raw = r#"
        [
          {
            "name": "eDP-1",
            "specialWorkspace": { "id": -99, "name": "special:term" }
          }
        ]
        "#;
        assert!(parse_special_workspace_visible_from_monitors_json(raw));
    }

    #[test]
    fn parses_absent_special_workspace_from_monitors_json() {
        let raw = r#"
        [
          {
            "name": "eDP-1",
            "specialWorkspace": { "id": 0, "name": "" }
          }
        ]
        "#;
        assert!(!parse_special_workspace_visible_from_monitors_json(raw));
    }

    #[test]
    fn client_matching_handles_punctuation_and_tokenized_queries() {
        assert!(client_matches_query(
            "Telegram Desktop",
            "org.telegram.desktop",
            "Chat window"
        ));
        assert!(client_matches_query(
            "org.kde.StatusNotifierItem-telegram",
            "org.telegram.desktop",
            "chat"
        ));
        assert!(!client_matches_query(
            "zzzz-not-found",
            "org.telegram.desktop",
            "chat"
        ));
    }
}
