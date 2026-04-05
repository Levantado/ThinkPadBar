use iced::{
    widget::{button, container, scrollable, text, Column, Row, Space},
    Alignment, Color, Element, Length, Padding,
};

use crate::{
    app::{Message, Popup},
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
}

pub fn view(
    theme: ThemeTokens,
    opacity: f32,
    model: BluetoothDevicesPopupModel,
) -> Element<'static, Message> {
    let mut content = Column::new()
        .spacing(14)
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
        .push(scan_button(model.bluetooth_enabled, model.scan_running))
        .push(
            button(
                Row::new()
                    .spacing(6)
                    .align_y(Alignment::Center)
                    .push(text("󰳋").size(14))
                    .push(text("Open Overskride").size(12)),
            )
            .padding(Padding::from([8, 10]))
            .on_press(Message::OpenOverskride),
        );

    if model.device_cards.is_empty() {
        content = content.push(empty_state_card());
    } else {
        for device in model.device_cards {
            content = content.push(device_card(device, model.bluetooth_enabled));
        }
    }

    container(
        container(scrollable(content))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(Padding::from([20, 24]))
            .style(move |_| iced::widget::container::Style {
                background: Some(iced::Background::Color(Color {
                    a: opacity,
                    ..Color::from_rgb8(0x11, 0x12, 0x1d)
                })),
                text_color: Some(Color::from_rgb8(0xc0, 0xca, 0xf5)),
                border: iced::Border {
                    radius: 12.0.into(),
                    ..Default::default()
                },
                ..Default::default()
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
}

fn empty_state_card() -> iced::widget::Container<'static, Message> {
    container(text("No Bluetooth devices discovered").size(12).style(|_| {
        iced::widget::text::Style {
            color: Some(Color::from_rgb8(0x86, 0x90, 0xb2)),
        }
    }))
    .padding(12)
    .style(|_| iced::widget::container::Style {
        background: Some(iced::Background::Color(Color::from_rgb8(0x21, 0x26, 0x38))),
        border: iced::Border {
            radius: 10.0.into(),
            ..Default::default()
        },
        ..Default::default()
    })
}

fn device_card(
    device: BluetoothDeviceCard,
    bluetooth_enabled: bool,
) -> iced::widget::Container<'static, Message> {
    let mut badges_row = Row::new().spacing(6);
    for badge_label in device.badges.iter().cloned() {
        badges_row = badges_row.push(badge(badge_label));
    }
    if device.is_new {
        badges_row = badges_row.push(badge("NEW".to_string()));
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
                    .push(action_button("Pair", pair_action))
                    .push(action_button("Trust", trust_action))
                    .push(action_button("Remove", remove_action)),
            ),
    )
    .padding(12)
    .style(|_| iced::widget::container::Style {
        background: Some(iced::Background::Color(Color::from_rgb8(0x21, 0x26, 0x38))),
        border: iced::Border {
            radius: 10.0.into(),
            ..Default::default()
        },
        ..Default::default()
    })
}

fn action_button(label: &'static str, message: Option<Message>) -> Element<'static, Message> {
    button(text(label).size(10))
        .padding(Padding::from([4, 8]))
        .on_press_maybe(message)
        .into()
}

fn badge(label: String) -> iced::widget::Container<'static, Message> {
    container(text(label).size(9))
        .padding(Padding::from([2, 6]))
        .style(|_| iced::widget::container::Style {
            background: Some(iced::Background::Color(Color::from_rgb8(0x41, 0x48, 0x68))),
            text_color: Some(Color::from_rgb8(0xc0, 0xca, 0xf5)),
            border: iced::Border {
                radius: 999.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
}

#[cfg(test)]
mod tests {
    use super::{BluetoothDeviceCard, BluetoothDevicesPopupModel};

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
}
