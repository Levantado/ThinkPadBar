use iced::Subscription;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::time::Instant;

pub use crate::modules::workspaces::WorkspaceInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompositorBackendKind {
    #[default]
    Hyprland,
    Niri,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositorEvent {
    StateChanged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompositorSnapshot {
    pub workspaces: Vec<WorkspaceInfo>,
    pub active_window: String,
    pub special_workspace_visible: bool,
    pub keyboard_layout: String,
    pub configured_backend: CompositorBackendKind,
    pub active_backend: CompositorBackendKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefreshResult {
    pub snapshot: CompositorSnapshot,
    pub elapsed_ms: u64,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct WorkspaceRefreshCoalescer {
    inflight: bool,
    queued: bool,
}

impl WorkspaceRefreshCoalescer {
    pub fn request(&mut self) -> bool {
        if self.inflight {
            self.queued = true;
            return false;
        }
        self.inflight = true;
        true
    }

    pub fn complete(&mut self) -> bool {
        self.inflight = false;
        if self.queued {
            self.queued = false;
            return true;
        }
        false
    }

    #[cfg(test)]
    fn state(&self) -> (bool, bool) {
        (self.inflight, self.queued)
    }
}

pub trait CompositorBackend {
    fn get_workspaces(&self) -> Vec<WorkspaceInfo>;
    fn is_special_workspace_visible(&self) -> bool;
    fn get_active_window_title(&self) -> String;
    fn get_keyboard_layout(&self) -> String;
    fn switch_workspace(&self, id: i32, name: &str);
    fn next_keyboard_layout(&self);
    fn subscription(&self) -> Subscription<CompositorEvent>;
    fn cursor_position(&self) -> Option<(i32, i32)>;
    fn find_and_switch_to_app(&self, name: String) -> Pin<Box<dyn Future<Output = bool> + Send>>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct HyprlandBackend;

impl HyprlandBackend {
    fn parse_cursor_pos(raw: &str) -> Option<(i32, i32)> {
        let value = serde_json::from_str::<Value>(raw).ok()?;
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

    fn get_keyboard_layout(&self) -> String {
        crate::modules::keyboard::get_layout()
    }

    fn switch_workspace(&self, id: i32, name: &str) {
        crate::modules::workspaces::switch_workspace(id, name);
    }

    fn next_keyboard_layout(&self) {
        crate::modules::keyboard::next_layout();
    }

    fn subscription(&self) -> Subscription<CompositorEvent> {
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

#[derive(Debug, Default, Clone, Copy)]
pub struct CompositorService {
    backend: HyprlandBackend,
    refresh: WorkspaceRefreshCoalescer,
    configured_backend: CompositorBackendKind,
    active_backend: CompositorBackendKind,
}

impl CompositorService {
    pub fn new(config: &crate::config::CompositorConfig) -> Self {
        let configured_backend = match config.backend.trim().to_ascii_lowercase().as_str() {
            "niri" => CompositorBackendKind::Niri,
            _ => CompositorBackendKind::Hyprland,
        };
        let active_backend = match configured_backend {
            CompositorBackendKind::Hyprland => CompositorBackendKind::Hyprland,
            CompositorBackendKind::Niri => CompositorBackendKind::Hyprland,
        };
        Self {
            backend: HyprlandBackend,
            refresh: WorkspaceRefreshCoalescer::default(),
            configured_backend,
            active_backend,
        }
    }

    pub fn snapshot(&self) -> CompositorSnapshot {
        CompositorSnapshot {
            workspaces: self.backend.get_workspaces(),
            active_window: self.backend.get_active_window_title(),
            special_workspace_visible: self.backend.is_special_workspace_visible(),
            keyboard_layout: self.backend.get_keyboard_layout(),
            configured_backend: self.configured_backend,
            active_backend: self.active_backend,
        }
    }

    pub async fn refresh(&self) -> RefreshResult {
        let started = Instant::now();
        RefreshResult {
            snapshot: self.snapshot(),
            elapsed_ms: started.elapsed().as_millis() as u64,
        }
    }

    pub fn request_refresh(&mut self) -> bool {
        self.refresh.request()
    }

    pub fn complete_refresh(&mut self) -> bool {
        self.refresh.complete()
    }

    pub fn switch_workspace(&self, id: i32, name: &str) {
        self.backend.switch_workspace(id, name);
    }

    pub fn next_keyboard_layout(&self) {
        self.backend.next_keyboard_layout();
    }

    pub fn subscription(&self) -> Subscription<CompositorEvent> {
        self.backend.subscription()
    }

    pub fn cursor_position(&self) -> Option<(i32, i32)> {
        self.backend.cursor_position()
    }

    pub async fn find_and_switch_to_app(&self, name: String) -> bool {
        self.backend.find_and_switch_to_app(name).await
    }
}

pub fn cursor_position() -> Option<(i32, i32)> {
    CompositorService::new(&crate::config::CompositorConfig::default()).cursor_position()
}

#[cfg(test)]
mod tests {
    use super::{CompositorBackendKind, CompositorService, WorkspaceRefreshCoalescer};

    #[test]
    fn cursor_position_absent_without_runtime_env() {
        let _ =
            CompositorService::new(&crate::config::CompositorConfig::default()).cursor_position();
    }

    #[test]
    fn refresh_coalescer_behaves_deterministically() {
        let mut coalescer = WorkspaceRefreshCoalescer::default();
        assert_eq!(coalescer.state(), (false, false));
        assert!(coalescer.request());
        assert_eq!(coalescer.state(), (true, false));
        assert!(!coalescer.request());
        assert_eq!(coalescer.state(), (true, true));
        assert!(coalescer.complete());
        assert_eq!(coalescer.state(), (false, false));
        assert!(!coalescer.complete());
        assert_eq!(coalescer.state(), (false, false));
    }

    #[test]
    fn service_snapshot_call_is_available() {
        let service = CompositorService::new(&crate::config::CompositorConfig::default());
        let snapshot = service.snapshot();
        assert_eq!(snapshot.configured_backend, CompositorBackendKind::Hyprland);
        assert_eq!(snapshot.active_backend, CompositorBackendKind::Hyprland);
    }

    #[test]
    fn niri_config_falls_back_to_hyprland_backend() {
        let service = CompositorService::new(&crate::config::CompositorConfig {
            backend: "niri".to_string(),
        });
        let snapshot = service.snapshot();
        assert_eq!(snapshot.configured_backend, CompositorBackendKind::Niri);
        assert_eq!(snapshot.active_backend, CompositorBackendKind::Hyprland);
    }
}
