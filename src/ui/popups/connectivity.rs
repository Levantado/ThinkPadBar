use iced::{
    widget::{button, container, scrollable, text, text_input, Column, Row, Space},
    Alignment, Color, Element, Length, Padding,
};

use crate::{
    app::{Message, Popup},
    ui::{chrome, popups::bluetooth_devices::BluetoothDevicesPopupModel, theme::ThemeTokens},
};

#[derive(Debug, Clone, PartialEq)]
pub struct ConnectivityPopupModel {
    pub wifi_snapshot: crate::services::network::NetworkSnapshot,
    pub bluetooth_enabled: bool,
    pub bluetooth_devices: crate::services::controls::BluetoothDeviceSummary,
    pub opacity: f32,
}

impl ConnectivityPopupModel {
    pub fn new(
        wifi_snapshot: &crate::services::network::NetworkSnapshot,
        controls: &crate::services::controls::ControlsSnapshot,
        opacity: f32,
    ) -> Self {
        Self {
            wifi_snapshot: wifi_snapshot.clone(),
            bluetooth_enabled: controls.bluetooth_enabled,
            bluetooth_devices: controls.bluetooth_devices.clone(),
            opacity,
        }
    }
}

pub fn view(theme: ThemeTokens, model: ConnectivityPopupModel) -> Element<'static, Message> {
    let type_scale = super::standard_popup_type_scale();
    let layout = super::standard_domain_popup_layout();
    let wifi_is_active = model.wifi_snapshot.wifi.enabled;
    let ssid = model.wifi_snapshot.wifi.ssid.trim();
    let has_real_ssid =
        !ssid.is_empty() && ssid != "Disconnected" && ssid != "Loading..." && ssid != "Unknown";
    let wifi_label = if wifi_is_active {
        if has_real_ssid {
            if ssid.len() > 10 {
                format!("{}...", ssid.chars().take(8).collect::<String>())
            } else {
                ssid.to_string()
            }
        } else {
            "On".to_string()
        }
    } else {
        "Off".to_string()
    };

    let wifi_btn = button(
        Row::new()
            .spacing(4)
            .align_y(Alignment::Center)
            .push(text(if wifi_is_active { "󰖩" } else { "󰖪" }).size(18))
            .push(text(wifi_label).size(type_scale.meta)),
    )
    .width(Length::FillPortion(1))
    .height(Length::Fixed(40.0))
    .padding(Padding::from([12, 12]))
    .on_press(Message::NetworkCommand(
        crate::services::network::NetworkCommand::ToggleMenu,
    ))
    .style(move |_, status| {
        chrome::popup_button_style(
            theme,
            status,
            if wifi_is_active {
                chrome::PopupButtonTone::Accent
            } else {
                chrome::PopupButtonTone::Surface
            },
            true,
        )
    });

    let bt_is_active = model.bluetooth_enabled;
    let bt_label = if bt_is_active { "On" } else { "Off" };
    let bt_btn = button(
        Row::new()
            .spacing(4)
            .align_y(Alignment::Center)
            .push(text(if bt_is_active { "󰂯" } else { "󰂲" }).size(18))
            .push(text(bt_label).size(type_scale.meta)),
    )
    .width(Length::FillPortion(1))
    .height(Length::Fixed(40.0))
    .padding(Padding::from([12, 12]))
    .on_press(Message::ToggleBluetooth(!bt_is_active))
    .style(move |_, status| {
        chrome::popup_button_style(
            theme,
            status,
            if bt_is_active {
                chrome::PopupButtonTone::Accent
            } else {
                chrome::PopupButtonTone::Surface
            },
            true,
        )
    });

    let connected_bluetooth_cards =
        BluetoothDevicesPopupModel::build_device_cards(&model.bluetooth_devices)
            .into_iter()
            .filter(|card| card.connected)
            .collect::<Vec<_>>();

    let bluetooth_quick_card = {
        let mut column = Column::new().spacing(8).push(
            Row::new()
                .spacing(8)
                .align_y(Alignment::Center)
                .push(text("󰂯").size(16))
                .push(text("Connected Bluetooth").size(type_scale.section))
                .push(Space::with_width(Length::Fill))
                .push(
                    button(text("Open").size(type_scale.meta))
                        .height(Length::Fixed(28.0))
                        .padding(Padding::from([4, 8]))
                        .on_press(Message::TogglePopup(Popup::BluetoothDevices)),
                ),
        );

        if connected_bluetooth_cards.is_empty() {
            column = column.push(
                text("No connected Bluetooth devices")
                    .size(type_scale.meta)
                    .style(|_| iced::widget::text::Style {
                        color: Some(Color::from_rgb8(0x86, 0x90, 0xb2)),
                    }),
            );
        } else {
            for card in connected_bluetooth_cards {
                let summary = if let Some(detail) = &card.detail {
                    format!("{} • {}", card.summary, detail)
                } else {
                    card.summary.clone()
                };
                column = column.push(
                    container(
                        Column::new()
                            .spacing(6)
                            .push(
                                Row::new()
                                    .spacing(8)
                                    .align_y(Alignment::Center)
                                    .push(text(card.label).size(type_scale.body))
                                    .push(Space::with_width(Length::Fill))
                                    .push(
                                        button(text("Disconnect").size(type_scale.meta))
                                            .height(Length::Fixed(28.0))
                                            .padding(Padding::from([6, 10]))
                                            .on_press(Message::DisconnectBluetoothDevice(
                                                card.address.clone(),
                                            )),
                                    ),
                            )
                            .push(text(summary).size(type_scale.meta).style(|_| {
                                iced::widget::text::Style {
                                    color: Some(Color::from_rgb8(0x9a, 0xb0, 0xe6)),
                                }
                            })),
                    )
                    .padding(10)
                    .style(move |_| chrome::popup_card_alt_style(theme)),
                );
            }
        }

        container(column)
            .padding(layout.card_padding)
            .style(move |_| chrome::popup_card_style(theme))
    };

    let mut container_col = Column::new()
        .spacing(layout.section_spacing)
        .push(chrome::detail_popup_header_row(
            theme,
            "Connectivity",
            &Popup::Connectivity,
        ))
        .push(chrome::domain_popup_nav_row(theme, &Popup::Connectivity))
        .push(Row::new().spacing(16).push(wifi_btn).push(bt_btn));

    if model.wifi_snapshot.menu_open {
        let mut inner_col = Column::new().spacing(8);
        if let Some(status_message) = model
            .wifi_snapshot
            .status_message()
            .map(|message| message.into_owned())
        {
            inner_col = inner_col.push(text(status_message).size(type_scale.meta).style(|_| {
                iced::widget::text::Style {
                    color: Some(Color::from_rgb8(0x9a, 0xb0, 0xe6)),
                }
            }));
        }
        let toggle_power_btn = button(
            text(if wifi_is_active {
                "Отключить Wi-Fi"
            } else {
                "Включить Wi-Fi"
            })
            .size(14),
        )
        .on_press(Message::NetworkCommand(
            crate::services::network::NetworkCommand::ToggleWifi(!wifi_is_active),
        ))
        .width(Length::Fill)
        .height(Length::Fixed(36.0))
        .padding(8)
        .style(move |_, status| {
            chrome::popup_button_style(theme, status, chrome::PopupButtonTone::SurfaceAlt, true)
        });
        inner_col = inner_col.push(toggle_power_btn);

        if let Some(ssid) = model.wifi_snapshot.awaiting_password_ssid() {
            let input = text_input("Enter password...", &model.wifi_snapshot.password_input)
                .on_input(|value| {
                    Message::NetworkCommand(
                        crate::services::network::NetworkCommand::UpdatePassword(value),
                    )
                })
                .on_submit(Message::NetworkCommand(
                    crate::services::network::NetworkCommand::SubmitPassword,
                ))
                .secure(true)
                .padding(10);
            let actions = Row::new()
                .spacing(8)
                .push(
                    button(text("Connect"))
                        .height(Length::Fixed(32.0))
                        .on_press(Message::NetworkCommand(
                            crate::services::network::NetworkCommand::SubmitPassword,
                        ))
                        .padding(8)
                        .style(move |_, status| {
                            chrome::popup_button_style(
                                theme,
                                status,
                                chrome::PopupButtonTone::Accent,
                                true,
                            )
                        }),
                )
                .push(
                    button(text("Cancel"))
                        .height(Length::Fixed(32.0))
                        .on_press(Message::NetworkCommand(
                            crate::services::network::NetworkCommand::CancelPassword,
                        ))
                        .padding(8)
                        .style(move |_, status| {
                            chrome::popup_button_style(
                                theme,
                                status,
                                chrome::PopupButtonTone::Surface,
                                true,
                            )
                        }),
                );
            inner_col = inner_col
                .push(text(format!("Connect to {}", ssid)))
                .push(input)
                .push(actions);
        } else {
            let mut net_list = Column::new().spacing(4);
            for net in &model.wifi_snapshot.available_networks {
                net_list = net_list.push(
                    button(text(net.ssid.clone()))
                        .width(Length::Fill)
                        .on_press(Message::NetworkCommand(
                            crate::services::network::NetworkCommand::SelectNetwork {
                                ssid: net.ssid.clone(),
                                security: net.security.clone(),
                            },
                        ))
                        .style(move |_, status| {
                            chrome::popup_button_style(
                                theme,
                                status,
                                chrome::PopupButtonTone::Ghost,
                                true,
                            )
                        }),
                );
            }
            inner_col = inner_col.push(scrollable(net_list).height(Length::Fixed(150.0)));
        }
        container_col = container_col.push(
            container(inner_col)
                .padding(layout.card_padding)
                .style(move |_| chrome::popup_card_style(theme)),
        );
    }

    container_col = container_col.push(bluetooth_quick_card).push(
        Row::new()
            .spacing(8)
            .push(shortcut_button(
                theme,
                "󰂯",
                "Bluetooth Devices".to_string(),
                Message::TogglePopup(Popup::BluetoothDevices),
            ))
            .push(shortcut_button(
                theme,
                "󰈈",
                "System Info".to_string(),
                Message::TogglePopup(Popup::SystemMonitor),
            )),
    );

    container(container_col)
        .padding(Padding::from([
            layout.outer_padding_y,
            layout.outer_padding_x,
        ]))
        .width(Length::Fixed(f32::from(layout.width)))
        .style(move |_| {
            let mut style = chrome::popup_panel_style(theme);
            style.background = Some(iced::Background::Color(Color {
                a: model.opacity,
                ..theme.panel
            }));
            style
        })
        .into()
}

fn shortcut_button(
    theme: ThemeTokens,
    icon: &'static str,
    label: String,
    message: Message,
) -> Element<'static, Message> {
    button(
        Row::new()
            .spacing(6)
            .align_y(Alignment::Center)
            .push(text(icon).size(14))
            .push(text(label).size(11)),
    )
    .width(Length::FillPortion(1))
    .height(Length::Fixed(40.0))
    .padding(Padding::from([12, 12]))
    .on_press(message)
    .style(move |_, status| {
        chrome::popup_button_style(theme, status, chrome::PopupButtonTone::SurfaceAlt, true)
    })
    .into()
}

#[cfg(test)]
mod tests {
    use super::ConnectivityPopupModel;
    use crate::services::{
        controls::{BluetoothConnectedDevice, ControlsSnapshot},
        network::{NetworkBackendKind, NetworkSnapshot, NetworkStatus, WifiInfo},
    };

    #[test]
    fn connectivity_popup_model_builder_preserves_wifi_and_bluetooth_state() {
        let wifi_snapshot = NetworkSnapshot {
            wifi: WifiInfo {
                enabled: true,
                ssid: "TestNet".to_string(),
            },
            menu_open: false,
            available_networks: Vec::new(),
            password_input: String::new(),
            status: NetworkStatus::Info("Connected".to_string()),
            configured_backend: NetworkBackendKind::Iwd,
            active_backend: NetworkBackendKind::Iwd,
        };
        let controls = ControlsSnapshot {
            bluetooth_enabled: true,
            bluetooth_devices: crate::services::controls::BluetoothDeviceSummary {
                connected_devices: vec!["Headphones".to_string()],
                device_details: vec![BluetoothConnectedDevice {
                    address: "AA:BB:CC:DD:EE:FF".to_string(),
                    name: "Headphones".to_string(),
                    connected: true,
                    paired: true,
                    trusted: true,
                    battery_percent: Some(70),
                    audio_profiles: vec!["A2DP".to_string()],
                }],
            },
            ..ControlsSnapshot::default()
        };

        let model = ConnectivityPopupModel::new(&wifi_snapshot, &controls, 0.82);

        assert!(model.wifi_snapshot.wifi.enabled);
        assert_eq!(model.wifi_snapshot.wifi.ssid, "TestNet");
        assert!(model.bluetooth_enabled);
        assert_eq!(
            model.bluetooth_devices.connected_devices,
            vec!["Headphones"]
        );
        assert_eq!(model.opacity, 0.82);
    }
}
