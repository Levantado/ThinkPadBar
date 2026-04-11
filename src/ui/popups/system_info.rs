use iced::{
    widget::{button, container, scrollable, text, Column, Row, Space},
    Alignment, Color, Element, Length, Padding,
};

use crate::{
    app::{Message, Popup},
    ui::{chrome, theme::ThemeTokens},
};

use super::PopupMetricRow;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SystemInfoTab {
    #[default]
    Overview,
    Power,
    Hardware,
    Runtime,
}

impl SystemInfoTab {
    pub fn all() -> &'static [SystemInfoTab] {
        &[
            SystemInfoTab::Overview,
            SystemInfoTab::Power,
            SystemInfoTab::Hardware,
            SystemInfoTab::Runtime,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            SystemInfoTab::Overview => "Metrics",
            SystemInfoTab::Power => "Power",
            SystemInfoTab::Hardware => "Hardware",
            SystemInfoTab::Runtime => "Runtime",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            SystemInfoTab::Overview => "",
            SystemInfoTab::Power => "",
            SystemInfoTab::Hardware => "󰈐",
            SystemInfoTab::Runtime => "󰈈",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemInfoPopupModel {
    pub version: &'static str,
    pub active_tab: SystemInfoTab,
    pub overview_rows: Vec<PopupMetricRow>,
    pub power_rows: Vec<PopupMetricRow>,
    pub hardware_rows: Vec<PopupMetricRow>,
    pub runtime_rows: Vec<PopupMetricRow>,
}

impl SystemInfoPopupModel {
    pub fn new(
        version: &'static str,
        active_tab: SystemInfoTab,
        overview_rows: Vec<PopupMetricRow>,
        power_rows: Vec<PopupMetricRow>,
        hardware_rows: Vec<PopupMetricRow>,
        runtime_rows: Vec<PopupMetricRow>,
    ) -> Self {
        Self {
            version,
            active_tab,
            overview_rows,
            power_rows,
            hardware_rows,
            runtime_rows,
        }
    }
}

pub fn power_rows(
    battery: &crate::services::controls::BatteryInfo,
    power_profile: &str,
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
    ]
}

pub fn hardware_rows(
    fan: &crate::services::controls::FanInfo,
    sys_data: &crate::modules::system::SysData,
    idle_snapshot: &crate::services::idle_inhibitor::IdleInhibitorSnapshot,
) -> Vec<PopupMetricRow> {
    vec![
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
    
    // Tab Navigation Row
    let mut tab_row = Row::new().spacing(8).align_y(Alignment::Center);
    for tab in SystemInfoTab::all() {
        let is_active = *tab == model.active_tab;
        let (bg, fg) = if is_active {
            (theme.accent, Color::from_rgb8(0x1a, 0x1b, 0x26))
        } else {
            (theme.surface_alt, theme.text_muted)
        };
        
        let tab_btn = button(
            Row::new()
                .spacing(6)
                .align_y(Alignment::Center)
                .push(text(tab.icon()).size(14))
                .push(text(tab.label()).size(12))
        )
        .padding(Padding::from([6, 12]))
        .on_press(Message::SetSystemInfoTab(*tab))
        .style(move |_, _| iced::widget::button::Style {
            background: Some(iced::Background::Color(bg)),
            text_color: fg,
            border: iced::Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            ..Default::default()
        });
        
        tab_row = tab_row.push(tab_btn);
    }

    let active_rows = match model.active_tab {
        SystemInfoTab::Overview => model.overview_rows,
        SystemInfoTab::Power => model.power_rows,
        SystemInfoTab::Hardware => model.hardware_rows,
        SystemInfoTab::Runtime => model.runtime_rows,
    };

    let content = Column::new()
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
        .push(
            container(scrollable(Row::new().push(tab_row).push(Space::with_width(Length::Fixed(12.0)))))
                .width(Length::Fill)
        )
        .push(Space::with_height(Length::Fixed(4.0)))
        .push_rows(active_rows.into_iter().map(metric_row));

    container(content)
        .width(Length::Fill)
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

fn metric_row(metric: PopupMetricRow) -> Element<'static, Message> {
    let value = compact_metric_value(&metric.value, 32);
    Row::new()
        .spacing(10)
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .height(Length::Fixed(28.0))
        .push(
            text(metric.icon)
                .size(16)
                .width(Length::Fixed(28.0))
                .align_x(iced::alignment::Horizontal::Center),
        )
        .push(
            text(metric.label)
                .size(13)
                .width(Length::Fill),
        )
        .push(
            text(value)
                .size(13)
                .width(Length::Fixed(160.0))
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
    use super::{compact_metric_value, SystemInfoPopupModel, SystemInfoTab};
    use crate::ui::popups::PopupMetricRow;

    #[test]
    fn system_info_model_initializes_with_tab() {
        let model = SystemInfoPopupModel::new(
            "1.0.0",
            SystemInfoTab::Overview,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );

        assert_eq!(model.active_tab, SystemInfoTab::Overview);
    }

    #[test]
    fn compact_metric_value_truncates_only_when_needed() {
        assert_eq!(compact_metric_value("123", 5), "123");
        assert_eq!(compact_metric_value("123456", 5), "1234…");
    }
}
