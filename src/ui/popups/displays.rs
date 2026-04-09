use iced::{
    widget::{container, scrollable, text, Column, Row, Space},
    Alignment, Color, Element, Length, Padding,
};

use crate::{
    app::{Message, Popup},
    ui::{chrome, theme::ThemeTokens},
};

use super::PopupMetricRow;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayOutputCard {
    pub label: String,
    pub summary: String,
    pub badges: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplaysPopupModel {
    pub summary_rows: Vec<PopupMetricRow>,
    pub missing_capabilities: Option<String>,
    pub output_cards: Vec<DisplayOutputCard>,
}

impl DisplaysPopupModel {
    pub fn new(
        summary_rows: Vec<PopupMetricRow>,
        missing_capabilities: Option<String>,
        output_cards: Vec<DisplayOutputCard>,
    ) -> Self {
        Self {
            summary_rows,
            missing_capabilities,
            output_cards,
        }
    }
}

pub fn summary_rows(
    wayland_snapshot: &crate::services::wayland_runtime::WaylandRuntimeSnapshot,
) -> Vec<PopupMetricRow> {
    vec![
        PopupMetricRow::new(
            "DSP",
            "Display Mode",
            wayland_snapshot.display_mode_summary(),
        ),
        PopupMetricRow::new(
            "TOP",
            "Display Topology",
            wayland_snapshot.output_topology_summary(),
        ),
        PopupMetricRow::new(
            "SCL",
            "Display Scale",
            wayland_snapshot.output_scale_summary(),
        ),
        PopupMetricRow::new("OUT", "Display Outputs", wayland_snapshot.output_summary()),
    ]
}

pub fn output_cards(
    wayland_snapshot: &crate::services::wayland_runtime::WaylandRuntimeSnapshot,
) -> Vec<DisplayOutputCard> {
    if wayland_snapshot.outputs.is_empty() {
        return vec![DisplayOutputCard {
            label: if wayland_snapshot.available {
                "No outputs".to_string()
            } else {
                "Wayland unavailable".to_string()
            },
            summary: wayland_snapshot
                .unavailable_reason
                .clone()
                .unwrap_or_else(|| "No wl_output state available".to_string()),
            badges: Vec::new(),
        }];
    }

    wayland_snapshot
        .outputs
        .iter()
        .map(|output| {
            let mut badges = vec![if output.is_internal() {
                "INTERNAL".to_string()
            } else {
                "EXTERNAL".to_string()
            }];
            if let (Some(width), Some(height)) = (output.width, output.height) {
                badges.push(format!("{width}x{height}"));
            }
            if let Some(refresh_mhz) = output.refresh_mhz {
                let refresh_hz = refresh_mhz as f64 / 1000.0;
                if (refresh_hz.fract() - 0.0).abs() < f64::EPSILON {
                    badges.push(format!("{refresh_hz:.0}Hz"));
                } else {
                    badges.push(format!("{refresh_hz:.1}Hz"));
                }
            }
            if let Some(scale_factor) = output.scale_factor.filter(|scale| *scale > 0) {
                badges.push(format!("{scale_factor}x SCALE"));
            }

            DisplayOutputCard {
                label: output.label(),
                summary: output.detail_label(),
                badges,
            }
        })
        .collect()
}

pub fn view(
    theme: ThemeTokens,
    opacity: f32,
    model: DisplaysPopupModel,
) -> Element<'static, Message> {
    let mut content = Column::new()
        .spacing(14)
        .push(chrome::detail_popup_header_row(
            theme,
            "Displays",
            &Popup::Displays,
        ))
        .push(chrome::domain_popup_nav_row(
            theme,
            &chrome::domain_nav_focus_popup(&Popup::Displays),
        ))
        .push_rows(model.summary_rows.into_iter().map(metric_row));

    if let Some(missing_caps) = model.missing_capabilities {
        content = content.push(metric_row(PopupMetricRow::new(
            "CAP",
            "Missing Caps",
            missing_caps,
        )));
    }

    content = content
        .push(Space::with_height(Length::Fixed(8.0)))
        .push(
            text("Output Details")
                .size(14)
                .style(|_| iced::widget::text::Style {
                    color: Some(Color::from_rgb8(0x7a, 0xa2, 0xf7)),
                }),
        )
        .push(output_cards_column(model.output_cards));

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

fn metric_row(metric: PopupMetricRow) -> Element<'static, Message> {
    Row::new()
        .spacing(12)
        .align_y(Alignment::Center)
        .push(text(metric.label).size(13).width(Length::FillPortion(2)))
        .push(
            text(metric.value)
                .size(13)
                .width(Length::FillPortion(3))
                .align_x(iced::alignment::Horizontal::Right),
        )
        .into()
}

fn output_cards_column(cards: Vec<DisplayOutputCard>) -> Column<'static, Message> {
    let mut column = Column::new().spacing(10);
    for card in cards {
        let mut badges_row = Row::new().spacing(6);
        for badge_label in card.badges {
            badges_row = badges_row.push(output_badge(badge_label));
        }
        column = column.push(
            container(
                Column::new()
                    .spacing(8)
                    .push(text(card.label).size(14))
                    .push(badges_row)
                    .push(
                        text(card.summary)
                            .size(12)
                            .style(|_| iced::widget::text::Style {
                                color: Some(Color::from_rgb8(0x9a, 0xb0, 0xe6)),
                            }),
                    ),
            )
            .padding(12)
            .style(|_| iced::widget::container::Style {
                background: Some(iced::Background::Color(Color::from_rgb8(0x29, 0x2e, 0x42))),
                border: iced::Border {
                    radius: 10.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
        );
    }
    column
}

fn output_badge(label: String) -> iced::widget::Container<'static, Message> {
    container(text(label).size(10))
        .padding(Padding::from([4, 8]))
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

trait ColumnRowsExt<'a> {
    fn push_rows(self, rows: impl IntoIterator<Item = Element<'a, Message>>)
        -> Column<'a, Message>;
}

impl<'a> ColumnRowsExt<'a> for Column<'a, Message> {
    fn push_rows(
        mut self,
        rows: impl IntoIterator<Item = Element<'a, Message>>,
    ) -> Column<'a, Message> {
        for row in rows {
            self = self.push(row);
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{DisplayOutputCard, DisplaysPopupModel};
    use crate::ui::popups::PopupMetricRow;

    #[test]
    fn displays_popup_model_preserves_summary_cards_and_missing_caps() {
        let model = DisplaysPopupModel::new(
            vec![PopupMetricRow::new("DSP", "Display Mode", "Hybrid")],
            Some("ext-session-lock-v1".to_string()),
            vec![DisplayOutputCard {
                label: "eDP-1".to_string(),
                summary: "eDP-1 1920x1200 60Hz 2x".to_string(),
                badges: vec!["INTERNAL".to_string(), "60Hz".to_string()],
            }],
        );

        assert_eq!(model.summary_rows.len(), 1);
        assert_eq!(model.summary_rows[0].label, "Display Mode");
        assert_eq!(
            model.missing_capabilities.as_deref(),
            Some("ext-session-lock-v1")
        );
        assert_eq!(model.output_cards.len(), 1);
        assert_eq!(model.output_cards[0].label, "eDP-1");
    }
}
