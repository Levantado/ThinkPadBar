use iced::{
    widget::{button, text, Row, Space},
    Alignment, Background, Length, Padding,
};

use crate::app::{Message, Popup};
use crate::ui::theme::ThemeTokens;

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceSummaryItem {
    pub icon: &'static str,
    pub label: &'static str,
    pub value: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetailHeaderAction {
    pub icon: &'static str,
    pub label: &'static str,
    pub target: Popup,
}

pub fn detail_popup_header_action(popup: &Popup) -> Option<DetailHeaderAction> {
    match popup {
        Popup::Displays => Some(DetailHeaderAction {
            icon: "󰈈",
            label: "System Info",
            target: Popup::SystemMonitor,
        }),
        _ => None,
    }
}

pub fn detail_popup_header_row(
    theme: ThemeTokens,
    title: &'static str,
    popup: &Popup,
) -> Row<'static, Message> {
    let mut row = Row::new()
        .spacing(theme.gap_medium)
        .align_y(Alignment::Center)
        .push(text(title).size(18))
        .push(Space::with_width(Length::Fill));

    if let Some(action) = detail_popup_header_action(popup) {
        row = row.push(chrome_button(
            theme,
            Row::new()
                .spacing(theme.gap_small)
                .align_y(Alignment::Center)
                .push(text(action.icon).size(13))
                .push(text(action.label).size(11)),
            Message::TogglePopup(action.target),
        ));
    }

    row.push(chrome_button(
        theme,
        text("Close").size(11),
        Message::TogglePopup(popup.clone()),
    ))
}

pub fn domain_nav_focus_popup(popup: &Popup) -> Popup {
    match popup {
        Popup::AudioRoutes | Popup::Displays => Popup::Controls,
        Popup::BluetoothDevices => Popup::Connectivity,
        Popup::Power => Popup::Power,
        Popup::Controls => Popup::Controls,
        Popup::Connectivity => Popup::Connectivity,
        _ => Popup::Stats,
    }
}

pub fn domain_popup_nav_row(theme: ThemeTokens, active_popup: &Popup) -> Row<'static, Message> {
    let mut row = Row::new().spacing(theme.gap_medium);

    for (icon, label, target, is_active) in domain_popup_nav_items(active_popup) {
        let mut btn = button(
            Row::new()
                .spacing(theme.gap_small)
                .align_y(Alignment::Center)
                .push(text(icon).size(14))
                .push(text(label).size(11)),
        )
        .width(Length::FillPortion(1))
        .padding(Padding::from([8, 10]))
        .style(move |_, _| {
            if is_active {
                iced::widget::button::Style {
                    background: Some(Background::Color(theme.accent)),
                    text_color: theme.text_on_accent,
                    border: iced::Border {
                        radius: theme.button_radius.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            } else {
                iced::widget::button::Style {
                    background: Some(Background::Color(theme.surface)),
                    text_color: theme.text,
                    border: iced::Border {
                        radius: theme.button_radius.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            }
        });
        if !is_active {
            btn = btn.on_press(Message::TogglePopup(target));
        }
        row = row.push(btn);
    }

    row
}

pub fn domain_popup_nav_items(
    active_popup: &Popup,
) -> [(&'static str, &'static str, Popup, bool); 4] {
    [
        ("", "Stats", Popup::Stats, active_popup == &Popup::Stats),
        ("", "Power", Popup::Power, active_popup == &Popup::Power),
        (
            "󰖀",
            "Controls",
            Popup::Controls,
            active_popup == &Popup::Controls,
        ),
        (
            "󰖩",
            "Connectivity",
            Popup::Connectivity,
            active_popup == &Popup::Connectivity,
        ),
    ]
}

pub fn battery_percent_label(battery: &crate::services::controls::BatteryInfo) -> String {
    format!("{}%", battery.capacity)
}

pub fn audio_percent_label(audio: &crate::services::controls::AudioInfo) -> String {
    format!("{}%", audio.volume)
}

pub fn bluetooth_pill_summary(
    controls: &crate::services::controls::ControlsSnapshot,
) -> Option<String> {
    if !controls.bluetooth_enabled {
        return None;
    }

    let connected = controls.bluetooth_devices.connected_devices.len();
    (connected > 0).then(|| connected.to_string())
}

#[cfg(test)]
pub fn display_pill_summary(
    wayland_snapshot: &crate::services::wayland_runtime::WaylandRuntimeSnapshot,
) -> Option<(&'static str, String)> {
    if !wayland_snapshot.available {
        return None;
    }

    let mode = wayland_snapshot.display_mode_summary();
    let icon = match mode.as_str() {
        "Laptop" => "󰌢",
        "Docked" => "󰍹",
        "Hybrid" => "󰍺",
        "Headless" => "󰹑",
        _ => "󰍹",
    };
    Some((icon, mode))
}

#[cfg(test)]
pub fn power_summary_items(
    battery: &crate::services::controls::BatteryInfo,
) -> Vec<(&'static str, &'static str, String)> {
    vec![
        (
            "󰁹",
            "Battery Runtime",
            crate::ui::popups::power::battery_runtime_summary(battery),
        ),
        (
            "󱐌",
            "Charge State",
            crate::ui::popups::power::battery_charge_state_summary(battery),
        ),
        (
            "󰚥",
            "AC Adapter",
            crate::ui::popups::power::battery_ac_summary(battery),
        ),
        (
            "󱐋",
            "Charge / Draw Power",
            crate::ui::popups::power::battery_power_summary(battery),
        ),
        (
            "󱞊",
            "Charge Thresholds",
            crate::ui::popups::power::battery_threshold_summary(battery),
        ),
    ]
}

#[cfg(test)]
pub fn device_summary_items(
    controls: &crate::services::controls::ControlsSnapshot,
) -> Vec<DeviceSummaryItem> {
    let output_summary = if controls.audio.muted {
        "Muted".to_string()
    } else {
        format!("{}% output", controls.audio.volume)
    };
    let mic_summary = if controls.mic.muted {
        "Muted".to_string()
    } else {
        format!("{}% input", controls.mic.volume)
    };
    let bluetooth_summary = if controls.bluetooth_enabled
        && !controls.bluetooth_devices.connected_devices.is_empty()
    {
        format!(
            "{} connected",
            controls.bluetooth_devices.connected_devices.len()
        )
    } else if controls.bluetooth_enabled && !controls.bluetooth_devices.device_details.is_empty() {
        format!("{} known", controls.bluetooth_devices.device_details.len())
    } else if controls.bluetooth_enabled {
        "Adapter enabled".to_string()
    } else {
        "Adapter disabled".to_string()
    };

    vec![
        DeviceSummaryItem {
            icon: "",
            label: "Speakers",
            value: output_summary,
            detail: controls
                .audio_devices
                .output_route
                .clone()
                .or_else(|| Some("No output route discovered".to_string())),
        },
        DeviceSummaryItem {
            icon: "",
            label: "Microphone",
            value: mic_summary,
            detail: controls
                .audio_devices
                .input_route
                .clone()
                .or_else(|| Some("No input route discovered".to_string())),
        },
        DeviceSummaryItem {
            icon: "󰂯",
            label: "Bluetooth",
            value: bluetooth_summary,
            detail: if controls.bluetooth_devices.connected_devices.is_empty() {
                None
            } else {
                Some(controls.bluetooth_devices.connected_devices.join(", "))
            },
        },
    ]
}

fn chrome_button<'a>(
    theme: ThemeTokens,
    content: impl Into<iced::Element<'a, Message>>,
    message: Message,
) -> iced::widget::Button<'a, Message> {
    button(content)
        .padding(Padding::from([6, 10]))
        .on_press(message)
        .style(move |_, _| iced::widget::button::Style {
            background: Some(Background::Color(theme.surface)),
            text_color: theme.text,
            border: iced::Border {
                radius: 9.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
}

#[cfg(test)]
mod tests {
    use super::{
        audio_percent_label, battery_percent_label, bluetooth_pill_summary,
        detail_popup_header_action, device_summary_items, display_pill_summary,
        domain_nav_focus_popup, domain_popup_nav_items, power_summary_items,
    };
    use crate::app::Popup;
    use crate::services::{
        controls::{
            AudioDeviceSummary, AudioInfo, BatteryInfo, BluetoothConnectedDevice,
            BluetoothDeviceSummary, BrightnessSnapshot, ControlsSnapshot, FanInfo, MicInfo,
        },
        wayland_runtime::{WaylandOutputInfo, WaylandRuntimeSnapshot},
    };

    #[test]
    fn detail_popup_header_action_remains_contextual() {
        assert_eq!(
            detail_popup_header_action(&Popup::Displays).map(|action| (
                action.icon,
                action.label,
                action.target
            )),
            Some(("󰈈", "System Info", Popup::SystemMonitor))
        );
        assert_eq!(detail_popup_header_action(&Popup::AudioRoutes), None);
    }

    #[test]
    fn detail_popup_focus_maps_to_primary_domains() {
        assert_eq!(domain_nav_focus_popup(&Popup::AudioRoutes), Popup::Controls);
        assert_eq!(domain_nav_focus_popup(&Popup::Displays), Popup::Controls);
        assert_eq!(
            domain_nav_focus_popup(&Popup::BluetoothDevices),
            Popup::Connectivity
        );
        assert_eq!(domain_nav_focus_popup(&Popup::SystemMonitor), Popup::Stats);
    }

    #[test]
    fn domain_popup_nav_items_cover_variant_a_domains() {
        assert_eq!(
            domain_popup_nav_items(&Popup::Controls),
            [
                ("", "Stats", Popup::Stats, false),
                ("", "Power", Popup::Power, false),
                ("󰖀", "Controls", Popup::Controls, true),
                ("󰖩", "Connectivity", Popup::Connectivity, false),
            ]
        );
    }

    #[test]
    fn power_summary_items_surface_daily_battery_state() {
        let battery = BatteryInfo {
            capacity: 84,
            status: "Discharging".to_string(),
            time_remaining: Some("4h 20m remaining".to_string()),
            health_percent: Some(91),
            power_rate_mw: Some(12_300),
            pack_voltage_mv: Some(15_400),
            cycle_count: Some(120),
            full_charge_mwh: Some(74_000),
            design_capacity_mwh: Some(80_800),
            ac_online: Some(false),
            charge_start_threshold: Some(75),
            charge_end_threshold: Some(95),
        };

        let items = power_summary_items(&battery);
        assert_eq!(
            items[0],
            ("󰁹", "Battery Runtime", "84% (4h 20m remaining)".to_string())
        );
        assert_eq!(items[1], ("󱐌", "Charge State", "Discharging".to_string()));
        assert_eq!(items[2], ("󰚥", "AC Adapter", "Disconnected".to_string()));
        assert_eq!(
            items[3],
            ("󱐋", "Charge / Draw Power", "12.3 W draw".to_string())
        );
        assert_eq!(
            items[4],
            ("󱞊", "Charge Thresholds", "75% -> 95%".to_string())
        );
    }

    #[test]
    fn device_summary_items_surface_audio_mic_and_bluetooth_state() {
        let items = device_summary_items(&ControlsSnapshot {
            brightness: BrightnessSnapshot {
                percent: 52,
                label: "52%".to_string(),
            },
            audio: AudioInfo {
                volume: 67,
                muted: false,
            },
            mic: MicInfo {
                volume: 42,
                muted: true,
            },
            fan: FanInfo {
                speed: "3200".to_string(),
                level: "3".to_string(),
            },
            battery: BatteryInfo {
                capacity: 0,
                status: "Unknown".to_string(),
                time_remaining: None,
                ac_online: None,
                health_percent: None,
                power_rate_mw: None,
                pack_voltage_mv: None,
                cycle_count: None,
                full_charge_mwh: None,
                design_capacity_mwh: None,
                charge_start_threshold: None,
                charge_end_threshold: None,
            },
            power_profile: "balanced".to_string(),
            bluetooth_enabled: true,
            bluetooth_devices: BluetoothDeviceSummary {
                connected_devices: vec!["WH-1000XM4".to_string()],
                device_details: vec![BluetoothConnectedDevice {
                    address: "AA:BB:CC:DD:EE:FF".to_string(),
                    name: "WH-1000XM4".to_string(),
                    connected: true,
                    paired: true,
                    trusted: true,
                    battery_percent: Some(88),
                    audio_profiles: vec!["A2DP".to_string()],
                }],
            },
            audio_devices: AudioDeviceSummary {
                output_route: Some("WH-1000XM4".to_string()),
                input_route: Some("Internal Microphone".to_string()),
                output_routes: Vec::new(),
                input_routes: Vec::new(),
            },
        });

        assert_eq!(items[0].icon, "");
        assert_eq!(items[0].value, "67% output");
        assert_eq!(items[0].detail.as_deref(), Some("WH-1000XM4"));
        assert_eq!(items[1].icon, "");
        assert_eq!(items[1].value, "Muted");
        assert_eq!(items[1].detail.as_deref(), Some("Internal Microphone"));
        assert_eq!(items[2].icon, "󰂯");
        assert_eq!(items[2].value, "1 connected");
        assert_eq!(items[2].detail.as_deref(), Some("WH-1000XM4"));
    }

    #[test]
    fn display_pill_summary_uses_wayland_mode_and_hides_when_unavailable() {
        assert_eq!(
            display_pill_summary(&WaylandRuntimeSnapshot {
                available: true,
                outputs: vec![WaylandOutputInfo {
                    global_name: 1,
                    version: 4,
                    name: Some("eDP-1".to_string()),
                    description: None,
                    scale_factor: Some(1),
                    width: Some(1920),
                    height: Some(1200),
                    refresh_mhz: Some(60_000),
                    make: Some("Lenovo".to_string()),
                    model: Some("Internal".to_string()),
                }],
                ..WaylandRuntimeSnapshot::default()
            }),
            Some(("󰌢", "Laptop".to_string()))
        );
        assert_eq!(
            display_pill_summary(&WaylandRuntimeSnapshot::default()),
            None
        );
    }

    #[test]
    fn bluetooth_pill_summary_hides_text_when_adapter_disabled() {
        let mut controls = ControlsSnapshot::default();
        assert_eq!(bluetooth_pill_summary(&controls), None);

        controls.bluetooth_enabled = true;
        assert_eq!(bluetooth_pill_summary(&controls), None);

        controls.bluetooth_devices.connected_devices = vec!["Headphones".to_string()];
        assert_eq!(bluetooth_pill_summary(&controls), Some("1".to_string()));
    }

    #[test]
    fn battery_and_audio_percent_labels_are_stable() {
        assert_eq!(
            battery_percent_label(&BatteryInfo {
                capacity: 42,
                status: "Unknown".to_string(),
                time_remaining: None,
                ac_online: None,
                health_percent: None,
                power_rate_mw: None,
                pack_voltage_mv: None,
                cycle_count: None,
                full_charge_mwh: None,
                design_capacity_mwh: None,
                charge_start_threshold: None,
                charge_end_threshold: None,
            }),
            "42%"
        );
        assert_eq!(
            audio_percent_label(&AudioInfo {
                volume: 65,
                muted: false
            }),
            "65%"
        );
    }
}
