use iced::window::Id;
use wayland_client::{
    globals::{registry_queue_init, GlobalListContents},
    protocol::{wl_registry, wl_surface::WlSurface},
    Connection, Dispatch, EventQueue, Proxy, QueueHandle,
};
use wayland_protocols::wp::idle_inhibit::zv1::client::{
    zwp_idle_inhibit_manager_v1::ZwpIdleInhibitManagerV1, zwp_idle_inhibitor_v1::ZwpIdleInhibitorV1,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdleInhibitorSurfaceEvent {
    pub window_id: Id,
    pub surface: WlSurface,
}

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

#[derive(Default)]
struct IdleInhibitorQueueState;

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for IdleInhibitorQueueState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_registry::WlRegistry,
        _event: wl_registry::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpIdleInhibitManagerV1, ()> for IdleInhibitorQueueState {
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

impl Dispatch<ZwpIdleInhibitorV1, ()> for IdleInhibitorQueueState {
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

struct IdleInhibitorBackend {
    connection: Connection,
    queue: EventQueue<IdleInhibitorQueueState>,
    queue_state: IdleInhibitorQueueState,
    manager: Option<ZwpIdleInhibitManagerV1>,
    surface: WlSurface,
    inhibitor: Option<ZwpIdleInhibitorV1>,
}

impl IdleInhibitorBackend {
    fn bind(surface: WlSurface) -> Option<Self> {
        let backend = surface.backend().upgrade()?;
        let connection = Connection::from_backend(backend);
        let (globals, queue) = registry_queue_init::<IdleInhibitorQueueState>(&connection).ok()?;
        let manager = globals
            .bind::<ZwpIdleInhibitManagerV1, IdleInhibitorQueueState, _>(&queue.handle(), 1..=1, ())
            .ok();
        Some(Self {
            connection,
            queue,
            queue_state: IdleInhibitorQueueState,
            manager,
            surface,
            inhibitor: None,
        })
    }

    fn available(&self) -> bool {
        self.manager.as_ref().is_some_and(Proxy::is_alive) && self.surface.is_alive()
    }

    fn surface_matches(&self, other: &WlSurface) -> bool {
        self.surface.id() == other.id()
    }

    fn set_enabled(&mut self, enabled: bool) {
        if enabled {
            if self.inhibitor.is_none() {
                if let Some(manager) = &self.manager {
                    self.inhibitor =
                        Some(manager.create_inhibitor(&self.surface, &self.queue.handle(), ()));
                    let _ = self.connection.flush();
                }
            }
        } else if let Some(inhibitor) = self.inhibitor.take() {
            inhibitor.destroy();
            let _ = self.connection.flush();
        }
    }

    fn enabled(&self) -> bool {
        self.inhibitor.as_ref().is_some_and(Proxy::is_alive)
    }

    fn dispatch_pending(&mut self) {
        let _ = self.queue.dispatch_pending(&mut self.queue_state);
    }
}

pub struct IdleInhibitorService {
    snapshot: IdleInhibitorSnapshot,
    requested_enabled: bool,
    bound_window_id: Option<Id>,
    backend: Option<IdleInhibitorBackend>,
}

impl IdleInhibitorService {
    pub fn new() -> Self {
        Self {
            snapshot: IdleInhibitorSnapshot::default(),
            requested_enabled: false,
            bound_window_id: None,
            backend: None,
        }
    }

    pub fn snapshot(&self) -> IdleInhibitorSnapshot {
        self.snapshot
    }

    pub fn observe_surface(
        &mut self,
        main_window_id: Option<Id>,
        event: IdleInhibitorSurfaceEvent,
    ) {
        if Some(event.window_id) != main_window_id {
            return;
        }

        let needs_rebind = self.bound_window_id != Some(event.window_id)
            || self
                .backend
                .as_ref()
                .is_none_or(|backend| !backend.surface_matches(&event.surface));

        if !needs_rebind {
            return;
        }

        self.bound_window_id = Some(event.window_id);
        self.backend = IdleInhibitorBackend::bind(event.surface);
        if self.backend.is_none() {
            self.requested_enabled = false;
        }
        self.apply_requested_state();
    }

    pub fn toggle(&mut self) {
        self.requested_enabled = !self.requested_enabled;
        self.apply_requested_state();
    }

    fn apply_requested_state(&mut self) {
        if let Some(backend) = self.backend.as_mut() {
            backend.dispatch_pending();
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
        let (available, enabled) = if let Some(backend) = self.backend.as_mut() {
            backend.dispatch_pending();
            (backend.available(), backend.enabled())
        } else {
            (false, false)
        };
        self.snapshot = IdleInhibitorSnapshot { available, enabled };
    }
}

#[cfg(test)]
mod tests {
    use super::{IdleInhibitorService, IdleInhibitorSnapshot};

    #[test]
    fn snapshot_defaults_to_unavailable_and_disabled() {
        assert_eq!(
            IdleInhibitorService::new().snapshot(),
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
    fn toggling_without_wayland_support_remains_disabled() {
        let mut service = IdleInhibitorService::new();
        service.toggle();
        assert_eq!(
            service.snapshot(),
            IdleInhibitorSnapshot {
                available: false,
                enabled: false,
            }
        );
    }
}
