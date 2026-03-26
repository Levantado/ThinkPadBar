use wayland_client::{
    protocol::{
        wl_compositor::WlCompositor,
        wl_output::{self, WlOutput},
        wl_registry::{self, WlRegistry},
        wl_shm::WlShm,
    },
    Connection, Dispatch, EventQueue, Proxy, QueueHandle,
};
use wayland_protocols::xdg::shell::client::xdg_wm_base::XdgWmBase;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WaylandOutputInfo {
    pub global_name: u32,
    pub version: u32,
    pub name: Option<String>,
    pub description: Option<String>,
    pub make: Option<String>,
    pub model: Option<String>,
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
    fn missing_capabilities_reports_protocols_beyond_idle_path() {
        let service = WaylandRuntimeService::unavailable_for_tests();
        assert_eq!(
            service.snapshot().missing_capabilities().as_deref(),
            Some(
                "wl_compositor, wl_shm, wl_output, xdg_wm_base, zwlr_layer_shell_v1, zwp_idle_inhibit_manager_v1"
            )
        );
    }
}
