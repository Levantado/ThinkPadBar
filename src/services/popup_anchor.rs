use iced::platform_specific::shell::commands::layer_surface::Anchor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupSurfaceKind {
    Hidden,
    Stats,
    Power,
    Controls,
    Connectivity,
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
    const RIGHT_EDGE_WIDTH: u32 = 472;
    const COMPACT_HEIGHT: u32 = 320;
    const MEDIUM_HEIGHT: u32 = 540;
    const TALL_HEIGHT: u32 = 640;
    const XL_HEIGHT: u32 = 700;
    const CALENDAR_HEIGHT: u32 = 420;
    const TOP_MARGIN_GAP: i32 = 8;
    const SIDE_MARGIN_GAP: i32 = 8;

    pub fn new(bar_height: u32) -> Self {
        Self {
            bar_height: bar_height as i32,
        }
    }

    fn right_edge_margin(&self) -> (i32, i32, i32, i32) {
        (
            self.bar_height + Self::TOP_MARGIN_GAP,
            Self::SIDE_MARGIN_GAP,
            0,
            0,
        )
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
                margin: self.right_edge_margin(),
            },
            PopupSurfaceKind::Calendar => PopupSurfacePlan {
                width: Self::RIGHT_EDGE_WIDTH,
                height: Self::CALENDAR_HEIGHT,
                anchor: Anchor::TOP | Anchor::RIGHT,
                margin: self.right_edge_margin(),
            },
            PopupSurfaceKind::Stats => PopupSurfacePlan {
                width: Self::RIGHT_EDGE_WIDTH,
                height: Self::COMPACT_HEIGHT,
                anchor: Anchor::TOP | Anchor::RIGHT,
                margin: self.right_edge_margin(),
            },
            PopupSurfaceKind::Power => PopupSurfacePlan {
                width: Self::RIGHT_EDGE_WIDTH,
                height: Self::XL_HEIGHT,
                anchor: Anchor::TOP | Anchor::RIGHT,
                margin: self.right_edge_margin(),
            },
            PopupSurfaceKind::Controls => PopupSurfacePlan {
                width: Self::RIGHT_EDGE_WIDTH,
                height: Self::MEDIUM_HEIGHT,
                anchor: Anchor::TOP | Anchor::RIGHT,
                margin: self.right_edge_margin(),
            },
            PopupSurfaceKind::Connectivity => PopupSurfacePlan {
                width: Self::RIGHT_EDGE_WIDTH,
                height: Self::TALL_HEIGHT,
                anchor: Anchor::TOP | Anchor::RIGHT,
                margin: self.right_edge_margin(),
            },
            PopupSurfaceKind::SystemMonitor => PopupSurfacePlan {
                width: Self::RIGHT_EDGE_WIDTH,
                height: Self::MEDIUM_HEIGHT,
                anchor: Anchor::TOP | Anchor::RIGHT,
                margin: self.right_edge_margin(),
            },
            PopupSurfaceKind::AudioRoutes => PopupSurfacePlan {
                width: Self::RIGHT_EDGE_WIDTH,
                height: Self::MEDIUM_HEIGHT,
                anchor: Anchor::TOP | Anchor::RIGHT,
                margin: self.right_edge_margin(),
            },
            PopupSurfaceKind::BluetoothDevices => PopupSurfacePlan {
                width: Self::RIGHT_EDGE_WIDTH,
                height: 580,
                anchor: Anchor::TOP | Anchor::RIGHT,
                margin: self.right_edge_margin(),
            },
            PopupSurfaceKind::Displays => PopupSurfacePlan {
                width: Self::RIGHT_EDGE_WIDTH,
                height: Self::MEDIUM_HEIGHT,
                anchor: Anchor::TOP | Anchor::RIGHT,
                margin: self.right_edge_margin(),
            },
            PopupSurfaceKind::TrayMenu => {
                let height = tray_menu_height.unwrap_or(240).clamp(140, 420);
                let margin = if let Some((cursor_x, cursor_y)) = tray_cursor {
                    (
                        (cursor_y + Self::TOP_MARGIN_GAP).max(self.bar_height + 4),
                        0,
                        0,
                        (cursor_x + Self::SIDE_MARGIN_GAP).max(Self::SIDE_MARGIN_GAP),
                    )
                } else {
                    self.right_edge_margin()
                };
                PopupSurfacePlan {
                    width: 288,
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
        assert_eq!(plan.margin, (32, 8, 0, 0));
    }

    #[test]
    fn tray_menu_plan_uses_cursor_when_available() {
        let service = PopupAnchorService::new(40);
        let plan = service.plan(PopupSurfaceKind::TrayMenu, Some((1200, 20)), Some(188));
        assert_eq!(plan.anchor, Anchor::TOP | Anchor::LEFT);
        assert_eq!(plan.width, 288);
        assert_eq!(plan.height, 188);
        assert_eq!(plan.margin, (44, 0, 0, 1208));
    }

    #[test]
    fn displays_plan_uses_dedicated_surface_size() {
        let service = PopupAnchorService::new(24);
        let plan = service.plan(PopupSurfaceKind::Displays, None, None);
        assert_eq!(plan.width, 472);
        assert_eq!(plan.height, 540);
        assert_eq!(plan.anchor, Anchor::TOP | Anchor::RIGHT);
        assert_eq!(plan.margin, (32, 8, 0, 0));
    }

    #[test]
    fn stats_plan_uses_compact_surface_size() {
        let service = PopupAnchorService::new(24);
        let plan = service.plan(PopupSurfaceKind::Stats, None, None);
        assert_eq!(plan.width, 472);
        assert_eq!(plan.height, 320);
        assert_eq!(plan.anchor, Anchor::TOP | Anchor::RIGHT);
        assert_eq!(plan.margin, (32, 8, 0, 0));
    }

    #[test]
    fn power_plan_uses_tall_surface_size() {
        let service = PopupAnchorService::new(24);
        let plan = service.plan(PopupSurfaceKind::Power, None, None);
        assert_eq!(plan.width, 472);
        assert_eq!(plan.height, 700);
        assert_eq!(plan.anchor, Anchor::TOP | Anchor::RIGHT);
        assert_eq!(plan.margin, (32, 8, 0, 0));
    }

    #[test]
    fn controls_plan_uses_medium_surface_size() {
        let service = PopupAnchorService::new(24);
        let plan = service.plan(PopupSurfaceKind::Controls, None, None);
        assert_eq!(plan.width, 472);
        assert_eq!(plan.height, 540);
        assert_eq!(plan.anchor, Anchor::TOP | Anchor::RIGHT);
        assert_eq!(plan.margin, (32, 8, 0, 0));
    }

    #[test]
    fn connectivity_plan_uses_tall_surface_size() {
        let service = PopupAnchorService::new(24);
        let plan = service.plan(PopupSurfaceKind::Connectivity, None, None);
        assert_eq!(plan.width, 472);
        assert_eq!(plan.height, 640);
        assert_eq!(plan.anchor, Anchor::TOP | Anchor::RIGHT);
        assert_eq!(plan.margin, (32, 8, 0, 0));
    }

    #[test]
    fn audio_routes_plan_uses_dedicated_surface_size() {
        let service = PopupAnchorService::new(24);
        let plan = service.plan(PopupSurfaceKind::AudioRoutes, None, None);
        assert_eq!(plan.width, 472);
        assert_eq!(plan.height, 540);
        assert_eq!(plan.anchor, Anchor::TOP | Anchor::RIGHT);
        assert_eq!(plan.margin, (32, 8, 0, 0));
    }

    #[test]
    fn bluetooth_devices_plan_uses_dedicated_surface_size() {
        let service = PopupAnchorService::new(24);
        let plan = service.plan(PopupSurfaceKind::BluetoothDevices, None, None);
        assert_eq!(plan.width, 472);
        assert_eq!(plan.height, 580);
        assert_eq!(plan.anchor, Anchor::TOP | Anchor::RIGHT);
        assert_eq!(plan.margin, (32, 8, 0, 0));
    }
}
