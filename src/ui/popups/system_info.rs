use iced::{
    widget::{container, scrollable, text, Column, Row, Space},
    Alignment, Color, Element, Length, Padding,
};

use crate::{
    app::{Message, Popup},
    ui::{chrome, theme::ThemeTokens},
};

use super::{PopupMetricRow, PopupSection, PopupSectionTone};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemInfoPopupModel {
    pub version: &'static str,
    pub overview_rows: Vec<PopupMetricRow>,
    pub hardware_rows: Vec<PopupMetricRow>,
    pub observability_rows: Vec<PopupMetricRow>,
    pub warning_rows: Vec<PopupMetricRow>,
}

impl SystemInfoPopupModel {
    pub fn new(
        version: &'static str,
        overview_rows: Vec<PopupMetricRow>,
        hardware_rows: Vec<PopupMetricRow>,
        observability_rows: Vec<PopupMetricRow>,
        warning_rows: Vec<PopupMetricRow>,
    ) -> Self {
        Self {
            version,
            overview_rows,
            hardware_rows,
            observability_rows,
            warning_rows,
        }
    }

    pub fn sections(&self) -> Vec<PopupSection> {
        let mut sections = Vec::new();
        if !self.hardware_rows.is_empty() {
            sections.push(PopupSection::new(
                "ThinkPad Hardware",
                PopupSectionTone::Success,
                self.hardware_rows.clone(),
            ));
        }

        if !self.observability_rows.is_empty() || !self.warning_rows.is_empty() {
            let mut rows = self.observability_rows.clone();
            rows.extend(self.warning_rows.clone());
            sections.push(PopupSection::new(
                "Observability",
                PopupSectionTone::Accent,
                rows,
            ));
        }

        sections
    }
}

pub fn hardware_rows(
    battery: &crate::services::controls::BatteryInfo,
    power_profile: &str,
    fan: &crate::services::controls::FanInfo,
    sys_data: &crate::modules::system::SysData,
    idle_snapshot: &crate::services::idle_inhibitor::IdleInhibitorSnapshot,
) -> Vec<PopupMetricRow> {
    vec![
        PopupMetricRow::new(
            "󰁹",
            "Battery Runtime",
            crate::ui::popups::power::battery_runtime_summary(battery),
        ),
        PopupMetricRow::new(
            "󰚥",
            "AC Adapter",
            crate::ui::popups::power::battery_ac_summary(battery),
        ),
        PopupMetricRow::new(
            "",
            "Battery Health",
            crate::ui::popups::power::battery_health_summary(battery),
        ),
        PopupMetricRow::new(
            "󰾹",
            "Battery Wear",
            crate::ui::popups::power::battery_wear_summary(battery),
        ),
        PopupMetricRow::new(
            "󱤅",
            "Pack Capacity",
            crate::ui::popups::power::battery_pack_summary(battery),
        ),
        PopupMetricRow::new(
            "󱈸",
            "Pack Voltage",
            crate::ui::popups::power::battery_voltage_summary(battery),
        ),
        PopupMetricRow::new(
            "󰂄",
            "Cycle Count",
            crate::ui::popups::power::battery_cycle_summary(battery),
        ),
        PopupMetricRow::new(
            "󱞊",
            "Charge Thresholds",
            crate::ui::popups::power::battery_threshold_summary(battery),
        ),
        PopupMetricRow::new(
            "󱐌",
            "Charge State",
            crate::ui::popups::power::battery_charge_state_summary(battery),
        ),
        PopupMetricRow::new(
            "󱐋",
            "Charge / Draw Power",
            crate::ui::popups::power::battery_power_summary(battery),
        ),
        PopupMetricRow::new("󰾆", "Power Profile", power_profile.to_string()),
        PopupMetricRow::new(
            "󰈐",
            "Fan Runtime",
            crate::ui::popups::power::fan_runtime_summary(fan),
        ),
        PopupMetricRow::new(
            "",
            "Thermal State",
            crate::ui::popups::power::thermal_state_summary(sys_data),
        ),
        PopupMetricRow::new("", "Idle Inhibitor", idle_snapshot.label().to_string()),
    ]
}

pub fn view(
    theme: ThemeTokens,
    opacity: f32,
    model: SystemInfoPopupModel,
) -> Element<'static, Message> {
    let sections = model.sections();
    let mut content = Column::new()
        .spacing(12)
        .push(
            Row::new()
                .align_y(Alignment::Center)
                .push(
                    text("System Info")
                        .size(18)
                        .style(move |_| iced::widget::text::Style {
                            color: Some(Color::from_rgb8(0xc0, 0xca, 0xf5)),
                        }),
                )
                .push(Space::with_width(Length::Fill))
                .push(
                    text(format!("ver {}", model.version))
                        .size(10)
                        .style(move |_| iced::widget::text::Style {
                            color: Some(Color::from_rgb8(0x56, 0x5f, 0x89)),
                        }),
                ),
        )
        .push(chrome::domain_popup_nav_row(
            theme,
            &chrome::domain_nav_focus_popup(&Popup::SystemMonitor),
        ))
        .push_rows(model.overview_rows.into_iter().map(metric_row));

    for section in sections {
        content = content
            .push(Space::with_height(Length::Fixed(8.0)))
            .push(section_heading(theme, &section))
            .push_rows(section.rows.into_iter().map(metric_row));
    }

    container(
        container(scrollable(container(content).padding([0, 18, 0, 0])))
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

fn section_heading(theme: ThemeTokens, section: &PopupSection) -> Element<'static, Message> {
    let color = match section.tone {
        PopupSectionTone::Accent => theme.accent,
        PopupSectionTone::Success => theme.success,
    };

    text(section.title)
        .size(14)
        .style(move |_| iced::widget::text::Style { color: Some(color) })
        .into()
}

fn metric_row(metric: PopupMetricRow) -> Element<'static, Message> {
    Row::new()
        .spacing(12)
        .align_y(Alignment::Center)
        .push(text(metric.icon).size(16))
        .push(text(metric.label).size(13).width(Length::Fill))
        .push(text(metric.value).size(13))
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
    use super::SystemInfoPopupModel;
    use crate::ui::popups::{PopupMetricRow, PopupSectionTone};

    #[test]
    fn system_info_sections_skip_empty_optional_groups() {
        let model = SystemInfoPopupModel::new(
            "1.0.0",
            vec![PopupMetricRow::new("CPU", "CPU Usage", "12%")],
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );

        assert!(model.sections().is_empty());
    }

    #[test]
    fn system_info_sections_append_warnings_to_observability() {
        let model = SystemInfoPopupModel::new(
            "1.0.0",
            vec![PopupMetricRow::new("CPU", "CPU Usage", "12%")],
            vec![PopupMetricRow::new("BAT", "Battery Runtime", "2h")],
            vec![PopupMetricRow::new("NET", "Network Runtime", "iwd")],
            vec![PopupMetricRow::new(
                "WRN",
                "Wayland Missing Caps",
                "layer-shell",
            )],
        );

        let sections = model.sections();
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].title, "ThinkPad Hardware");
        assert_eq!(sections[0].tone, PopupSectionTone::Success);
        assert_eq!(sections[1].title, "Observability");
        assert_eq!(sections[1].tone, PopupSectionTone::Accent);
        assert_eq!(sections[1].rows.len(), 2);
        assert_eq!(sections[1].rows[1].label, "Wayland Missing Caps");
    }
}
