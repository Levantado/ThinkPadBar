use iced::{
    widget::{button, container, text, Column, Row, Space},
    Alignment, Color, Element, Length, Padding,
};

use crate::{
    app::{Message, Popup, PowerAction},
    ui::{chrome, theme::ThemeTokens},
};

#[derive(Debug, Clone, PartialEq)]
pub struct PowerPopupModel {
    pub battery: crate::services::controls::BatteryInfo,
    pub power_profile: String,
    pub fan: crate::services::controls::FanInfo,
    pub performance_config: crate::config::PerformanceConfig,
    pub idle_snapshot: crate::services::idle_inhibitor::IdleInhibitorSnapshot,
    pub power_menu_open: bool,
    pub opacity: f32,
}

impl PowerPopupModel {
    pub fn new(
        controls: &crate::services::controls::ControlsSnapshot,
        performance_config: &crate::config::PerformanceConfig,
        idle_snapshot: crate::services::idle_inhibitor::IdleInhibitorSnapshot,
        power_menu_open: bool,
        opacity: f32,
    ) -> Self {
        Self {
            battery: controls.battery.clone(),
            power_profile: controls.power_profile.clone(),
            fan: controls.fan.clone(),
            performance_config: performance_config.clone(),
            idle_snapshot,
            power_menu_open,
            opacity,
        }
    }
}

pub fn view(theme: ThemeTokens, model: PowerPopupModel) -> Element<'static, Message> {
    let layout = super::standard_domain_popup_layout();
    if model.power_menu_open {
        return view_power_menu(model.opacity);
    }

    let (bat_icon, bat_color) = battery_icon_and_color(&model.battery);
    let (perf_b, perf_t, perf_s) = model.performance_config.effective_intervals();
    let time_remaining = model.battery.time_remaining.clone().unwrap_or_default();

    let top_row = Row::new()
        .align_y(Alignment::Center)
        .push(
            Row::new()
                .spacing(8)
                .align_y(Alignment::Center)
                .push(
                    text(bat_icon)
                        .size(16)
                        .style(move |_| iced::widget::text::Style {
                            color: Some(bat_color),
                        }),
                )
                .push(
                    text(format!("{}%", model.battery.capacity))
                        .size(14)
                        .style(move |_| iced::widget::text::Style {
                            color: Some(bat_color),
                        }),
                )
                .push(Space::with_width(8))
                .push(
                    text(time_remaining)
                        .size(12)
                        .style(|_| iced::widget::text::Style {
                            color: Some(Color::from_rgb8(0x56, 0x5f, 0x89)),
                        }),
                ),
        )
        .push(Space::with_width(Length::Fill))
        .push(
            button(
                text(format!(
                    "Perf {} {}/{}/{}",
                    model.performance_config.profile_badge(),
                    perf_b,
                    perf_t,
                    perf_s
                ))
                .size(12),
            )
            .padding(8)
            .on_press(Message::CyclePerformanceProfile)
            .style(move |_, status| circular_btn_style(theme, status)),
        )
        .push(
            button(text("󰌾").size(16))
                .padding(8)
                .on_press(Message::PowerAction(PowerAction::Lock))
                .style(move |_, status| circular_btn_style(theme, status)),
        )
        .push(
            button(text("").size(16))
                .padding(8)
                .on_press(Message::TogglePowerMenu)
                .style(move |_, status| circular_btn_style(theme, status)),
        );

    let mut prof_row = Row::new().width(Length::Fill).spacing(8);
    for vid in ["low-power", "balanced", "performance"].iter() {
        let is_active = model.power_profile == *vid;
        let vid_str = vid.to_string();
        let (label, profile_color) = power_profile_visual(vid);
        let caption = power_profile_caption(vid);
        let btn = button(
            container(
                Row::new()
                    .spacing(3)
                    .align_y(Alignment::Center)
                    .push(text(label).size(10))
                    .push(text(caption).size(10)),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
        )
        .width(Length::FillPortion(1))
        .height(Length::Fixed(32.0))
        .padding(Padding::from([0, 4]))
        .on_press(Message::SetPowerProfile(vid_str))
        .style(move |_, status| {
            if is_active {
                let mut style = chrome::popup_button_style(
                    theme,
                    status,
                    chrome::PopupButtonTone::SurfaceAlt,
                    true,
                );
                style.text_color = profile_color;
                style.border.color = profile_color;
                style
            } else {
                let mut style = chrome::popup_button_style(
                    theme,
                    status,
                    chrome::PopupButtonTone::Surface,
                    true,
                );
                style.text_color = profile_color;
                style
            }
        });
        prof_row = prof_row.push(btn);
    }

    let idle_btn = {
        let mut btn = button(
            Row::new()
                .spacing(4)
                .align_y(Alignment::Center)
                .push(text("").size(18))
                .push(text(model.idle_snapshot.label()).size(12)),
        )
        .width(Length::FillPortion(1))
        .padding(Padding::from([12, 12]))
        .style(move |_, status| {
            if model.idle_snapshot.enabled {
                chrome::popup_button_style(theme, status, chrome::PopupButtonTone::Accent, true)
            } else if model.idle_snapshot.available {
                chrome::popup_button_style(theme, status, chrome::PopupButtonTone::Surface, true)
            } else {
                chrome::popup_button_style(
                    theme,
                    status,
                    chrome::PopupButtonTone::SurfaceAlt,
                    false,
                )
            }
        });
        if model.idle_snapshot.available {
            btn = btn.on_press(Message::ToggleIdleInhibitor);
        }
        btn
    };

    let battery_care_card = {
        let info_row = |icon: &'static str, label: &'static str, value: String| {
            Row::new()
                .spacing(8)
                .align_y(Alignment::Center)
                .push(text(icon).size(13))
                .push(
                    text(label)
                        .size(12)
                        .width(Length::FillPortion(2))
                        .style(|_| iced::widget::text::Style {
                            color: Some(Color::from_rgb8(0x86, 0x90, 0xb2)),
                        }),
                )
                .push(
                    text(value)
                        .size(12)
                        .width(Length::FillPortion(3))
                        .align_x(iced::alignment::Horizontal::Right),
                )
        };

        container(
            Column::new()
                .spacing(8)
                .push(
                    Row::new()
                        .spacing(8)
                        .align_y(Alignment::Center)
                        .push(text("󰚥").size(16))
                        .push(text("Battery Care").size(14)),
                )
                .push(info_row(
                    "󰁹",
                    "Charge State",
                    battery_charge_state_summary(&model.battery),
                ))
                .push(info_row(
                    "󰂄",
                    "Thresholds",
                    battery_threshold_summary(&model.battery),
                ))
                .push(info_row(
                    "󰛨",
                    "Control Mode",
                    "System-managed (read-only)".to_string(),
                )),
        )
        .padding(layout.card_padding)
        .style(move |_| chrome::popup_card_style(theme))
    };

    let content = Column::new()
        .spacing(layout.section_spacing)
        .push(chrome::detail_popup_header_row(
            theme,
            "Power",
            &Popup::Power,
        ))
        .push(chrome::domain_popup_nav_row(theme, &Popup::Power))
        .push(top_row)
        .push(
            Row::new()
                .spacing(16)
                .width(Length::Fill)
                .push(idle_btn)
                .push(shortcut_button(
                    theme,
                    "󰈈",
                    "System Info".to_string(),
                    Message::TogglePopup(Popup::SystemMonitor),
                ))
                .push(shortcut_button(
                    theme,
                    "󰍹",
                    "Displays".to_string(),
                    Message::TogglePopup(Popup::Displays),
                )),
        )
        .push(
            Column::new()
                .spacing(8)
                .push(
                    Row::new()
                        .spacing(8)
                        .align_y(Alignment::Center)
                        .push(text("󰒓").size(16))
                        .push(text("Power Profiles (PPD)").size(14)),
                )
                .push(
                    container(prof_row)
                        .width(Length::Fill)
                        .align_x(iced::alignment::Horizontal::Center),
                ),
        )
        .push(
            Column::new()
                .spacing(8)
                .push(
                    Row::new()
                        .spacing(8)
                        .align_y(Alignment::Center)
                        .push(text("󰈐").size(16))
                        .push(text(format!("Fan Control: {} RPM", model.fan.speed)).size(14)),
                )
                .push(
                    container(fan_row(&model.fan))
                        .width(Length::Fill)
                        .align_x(iced::alignment::Horizontal::Center),
                ),
        )
        .push(battery_care_card);

    container(content)
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

fn view_power_menu(opacity: f32) -> Element<'static, Message> {
    let theme = ThemeTokens::from_config(&crate::config::Config::default());
    let power_action_btn = |label: &str, icon: &str, action: PowerAction| {
        button(
            Row::new()
                .spacing(8)
                .align_y(Alignment::Center)
                .push(text(icon.to_string()).size(18))
                .push(text(label.to_string()).size(14)),
        )
        .width(Length::Fill)
        .height(Length::Fixed(40.0))
        .padding(12)
        .on_press(Message::PowerAction(action))
        .style(move |_, status| {
            chrome::popup_button_style(theme, status, chrome::PopupButtonTone::Ghost, true)
        })
    };

    let separator = container(Space::with_height(1))
        .width(Length::Fill)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(Color::from_rgb8(0x29, 0x2e, 0x42))),
            ..Default::default()
        });

    let power_col = Column::new()
        .spacing(4)
        .push(
            Row::new()
                .spacing(12)
                .align_y(Alignment::Center)
                .push(text("Power Menu").size(18).width(Length::Fill))
                .push(
                    button(text("󰁝 Back").size(14))
                        .on_press(Message::TogglePowerMenu)
                        .padding(8),
                ),
        )
        .push(Space::with_height(12))
        .push(power_action_btn("Suspend", "󰒲", PowerAction::Sleep))
        .push(power_action_btn("Hibernate", "󰖕", PowerAction::Hibernate))
        .push(power_action_btn("Reboot", "󰑓", PowerAction::Restart))
        .push(power_action_btn("Shutdown", "", PowerAction::Shutdown))
        .push(Space::with_height(8))
        .push(separator)
        .push(Space::with_height(8))
        .push(power_action_btn("Logout", "󰍃", PowerAction::Logout));

    container(power_col)
        .padding(24)
        .width(Length::Fixed(440.0))
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(Color {
                a: opacity,
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

fn fan_row(fan: &crate::services::controls::FanInfo) -> Row<'static, Message> {
    let mut row = Row::new().width(Length::Shrink).spacing(4);
    for l in ["1", "2", "3", "4", "5", "6", "7", "auto", "max"].iter() {
        let lvl = if *l == "max" {
            "full-speed".to_string()
        } else {
            l.to_string()
        };
        let current_level = fan.level.trim();
        let is_active =
            current_level == lvl || (lvl == "full-speed" && current_level == "disengaged");

        let btn_width = if *l == "auto" || *l == "max" {
            Length::Fixed(42.0)
        } else {
            Length::Fixed(26.0)
        };

        let btn = button(
            container(text(*l).size(11))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center),
        )
        .on_press(Message::SetFanLevel(lvl.clone()))
        .width(btn_width)
        .height(Length::Fixed(26.0))
        .padding(Padding::from([0, 0]))
        .style(move |_, _| {
            if is_active {
                iced::widget::button::Style {
                    background: Some(iced::Background::Color(Color::from_rgb8(0x7a, 0xa2, 0xf7))),
                    text_color: Color::from_rgb8(0x1a, 0x1b, 0x26),
                    border: iced::Border {
                        radius: 6.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            } else {
                iced::widget::button::Style {
                    background: Some(iced::Background::Color(Color::from_rgb8(0x41, 0x48, 0x68))),
                    text_color: Color::from_rgb8(0xc0, 0xca, 0xf5),
                    border: iced::Border {
                        radius: 6.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            }
        });
        row = row.push(btn);
    }
    row
}

fn circular_btn_style(
    theme: ThemeTokens,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let mut style =
        chrome::popup_button_style(theme, status, chrome::PopupButtonTone::Surface, true);
    style.border.radius = 24.0.into();
    style
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

pub fn battery_icon_and_color(
    battery: &crate::services::controls::BatteryInfo,
) -> (&'static str, Color) {
    let bat_cap = battery.capacity;
    let bat_status = &battery.status;
    if bat_status.contains("Charging") {
        ("󰂄", Color::from_rgb8(0x9e, 0xce, 0x6a))
    } else if bat_status.contains("Full") || bat_status.contains("Not charging") {
        ("", Color::from_rgb8(0xc0, 0xca, 0xf5))
    } else {
        let icon = if bat_cap >= 90 {
            "󰁹"
        } else if bat_cap >= 80 {
            "󰂂"
        } else if bat_cap >= 70 {
            "󰂁"
        } else if bat_cap >= 60 {
            "󰂀"
        } else if bat_cap >= 50 {
            "󰁿"
        } else if bat_cap >= 40 {
            "󰁾"
        } else if bat_cap >= 30 {
            "󰁽"
        } else if bat_cap >= 20 {
            "󰁼"
        } else if bat_cap >= 10 {
            "󰁻"
        } else {
            "󰁺"
        };
        let color = if bat_cap <= 10 {
            Color::from_rgb8(0xf7, 0x76, 0x8e)
        } else if bat_cap <= 20 {
            Color::from_rgb8(0xe0, 0xaf, 0x68)
        } else {
            Color::from_rgb8(0xc0, 0xca, 0xf5)
        };
        (icon, color)
    }
}

pub fn power_profile_visual(profile: &str) -> (&'static str, Color) {
    match profile {
        "low-power" => ("󰾆", Color::from_rgb8(0x9e, 0xce, 0x6a)),
        "balanced" => ("󰾅", Color::from_rgb8(0xe0, 0xaf, 0x68)),
        "performance" => ("󰓅", Color::from_rgb8(0xf7, 0x76, 0x8e)),
        _ => ("󰓅 ?", Color::WHITE),
    }
}

pub fn power_profile_caption(profile: &str) -> &'static str {
    match profile {
        "low-power" => "LOW",
        "balanced" => "BAL",
        "performance" => "HIGH",
        _ => "?",
    }
}

pub fn battery_runtime_summary(battery: &crate::services::controls::BatteryInfo) -> String {
    let mut summary = format!("{}%", battery.capacity);
    if let Some(time) = &battery.time_remaining {
        summary.push_str(" (");
        summary.push_str(time);
        summary.push(')');
    }
    summary
}

pub fn battery_ac_summary(battery: &crate::services::controls::BatteryInfo) -> String {
    match battery.ac_online {
        Some(true) => "Connected".to_string(),
        Some(false) => "Disconnected".to_string(),
        None => "Unknown".to_string(),
    }
}

pub fn battery_health_summary(battery: &crate::services::controls::BatteryInfo) -> String {
    battery
        .health_percent
        .map(|percent| format!("{percent}% of design"))
        .unwrap_or_else(|| "N/A".to_string())
}

pub fn battery_wear_summary(battery: &crate::services::controls::BatteryInfo) -> String {
    let Some(health_percent) = battery.health_percent else {
        return "N/A".to_string();
    };
    let wear_percent = 100u8.saturating_sub(health_percent);
    match (battery.full_charge_mwh, battery.design_capacity_mwh) {
        (Some(full), Some(design)) if design >= full => format!(
            "{wear_percent}% worn (-{:.1} Wh)",
            (design - full) as f64 / 1000.0
        ),
        _ => format!("{wear_percent}% worn"),
    }
}

pub fn battery_power_summary(battery: &crate::services::controls::BatteryInfo) -> String {
    let Some(power_rate_mw) = battery.power_rate_mw else {
        return "N/A".to_string();
    };

    let label = match battery.status.as_str() {
        "Charging" => "charging",
        "Discharging" => "draw",
        _ => "rate",
    };
    format!("{:.1} W {label}", power_rate_mw as f64 / 1000.0)
}

pub fn battery_voltage_summary(battery: &crate::services::controls::BatteryInfo) -> String {
    battery
        .pack_voltage_mv
        .map(|mv| format!("{:.1} V", mv as f64 / 1000.0))
        .unwrap_or_else(|| "N/A".to_string())
}

pub fn battery_pack_summary(battery: &crate::services::controls::BatteryInfo) -> String {
    match (battery.full_charge_mwh, battery.design_capacity_mwh) {
        (Some(full), Some(design)) if design >= full => {
            format!(
                "{:.1} / {:.1} Wh (-{:.1} Wh)",
                full as f64 / 1000.0,
                design as f64 / 1000.0,
                (design - full) as f64 / 1000.0
            )
        }
        (Some(full), Some(design)) => {
            format!(
                "{:.1} / {:.1} Wh",
                full as f64 / 1000.0,
                design as f64 / 1000.0
            )
        }
        (Some(full), None) => format!("{:.1} Wh current full", full as f64 / 1000.0),
        _ => "N/A".to_string(),
    }
}

pub fn battery_cycle_summary(battery: &crate::services::controls::BatteryInfo) -> String {
    battery
        .cycle_count
        .map(|count| format!("{count} cycles"))
        .unwrap_or_else(|| "N/A".to_string())
}

pub fn battery_threshold_summary(battery: &crate::services::controls::BatteryInfo) -> String {
    match (battery.charge_start_threshold, battery.charge_end_threshold) {
        (Some(0), Some(100)) => "Full charge allowed".to_string(),
        (Some(start), Some(end)) if start == end => format!("Pinned at {end}%"),
        (Some(start), Some(end)) => format!("{start}% -> {end}%"),
        (Some(start), None) => format!("Start at {start}%"),
        (None, Some(end)) => format!("Stop at {end}%"),
        (None, None) => "N/A".to_string(),
    }
}

pub fn battery_charge_state_summary(battery: &crate::services::controls::BatteryInfo) -> String {
    match battery.status.as_str() {
        "Charging" => battery
            .charge_end_threshold
            .map(|end| format!("Charging toward {end}% ceiling"))
            .unwrap_or_else(|| "Charging".to_string()),
        "Discharging" => "Discharging".to_string(),
        "Full" => "Full".to_string(),
        _ => match (
            battery.ac_online,
            battery.charge_start_threshold,
            battery.charge_end_threshold,
        ) {
            (Some(true), Some(_start), Some(end)) if battery.capacity >= end => {
                format!("Holding at {end}% ceiling")
            }
            (Some(true), Some(start), Some(end))
                if battery.capacity >= start && battery.capacity < end =>
            {
                format!("Within {start}-{end}% hold window")
            }
            (Some(true), Some(start), Some(_end)) if battery.capacity < start => {
                format!("Waiting to resume below {start}%")
            }
            (Some(true), _, _) => "AC idle".to_string(),
            (Some(false), _, _) => battery.status.clone(),
            (None, _, _) => battery.status.clone(),
        },
    }
}

pub fn fan_runtime_summary(fan: &crate::services::controls::FanInfo) -> String {
    format!("{} RPM ({})", fan.speed, fan.level)
}

pub fn thermal_state_summary(sys_data: &crate::modules::system::SysData) -> String {
    let temp = sys_data.temp;
    if temp <= 0.0 {
        return "Sensor unavailable".to_string();
    }

    let state = if temp >= 85.0 {
        "Critical"
    } else if temp >= 70.0 {
        "Hot"
    } else if temp >= 55.0 {
        "Warm"
    } else {
        "Cool"
    };
    format!("{state} ({})", sys_data.temp_str)
}

pub fn cpu_usage_summary(sys_data: &crate::modules::system::SysData) -> String {
    let normalized = sys_data.cpu_str.trim();
    if normalized.is_empty() || !normalized.chars().any(|ch| ch.is_ascii_digit()) {
        format!("{}%", sys_data.cpu_usage.round() as i32)
    } else {
        normalized.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::Color;

    #[test]
    fn battery_runtime_summary_includes_time_when_present() {
        let battery = crate::services::controls::BatteryInfo {
            capacity: 64,
            status: "Discharging".to_string(),
            time_remaining: Some("2h 6m remaining".to_string()),
            ac_online: Some(false),
            health_percent: Some(92),
            power_rate_mw: Some(12_400),
            pack_voltage_mv: Some(15_420),
            cycle_count: Some(187),
            full_charge_mwh: Some(48_000),
            design_capacity_mwh: Some(52_000),
            charge_start_threshold: Some(40),
            charge_end_threshold: Some(80),
        };

        assert_eq!(battery_runtime_summary(&battery), "64% (2h 6m remaining)");
        assert_eq!(
            battery_runtime_summary(&crate::services::controls::BatteryInfo {
                capacity: 100,
                status: "Full".to_string(),
                time_remaining: None,
                ac_online: Some(true),
                health_percent: Some(100),
                power_rate_mw: None,
                pack_voltage_mv: None,
                cycle_count: None,
                full_charge_mwh: None,
                design_capacity_mwh: None,
                charge_start_threshold: None,
                charge_end_threshold: None,
            }),
            "100%"
        );
    }

    #[test]
    fn battery_detail_summaries_format_actionable_hardware_state() {
        let battery = crate::services::controls::BatteryInfo {
            capacity: 64,
            status: "Discharging".to_string(),
            time_remaining: Some("2h 6m remaining".to_string()),
            ac_online: Some(false),
            health_percent: Some(92),
            power_rate_mw: Some(12_400),
            pack_voltage_mv: Some(15_420),
            cycle_count: Some(187),
            full_charge_mwh: Some(48_000),
            design_capacity_mwh: Some(52_000),
            charge_start_threshold: Some(40),
            charge_end_threshold: Some(80),
        };

        assert_eq!(battery_ac_summary(&battery), "Disconnected");
        assert_eq!(battery_health_summary(&battery), "92% of design");
        assert_eq!(battery_wear_summary(&battery), "8% worn (-4.0 Wh)");
        assert_eq!(battery_power_summary(&battery), "12.4 W draw");
        assert_eq!(battery_voltage_summary(&battery), "15.4 V");
        assert_eq!(battery_pack_summary(&battery), "48.0 / 52.0 Wh (-4.0 Wh)");
        assert_eq!(battery_cycle_summary(&battery), "187 cycles");
        assert_eq!(battery_threshold_summary(&battery), "40% -> 80%");
        assert_eq!(battery_charge_state_summary(&battery), "Discharging");
    }

    #[test]
    fn battery_charge_state_summary_interprets_threshold_policy() {
        let holding = crate::services::controls::BatteryInfo {
            capacity: 80,
            status: "Not charging".to_string(),
            time_remaining: None,
            ac_online: Some(true),
            health_percent: None,
            power_rate_mw: None,
            pack_voltage_mv: None,
            cycle_count: None,
            full_charge_mwh: None,
            design_capacity_mwh: None,
            charge_start_threshold: Some(40),
            charge_end_threshold: Some(80),
        };
        assert_eq!(
            battery_charge_state_summary(&holding),
            "Holding at 80% ceiling"
        );

        let window = crate::services::controls::BatteryInfo {
            capacity: 63,
            status: "Not charging".to_string(),
            time_remaining: None,
            ac_online: Some(true),
            health_percent: None,
            power_rate_mw: None,
            pack_voltage_mv: None,
            cycle_count: None,
            full_charge_mwh: None,
            design_capacity_mwh: None,
            charge_start_threshold: Some(40),
            charge_end_threshold: Some(80),
        };
        assert_eq!(
            battery_charge_state_summary(&window),
            "Within 40-80% hold window"
        );

        let resume = crate::services::controls::BatteryInfo {
            capacity: 35,
            status: "Not charging".to_string(),
            time_remaining: None,
            ac_online: Some(true),
            health_percent: None,
            power_rate_mw: None,
            pack_voltage_mv: None,
            cycle_count: None,
            full_charge_mwh: None,
            design_capacity_mwh: None,
            charge_start_threshold: Some(40),
            charge_end_threshold: Some(80),
        };
        assert_eq!(
            battery_charge_state_summary(&resume),
            "Waiting to resume below 40%"
        );
    }

    #[test]
    fn battery_threshold_summary_polishes_common_policy_shapes() {
        let full_charge = crate::services::controls::BatteryInfo {
            capacity: 100,
            status: "Full".to_string(),
            time_remaining: None,
            ac_online: Some(true),
            health_percent: None,
            power_rate_mw: None,
            pack_voltage_mv: None,
            cycle_count: None,
            full_charge_mwh: None,
            design_capacity_mwh: None,
            charge_start_threshold: Some(0),
            charge_end_threshold: Some(100),
        };
        assert_eq!(
            battery_threshold_summary(&full_charge),
            "Full charge allowed"
        );

        let pinned = crate::services::controls::BatteryInfo {
            capacity: 80,
            status: "Not charging".to_string(),
            time_remaining: None,
            ac_online: Some(true),
            health_percent: None,
            power_rate_mw: None,
            pack_voltage_mv: None,
            cycle_count: None,
            full_charge_mwh: None,
            design_capacity_mwh: None,
            charge_start_threshold: Some(80),
            charge_end_threshold: Some(80),
        };
        assert_eq!(battery_threshold_summary(&pinned), "Pinned at 80%");
    }

    #[test]
    fn fan_runtime_summary_formats_speed_and_level() {
        assert_eq!(
            fan_runtime_summary(&crate::services::controls::FanInfo {
                speed: "2700".to_string(),
                level: "auto".to_string(),
            }),
            "2700 RPM (auto)"
        );
    }

    #[test]
    fn power_popup_model_retains_fan_runtime_in_power_domain() {
        let model = super::PowerPopupModel {
            battery: crate::services::controls::BatteryInfo {
                capacity: 63,
                status: "Discharging".to_string(),
                time_remaining: Some("2h 06m remaining".to_string()),
                ac_online: Some(false),
                health_percent: Some(93),
                power_rate_mw: Some(11_200),
                pack_voltage_mv: Some(15_380),
                cycle_count: Some(182),
                full_charge_mwh: Some(54_200),
                design_capacity_mwh: Some(58_000),
                charge_start_threshold: Some(80),
                charge_end_threshold: Some(100),
            },
            power_profile: "balanced".to_string(),
            fan: crate::services::controls::FanInfo {
                speed: "3100".to_string(),
                level: "3".to_string(),
            },
            performance_config: crate::config::PerformanceConfig::default(),
            idle_snapshot: crate::services::idle_inhibitor::IdleInhibitorSnapshot::default(),
            power_menu_open: false,
            opacity: 0.9,
        };

        assert_eq!(model.fan.speed, "3100");
        assert_eq!(model.fan.level, "3");
        assert_eq!(model.battery.capacity, 63);
        assert_eq!(model.power_profile, "balanced");
    }

    #[test]
    fn power_popup_model_builder_maps_only_power_domain_inputs() {
        let mut controls = crate::services::controls::ControlsSnapshot::default();
        controls.battery.capacity = 71;
        controls.power_profile = "performance".to_string();
        controls.fan = crate::services::controls::FanInfo {
            speed: "4200".to_string(),
            level: "7".to_string(),
        };

        let model = PowerPopupModel::new(
            &controls,
            &crate::config::PerformanceConfig::default(),
            crate::services::idle_inhibitor::IdleInhibitorSnapshot::default(),
            true,
            0.78,
        );

        assert_eq!(model.battery.capacity, 71);
        assert_eq!(model.power_profile, "performance");
        assert_eq!(model.fan.speed, "4200");
        assert!(model.power_menu_open);
        assert_eq!(model.opacity, 0.78);
    }

    #[test]
    fn thermal_state_summary_interprets_temperature_band() {
        assert_eq!(
            thermal_state_summary(&crate::modules::system::SysData {
                temp: 48.0,
                temp_str: "48°C".to_string(),
                ..crate::modules::system::SysData::default()
            }),
            "Cool (48°C)"
        );
        assert_eq!(
            thermal_state_summary(&crate::modules::system::SysData {
                temp: 73.0,
                temp_str: "73°C".to_string(),
                ..crate::modules::system::SysData::default()
            }),
            "Hot (73°C)"
        );
    }

    #[test]
    fn cpu_usage_summary_falls_back_when_formatted_value_is_blank() {
        assert_eq!(
            cpu_usage_summary(&crate::modules::system::SysData {
                cpu_usage: 12.6,
                cpu_str: " ".to_string(),
                ..crate::modules::system::SysData::default()
            }),
            "13%"
        );
        assert_eq!(
            cpu_usage_summary(&crate::modules::system::SysData {
                cpu_usage: 99.0,
                cpu_str: "7%".to_string(),
                ..crate::modules::system::SysData::default()
            }),
            "7%"
        );
    }

    #[test]
    fn power_profile_visual_uses_speedometer_labels() {
        assert_eq!(power_profile_visual("low-power").0, "󰾆");
        assert_eq!(power_profile_visual("balanced").0, "󰾅");
        assert_eq!(power_profile_visual("performance").0, "󰓅");
        assert_eq!(power_profile_visual("unknown").0, "󰓅 ?");
        assert_eq!(power_profile_visual("unknown").1, Color::WHITE);
    }
}
