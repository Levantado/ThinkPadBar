use iced::{
    widget::{container, text, Column, Row, Space},
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
    let layout = super::standard_domain_popup_layout();
    let content = Column::new()
        .width(Length::Shrink)
        .spacing(layout.section_spacing)
        .push(chrome::detail_popup_header_row(
            theme,
            "Stats",
            &Popup::Stats,
        ))
        .push(chrome::domain_popup_nav_row(theme, &Popup::Stats))
        .push_rows(model.rows.into_iter().map(metric_row));

    let card = container(content)
        .padding(Padding::from([
            layout.outer_padding_y,
            layout.outer_padding_x,
        ]))
        .max_width(f32::from(layout.width))
        .style(move |_| {
            let mut style = chrome::popup_panel_style(theme);
            style.background = Some(iced::Background::Color(Color {
                a: model.background_alpha,
                ..theme.panel
            }));
            style
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
    let type_scale = super::standard_popup_type_scale();
    Row::new()
        .spacing(12)
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .push(
            container(text(metric.icon).size(type_scale.section))
                .width(Length::Fixed(16.0))
                .align_x(iced::alignment::Horizontal::Center),
        )
        .push(text(metric.label).size(type_scale.body))
        .push(Space::with_width(Length::Fill))
        .push(
            container(text(metric.value).size(type_scale.body).style(|_| {
                iced::widget::text::Style {
                    color: Some(Color::from_rgb8(0xc0, 0xca, 0xf5)),
                }
            }))
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
