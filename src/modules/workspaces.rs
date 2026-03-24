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

pub fn get_workspaces() -> Vec<WorkspaceInfo> {
    let mut infos = Vec::new();

    let active_id = if let Some(s) = hyprland_command("j/activeworkspace") {
        serde_json::from_str::<HyprActiveWorkspace>(&s)
            .map(|a| a.id)
            .unwrap_or(0)
    } else {
        0
    };

    if let Some(s) = hyprland_command("j/workspaces") {
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
    let _ = hyprland_command(&format!("dispatch workspace {}", id));
}

pub async fn find_and_switch_to_app(name: String) -> bool {
    if let Some(s) = hyprland_command("j/clients") {
        if let Ok(clients) = serde_json::from_str::<Vec<HyprClient>>(&s) {
            let name_lower = name.to_lowercase();
            for client in clients {
                if client.initial_class.to_lowercase().contains(&name_lower)
                    || client.title.to_lowercase().contains(&name_lower)
                {
                    switch_workspace(client.workspace.id);
                    return true;
                }
            }
        }
    }
    false
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
