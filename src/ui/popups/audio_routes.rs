use iced::{
    widget::{button, container, scrollable, text, Column, Row, Space},
    Alignment, Color, Element, Length, Padding,
};

use crate::{
    app::{Message, Popup},
    ui::{chrome, theme::ThemeTokens},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioRoutePopupItem {
    pub id: String,
    pub label: String,
    pub icon: &'static str,
    pub capability_label: &'static str,
    pub origin_label: &'static str,
    pub profile_label: &'static str,
    pub status_label: &'static str,
    pub warning_label: Option<&'static str>,
    pub detail: String,
    pub is_default: bool,
    pub available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioRoutesPopupModel {
    pub output_summary: String,
    pub input_summary: String,
    pub output_routes: Vec<AudioRoutePopupItem>,
    pub input_routes: Vec<AudioRoutePopupItem>,
}

impl AudioRoutesPopupModel {
    pub fn new(
        output_summary: String,
        input_summary: String,
        output_routes: Vec<AudioRoutePopupItem>,
        input_routes: Vec<AudioRoutePopupItem>,
    ) -> Self {
        Self {
            output_summary,
            input_summary,
            output_routes,
            input_routes,
        }
    }
}

pub fn view(
    theme: ThemeTokens,
    opacity: f32,
    popup: Popup,
    model: AudioRoutesPopupModel,
) -> Element<'static, Message> {
    let summary_item = |label: &str, val: String| -> Element<'static, Message> {
        Row::new()
            .spacing(12)
            .align_y(Alignment::Center)
            .push(
                text(label.to_string())
                    .size(13)
                    .width(Length::FillPortion(2)),
            )
            .push(
                text(val)
                    .size(13)
                    .width(Length::FillPortion(3))
                    .align_x(iced::alignment::Horizontal::Right),
            )
            .into()
    };

    let content = Column::new()
        .spacing(14)
        .push(chrome::detail_popup_header_row(
            theme,
            "Audio Routes",
            &popup,
        ))
        .push(chrome::domain_popup_nav_row(
            theme,
            &chrome::domain_nav_focus_popup(&popup),
        ))
        .push(summary_item("Active Output Device", model.output_summary))
        .push(summary_item("Active Input Device", model.input_summary))
        .push(route_section(
            "Output Routes",
            "󰕾",
            model.output_routes,
            true,
        ))
        .push(route_section(
            "Input Routes",
            "",
            model.input_routes,
            false,
        ));

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

fn route_section(
    title: &'static str,
    icon: &'static str,
    items: Vec<AudioRoutePopupItem>,
    is_output: bool,
) -> iced::widget::Container<'static, Message> {
    let mut column = Column::new().spacing(8).push(
        Row::new()
            .spacing(8)
            .align_y(Alignment::Center)
            .push(text(icon).size(16))
            .push(text(title).size(14)),
    );

    if items.is_empty() {
        column = column.push(text("No routes discovered").size(12).style(|_| {
            iced::widget::text::Style {
                color: Some(Color::from_rgb8(0x86, 0x90, 0xb2)),
            }
        }));
    } else {
        for (origin_label, grouped_items) in group_by_origin(&items) {
            column = column.push(text(format!("{origin_label} family")).size(11).style(|_| {
                iced::widget::text::Style {
                    color: Some(Color::from_rgb8(0x86, 0x90, 0xb2)),
                }
            }));
            for item in grouped_items {
                column = column.push(route_button(item, is_output));
            }
        }
    }

    container(column).padding(16).style(|_| container::Style {
        background: Some(iced::Background::Color(Color::from_rgb8(0x21, 0x26, 0x38))),
        border: iced::Border {
            radius: 12.0.into(),
            ..Default::default()
        },
        ..Default::default()
    })
}

fn route_button(
    item: AudioRoutePopupItem,
    is_output: bool,
) -> iced::widget::Button<'static, Message> {
    let message = if is_output {
        Message::SetAudioOutputRoute(item.id.clone())
    } else {
        Message::SetAudioInputRoute(item.id.clone())
    };
    let detail = if item.is_default {
        format!("Selected default • {}", item.detail)
    } else {
        item.detail.clone()
    };
    button(
        Column::new()
            .spacing(4)
            .push(
                Row::new()
                    .spacing(6)
                    .align_y(Alignment::Center)
                    .push(text(item.icon).size(13))
                    .push(text(item.label.clone()).size(13))
                    .push(route_badge(
                        item.capability_label,
                        Color::from_rgb8(0x36, 0x3d, 0x59),
                        Color::from_rgb8(0xc0, 0xca, 0xf5),
                    ))
                    .push(route_badge(
                        item.status_label,
                        Color::from_rgb8(0x2f, 0x43, 0x52),
                        Color::from_rgb8(0xc8, 0xdf, 0xf8),
                    ))
                    .push(route_badge(
                        item.profile_label,
                        Color::from_rgb8(0x4b, 0x36, 0x59),
                        Color::from_rgb8(0xe1, 0xcf, 0xf8),
                    ))
                    .push_maybe(item.warning_label.map(|warning| {
                        route_badge(
                            warning,
                            Color::from_rgb8(0x4f, 0x34, 0x1a),
                            Color::from_rgb8(0xf6, 0xd7, 0x92),
                        )
                    }))
                    .push(route_badge(
                        item.origin_label,
                        if item.available {
                            Color::from_rgb8(0x3a, 0x3f, 0x61)
                        } else {
                            Color::from_rgb8(0x53, 0x31, 0x31)
                        },
                        if item.available {
                            Color::from_rgb8(0xc0, 0xca, 0xf5)
                        } else {
                            Color::from_rgb8(0xf7, 0xc0, 0xc0)
                        },
                    ))
                    .push(Space::with_width(Length::Fill))
                    .push_maybe((!item.id.is_empty()).then(|| {
                        text(format!("#{}", item.id)).size(10).style(|_| {
                            iced::widget::text::Style {
                                color: Some(Color::from_rgb8(0x86, 0x90, 0xb2)),
                            }
                        })
                    })),
            )
            .push(text(detail).size(11).style(|_| iced::widget::text::Style {
                color: Some(Color::from_rgb8(0x9a, 0xb0, 0xe6)),
            })),
    )
    .width(Length::Fill)
    .padding(12)
    .on_press_maybe((item.available && !item.is_default).then_some(message))
    .style(|_, _| iced::widget::button::Style {
        background: Some(iced::Background::Color(Color::from_rgb8(0x29, 0x2e, 0x42))),
        text_color: Color::from_rgb8(0xc0, 0xca, 0xf5),
        border: iced::Border {
            radius: 10.0.into(),
            ..Default::default()
        },
        ..Default::default()
    })
}

fn route_badge(
    label: &'static str,
    bg: Color,
    fg: Color,
) -> iced::widget::Container<'static, Message> {
    container(text(label).size(9))
        .padding(Padding::from([2, 6]))
        .style(move |_| iced::widget::container::Style {
            background: Some(iced::Background::Color(bg)),
            text_color: Some(fg),
            border: iced::Border {
                radius: 999.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
}

pub fn group_by_origin(
    items: &[AudioRoutePopupItem],
) -> Vec<(&'static str, Vec<AudioRoutePopupItem>)> {
    let mut groups: Vec<(&'static str, Vec<AudioRoutePopupItem>)> = Vec::new();
    for item in items {
        if let Some((_, grouped_items)) = groups
            .iter_mut()
            .find(|(origin_label, _)| *origin_label == item.origin_label)
        {
            grouped_items.push(item.clone());
        } else {
            groups.push((item.origin_label, vec![item.clone()]));
        }
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::{AudioRoutePopupItem, AudioRoutesPopupModel};

    #[test]
    fn audio_routes_popup_model_preserves_routes_and_summaries() {
        let model = AudioRoutesPopupModel::new(
            "Built-in Audio • Integrated device path".to_string(),
            "Internal Microphone • Integrated device path".to_string(),
            vec![AudioRoutePopupItem {
                id: "1".to_string(),
                label: "Built-in Audio".to_string(),
                icon: "󰓃",
                capability_label: "SINK",
                origin_label: "INTERNAL",
                profile_label: "ANALOG",
                status_label: "ACTIVE",
                warning_label: None,
                detail: "Integrated device path".to_string(),
                is_default: true,
                available: true,
            }],
            vec![],
        );

        assert_eq!(model.output_routes.len(), 1);
        assert_eq!(model.output_routes[0].label, "Built-in Audio");
        assert_eq!(
            model.output_summary,
            "Built-in Audio • Integrated device path"
        );
    }
}
