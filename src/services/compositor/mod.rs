mod hyprland;
pub mod types;

use iced::Subscription;
use std::time::Instant;

pub use types::{CompositorBackendKind, CompositorEvent, CompositorSnapshot, RefreshResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompositorDiagnostics {
    pub configured_backend: CompositorBackendKind,
    pub active_backend: CompositorBackendKind,
    pub refresh_inflight: bool,
    pub refresh_queued: bool,
    pub last_refresh_ms: Option<u64>,
    pub unavailable_reason: Option<String>,
}

impl CompositorDiagnostics {
    pub fn summary(&self) -> String {
        let why = self.unavailable_reason.as_deref().unwrap_or("-");
        let last = self
            .last_refresh_ms
            .map_or_else(|| "-".to_string(), |elapsed| elapsed.to_string());
        format!(
            "cfg {:?} act {:?} inflight:{} queued:{} last:{} why:{}",
            self.configured_backend,
            self.active_backend,
            self.refresh_inflight,
            self.refresh_queued,
            last,
            why
        )
    }
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

    fn state(&self) -> (bool, bool) {
        (self.inflight, self.queued)
    }
}

#[derive(Debug, Clone)]
pub struct CompositorService {
    backend: hyprland::HyprlandBackend,
    snapshot: CompositorSnapshot,
    refresh: WorkspaceRefreshCoalescer,
    last_refresh_ms: Option<u64>,
}

impl Default for CompositorService {
    fn default() -> Self {
        Self::new(&crate::config::CompositorConfig::default())
    }
}

impl CompositorService {
    fn resolve_backend_kinds(
        config: &crate::config::CompositorConfig,
    ) -> (CompositorBackendKind, CompositorBackendKind) {
        let configured_backend = match config.backend.trim().to_ascii_lowercase().as_str() {
            "niri" => CompositorBackendKind::Niri,
            _ => CompositorBackendKind::Hyprland,
        };
        let active_backend = CompositorBackendKind::Hyprland;
        (configured_backend, active_backend)
    }

    pub fn new(config: &crate::config::CompositorConfig) -> Self {
        let (configured_backend, active_backend) = Self::resolve_backend_kinds(config);
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
            last_refresh_ms: None,
        }
    }

    pub fn snapshot(&self) -> &CompositorSnapshot {
        &self.snapshot
    }

    pub fn diagnostics(&self) -> CompositorDiagnostics {
        let (refresh_inflight, refresh_queued) = self.refresh.state();
        CompositorDiagnostics {
            configured_backend: self.snapshot.configured_backend,
            active_backend: self.snapshot.active_backend,
            refresh_inflight,
            refresh_queued,
            last_refresh_ms: self.last_refresh_ms,
            unavailable_reason: (self.snapshot.configured_backend != self.snapshot.active_backend)
                .then(|| {
                    format!(
                        "configured {:?}, runtime {:?}: backend fallback active",
                        self.snapshot.configured_backend, self.snapshot.active_backend
                    )
                }),
        }
    }

    pub fn capability_status(&self) -> crate::services::capabilities::CapabilityStatus {
        crate::services::capabilities::CapabilityStatus {
            key: "cmp",
            label: "Compositor",
            mode: if self.snapshot.configured_backend == self.snapshot.active_backend {
                crate::services::capabilities::CapabilityMode::Native
            } else {
                crate::services::capabilities::CapabilityMode::Fallback
            },
            provider: format!(
                "{:?}->{:?}",
                self.snapshot.configured_backend, self.snapshot.active_backend
            ),
            detail: (self.snapshot.configured_backend != self.snapshot.active_backend)
                .then(|| "configured backend falls back to active runtime backend".to_string()),
        }
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
        self.last_refresh_ms = Some(refresh.elapsed_ms);
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

    #[cfg(test)]
    pub fn hermetic_for_tests(
        configured_backend: CompositorBackendKind,
        active_backend: CompositorBackendKind,
    ) -> Self {
        Self {
            backend: hyprland::HyprlandBackend::unavailable_for_tests(),
            snapshot: CompositorSnapshot {
                workspaces: Vec::new(),
                active_window: String::new(),
                special_workspace_visible: false,
                keyboard_layout: "N/A".to_string(),
                configured_backend,
                active_backend,
            },
            refresh: WorkspaceRefreshCoalescer::default(),
            last_refresh_ms: None,
        }
    }
}

pub fn cursor_position() -> Option<(i32, i32)> {
    hyprland::HyprlandBackend::new().cursor_position()
}

#[cfg(test)]
mod tests {
    use super::{
        CompositorBackendKind, CompositorService, RefreshResult, WorkspaceRefreshCoalescer,
    };

    #[test]
    fn cursor_position_absent_without_runtime_env() {
        assert_eq!(
            CompositorService::hermetic_for_tests(
                CompositorBackendKind::Hyprland,
                CompositorBackendKind::Hyprland,
            )
            .cursor_position(),
            None
        );
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
        let service = CompositorService::hermetic_for_tests(
            CompositorBackendKind::Hyprland,
            CompositorBackendKind::Hyprland,
        );
        let snapshot = service.snapshot();
        assert_eq!(snapshot.configured_backend, CompositorBackendKind::Hyprland);
        assert_eq!(snapshot.active_backend, CompositorBackendKind::Hyprland);
    }

    #[test]
    fn backend_kind_resolution_falls_back_from_niri_config_to_hyprland_runtime() {
        let (configured_backend, active_backend) =
            CompositorService::resolve_backend_kinds(&crate::config::CompositorConfig {
                backend: "niri".to_string(),
            });
        assert_eq!(configured_backend, CompositorBackendKind::Niri);
        assert_eq!(active_backend, CompositorBackendKind::Hyprland);
    }

    #[test]
    fn diagnostics_report_backend_fallback_and_refresh_state() {
        let mut service = CompositorService::hermetic_for_tests(
            CompositorBackendKind::Niri,
            CompositorBackendKind::Hyprland,
        );
        assert!(service.request_refresh());
        assert!(!service.request_refresh());

        let diagnostics = service.diagnostics();
        assert!(diagnostics.refresh_inflight);
        assert!(diagnostics.refresh_queued);
        assert_eq!(
            diagnostics.unavailable_reason.as_deref(),
            Some("configured Niri, runtime Hyprland: backend fallback active")
        );
    }

    #[test]
    fn diagnostics_capture_last_refresh_elapsed_ms() {
        let mut service = CompositorService::hermetic_for_tests(
            CompositorBackendKind::Hyprland,
            CompositorBackendKind::Hyprland,
        );
        service.apply_refresh(RefreshResult {
            snapshot: service.snapshot().clone(),
            elapsed_ms: 17,
        });

        assert_eq!(service.diagnostics().last_refresh_ms, Some(17));
    }

    #[test]
    fn capability_status_reports_backend_fallback() {
        let service = CompositorService::hermetic_for_tests(
            CompositorBackendKind::Niri,
            CompositorBackendKind::Hyprland,
        );

        let status = service.capability_status();

        assert_eq!(status.key, "cmp");
        assert_eq!(
            status.mode,
            crate::services::capabilities::CapabilityMode::Fallback
        );
        assert_eq!(status.provider, "Niri->Hyprland");
        assert_eq!(
            status.detail.as_deref(),
            Some("configured backend falls back to active runtime backend")
        );
    }
}
