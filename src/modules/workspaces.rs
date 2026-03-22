use iced::Subscription;
use serde::Deserialize;
use std::process::Command;
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

pub fn get_active_window_title() -> String {
    if let Ok(out) = Command::new("hyprctl").args(["activewindow", "-j"]).output() {
        if let Ok(s) = String::from_utf8(out.stdout) {
            if let Ok(window) = serde_json::from_str::<HyprWindow>(&s) {
                if !window.title.is_empty() {
                    return window.title;
                }
                return window.class;
            }
        }
    }
    String::new()
}

pub fn get_workspaces() -> Vec<WorkspaceInfo> {
    let mut infos = Vec::new();

    // Get active workspace ID
    let mut active_id = 0;
    if let Ok(out) = Command::new("hyprctl").args(["activeworkspace", "-j"]).output() {
        if let Ok(s) = String::from_utf8(out.stdout) {
            if let Ok(active) = serde_json::from_str::<HyprActiveWorkspace>(&s) {
                active_id = active.id;
            }
        }
    }

    // Get all workspaces
    if let Ok(out) = Command::new("hyprctl").args(["workspaces", "-j"]).output() {
        if let Ok(s) = String::from_utf8(out.stdout) {
            if let Ok(mut workspaces) = serde_json::from_str::<Vec<HyprWorkspace>>(&s) {
                workspaces.sort_by_key(|w| w.id);
                for w in workspaces {
                    infos.push(WorkspaceInfo {
                        id: w.id,
                        name: w.name,
                        active: w.id == active_id,
                    });
                }
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

    infos
}

pub fn switch_workspace(id: i32) {
    let _ = Command::new("hyprctl")
        .args(["dispatch", "workspace", &id.to_string()])
        .output();
}

pub async fn find_and_switch_to_app(name: String) -> bool {
    if let Ok(out) = Command::new("hyprctl").args(["clients", "-j"]).output() {
        if let Ok(s) = String::from_utf8(out.stdout) {
            if let Ok(clients) = serde_json::from_str::<Vec<HyprClient>>(&s) {
                let name_lower = name.to_lowercase();
                for client in clients {
                    if client.initial_class.to_lowercase().contains(&name_lower) 
                       || client.title.to_lowercase().contains(&name_lower) {
                        switch_workspace(client.workspace.id);
                        return true;
                    }
                }
            }
        }
    }
    false
}

pub fn tick() -> Subscription<crate::app::Message> {
    iced::time::every(Duration::from_millis(250))
        .map(|_| crate::app::Message::UpdateWorkspaces)
}
