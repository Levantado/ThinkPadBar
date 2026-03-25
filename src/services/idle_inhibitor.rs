use tracing::warn;
use wayland_client::{
    protocol::{
        wl_compositor::WlCompositor,
        wl_display::WlDisplay,
        wl_registry::{self, WlRegistry},
        wl_surface::WlSurface,
    },
    Connection, Dispatch, EventQueue, Proxy, QueueHandle,
};
use wayland_protocols::wp::idle_inhibit::zv1::client::{
    zwp_idle_inhibit_manager_v1::ZwpIdleInhibitManagerV1, zwp_idle_inhibitor_v1::ZwpIdleInhibitorV1,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IdleInhibitorSnapshot {
    pub available: bool,
    pub enabled: bool,
}

impl IdleInhibitorSnapshot {
    pub fn label(self) -> &'static str {
        if !self.available {
            "N/A"
        } else if self.enabled {
            "On"
        } else {
            "Off"
        }
    }
}

trait IdleInhibitorBackend {
    fn available(&self) -> bool;
    fn enabled(&self) -> bool;
    fn set_enabled(&mut self, enabled: bool);
}

#[derive(Default)]
struct IdleInhibitorWaylandData {
    compositor: Option<(WlCompositor, u32)>,
    surface: Option<WlSurface>,
    idle_manager: Option<(ZwpIdleInhibitManagerV1, u32)>,
    inhibitor: Option<ZwpIdleInhibitorV1>,
}

struct WaylandIdleInhibitorBackend {
    _connection: Connection,
    _display: WlDisplay,
    _registry: WlRegistry,
    event_queue: EventQueue<IdleInhibitorWaylandData>,
    handle: QueueHandle<IdleInhibitorWaylandData>,
    state: IdleInhibitorWaylandData,
}

impl WaylandIdleInhibitorBackend {
    fn connect() -> Option<Self> {
        let init = || -> Result<Self, Box<dyn std::error::Error>> {
            let connection = Connection::connect_to_env()?;
            let display = connection.display();
            let event_queue = connection.new_event_queue();
            let handle = event_queue.handle();
            let registry = display.get_registry(&handle, ());

            let mut backend = Self {
                _connection: connection,
                _display: display,
                _registry: registry,
                event_queue,
                handle,
                state: IdleInhibitorWaylandData::default(),
            };
            let _ = backend.event_queue.roundtrip(&mut backend.state)?;
            Ok(backend)
        };

        match init() {
            Ok(backend) => Some(backend),
            Err(err) => {
                warn!("Failed to initialize Wayland idle inhibitor backend: {err}");
                None
            }
        }
    }

    fn sync(&mut self) {
        let _ = self.event_queue.dispatch_pending(&mut self.state);
    }
}

impl IdleInhibitorBackend for WaylandIdleInhibitorBackend {
    fn available(&self) -> bool {
        self.state.surface.as_ref().is_some_and(Proxy::is_alive)
            && self
                .state
                .idle_manager
                .as_ref()
                .is_some_and(|(manager, _)| manager.is_alive())
    }

    fn enabled(&self) -> bool {
        self.state.inhibitor.as_ref().is_some_and(Proxy::is_alive)
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.sync();

        if enabled {
            if self.state.inhibitor.is_none() {
                let Some(surface) = self.state.surface.as_ref() else {
                    return;
                };
                let Some((manager, _)) = self.state.idle_manager.as_ref() else {
                    return;
                };

                self.state.inhibitor = Some(manager.create_inhibitor(surface, &self.handle, ()));
                let _ = self.event_queue.roundtrip(&mut self.state);
            }
        } else if let Some(inhibitor) = self.state.inhibitor.take() {
            inhibitor.destroy();
            let _ = self.event_queue.roundtrip(&mut self.state);
        }
    }
}

impl Dispatch<WlRegistry, ()> for IdleInhibitorWaylandData {
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
                if interface == WlCompositor::interface().name && state.compositor.is_none() {
                    let compositor: WlCompositor = proxy.bind(name, version, handle, ());
                    state.surface = Some(compositor.create_surface(handle, ()));
                    state.compositor = Some((compositor, name));
                } else if interface == ZwpIdleInhibitManagerV1::interface().name
                    && state.idle_manager.is_none()
                {
                    state.idle_manager = Some((proxy.bind(name, version, handle, ()), name));
                }
            }
            wl_registry::Event::GlobalRemove { name } => {
                if state
                    .compositor
                    .as_ref()
                    .is_some_and(|(_, compositor_name)| *compositor_name == name)
                {
                    state.compositor = None;
                    state.surface = None;
                    state.inhibitor = None;
                }
                if state
                    .idle_manager
                    .as_ref()
                    .is_some_and(|(_, manager_name)| *manager_name == name)
                {
                    state.idle_manager = None;
                    state.inhibitor = None;
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<WlCompositor, ()> for IdleInhibitorWaylandData {
    fn event(
        _state: &mut Self,
        _proxy: &WlCompositor,
        _event: <WlCompositor as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlSurface, ()> for IdleInhibitorWaylandData {
    fn event(
        _state: &mut Self,
        _proxy: &WlSurface,
        _event: <WlSurface as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpIdleInhibitManagerV1, ()> for IdleInhibitorWaylandData {
    fn event(
        _state: &mut Self,
        _proxy: &ZwpIdleInhibitManagerV1,
        _event: <ZwpIdleInhibitManagerV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpIdleInhibitorV1, ()> for IdleInhibitorWaylandData {
    fn event(
        _state: &mut Self,
        _proxy: &ZwpIdleInhibitorV1,
        _event: <ZwpIdleInhibitorV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

pub struct IdleInhibitorService {
    snapshot: IdleInhibitorSnapshot,
    requested_enabled: bool,
    backend: Option<Box<dyn IdleInhibitorBackend>>,
}

impl IdleInhibitorService {
    pub fn new() -> Self {
        let backend = WaylandIdleInhibitorBackend::connect()
            .map(|backend| Box::new(backend) as Box<dyn IdleInhibitorBackend>);
        let mut service = Self {
            snapshot: IdleInhibitorSnapshot::default(),
            requested_enabled: false,
            backend,
        };
        service.refresh_snapshot();
        service
    }

    pub fn snapshot(&self) -> IdleInhibitorSnapshot {
        self.snapshot
    }

    pub fn toggle(&mut self) {
        self.requested_enabled = !self.requested_enabled;
        self.apply_requested_state();
    }

    fn apply_requested_state(&mut self) {
        if let Some(backend) = self.backend.as_mut() {
            if backend.available() {
                backend.set_enabled(self.requested_enabled);
            } else {
                self.requested_enabled = false;
                backend.set_enabled(false);
            }
        } else {
            self.requested_enabled = false;
        }
        self.refresh_snapshot();
    }

    fn refresh_snapshot(&mut self) {
        let (available, enabled) = if let Some(backend) = self.backend.as_ref() {
            (backend.available(), backend.enabled())
        } else {
            (false, false)
        };
        self.snapshot = IdleInhibitorSnapshot { available, enabled };
    }
}

#[cfg(test)]
mod tests {
    use super::{IdleInhibitorBackend, IdleInhibitorService, IdleInhibitorSnapshot};

    #[derive(Default)]
    struct FakeBackend {
        available: bool,
        enabled: bool,
        set_enabled_calls: Vec<bool>,
    }

    impl IdleInhibitorBackend for FakeBackend {
        fn available(&self) -> bool {
            self.available
        }

        fn enabled(&self) -> bool {
            self.enabled
        }

        fn set_enabled(&mut self, enabled: bool) {
            self.set_enabled_calls.push(enabled);
            self.enabled = self.available && enabled;
        }
    }

    impl IdleInhibitorService {
        fn with_backend(backend: impl IdleInhibitorBackend + 'static) -> Self {
            let mut service = Self {
                snapshot: IdleInhibitorSnapshot::default(),
                requested_enabled: false,
                backend: Some(Box::new(backend)),
            };
            service.refresh_snapshot();
            service
        }
    }

    #[test]
    fn snapshot_defaults_to_unavailable_and_disabled() {
        assert_eq!(
            IdleInhibitorService {
                snapshot: IdleInhibitorSnapshot::default(),
                requested_enabled: false,
                backend: None,
            }
            .snapshot(),
            IdleInhibitorSnapshot {
                available: false,
                enabled: false,
            }
        );
    }

    #[test]
    fn snapshot_label_matches_state() {
        assert_eq!(
            IdleInhibitorSnapshot {
                available: false,
                enabled: false,
            }
            .label(),
            "N/A"
        );
        assert_eq!(
            IdleInhibitorSnapshot {
                available: true,
                enabled: false,
            }
            .label(),
            "Off"
        );
        assert_eq!(
            IdleInhibitorSnapshot {
                available: true,
                enabled: true,
            }
            .label(),
            "On"
        );
    }

    #[test]
    fn toggling_without_backend_remains_disabled() {
        let mut service = IdleInhibitorService {
            snapshot: IdleInhibitorSnapshot::default(),
            requested_enabled: false,
            backend: None,
        };
        service.toggle();
        assert_eq!(
            service.snapshot(),
            IdleInhibitorSnapshot {
                available: false,
                enabled: false,
            }
        );
    }

    #[test]
    fn available_backend_starts_as_off() {
        let service = IdleInhibitorService::with_backend(FakeBackend {
            available: true,
            enabled: false,
            set_enabled_calls: Vec::new(),
        });
        assert_eq!(
            service.snapshot(),
            IdleInhibitorSnapshot {
                available: true,
                enabled: false,
            }
        );
    }

    #[test]
    fn toggle_enables_available_backend() {
        let mut service = IdleInhibitorService::with_backend(FakeBackend {
            available: true,
            enabled: false,
            set_enabled_calls: Vec::new(),
        });
        service.toggle();
        assert_eq!(
            service.snapshot(),
            IdleInhibitorSnapshot {
                available: true,
                enabled: true,
            }
        );
    }
}
