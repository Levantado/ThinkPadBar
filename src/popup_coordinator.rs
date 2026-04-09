use crate::{
    app::Popup,
    services::{controls::ControlsRefreshKind, system_info::SystemInfoRefreshKind},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PopupVisibilityAction {
    Show(Popup),
    Hide,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PopupTransitionPlan {
    pub next_popup: Popup,
    pub visibility_action: PopupVisibilityAction,
    pub close_transient_ui: bool,
    pub reset_bluetooth_scan: bool,
    pub reset_calendar_offset: bool,
    pub controls_refreshes: Vec<ControlsRefreshKind>,
    pub system_info_refresh: Option<SystemInfoRefreshKind>,
}

impl PopupTransitionPlan {
    pub fn toggle(current: &Popup, target: Popup) -> Self {
        if current == &target {
            return Self {
                next_popup: Popup::None,
                visibility_action: PopupVisibilityAction::Hide,
                close_transient_ui: true,
                reset_bluetooth_scan: target == Popup::BluetoothDevices,
                reset_calendar_offset: target == Popup::Calendar,
                controls_refreshes: Vec::new(),
                system_info_refresh: None,
            };
        }

        Self {
            next_popup: target.clone(),
            visibility_action: PopupVisibilityAction::Show(target.clone()),
            close_transient_ui: false,
            reset_bluetooth_scan: target == Popup::BluetoothDevices,
            reset_calendar_offset: target == Popup::Calendar,
            controls_refreshes: controls_refreshes_for_open(&target),
            system_info_refresh: system_info_refresh_for_open(&target),
        }
    }

    pub fn close_on_unfocus() -> Self {
        Self {
            next_popup: Popup::None,
            visibility_action: PopupVisibilityAction::Hide,
            close_transient_ui: true,
            reset_bluetooth_scan: false,
            reset_calendar_offset: true,
            controls_refreshes: Vec::new(),
            system_info_refresh: None,
        }
    }

    pub fn open_tray_menu() -> Self {
        Self {
            next_popup: Popup::TrayMenu,
            visibility_action: PopupVisibilityAction::Show(Popup::TrayMenu),
            close_transient_ui: false,
            reset_bluetooth_scan: false,
            reset_calendar_offset: false,
            controls_refreshes: Vec::new(),
            system_info_refresh: None,
        }
    }

    pub fn close_popup() -> Self {
        Self {
            next_popup: Popup::None,
            visibility_action: PopupVisibilityAction::Hide,
            close_transient_ui: false,
            reset_bluetooth_scan: false,
            reset_calendar_offset: false,
            controls_refreshes: Vec::new(),
            system_info_refresh: None,
        }
    }

    pub fn close_for_power_action() -> Self {
        Self {
            next_popup: Popup::None,
            visibility_action: PopupVisibilityAction::Hide,
            close_transient_ui: true,
            reset_bluetooth_scan: false,
            reset_calendar_offset: false,
            controls_refreshes: Vec::new(),
            system_info_refresh: None,
        }
    }
}

fn controls_refreshes_for_open(target: &Popup) -> Vec<ControlsRefreshKind> {
    match target {
        Popup::Controls | Popup::AudioRoutes => vec![ControlsRefreshKind::AudioMic],
        Popup::Connectivity | Popup::BluetoothDevices => vec![ControlsRefreshKind::Bluetooth],
        Popup::Power => vec![ControlsRefreshKind::BatteryPower],
        _ => Vec::new(),
    }
}

fn system_info_refresh_for_open(target: &Popup) -> Option<SystemInfoRefreshKind> {
    match target {
        Popup::Stats | Popup::SystemMonitor => Some(SystemInfoRefreshKind::Fast),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{PopupTransitionPlan, PopupVisibilityAction};
    use crate::{
        app::Popup,
        services::{controls::ControlsRefreshKind, system_info::SystemInfoRefreshKind},
    };

    #[test]
    fn toggle_open_controls_requests_audio_refresh() {
        let plan = PopupTransitionPlan::toggle(&Popup::None, Popup::Controls);

        assert_eq!(plan.next_popup, Popup::Controls);
        assert_eq!(
            plan.visibility_action,
            PopupVisibilityAction::Show(Popup::Controls)
        );
        assert_eq!(plan.controls_refreshes, vec![ControlsRefreshKind::AudioMic]);
        assert_eq!(plan.system_info_refresh, None);
    }

    #[test]
    fn toggle_open_stats_requests_fast_system_refresh() {
        let plan = PopupTransitionPlan::toggle(&Popup::None, Popup::Stats);

        assert_eq!(plan.next_popup, Popup::Stats);
        assert_eq!(plan.system_info_refresh, Some(SystemInfoRefreshKind::Fast));
    }

    #[test]
    fn toggle_close_calendar_resets_offset_and_hides() {
        let plan = PopupTransitionPlan::toggle(&Popup::Calendar, Popup::Calendar);

        assert_eq!(plan.next_popup, Popup::None);
        assert_eq!(plan.visibility_action, PopupVisibilityAction::Hide);
        assert!(plan.close_transient_ui);
        assert!(plan.reset_calendar_offset);
    }

    #[test]
    fn unfocus_close_keeps_policy_centralized() {
        let plan = PopupTransitionPlan::close_on_unfocus();

        assert_eq!(plan.next_popup, Popup::None);
        assert_eq!(plan.visibility_action, PopupVisibilityAction::Hide);
        assert!(plan.close_transient_ui);
        assert!(plan.reset_calendar_offset);
    }

    #[test]
    fn tray_menu_open_and_close_have_no_refresh_side_effects() {
        let open_plan = PopupTransitionPlan::open_tray_menu();
        let close_plan = PopupTransitionPlan::close_popup();

        assert_eq!(open_plan.next_popup, Popup::TrayMenu);
        assert_eq!(
            open_plan.visibility_action,
            PopupVisibilityAction::Show(Popup::TrayMenu)
        );
        assert!(open_plan.controls_refreshes.is_empty());
        assert_eq!(close_plan.visibility_action, PopupVisibilityAction::Hide);
        assert!(!close_plan.close_transient_ui);
    }
}
