use iced::Subscription;
use std::future::Future;
use std::pin::Pin;

pub use crate::modules::workspaces::WorkspaceInfo;

pub trait CompositorBackend {
    fn get_workspaces(&self) -> Vec<WorkspaceInfo>;
    fn is_special_workspace_visible(&self) -> bool;
    fn get_active_window_title(&self) -> String;
    fn switch_workspace(&self, id: i32, name: &str);
    fn subscription(&self) -> Subscription<crate::app::Message>;
    fn cursor_position(&self) -> Option<(i32, i32)>;
    fn find_and_switch_to_app(&self, name: String) -> Pin<Box<dyn Future<Output = bool> + Send>>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct HyprlandBackend;

impl HyprlandBackend {
    fn parse_cursor_pos(raw: &str) -> Option<(i32, i32)> {
        let value = serde_json::from_str::<serde_json::Value>(raw).ok()?;
        let x = value.get("x")?.as_f64()?.round() as i32;
        let y = value.get("y")?.as_f64()?.round() as i32;
        Some((x, y))
    }
}

impl CompositorBackend for HyprlandBackend {
    fn get_workspaces(&self) -> Vec<WorkspaceInfo> {
        crate::modules::workspaces::get_workspaces()
    }

    fn is_special_workspace_visible(&self) -> bool {
        crate::modules::workspaces::is_special_workspace_visible()
    }

    fn get_active_window_title(&self) -> String {
        crate::modules::workspaces::get_active_window_title()
    }

    fn switch_workspace(&self, id: i32, name: &str) {
        crate::modules::workspaces::switch_workspace(id, name);
    }

    fn subscription(&self) -> Subscription<crate::app::Message> {
        crate::modules::workspaces::subscription()
    }

    fn cursor_position(&self) -> Option<(i32, i32)> {
        crate::modules::workspaces::hyprland_command("j/cursorpos")
            .and_then(|raw| Self::parse_cursor_pos(&raw))
    }

    fn find_and_switch_to_app(&self, name: String) -> Pin<Box<dyn Future<Output = bool> + Send>> {
        Box::pin(async move { crate::modules::workspaces::find_and_switch_to_app(name).await })
    }
}

fn active_backend() -> HyprlandBackend {
    HyprlandBackend
}

pub fn get_workspaces() -> Vec<WorkspaceInfo> {
    active_backend().get_workspaces()
}

pub fn is_special_workspace_visible() -> bool {
    active_backend().is_special_workspace_visible()
}

pub fn get_active_window_title() -> String {
    active_backend().get_active_window_title()
}

pub fn switch_workspace(id: i32, name: &str) {
    active_backend().switch_workspace(id, name);
}

pub fn subscription() -> Subscription<crate::app::Message> {
    active_backend().subscription()
}

pub fn cursor_position() -> Option<(i32, i32)> {
    active_backend().cursor_position()
}

pub async fn find_and_switch_to_app(name: String) -> bool {
    active_backend().find_and_switch_to_app(name).await
}

#[cfg(test)]
mod tests {
    use super::{cursor_position, HyprlandBackend};

    #[test]
    fn cursor_position_absent_without_runtime_env() {
        // In unit-test environment Hyprland runtime is typically absent.
        let _ = cursor_position();
        let _ = HyprlandBackend;
    }
}
