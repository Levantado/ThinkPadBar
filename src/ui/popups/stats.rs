use iced::{
    widget::{button, container, text, Column, Row, Space},
    Alignment, Color, Element, Length, Padding,
};

use crate::{
    app::{Message, Popup},
    ui::{chrome, theme::ThemeTokens},
};

use super::PopupMetricRow;

#[derive(Debug, Clone, PartialEq)]
pub struct StatsPopupModel {
    pub background_alpha: f32,
    pub rows: Vec<PopupMetricRow>,
}

impl StatsPopupModel {
    pub fn new(
        background_alpha: f32,
        cpu_summary: impl Into<String>,
        mem_summary: impl Into<String>,
        temp_summary: impl Into<String>,
        fan_summary: impl Into<String>,
    ) -> Self {
        Self {
            background_alpha,
            rows: vec![
                PopupMetricRow::new("", "CPU Usage", normalize_value(cpu_summary)),
                PopupMetricRow::new("󰍛", "Memory Usage", normalize_value(mem_summary)),
                PopupMetricRow::new("", "Temperature", normalize_value(temp_summary)),
                PopupMetricRow::new("󰈐", "Fan Runtime", normalize_value(fan_summary)),
            ],
        }
    }
}

pub fn opaque_background_alpha(_configured_alpha: f32) -> f32 {
    1.0
}

pub fn normalize_value(value: impl Into<String>) -> String {
    let value = value.into();
    let normalized = value.trim();
    if normalized.is_empty() {
        "--".to_string()
    } else {
        normalized.to_string()
    }
}

pub fn view(theme: ThemeTokens, model: StatsPopupModel) -> Element<'static, Message> {
    let content = Column::new()
        .width(Length::Shrink)
        .spacing(12)
        .push(
            Row::new()
                .align_y(Alignment::Center)
                .push(text("Stats").size(18))
                .push(Space::with_width(Length::Fill))
                .push(
                    button(text("System Info").size(11))
                        .padding(Padding::from([6, 10]))
                        .on_press(Message::TogglePopup(Popup::SystemMonitor)),
                ),
        )
        .push(chrome::domain_popup_nav_row(theme, &Popup::Stats))
        .push_rows(model.rows.into_iter().map(metric_row));

    let card = container(content)
        .padding(Padding::from([16, 20]))
        .max_width(420.0)
        .style(move |_| iced::widget::container::Style {
            background: Some(iced::Background::Color(Color {
                a: model.background_alpha,
                ..Color::from_rgb8(0x11, 0x12, 0x1d)
            })),
            text_color: Some(Color::from_rgb8(0xc0, 0xca, 0xf5)),
            border: iced::Border {
                radius: 12.0.into(),
                ..Default::default()
            },
            ..Default::default()
        });

    container(card)
        .padding(Padding::from([8, 12]))
        .style(|_| iced::widget::container::Style {
            background: None,
            ..Default::default()
        })
        .width(Length::Shrink)
        .height(Length::Shrink)
        .into()
}

fn metric_row(metric: PopupMetricRow) -> Element<'static, Message> {
    Row::new()
        .spacing(12)
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .push(
            container(text(metric.icon).size(14))
                .width(Length::Fixed(16.0))
                .align_x(iced::alignment::Horizontal::Center),
        )
        .push(text(metric.label).size(13))
        .push(Space::with_width(Length::Fill))
        .push(
            container(
                text(metric.value)
                    .size(13)
                    .style(|_| iced::widget::text::Style {
                        color: Some(Color::from_rgb8(0xc0, 0xca, 0xf5)),
                    }),
            )
            .width(Length::Fixed(108.0))
            .align_x(iced::alignment::Horizontal::Right),
        )
        .into()
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
    use super::{normalize_value, opaque_background_alpha, StatsPopupModel};

    #[test]
    fn opaque_background_alpha_always_forces_opaque_surface() {
        assert_eq!(opaque_background_alpha(0.25), 1.0);
        assert_eq!(opaque_background_alpha(0.85), 1.0);
    }

    #[test]
    fn normalize_value_collapses_blank_strings() {
        assert_eq!(normalize_value(""), "--");
        assert_eq!(normalize_value("   "), "--");
        assert_eq!(normalize_value("11%"), "11%");
        assert_eq!(normalize_value(" 34°C "), "34°C");
    }

    #[test]
    fn stats_model_normalizes_primary_rows() {
        let model = StatsPopupModel::new(1.0, "", "26%", "40°C", "2700 RPM (2)");

        assert_eq!(
            model
                .rows
                .iter()
                .map(|row| row.value.as_str())
                .collect::<Vec<_>>(),
            vec!["--", "26%", "40°C", "2700 RPM (2)"]
        );
    }
}
