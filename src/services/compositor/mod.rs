mod hyprland;
pub mod types;

use iced::Subscription;
use std::time::Instant;

pub use types::{CompositorBackendKind, CompositorEvent, CompositorSnapshot, RefreshResult};

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

#[derive(Debug, Clone)]
pub struct CompositorService {
    backend: hyprland::HyprlandBackend,
    snapshot: CompositorSnapshot,
    refresh: WorkspaceRefreshCoalescer,
}

impl Default for CompositorService {
    fn default() -> Self {
        Self::new(&crate::config::CompositorConfig::default())
    }
}

impl CompositorService {
    pub fn new(config: &crate::config::CompositorConfig) -> Self {
        let configured_backend = match config.backend.trim().to_ascii_lowercase().as_str() {
            "niri" => CompositorBackendKind::Niri,
            _ => CompositorBackendKind::Hyprland,
        };
        let active_backend = CompositorBackendKind::Hyprland;
        let backend = hyprland::HyprlandBackend::new();
        let snapshot = CompositorSnapshot {
            workspaces: backend.get_workspaces(),
            active_window: backend.get_active_window_title(),
            special_workspace_visible: backend.is_special_workspace_visible(),
            keyboard_layout: backend.get_keyboard_layout(),
            configured_backend,
            active_backend,
        };
        Self {
            backend,
            snapshot,
            refresh: WorkspaceRefreshCoalescer::default(),
        }
    }

    pub fn snapshot(&self) -> &CompositorSnapshot {
        &self.snapshot
    }

    fn collect_snapshot(&self) -> CompositorSnapshot {
        CompositorSnapshot {
            workspaces: self.backend.get_workspaces(),
            active_window: self.backend.get_active_window_title(),
            special_workspace_visible: self.backend.is_special_workspace_visible(),
            keyboard_layout: self.backend.get_keyboard_layout(),
            configured_backend: self.snapshot.configured_backend,
            active_backend: self.snapshot.active_backend,
        }
    }

    pub async fn refresh(&self) -> RefreshResult {
        let started = Instant::now();
        RefreshResult {
            snapshot: self.collect_snapshot(),
            elapsed_ms: started.elapsed().as_millis() as u64,
        }
    }

    pub fn apply_refresh(&mut self, refresh: RefreshResult) {
        self.snapshot = refresh.snapshot;
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
    hyprland::HyprlandBackend::new().cursor_position()
}

#[cfg(test)]
mod tests {
    use super::{CompositorBackendKind, CompositorService, WorkspaceRefreshCoalescer};

    #[test]
    fn cursor_position_absent_without_runtime_env() {
        let _ = CompositorService::default().cursor_position();
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
        let service = CompositorService::default();
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
