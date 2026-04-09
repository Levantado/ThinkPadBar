use iced::futures::SinkExt;
use wayland_client::{
    protocol::{
        wl_compositor::WlCompositor,
        wl_output::{self, WlOutput},
        wl_registry::{self, WlRegistry},
        wl_shm::WlShm,
    },
    Connection, Dispatch, EventQueue, Proxy, QueueHandle, WEnum,
};
use wayland_protocols::xdg::shell::client::xdg_wm_base::XdgWmBase;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WaylandRuntimeEvent {
    SnapshotUpdated(WaylandRuntimeSnapshot),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WaylandOutputInfo {
    pub global_name: u32,
    pub version: u32,
    pub name: Option<String>,
    pub description: Option<String>,
    pub make: Option<String>,
    pub model: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub refresh_mhz: Option<i32>,
    pub scale_factor: Option<i32>,
}

impl WaylandOutputInfo {
    pub fn label(&self) -> String {
        if let Some(name) = self.name.as_deref().filter(|name| !name.is_empty()) {
            return name.to_string();
        }
        if let Some(description) = self
            .description
            .as_deref()
            .filter(|description| !description.is_empty())
        {
            return description.to_string();
        }

        match (
            self.make.as_deref().filter(|make| !make.is_empty()),
            self.model.as_deref().filter(|model| !model.is_empty()),
        ) {
            (Some(make), Some(model)) => format!("{make} {model}"),
            (Some(make), None) => make.to_string(),
            (None, Some(model)) => model.to_string(),
            (None, None) => format!("output-{}", self.global_name),
        }
    }

    pub fn detail_label(&self) -> String {
        let mut parts = vec![self.label()];
        if let (Some(width), Some(height)) = (self.width, self.height) {
            parts.push(format!("{width}x{height}"));
        }
        if let Some(refresh_mhz) = self.refresh_mhz {
            let refresh_hz = refresh_mhz as f64 / 1000.0;
            if (refresh_hz.fract() - 0.0).abs() < f64::EPSILON {
                parts.push(format!("{refresh_hz:.0}Hz"));
            } else {
                parts.push(format!("{refresh_hz:.1}Hz"));
            }
        }
        if let Some(scale_factor) = self.scale_factor {
            if scale_factor > 0 {
                parts.push(format!("{scale_factor}x"));
            }
        }
        parts.join(" ")
    }

    pub fn is_internal(&self) -> bool {
        let Some(name) = self.name.as_deref() else {
            return false;
        };
        let upper = name.to_ascii_uppercase();
        upper.starts_with("EDP") || upper.starts_with("LVDS") || upper.starts_with("DSI")
    }

    pub fn scale_label(&self) -> Option<String> {
        let scale_factor = self.scale_factor?;
        (scale_factor > 0).then(|| format!("{} {}x", self.label(), scale_factor))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WaylandRuntimeSnapshot {
    pub available: bool,
    pub compositor_version: Option<u32>,
    pub shm_version: Option<u32>,
    pub xdg_wm_base_version: Option<u32>,
    pub layer_shell_version: Option<u32>,
    pub idle_inhibit_version: Option<u32>,
    pub outputs: Vec<WaylandOutputInfo>,
    pub unavailable_reason: Option<String>,
}

impl WaylandRuntimeSnapshot {
    pub fn runtime_summary(&self) -> String {
        format!(
            "avail:{} outputs:{} cmp:{} shm:{} xdg:{} layer:{} idle:{}",
            self.available,
            self.outputs.len(),
            version_or_dash(self.compositor_version),
            version_or_dash(self.shm_version),
            version_or_dash(self.xdg_wm_base_version),
            version_or_dash(self.layer_shell_version),
            version_or_dash(self.idle_inhibit_version),
        )
    }

    pub fn capability_summary(&self) -> String {
        format!(
            "wl_compositor:{} wl_shm:{} wl_output:{} xdg_wm_base:{} layer_shell:{} idle_inhibit:{}",
            version_or_missing(self.compositor_version),
            version_or_missing(self.shm_version),
            if self.outputs.is_empty() {
                "missing".to_string()
            } else {
                self.outputs.len().to_string()
            },
            version_or_missing(self.xdg_wm_base_version),
            version_or_missing(self.layer_shell_version),
            version_or_missing(self.idle_inhibit_version),
        )
    }

    pub fn output_summary(&self) -> String {
        if self.outputs.is_empty() {
            return if self.available {
                "No outputs".to_string()
            } else {
                "Wayland unavailable".to_string()
            };
        }

        let labels = self
            .outputs
            .iter()
            .map(WaylandOutputInfo::label)
            .collect::<Vec<_>>();
        format!("{} outputs: {}", labels.len(), labels.join(", "))
    }

    pub fn output_topology_summary(&self) -> String {
        if self.outputs.is_empty() {
            return if self.available {
                "No outputs".to_string()
            } else {
                "Wayland unavailable".to_string()
            };
        }

        let internal = self
            .outputs
            .iter()
            .filter(|output| output.is_internal())
            .count();
        let external = self.outputs.len().saturating_sub(internal);
        match (internal, external) {
            (0, 0) => "No outputs".to_string(),
            (0, external) => format!("{external} external"),
            (internal, 0) => format!("{internal} internal"),
            (internal, external) => format!("{internal} internal + {external} external"),
        }
    }

    pub fn display_mode_summary(&self) -> String {
        if self.outputs.is_empty() {
            return if self.available {
                "Headless".to_string()
            } else {
                "Wayland unavailable".to_string()
            };
        }

        let internal = self
            .outputs
            .iter()
            .filter(|output| output.is_internal())
            .count();
        let external = self.outputs.len().saturating_sub(internal);

        match (internal, external) {
            (0, 0) => "Headless".to_string(),
            (internal, 0) if internal > 0 => "Laptop".to_string(),
            (0, external) if external > 0 => "Docked".to_string(),
            (internal, external) if internal > 0 && external > 0 => "Hybrid".to_string(),
            _ => "Unknown".to_string(),
        }
    }

    pub fn output_detail_summary(&self) -> String {
        if self.outputs.is_empty() {
            return if self.available {
                "No outputs".to_string()
            } else {
                "Wayland unavailable".to_string()
            };
        }

        self.outputs
            .iter()
            .map(WaylandOutputInfo::detail_label)
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn output_scale_summary(&self) -> String {
        if self.outputs.is_empty() {
            return if self.available {
                "No outputs".to_string()
            } else {
                "Wayland unavailable".to_string()
            };
        }

        let labels = self
            .outputs
            .iter()
            .filter_map(WaylandOutputInfo::scale_label)
            .collect::<Vec<_>>();
        if labels.is_empty() {
            "Scale unavailable".to_string()
        } else {
            labels.join(", ")
        }
    }

    pub fn missing_capabilities(&self) -> Option<String> {
        let mut missing = Vec::new();
        if self.compositor_version.is_none() {
            missing.push("wl_compositor");
        }
        if self.shm_version.is_none() {
            missing.push("wl_shm");
        }
        if self.outputs.is_empty() {
            missing.push("wl_output");
        }
        if self.xdg_wm_base_version.is_none() {
            missing.push("xdg_wm_base");
        }
        if self.layer_shell_version.is_none() {
            missing.push("zwlr_layer_shell_v1");
        }
        if self.idle_inhibit_version.is_none() {
            missing.push("zwp_idle_inhibit_manager_v1");
        }
        (!missing.is_empty()).then(|| missing.join(", "))
    }

    pub fn capability_status(&self) -> crate::services::capabilities::CapabilityStatus {
        let detail = if self.available {
            self.missing_capabilities()
                .map(|missing| format!("missing protocols: {missing}"))
        } else {
            self.unavailable_reason.clone()
        };

        crate::services::capabilities::CapabilityStatus {
            key: "way",
            label: "Wayland Runtime",
            mode: if !self.available {
                crate::services::capabilities::CapabilityMode::Unavailable
            } else if detail.is_some() {
                crate::services::capabilities::CapabilityMode::Hybrid
            } else {
                crate::services::capabilities::CapabilityMode::Native
            },
            provider: "wayland".to_string(),
            detail,
        }
    }
}

fn version_or_dash(version: Option<u32>) -> String {
    version.map_or_else(|| "-".to_string(), |version| format!("v{version}"))
}

fn version_or_missing(version: Option<u32>) -> String {
    version.map_or_else(|| "missing".to_string(), |version| format!("v{version}"))
}

#[derive(Default)]
struct WaylandRuntimeData {
    compositor_version: Option<u32>,
    shm_version: Option<u32>,
    xdg_wm_base_version: Option<u32>,
    layer_shell_version: Option<u32>,
    idle_inhibit_version: Option<u32>,
    outputs: Vec<WaylandOutputInfo>,
}

pub struct WaylandRuntimeService {
    snapshot: WaylandRuntimeSnapshot,
}

impl WaylandRuntimeService {
    pub fn new() -> Self {
        Self {
            snapshot: collect_snapshot().unwrap_or_else(|reason| WaylandRuntimeSnapshot {
                available: false,
                unavailable_reason: Some(reason),
                ..WaylandRuntimeSnapshot::default()
            }),
        }
    }

    pub fn snapshot(&self) -> &WaylandRuntimeSnapshot {
        &self.snapshot
    }

    pub fn capability_status(&self) -> crate::services::capabilities::CapabilityStatus {
        self.snapshot.capability_status()
    }

    pub fn apply_snapshot(&mut self, snapshot: WaylandRuntimeSnapshot) {
        self.snapshot = snapshot;
    }

    pub fn subscription() -> iced::Subscription<WaylandRuntimeEvent> {
        struct WaylandRuntimeListener;

        iced::Subscription::run_with_id(
            std::any::TypeId::of::<WaylandRuntimeListener>(),
            iced::stream::channel(1, move |output| async move {
                std::thread::spawn(move || run_wayland_runtime_listener(output));
                std::future::pending::<()>().await;
            }),
        )
    }

    #[cfg(test)]
    pub fn unavailable_for_tests() -> Self {
        Self {
            snapshot: WaylandRuntimeSnapshot {
                available: false,
                unavailable_reason: Some("wayland connection unavailable".to_string()),
                ..WaylandRuntimeSnapshot::default()
            },
        }
    }

    #[cfg(test)]
    pub fn with_snapshot_for_tests(snapshot: WaylandRuntimeSnapshot) -> Self {
        Self { snapshot }
    }
}

fn collect_snapshot() -> Result<WaylandRuntimeSnapshot, String> {
    let connection = Connection::connect_to_env()
        .map_err(|err| format!("wayland connection unavailable: {err}"))?;
    let display = connection.display();
    let event_queue = connection.new_event_queue();
    let handle = event_queue.handle();
    let registry = display.get_registry(&handle, ());
    let mut state = WaylandRuntimeData::default();

    let mut queue: EventQueue<WaylandRuntimeData> = event_queue;
    let _ = registry;
    queue
        .roundtrip(&mut state)
        .map_err(|err| format!("wayland registry roundtrip failed: {err}"))?;
    queue
        .roundtrip(&mut state)
        .map_err(|err| format!("wayland output roundtrip failed: {err}"))?;

    Ok(WaylandRuntimeSnapshot {
        available: true,
        compositor_version: state.compositor_version,
        shm_version: state.shm_version,
        xdg_wm_base_version: state.xdg_wm_base_version,
        layer_shell_version: state.layer_shell_version,
        idle_inhibit_version: state.idle_inhibit_version,
        outputs: state.outputs,
        unavailable_reason: None,
    })
}

fn snapshot_from_state(state: &WaylandRuntimeData) -> WaylandRuntimeSnapshot {
    WaylandRuntimeSnapshot {
        available: true,
        compositor_version: state.compositor_version,
        shm_version: state.shm_version,
        xdg_wm_base_version: state.xdg_wm_base_version,
        layer_shell_version: state.layer_shell_version,
        idle_inhibit_version: state.idle_inhibit_version,
        outputs: state.outputs.clone(),
        unavailable_reason: None,
    }
}

fn run_wayland_runtime_listener(
    mut output: iced::futures::channel::mpsc::Sender<WaylandRuntimeEvent>,
) {
    let Ok(connection) = Connection::connect_to_env() else {
        return;
    };
    let display = connection.display();
    let event_queue = connection.new_event_queue();
    let handle = event_queue.handle();
    let registry = display.get_registry(&handle, ());
    let mut state = WaylandRuntimeData::default();
    let mut queue: EventQueue<WaylandRuntimeData> = event_queue;
    let _ = registry;

    if queue.roundtrip(&mut state).is_err() {
        return;
    }
    if queue.roundtrip(&mut state).is_err() {
        return;
    }

    let mut last_snapshot = snapshot_from_state(&state);
    let _ = iced::futures::executor::block_on(
        output.send(WaylandRuntimeEvent::SnapshotUpdated(last_snapshot.clone())),
    );

    loop {
        if queue.blocking_dispatch(&mut state).is_err() {
            break;
        }
        let snapshot = snapshot_from_state(&state);
        if snapshot != last_snapshot {
            last_snapshot = snapshot.clone();
            if iced::futures::executor::block_on(
                output.send(WaylandRuntimeEvent::SnapshotUpdated(snapshot)),
            )
            .is_err()
            {
                break;
            }
        }
    }
}

impl Dispatch<WlRegistry, ()> for WaylandRuntimeData {
    fn event(
        state: &mut Self,
        proxy: &WlRegistry,
        event: wl_registry::Event,
        _data: &(),
        _conn: &Connection,
        handle: &QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => {
                if interface == WlCompositor::interface().name {
                    state.compositor_version = Some(version);
                } else if interface == WlShm::interface().name {
                    state.shm_version = Some(version);
                } else if interface == WlOutput::interface().name {
                    let bound_version = version.min(4);
                    let _ = proxy.bind::<WlOutput, _, _>(name, bound_version, handle, name);
                    state.outputs.push(WaylandOutputInfo {
                        global_name: name,
                        version: bound_version,
                        ..WaylandOutputInfo::default()
                    });
                } else if interface == XdgWmBase::interface().name {
                    state.xdg_wm_base_version = Some(version);
                } else if interface == "zwlr_layer_shell_v1" {
                    state.layer_shell_version = Some(version);
                } else if interface == "zwp_idle_inhibit_manager_v1" {
                    state.idle_inhibit_version = Some(version);
                }
            }
            wl_registry::Event::GlobalRemove { name } => {
                state.outputs.retain(|output| output.global_name != name);
            }
            _ => {}
        }
    }
}

impl Dispatch<WlOutput, u32> for WaylandRuntimeData {
    fn event(
        state: &mut Self,
        _proxy: &WlOutput,
        event: wl_output::Event,
        global_name: &u32,
        _conn: &Connection,
        _handle: &QueueHandle<Self>,
    ) {
        let Some(output) = state
            .outputs
            .iter_mut()
            .find(|output| output.global_name == *global_name)
        else {
            return;
        };

        match event {
            wl_output::Event::Geometry { make, model, .. } => {
                output.make = Some(make);
                output.model = Some(model);
            }
            wl_output::Event::Name { name } => {
                output.name = Some(name);
            }
            wl_output::Event::Description { description } => {
                output.description = Some(description);
            }
            wl_output::Event::Mode {
                flags,
                width,
                height,
                refresh,
            } => {
                let flags = match flags {
                    WEnum::Value(flags) => flags,
                    WEnum::Unknown(_) => return,
                };

                if flags.contains(wl_output::Mode::Current) {
                    output.width = Some(width);
                    output.height = Some(height);
                    output.refresh_mhz = Some(refresh);
                }
            }
            wl_output::Event::Scale { factor } => {
                output.scale_factor = Some(factor);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{WaylandOutputInfo, WaylandRuntimeService, WaylandRuntimeSnapshot};

    #[test]
    fn output_summary_prefers_named_outputs() {
        let service = WaylandRuntimeService::with_snapshot_for_tests(WaylandRuntimeSnapshot {
            available: true,
            outputs: vec![
                WaylandOutputInfo {
                    global_name: 1,
                    version: 4,
                    name: Some("eDP-1".to_string()),
                    ..WaylandOutputInfo::default()
                },
                WaylandOutputInfo {
                    global_name: 2,
                    version: 4,
                    description: Some("Dell U2720Q".to_string()),
                    ..WaylandOutputInfo::default()
                },
            ],
            ..WaylandRuntimeSnapshot::default()
        });

        assert_eq!(
            service.snapshot().output_summary(),
            "2 outputs: eDP-1, Dell U2720Q"
        );
    }

    #[test]
    fn output_topology_summary_counts_internal_and_external_outputs() {
        let service = WaylandRuntimeService::with_snapshot_for_tests(WaylandRuntimeSnapshot {
            available: true,
            outputs: vec![
                WaylandOutputInfo {
                    global_name: 1,
                    version: 4,
                    name: Some("eDP-1".to_string()),
                    ..WaylandOutputInfo::default()
                },
                WaylandOutputInfo {
                    global_name: 2,
                    version: 4,
                    name: Some("HDMI-A-1".to_string()),
                    ..WaylandOutputInfo::default()
                },
            ],
            ..WaylandRuntimeSnapshot::default()
        });

        assert_eq!(
            service.snapshot().output_topology_summary(),
            "1 internal + 1 external"
        );
    }

    #[test]
    fn display_mode_summary_classifies_common_topologies() {
        let unavailable = WaylandRuntimeService::unavailable_for_tests();
        assert_eq!(
            unavailable.snapshot().display_mode_summary(),
            "Wayland unavailable"
        );

        let laptop = WaylandRuntimeService::with_snapshot_for_tests(WaylandRuntimeSnapshot {
            available: true,
            outputs: vec![WaylandOutputInfo {
                global_name: 1,
                version: 4,
                name: Some("eDP-1".to_string()),
                ..WaylandOutputInfo::default()
            }],
            ..WaylandRuntimeSnapshot::default()
        });
        assert_eq!(laptop.snapshot().display_mode_summary(), "Laptop");

        let docked = WaylandRuntimeService::with_snapshot_for_tests(WaylandRuntimeSnapshot {
            available: true,
            outputs: vec![WaylandOutputInfo {
                global_name: 2,
                version: 4,
                name: Some("HDMI-A-1".to_string()),
                ..WaylandOutputInfo::default()
            }],
            ..WaylandRuntimeSnapshot::default()
        });
        assert_eq!(docked.snapshot().display_mode_summary(), "Docked");

        let hybrid = WaylandRuntimeService::with_snapshot_for_tests(WaylandRuntimeSnapshot {
            available: true,
            outputs: vec![
                WaylandOutputInfo {
                    global_name: 1,
                    version: 4,
                    name: Some("eDP-1".to_string()),
                    ..WaylandOutputInfo::default()
                },
                WaylandOutputInfo {
                    global_name: 2,
                    version: 4,
                    name: Some("DP-2".to_string()),
                    ..WaylandOutputInfo::default()
                },
            ],
            ..WaylandRuntimeSnapshot::default()
        });
        assert_eq!(hybrid.snapshot().display_mode_summary(), "Hybrid");
    }

    #[test]
    fn output_detail_summary_includes_mode_and_scale() {
        let service = WaylandRuntimeService::with_snapshot_for_tests(WaylandRuntimeSnapshot {
            available: true,
            outputs: vec![WaylandOutputInfo {
                global_name: 1,
                version: 4,
                name: Some("eDP-1".to_string()),
                width: Some(1920),
                height: Some(1200),
                refresh_mhz: Some(60000),
                scale_factor: Some(2),
                ..WaylandOutputInfo::default()
            }],
            ..WaylandRuntimeSnapshot::default()
        });

        assert_eq!(
            service.snapshot().output_detail_summary(),
            "eDP-1 1920x1200 60Hz 2x"
        );
    }

    #[test]
    fn output_scale_summary_lists_per_output_scale() {
        let service = WaylandRuntimeService::with_snapshot_for_tests(WaylandRuntimeSnapshot {
            available: true,
            outputs: vec![
                WaylandOutputInfo {
                    global_name: 1,
                    version: 4,
                    name: Some("eDP-1".to_string()),
                    scale_factor: Some(2),
                    ..WaylandOutputInfo::default()
                },
                WaylandOutputInfo {
                    global_name: 2,
                    version: 4,
                    name: Some("HDMI-A-1".to_string()),
                    scale_factor: Some(1),
                    ..WaylandOutputInfo::default()
                },
            ],
            ..WaylandRuntimeSnapshot::default()
        });

        assert_eq!(
            service.snapshot().output_scale_summary(),
            "eDP-1 2x, HDMI-A-1 1x"
        );
    }

    #[test]
    fn missing_capabilities_reports_protocols_beyond_idle_path() {
        let service = WaylandRuntimeService::unavailable_for_tests();
        assert_eq!(
            service.snapshot().missing_capabilities().as_deref(),
            Some(
                "wl_compositor, wl_shm, wl_output, xdg_wm_base, zwlr_layer_shell_v1, zwp_idle_inhibit_manager_v1"
            )
        );
    }

    #[test]
    fn capability_status_surfaces_missing_protocols_as_hybrid() {
        let service = WaylandRuntimeService::with_snapshot_for_tests(WaylandRuntimeSnapshot {
            available: true,
            compositor_version: Some(6),
            shm_version: Some(1),
            xdg_wm_base_version: Some(4),
            layer_shell_version: Some(5),
            idle_inhibit_version: None,
            outputs: vec![WaylandOutputInfo {
                global_name: 1,
                version: 4,
                name: Some("eDP-1".to_string()),
                ..WaylandOutputInfo::default()
            }],
            unavailable_reason: None,
        });

        let status = service.capability_status();

        assert_eq!(status.key, "way");
        assert_eq!(
            status.mode,
            crate::services::capabilities::CapabilityMode::Hybrid
        );
        assert_eq!(
            status.detail.as_deref(),
            Some("missing protocols: zwp_idle_inhibit_manager_v1")
        );
    }
}
