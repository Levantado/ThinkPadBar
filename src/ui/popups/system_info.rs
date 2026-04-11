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
    let type_scale = super::standard_popup_type_scale();
    let layout = super::standard_domain_popup_layout();
    let sections = model.sections();
    let mut content = Column::new()
        .spacing(layout.section_spacing)
        .push(chrome::detail_popup_header_row(
            theme,
            "System Info",
            &Popup::SystemMonitor,
        ))
        .push(
            text(format!("ver {}", model.version))
                .size(type_scale.micro)
                .style(move |_| iced::widget::text::Style {
                    color: Some(theme.text_muted),
                }),
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

    let scroll = scrollable(container(content).padding([0, 28, 12, 0]))
        .id(iced::id::Id::new("system-info-scroll"))
        .width(Length::Fill)
        .height(Length::Fixed(500.0));

    container(scroll)
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
        })
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
    let value = compact_metric_value(&metric.value, 24);
    Row::new()
        .spacing(10)
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .height(Length::Fixed(24.0))
        .push(
            container(text(metric.icon).size(16))
                .width(Length::Fixed(28.0))
                .align_x(iced::alignment::Horizontal::Center),
        )
        .push(
            text(metric.label)
                .size(13)
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Left),
        )
        .push(Space::with_width(Length::Fixed(8.0)))
        .push(
            container(
                text(value)
                    .size(13)
                    .align_x(iced::alignment::Horizontal::Right),
            )
            .width(Length::Fixed(148.0))
            .align_x(iced::alignment::Horizontal::Right),
        )
        .into()
}

fn compact_metric_value(value: &str, max_chars: usize) -> String {
    let value = value.trim();
    if value.chars().count() <= max_chars {
        return value.to_string();
    }

    let keep = max_chars.saturating_sub(1);
    let mut compact = value.chars().take(keep).collect::<String>();
    compact.push('…');
    compact
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
    use super::{compact_metric_value, SystemInfoPopupModel};
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

    #[test]
    fn compact_metric_value_truncates_only_when_needed() {
        assert_eq!(compact_metric_value("192.168.100.13", 24), "192.168.100.13");
        assert_eq!(
            compact_metric_value("1 outputs: eDP-1, DP-3", 14),
            "1 outputs: eD…"
        );
    }
}
