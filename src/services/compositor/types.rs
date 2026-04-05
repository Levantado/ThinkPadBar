#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceInfo {
    pub id: i32,
    pub name: String,
    pub active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompositorBackendKind {
    #[default]
    Hyprland,
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
