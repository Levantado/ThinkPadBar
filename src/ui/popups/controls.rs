use iced::{
    widget::{button, container, slider, text, Column, Row, Space},
    Alignment, Color, Element, Length, Padding,
};

use crate::{
    app::{Message, Popup},
    ui::{chrome, theme::ThemeTokens},
};

#[derive(Debug, Clone, PartialEq)]
pub struct ControlsPopupModel {
    pub brightness: crate::services::controls::BrightnessSnapshot,
    pub audio: crate::services::controls::AudioInfo,
    pub mic: crate::services::controls::MicInfo,
    pub opacity: f32,
}

impl ControlsPopupModel {
    pub fn new(controls: &crate::services::controls::ControlsSnapshot, opacity: f32) -> Self {
        Self {
            brightness: controls.brightness.clone(),
            audio: controls.audio.clone(),
            mic: controls.mic.clone(),
            opacity,
        }
    }
}

pub fn view(theme: ThemeTokens, model: ControlsPopupModel) -> Element<'static, Message> {
    let layout = super::standard_domain_popup_layout();
    let vol_muted = model.audio.muted;
    let brightness_label = model.brightness.label.clone();
    let vol_row = Row::new()
        .spacing(12)
        .align_y(Alignment::Center)
        .push(
            button(
                container(
                    text(if vol_muted || model.audio.volume == 0 {
                        "󰝟"
                    } else {
                        ""
                    })
                    .size(18),
                )
                .width(28)
                .height(28)
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center),
            )
            .on_press(Message::ToggleAudioMute)
            .style(move |_, _| {
                if vol_muted {
                    iced::widget::button::Style {
                        text_color: Color::from_rgb8(0x56, 0x5f, 0x89),
                        ..Default::default()
                    }
                } else {
                    iced::widget::button::Style {
                        text_color: Color::WHITE,
                        ..Default::default()
                    }
                }
            }),
        )
        .push(
            slider(0..=100, model.audio.volume, Message::SetVolume)
                .width(Length::Fill)
                .style(move |_, _| {
                    if vol_muted {
                        iced::widget::slider::Style {
                            rail: iced::widget::slider::Rail {
                                backgrounds: (
                                    iced::Background::Color(Color::from_rgb8(0x41, 0x48, 0x68)),
                                    iced::Background::Color(Color::from_rgb8(0x29, 0x2e, 0x42)),
                                ),
                                width: 4.0,
                                border: iced::Border {
                                    radius: 2.0.into(),
                                    width: 0.0,
                                    color: Color::TRANSPARENT,
                                },
                            },
                            handle: iced::widget::slider::Handle {
                                shape: iced::widget::slider::HandleShape::Circle { radius: 6.0 },
                                background: iced::Background::Color(Color::from_rgb8(
                                    0x56, 0x5f, 0x89,
                                )),
                                border_width: 0.0,
                                border_color: Color::TRANSPARENT,
                            },
                            breakpoint: iced::widget::slider::Breakpoint {
                                color: Color::TRANSPARENT,
                            },
                        }
                    } else {
                        iced::widget::slider::Style {
                            rail: iced::widget::slider::Rail {
                                backgrounds: (
                                    iced::Background::Color(Color::from_rgb8(0x7a, 0xa2, 0xf7)),
                                    iced::Background::Color(Color::from_rgb8(0x29, 0x2e, 0x42)),
                                ),
                                width: 4.0,
                                border: iced::Border {
                                    radius: 2.0.into(),
                                    width: 0.0,
                                    color: Color::TRANSPARENT,
                                },
                            },
                            handle: iced::widget::slider::Handle {
                                shape: iced::widget::slider::HandleShape::Circle { radius: 8.0 },
                                background: iced::Background::Color(Color::from_rgb8(
                                    0x7a, 0xa2, 0xf7,
                                )),
                                border_width: 0.0,
                                border_color: Color::TRANSPARENT,
                            },
                            breakpoint: iced::widget::slider::Breakpoint {
                                color: Color::TRANSPARENT,
                            },
                        }
                    }
                }),
        )
        .push(
            text(format!("{}%", model.audio.volume))
                .size(13)
                .width(Length::Fixed(44.0))
                .align_x(iced::alignment::Horizontal::Right),
        );

    let mic_muted = model.mic.muted;
    let mic_row = Row::new()
        .spacing(12)
        .align_y(Alignment::Center)
        .push(
            button(
                container(
                    text(if mic_muted || model.mic.volume == 0 {
                        "󰍭"
                    } else {
                        ""
                    })
                    .size(18),
                )
                .width(28)
                .height(28)
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center),
            )
            .on_press(Message::ToggleMicMute)
            .style(move |_, _| {
                if mic_muted {
                    iced::widget::button::Style {
                        text_color: Color::from_rgb8(0x56, 0x5f, 0x89),
                        ..Default::default()
                    }
                } else {
                    iced::widget::button::Style {
                        text_color: Color::WHITE,
                        ..Default::default()
                    }
                }
            }),
        )
        .push(
            slider(0..=100, model.mic.volume, Message::SetMicVolume)
                .width(Length::Fill)
                .style(move |_, _| {
                    if mic_muted {
                        iced::widget::slider::Style {
                            rail: iced::widget::slider::Rail {
                                backgrounds: (
                                    iced::Background::Color(Color::from_rgb8(0x41, 0x48, 0x68)),
                                    iced::Background::Color(Color::from_rgb8(0x29, 0x2e, 0x42)),
                                ),
                                width: 4.0,
                                border: iced::Border {
                                    radius: 2.0.into(),
                                    width: 0.0,
                                    color: Color::TRANSPARENT,
                                },
                            },
                            handle: iced::widget::slider::Handle {
                                shape: iced::widget::slider::HandleShape::Circle { radius: 6.0 },
                                background: iced::Background::Color(Color::from_rgb8(
                                    0x56, 0x5f, 0x89,
                                )),
                                border_width: 0.0,
                                border_color: Color::TRANSPARENT,
                            },
                            breakpoint: iced::widget::slider::Breakpoint {
                                color: Color::TRANSPARENT,
                            },
                        }
                    } else {
                        iced::widget::slider::Style {
                            rail: iced::widget::slider::Rail {
                                backgrounds: (
                                    iced::Background::Color(Color::from_rgb8(0x7a, 0xa2, 0xf7)),
                                    iced::Background::Color(Color::from_rgb8(0x29, 0x2e, 0x42)),
                                ),
                                width: 4.0,
                                border: iced::Border {
                                    radius: 2.0.into(),
                                    width: 0.0,
                                    color: Color::TRANSPARENT,
                                },
                            },
                            handle: iced::widget::slider::Handle {
                                shape: iced::widget::slider::HandleShape::Circle { radius: 8.0 },
                                background: iced::Background::Color(Color::from_rgb8(
                                    0x7a, 0xa2, 0xf7,
                                )),
                                border_width: 0.0,
                                border_color: Color::TRANSPARENT,
                            },
                            breakpoint: iced::widget::slider::Breakpoint {
                                color: Color::TRANSPARENT,
                            },
                        }
                    }
                }),
        )
        .push(
            text(format!("{}%", model.mic.volume))
                .size(13)
                .width(Length::Fixed(44.0))
                .align_x(iced::alignment::Horizontal::Right),
        );

    let brt_row = Row::new()
        .spacing(12)
        .align_y(Alignment::Center)
        .push(
            button(
                container(text("󰃠").size(18))
                    .width(28)
                    .height(28)
                    .align_x(iced::alignment::Horizontal::Center)
                    .align_y(iced::alignment::Vertical::Center),
            )
            .on_press(Message::RefreshControls(
                crate::services::controls::ControlsRefreshKind::Brightness,
            ))
            .style(move |_, _| iced::widget::button::Style {
                text_color: Color::WHITE,
                ..Default::default()
            }),
        )
        .push(
            slider(
                1..=100,
                model.brightness.percent.clamp(1, 100),
                Message::SetBrightness,
            )
            .width(Length::Fill)
            .style(move |_, _| iced::widget::slider::Style {
                rail: iced::widget::slider::Rail {
                    backgrounds: (
                        iced::Background::Color(Color::from_rgb8(0x7a, 0xa2, 0xf7)),
                        iced::Background::Color(Color::from_rgb8(0x29, 0x2e, 0x42)),
                    ),
                    width: 4.0,
                    border: iced::Border {
                        radius: 2.0.into(),
                        width: 0.0,
                        color: Color::TRANSPARENT,
                    },
                },
                handle: iced::widget::slider::Handle {
                    shape: iced::widget::slider::HandleShape::Circle { radius: 8.0 },
                    background: iced::Background::Color(Color::from_rgb8(0x7a, 0xa2, 0xf7)),
                    border_width: 0.0,
                    border_color: Color::TRANSPARENT,
                },
                breakpoint: iced::widget::slider::Breakpoint {
                    color: Color::TRANSPARENT,
                },
            }),
        )
        .push(
            text(brightness_label)
                .size(13)
                .width(Length::Fixed(44.0))
                .align_x(iced::alignment::Horizontal::Right),
        );

    let content = Column::new()
        .spacing(layout.section_spacing)
        .push(
            Row::new()
                .align_y(Alignment::Center)
                .push(text("Controls").size(18))
                .push(Space::with_width(Length::Fill))
                .push(
                    button(text("Close").size(12))
                        .padding(Padding::from([6, 10]))
                        .on_press(Message::TogglePopup(Popup::Controls)),
                ),
        )
        .push(chrome::domain_popup_nav_row(theme, &Popup::Controls))
        .push(
            Column::new()
                .spacing(8)
                .push(brt_row)
                .push(vol_row)
                .push(mic_row),
        )
        .push(
            Row::new()
                .spacing(8)
                .push(shortcut_button(
                    "󰑓",
                    "Audio Routes".to_string(),
                    Message::TogglePopup(Popup::AudioRoutes),
                ))
                .push(shortcut_button(
                    "󰈈",
                    "System Info".to_string(),
                    Message::TogglePopup(Popup::SystemMonitor),
                )),
        );

    container(content)
        .padding(Padding::from([
            layout.outer_padding_y,
            layout.outer_padding_x,
        ]))
        .width(Length::Fixed(f32::from(layout.width)))
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(Color {
                a: model.opacity,
                ..Color::from_rgb8(0x1a, 0x1b, 0x26)
            })),
            border: iced::Border {
                radius: 16.0.into(),
                color: Color::from_rgb8(0x29, 0x2e, 0x42),
                width: 1.5,
            },
            ..Default::default()
        })
        .into()
}

fn shortcut_button(
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
    .padding(Padding::from([12, 12]))
    .on_press(message)
    .style(|_, _| iced::widget::button::Style {
        background: Some(iced::Background::Color(Color::from_rgb8(0x41, 0x48, 0x68))),
        text_color: Color::from_rgb8(0xc0, 0xca, 0xf5),
        border: iced::Border {
            radius: 16.0.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}

#[cfg(test)]
mod tests {
    use super::ControlsPopupModel;

    #[test]
    fn controls_popup_model_keeps_only_controls_domain_state() {
        let model = ControlsPopupModel {
            brightness: crate::services::controls::BrightnessSnapshot::from_percent(42),
            audio: crate::services::controls::AudioInfo {
                volume: 55,
                muted: false,
            },
            mic: crate::services::controls::MicInfo {
                volume: 12,
                muted: true,
            },
            opacity: 0.85,
        };

        assert_eq!(model.brightness.percent, 42);
        assert_eq!(model.audio.volume, 55);
        assert!(model.mic.muted);
        assert_eq!(model.opacity, 0.85);
    }

    #[test]
    fn controls_popup_model_builder_maps_only_device_controls_inputs() {
        let mut controls = crate::services::controls::ControlsSnapshot::default();
        controls.brightness = crate::services::controls::BrightnessSnapshot::from_percent(88);
        controls.audio = crate::services::controls::AudioInfo {
            volume: 61,
            muted: true,
        };
        controls.mic = crate::services::controls::MicInfo {
            volume: 27,
            muted: false,
        };
        controls.fan = crate::services::controls::FanInfo {
            speed: "3900".to_string(),
            level: "6".to_string(),
        };

        let model = ControlsPopupModel::new(&controls, 0.72);

        assert_eq!(model.brightness.percent, 88);
        assert_eq!(model.audio.volume, 61);
        assert!(model.audio.muted);
        assert_eq!(model.mic.volume, 27);
        assert!(!model.mic.muted);
        assert_eq!(model.opacity, 0.72);
    }
}
