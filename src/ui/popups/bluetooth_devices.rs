use iced::{
    widget::{button, container, scrollable, text, Column, Row, Space},
    Alignment, Color, Element, Length, Padding,
};

use crate::{
    app::{BluetoothScanState, Message, Popup},
    ui::{chrome, theme::ThemeTokens},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BluetoothDeviceCard {
    pub address: String,
    pub label: String,
    pub summary: String,
    pub detail: Option<String>,
    pub badges: Vec<String>,
    pub connected: bool,
    pub paired: bool,
    pub trusted: bool,
    pub is_new: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BluetoothDevicesPopupModel {
    pub adapter_summary: String,
    pub scan_status: String,
    pub bluetooth_enabled: bool,
    pub scan_running: bool,
    pub device_cards: Vec<BluetoothDeviceCard>,
}

impl BluetoothDevicesPopupModel {
    pub fn new(
        adapter_summary: String,
        scan_status: String,
        bluetooth_enabled: bool,
        scan_running: bool,
        device_cards: Vec<BluetoothDeviceCard>,
    ) -> Self {
        Self {
            adapter_summary,
            scan_status,
            bluetooth_enabled,
            scan_running,
            device_cards,
        }
    }

    pub fn build_device_cards(
        bluetooth: &crate::services::controls::BluetoothDeviceSummary,
    ) -> Vec<BluetoothDeviceCard> {
        bluetooth
            .device_details
            .iter()
            .map(|device| {
                let summary = match (device.connected, device.battery_percent) {
                    (true, Some(percent)) => format!("Connected • Battery {percent}%"),
                    (true, None) => "Connected".to_string(),
                    (false, Some(percent)) => format!("Disconnected • Battery {percent}%"),
                    (false, None) => "Disconnected".to_string(),
                };
                let detail_parts = std::iter::once(device.address.clone())
                    .chain(
                        (!device.audio_profiles.is_empty())
                            .then(|| device.audio_profiles.join(" • ")),
                    )
                    .collect::<Vec<_>>();
                let detail = (!detail_parts.is_empty()).then(|| detail_parts.join(" • "));
                let mut badges = vec![if device.connected {
                    "CONNECTED".to_string()
                } else {
                    "DISCONNECTED".to_string()
                }];
                if device.paired {
                    badges.push("PAIRED".to_string());
                }
                if device.trusted {
                    badges.push("TRUSTED".to_string());
                }
                if let Some(percent) = device.battery_percent {
                    badges.push(format!("BAT {percent}%"));
                }

                BluetoothDeviceCard {
                    address: device.address.clone(),
                    label: device.name.clone(),
                    summary,
                    detail,
                    badges,
                    connected: device.connected,
                    paired: device.paired,
                    trusted: device.trusted,
                    is_new: false,
                }
            })
            .collect()
    }

    pub fn from_state(
        controls: &crate::services::controls::ControlsSnapshot,
        scan_state: &BluetoothScanState,
    ) -> Self {
        let adapter_summary = if controls.bluetooth_enabled {
            if controls.bluetooth_devices.connected_devices.is_empty() {
                "Adapter enabled".to_string()
            } else {
                format!(
                    "{} connected",
                    controls.bluetooth_devices.connected_devices.len()
                )
            }
        } else {
            "Adapter disabled".to_string()
        };

        let device_cards = Self::build_device_cards(&controls.bluetooth_devices)
            .into_iter()
            .map(|mut card| {
                card.is_new = bluetooth_device_is_new(scan_state, &card.address);
                card
            })
            .collect();
        let scan_running = matches!(scan_state, BluetoothScanState::Scanning { .. });

        Self::new(
            adapter_summary,
            scan_status_summary(scan_state),
            controls.bluetooth_enabled,
            scan_running,
            device_cards,
        )
    }
}

pub fn scan_status_summary(state: &BluetoothScanState) -> String {
    match state {
        BluetoothScanState::Idle => "Idle".to_string(),
        BluetoothScanState::Scanning { remaining_secs, .. } => {
            if *remaining_secs == 0 {
                "Finishing scan...".to_string()
            } else {
                format!("Scanning ({remaining_secs}s left)")
            }
        }
        BluetoothScanState::Completed {
            total_devices,
            newly_discovered_addresses,
            remaining_secs,
        } => {
            if newly_discovered_addresses.is_empty() {
                format!("{total_devices} devices known; no new devices • idle in {remaining_secs}s")
            } else {
                format!(
                    "{total_devices} devices known; {} newly discovered • idle in {remaining_secs}s",
                    newly_discovered_addresses.len()
                )
            }
        }
    }
}

pub fn bluetooth_device_is_new(scan_state: &BluetoothScanState, address: &str) -> bool {
    match scan_state {
        BluetoothScanState::Completed {
            newly_discovered_addresses,
            ..
        } => newly_discovered_addresses
            .iter()
            .any(|candidate| candidate == address),
        _ => false,
    }
}

pub fn view(
    theme: ThemeTokens,
    opacity: f32,
    model: BluetoothDevicesPopupModel,
) -> Element<'static, Message> {
    let type_scale = super::standard_popup_type_scale();
    let layout = super::standard_domain_popup_layout();
    let mut content = Column::new()
        .spacing(layout.section_spacing)
        .push(chrome::detail_popup_header_row(
            theme,
            "Bluetooth Devices",
            &Popup::BluetoothDevices,
        ))
        .push(chrome::domain_popup_nav_row(
            theme,
            &chrome::domain_nav_focus_popup(&Popup::BluetoothDevices),
        ))
        .push(summary_row("Bluetooth Adapter", model.adapter_summary))
        .push(summary_row("Scan Status", model.scan_status))
        .push(scan_button(
            theme,
            model.bluetooth_enabled,
            model.scan_running,
        ))
        .push(
            button(
                Row::new()
                    .spacing(6)
                    .align_y(Alignment::Center)
                    .push(text("󰳋").size(type_scale.section))
                    .push(text("Open Overskride").size(type_scale.meta)),
            )
            .padding(Padding::from([8, 10]))
            .on_press(Message::OpenOverskride),
        );

    if model.device_cards.is_empty() {
        content = content.push(empty_state_card(theme));
    } else {
        for device in model.device_cards {
            content = content.push(device_card(theme, device, model.bluetooth_enabled));
        }
    }

    container(
        container(scrollable(container(content).padding([0, 14, 12, 0])))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(Padding::from([
                layout.outer_padding_y,
                layout.outer_padding_x,
            ]))
            .style(move |_| {
                let mut style = chrome::popup_panel_style(theme);
                style.background = Some(iced::Background::Color(Color {
                    a: opacity,
                    ..theme.panel
                }));
                style
            }),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn summary_row(label: &'static str, value: String) -> Element<'static, Message> {
    Row::new()
        .spacing(12)
        .align_y(Alignment::Center)
        .push(text(label).size(13).width(Length::FillPortion(2)))
        .push(
            text(value)
                .size(13)
                .width(Length::FillPortion(3))
                .align_x(iced::alignment::Horizontal::Right),
        )
        .into()
}

fn scan_button(
    theme: ThemeTokens,
    bluetooth_enabled: bool,
    scan_running: bool,
) -> iced::widget::Button<'static, Message> {
    button(
        Row::new()
            .spacing(6)
            .align_y(Alignment::Center)
            .push(text(if scan_running { "󰓅" } else { "󰥔" }).size(14))
            .push(text(if scan_running { "Stop Scan" } else { "Scan 5s" }).size(12)),
    )
    .padding(Padding::from([8, 10]))
    .on_press_maybe(if bluetooth_enabled {
        Some(if scan_running {
            Message::StopBluetoothScan
        } else {
            Message::ScanBluetoothDevices
        })
    } else {
        None
    })
    .height(Length::Fixed(32.0))
    .style(move |_, status| {
        chrome::popup_button_style(
            theme,
            status,
            if scan_running {
                chrome::PopupButtonTone::Accent
            } else {
                chrome::PopupButtonTone::SurfaceAlt
            },
            bluetooth_enabled,
        )
    })
}

fn empty_state_card(theme: ThemeTokens) -> iced::widget::Container<'static, Message> {
    container(text("No Bluetooth devices discovered").size(12).style(|_| {
        iced::widget::text::Style {
            color: Some(Color::from_rgb8(0x86, 0x90, 0xb2)),
        }
    }))
    .padding(12)
    .style(move |_| chrome::popup_card_alt_style(theme))
}

fn device_card(
    theme: ThemeTokens,
    device: BluetoothDeviceCard,
    bluetooth_enabled: bool,
) -> iced::widget::Container<'static, Message> {
    let mut badges_row = Row::new().spacing(6);
    for badge_label in device.badges.iter().cloned() {
        badges_row = badges_row.push(badge(theme, badge_label));
    }
    if device.is_new {
        badges_row = badges_row.push(badge(theme, "NEW".to_string()));
    }

    let connect_action = if device.connected {
        Message::DisconnectBluetoothDevice(device.address.clone())
    } else {
        Message::ConnectBluetoothDevice(device.address.clone())
    };
    let can_press = bluetooth_enabled || device.connected;
    let pair_action = (!device.paired && bluetooth_enabled)
        .then(|| Message::PairBluetoothDevice(device.address.clone()));
    let trust_action = (device.paired && !device.trusted && bluetooth_enabled)
        .then(|| Message::TrustBluetoothDevice(device.address.clone()));
    let remove_action =
        (!device.connected).then(|| Message::RemoveBluetoothDevice(device.address.clone()));

    container(
        Column::new()
            .spacing(8)
            .push(
                Row::new()
                    .spacing(6)
                    .align_y(Alignment::Center)
                    .push(text("󰂯").size(13))
                    .push(text(device.label.clone()).size(13))
                    .push(Space::with_width(Length::Fill))
                    .push(action_button(
                        theme,
                        if device.connected {
                            "Disconnect"
                        } else {
                            "Connect"
                        },
                        can_press.then_some(connect_action),
                    )),
            )
            .push(badges_row)
            .push(
                text(device.summary)
                    .size(11)
                    .style(|_| iced::widget::text::Style {
                        color: Some(Color::from_rgb8(0xc0, 0xca, 0xf5)),
                    }),
            )
            .push_maybe(device.detail.map(|detail| {
                text(detail).size(11).style(|_| iced::widget::text::Style {
                    color: Some(Color::from_rgb8(0x9a, 0xb0, 0xe6)),
                })
            }))
            .push(
                Row::new()
                    .spacing(8)
                    .push(action_button(theme, "Pair", pair_action))
                    .push(action_button(theme, "Trust", trust_action))
                    .push(action_button(theme, "Remove", remove_action)),
            ),
    )
    .padding(12)
    .style(move |_| chrome::popup_card_alt_style(theme))
}

fn action_button(
    theme: ThemeTokens,
    label: &'static str,
    message: Option<Message>,
) -> Element<'static, Message> {
    let enabled = message.is_some();
    button(text(label).size(10))
        .padding(Padding::from([4, 8]))
        .height(Length::Fixed(28.0))
        .on_press_maybe(message)
        .style(move |_, status| {
            chrome::popup_button_style(theme, status, chrome::PopupButtonTone::Surface, enabled)
        })
        .into()
}

fn badge(theme: ThemeTokens, label: String) -> iced::widget::Container<'static, Message> {
    container(text(label).size(9))
        .padding(Padding::from([2, 6]))
        .style(move |_| chrome::popup_badge_style(theme))
}

#[cfg(test)]
mod tests {
    use super::{
        bluetooth_device_is_new, scan_status_summary, BluetoothDeviceCard,
        BluetoothDevicesPopupModel,
    };
    use crate::app::BluetoothScanState;
    use crate::services::controls::{
        AudioDeviceSummary, AudioInfo, BatteryInfo, BrightnessSnapshot, ControlsSnapshot, FanInfo,
        MicInfo,
    };

    #[test]
    fn bluetooth_devices_popup_model_preserves_scan_state_and_card_flags() {
        let model = BluetoothDevicesPopupModel::new(
            "1 connected".to_string(),
            "Scanning (3s left)".to_string(),
            true,
            true,
            vec![BluetoothDeviceCard {
                address: "AA:BB:CC:DD:EE:FF".to_string(),
                label: "WH-1000XM5".to_string(),
                summary: "Connected • Battery 87%".to_string(),
                detail: Some("A2DP • HFP".to_string()),
                badges: vec!["CONNECTED".to_string(), "BAT 87%".to_string()],
                connected: true,
                paired: true,
                trusted: true,
                is_new: true,
            }],
        );

        assert!(model.bluetooth_enabled);
        assert!(model.scan_running);
        assert_eq!(model.device_cards.len(), 1);
        assert!(model.device_cards[0].is_new);
        assert_eq!(model.device_cards[0].label, "WH-1000XM5");
    }

    #[test]
    fn bluetooth_scan_status_summary_counts_down_and_finishes() {
        assert_eq!(
            scan_status_summary(&BluetoothScanState::Scanning {
                remaining_secs: 3,
                baseline_addresses: Vec::new(),
            }),
            "Scanning (3s left)"
        );
        assert_eq!(
            scan_status_summary(&BluetoothScanState::Scanning {
                remaining_secs: 0,
                baseline_addresses: Vec::new(),
            }),
            "Finishing scan..."
        );
        assert_eq!(
            scan_status_summary(&BluetoothScanState::Completed {
                total_devices: 4,
                newly_discovered_addresses: vec!["AA:BB".to_string(), "CC:DD".to_string()],
                remaining_secs: 6,
            }),
            "4 devices known; 2 newly discovered • idle in 6s"
        );
    }

    #[test]
    fn bluetooth_device_is_new_only_for_completed_scan_results() {
        assert!(bluetooth_device_is_new(
            &BluetoothScanState::Completed {
                total_devices: 2,
                newly_discovered_addresses: vec!["AA:BB".to_string()],
                remaining_secs: 5,
            },
            "AA:BB"
        ));
        assert!(!bluetooth_device_is_new(&BluetoothScanState::Idle, "AA:BB"));
    }

    #[test]
    fn bluetooth_popup_model_builder_maps_state_and_marks_new_devices() {
        let controls = ControlsSnapshot {
            brightness: BrightnessSnapshot {
                percent: 50,
                label: "50%".to_string(),
            },
            audio: AudioInfo {
                volume: 0,
                muted: false,
            },
            mic: MicInfo {
                volume: 0,
                muted: false,
            },
            fan: FanInfo {
                speed: "---".to_string(),
                level: "auto".to_string(),
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
            bluetooth_devices: crate::services::controls::BluetoothDeviceSummary {
                connected_devices: vec!["WH-1000XM5".to_string()],
                device_details: vec![crate::services::controls::BluetoothConnectedDevice {
                    address: "AA:BB:CC:DD:EE:FF".to_string(),
                    name: "WH-1000XM5".to_string(),
                    connected: true,
                    paired: true,
                    trusted: true,
                    battery_percent: Some(87),
                    audio_profiles: vec!["A2DP".to_string()],
                }],
            },
            audio_devices: AudioDeviceSummary::default(),
        };
        let scan_state = BluetoothScanState::Completed {
            total_devices: 1,
            newly_discovered_addresses: vec!["AA:BB:CC:DD:EE:FF".to_string()],
            remaining_secs: 4,
        };

        let model = BluetoothDevicesPopupModel::from_state(&controls, &scan_state);

        assert_eq!(model.adapter_summary, "1 connected");
        assert_eq!(
            model.scan_status,
            "1 devices known; 1 newly discovered • idle in 4s"
        );
        assert!(model.bluetooth_enabled);
        assert!(!model.scan_running);
        assert!(model.device_cards[0].is_new);
    }
}
