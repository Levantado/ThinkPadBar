use iced::platform_specific::shell::commands::layer_surface::Anchor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupSurfaceKind {
    Hidden,
    Stats,
    Power,
    Controls,
    Connectivity,
    ControlCenter,
    AudioRoutes,
    BluetoothDevices,
    SystemMonitor,
    Displays,
    Calendar,
    TrayMenu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PopupSurfacePlan {
    pub width: u32,
    pub height: u32,
    pub anchor: Anchor,
    pub margin: (i32, i32, i32, i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PopupAnchorService {
    bar_height: i32,
}

impl PopupAnchorService {
    pub fn new(bar_height: u32) -> Self {
        Self {
            bar_height: bar_height as i32,
        }
    }

    pub fn plan(
        &self,
        kind: PopupSurfaceKind,
        tray_cursor: Option<(i32, i32)>,
        tray_menu_height: Option<u32>,
    ) -> PopupSurfacePlan {
        match kind {
            PopupSurfaceKind::Hidden => PopupSurfacePlan {
                width: 1,
                height: 1,
                anchor: Anchor::TOP | Anchor::RIGHT,
                margin: (self.bar_height, 8, 0, 0),
            },
            PopupSurfaceKind::Calendar => PopupSurfacePlan {
                width: 400,
                height: 420,
                anchor: Anchor::TOP | Anchor::RIGHT,
                margin: (self.bar_height, 8, 0, 0),
            },
            PopupSurfaceKind::Stats => PopupSurfacePlan {
                width: 420,
                height: 420,
                anchor: Anchor::TOP | Anchor::RIGHT,
                margin: (self.bar_height, 8, 0, 0),
            },
            PopupSurfaceKind::Power => PopupSurfacePlan {
                width: 460,
                height: 680,
                anchor: Anchor::TOP | Anchor::RIGHT,
                margin: (self.bar_height, 8, 0, 0),
            },
            PopupSurfaceKind::Controls => PopupSurfacePlan {
                width: 440,
                height: 520,
                anchor: Anchor::TOP | Anchor::RIGHT,
                margin: (self.bar_height, 8, 0, 0),
            },
            PopupSurfaceKind::Connectivity => PopupSurfacePlan {
                width: 460,
                height: 620,
                anchor: Anchor::TOP | Anchor::RIGHT,
                margin: (self.bar_height, 8, 0, 0),
            },
            PopupSurfaceKind::SystemMonitor => PopupSurfacePlan {
                width: 400,
                height: 520,
                anchor: Anchor::TOP | Anchor::RIGHT,
                margin: (self.bar_height, 8, 0, 0),
            },
            PopupSurfaceKind::AudioRoutes => PopupSurfacePlan {
                width: 460,
                height: 520,
                anchor: Anchor::TOP | Anchor::RIGHT,
                margin: (self.bar_height, 8, 0, 0),
            },
            PopupSurfaceKind::BluetoothDevices => PopupSurfacePlan {
                width: 460,
                height: 560,
                anchor: Anchor::TOP | Anchor::RIGHT,
                margin: (self.bar_height, 8, 0, 0),
            },
            PopupSurfaceKind::Displays => PopupSurfacePlan {
                width: 460,
                height: 520,
                anchor: Anchor::TOP | Anchor::RIGHT,
                margin: (self.bar_height, 8, 0, 0),
            },
            PopupSurfaceKind::ControlCenter => PopupSurfacePlan {
                width: 420,
                height: 760,
                anchor: Anchor::TOP | Anchor::RIGHT,
                margin: (self.bar_height, 8, 0, 0),
            },
            PopupSurfaceKind::TrayMenu => {
                let height = tray_menu_height.unwrap_or(240).clamp(140, 420);
                let margin = if let Some((cursor_x, cursor_y)) = tray_cursor {
                    (
                        (cursor_y + 8).max(self.bar_height + 4),
                        0,
                        0,
                        (cursor_x + 8).max(8),
                    )
                } else {
                    (self.bar_height, 8, 0, 0)
                };
                PopupSurfacePlan {
                    width: 320,
                    height,
                    anchor: Anchor::TOP | Anchor::LEFT,
                    margin,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PopupAnchorService, PopupSurfaceKind};
    use iced::platform_specific::shell::commands::layer_surface::Anchor;

    #[test]
    fn hidden_plan_uses_minimal_surface() {
        let service = PopupAnchorService::new(24);
        let plan = service.plan(PopupSurfaceKind::Hidden, None, None);
        assert_eq!(plan.width, 1);
        assert_eq!(plan.height, 1);
        assert_eq!(plan.anchor, Anchor::TOP | Anchor::RIGHT);
    }

    #[test]
    fn tray_menu_plan_uses_cursor_when_available() {
        let service = PopupAnchorService::new(40);
        let plan = service.plan(PopupSurfaceKind::TrayMenu, Some((1200, 20)), Some(188));
        assert_eq!(plan.anchor, Anchor::TOP | Anchor::LEFT);
        assert_eq!(plan.height, 188);
        assert_eq!(plan.margin, (44, 0, 0, 1208));
    }

    #[test]
    fn displays_plan_uses_dedicated_surface_size() {
        let service = PopupAnchorService::new(24);
        let plan = service.plan(PopupSurfaceKind::Displays, None, None);
        assert_eq!(plan.width, 460);
        assert_eq!(plan.height, 520);
        assert_eq!(plan.anchor, Anchor::TOP | Anchor::RIGHT);
    }

    #[test]
    fn stats_plan_uses_compact_surface_size() {
        let service = PopupAnchorService::new(24);
        let plan = service.plan(PopupSurfaceKind::Stats, None, None);
        assert_eq!(plan.width, 420);
        assert_eq!(plan.height, 420);
        assert_eq!(plan.anchor, Anchor::TOP | Anchor::RIGHT);
    }

    #[test]
    fn power_plan_uses_tall_surface_size() {
        let service = PopupAnchorService::new(24);
        let plan = service.plan(PopupSurfaceKind::Power, None, None);
        assert_eq!(plan.width, 460);
        assert_eq!(plan.height, 680);
        assert_eq!(plan.anchor, Anchor::TOP | Anchor::RIGHT);
    }

    #[test]
    fn controls_plan_uses_medium_surface_size() {
        let service = PopupAnchorService::new(24);
        let plan = service.plan(PopupSurfaceKind::Controls, None, None);
        assert_eq!(plan.width, 440);
        assert_eq!(plan.height, 520);
        assert_eq!(plan.anchor, Anchor::TOP | Anchor::RIGHT);
    }

    #[test]
    fn connectivity_plan_uses_tall_surface_size() {
        let service = PopupAnchorService::new(24);
        let plan = service.plan(PopupSurfaceKind::Connectivity, None, None);
        assert_eq!(plan.width, 460);
        assert_eq!(plan.height, 620);
        assert_eq!(plan.anchor, Anchor::TOP | Anchor::RIGHT);
    }

    #[test]
    fn audio_routes_plan_uses_dedicated_surface_size() {
        let service = PopupAnchorService::new(24);
        let plan = service.plan(PopupSurfaceKind::AudioRoutes, None, None);
        assert_eq!(plan.width, 460);
        assert_eq!(plan.height, 520);
        assert_eq!(plan.anchor, Anchor::TOP | Anchor::RIGHT);
    }

    #[test]
    fn bluetooth_devices_plan_uses_dedicated_surface_size() {
        let service = PopupAnchorService::new(24);
        let plan = service.plan(PopupSurfaceKind::BluetoothDevices, None, None);
        assert_eq!(plan.width, 460);
        assert_eq!(plan.height, 560);
        assert_eq!(plan.anchor, Anchor::TOP | Anchor::RIGHT);
    }
}
