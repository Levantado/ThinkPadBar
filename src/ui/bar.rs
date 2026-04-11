use iced::{
    mouse,
    widget::{button, container, image, mouse_area, stack, text, Row, Space},
    Alignment, Background, Color, Element, Font, Length, Padding,
};

use crate::app::{Message, Popup};

#[derive(Debug, Clone)]
pub struct WorkspaceChip {
    pub id: i32,
    pub name: String,
    pub active: bool,
    pub special: bool,
}

#[derive(Debug, Clone)]
pub struct TrayItemModel {
    pub id: String,
    pub icon_handle: Option<iced::widget::image::Handle>,
    pub fallback_label: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StatsPillModel {
    pub cpu_summary: String,
    pub temp_summary: String,
    pub temp_color: Color,
    pub fan_speed: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlsPillModel {
    pub brightness_label: String,
    pub volume_icon: &'static str,
    pub volume_label: String,
    pub mic_icon: &'static str,
    pub mic_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectivityPillModel {
    pub wifi_icon: &'static str,
    pub wifi_label: String,
    pub bluetooth_icon: &'static str,
    pub bluetooth_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BatteryPillModel {
    pub battery_icon: &'static str,
    pub battery_color: Color,
    pub battery_label: String,
    pub power_profile_label: String,
    pub power_profile_color: Color,
    pub idle_enabled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioVisualizerModel {
    pub enabled: bool,
    pub bars: Vec<u8>,
    pub active: bool,
    pub min_height: f32,
    pub max_height: f32,
    pub bar_width: f32,
    pub gap: u16,
    pub padding_x: u16,
    pub padding_y: u16,
    pub color_profile: crate::services::audio_visualizer::VisualizerColorProfile,
}

impl AudioVisualizerModel {
    pub fn is_visible(&self) -> bool {
        self.enabled && !self.bars.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaPillModel {
    pub title: String,
    pub artist: String,
    pub playback_status: String,
    pub has_player: bool,
}

#[derive(Debug, Clone)]
pub struct MainBarModel {
    pub opacity: f32,
    pub bar_height: f32,
    pub workspaces: Vec<WorkspaceChip>,
    pub tray_items: Vec<TrayItemModel>,
    pub center_title: String,
    pub special_workspace_visible: bool,
    pub visualizer: AudioVisualizerModel,
    pub media: MediaPillModel,
    pub stats: StatsPillModel,
    pub controls: ControlsPillModel,
    pub connectivity: ConnectivityPillModel,
    pub battery: BatteryPillModel,
    pub keyboard_layout: String,
    pub clock: String,
    pub show_debug_toggle: bool,
    pub show_debug_overlay: bool,
    pub debug_overlay_text: String,
}

pub fn view(model: MainBarModel) -> Element<'static, Message> {
    let mut left = Row::new()
        .spacing(12)
        .align_y(Alignment::Center)
        .push(launcher_button(model.opacity))
        .push(container(workspace_row(&model.workspaces, model.opacity)).width(Length::Shrink));
    if model.visualizer.is_visible() {
        left = left.push(audio_visualizer(
            &model.visualizer,
            model.opacity,
            model.bar_height,
        ));
    }

    let center_bg = if model.special_workspace_visible {
        Color::from_rgb8(0x64, 0x2f, 0x37)
    } else {
        Color::from_rgb8(0x29, 0x2e, 0x42)
    };
    let center = container(
        container(
            text(model.center_title)
                .size(11)
                .style(|_| iced::widget::text::Style {
                    color: Some(Color::from_rgb8(0xc0, 0xca, 0xf5)),
                }),
        )
        .padding(Padding::from([2, 12]))
        .style(move |_| container::Style {
            background: Some(Background::Color(Color {
                a: model.opacity,
                ..center_bg
            })),
            border: iced::Border {
                radius: 12.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }),
    )
    .width(Length::Shrink);

    let pill_bg = Color::from_rgb8(0x29, 0x2e, 0x42);
    let pill_fg = Color::from_rgb8(0xc0, 0xca, 0xf5);
    let pill_border_radius = 12.0;

    let stats_pill = stats_pill(
        &model.stats,
        pill_bg,
        pill_fg,
        pill_border_radius,
        model.opacity,
    );
    let controls_pill = controls_pill(
        &model.controls,
        pill_bg,
        pill_fg,
        pill_border_radius,
        model.opacity,
    );
    let connectivity_pill = connectivity_pill(
        &model.connectivity,
        pill_bg,
        pill_fg,
        pill_border_radius,
        model.opacity,
    );
    let battery_pill = battery_pill(
        &model.battery,
        pill_bg,
        pill_fg,
        pill_border_radius,
        model.opacity,
    );

    if model.media.has_player {
        left = left.push(media_pill(
            &model.media,
            pill_bg,
            pill_fg,
            pill_border_radius,
            model.opacity,
        ));
    }
    let kbd_pill = plain_pill(
        model.keyboard_layout,
        14,
        pill_bg,
        pill_fg,
        pill_border_radius,
        model.opacity,
    );
    let clock_pill = plain_pill(
        model.clock,
        14,
        pill_bg,
        pill_fg,
        pill_border_radius,
        model.opacity,
    );

    let mut right_row = Row::new()
        .spacing(4)
        .align_y(Alignment::Center)
        .push(tray_row(&model.tray_items))
        .push(stats_pill)
        .push(controls_pill)
        .push(connectivity_pill)
        .push(battery_pill)
        .push(kbd_pill.on_press(Message::NextKeyboardLayout));

    if model.show_debug_toggle {
        let dbg_pill = plain_pill_with_color(
            "DBG",
            12,
            pill_bg,
            Color::from_rgb8(0x7a, 0xa2, 0xf7),
            pill_border_radius,
            model.opacity,
            Padding::from([4, 10]),
        );
        right_row = right_row.push(dbg_pill.on_press(Message::ToggleDebugOverlay));
    }

    right_row = right_row.push(clock_pill.on_press(Message::TogglePopup(Popup::Calendar)));

    let right = container(right_row).width(Length::Shrink);

    let center_overlay = container(center)
        .width(Length::Fixed(340.0))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center);

    let mut layers = Vec::new();
    layers.push(
        container(
            Row::new()
                .align_y(Alignment::Center)
                .push(left)
                .push(Space::with_width(Length::Fill))
                .push(right),
        )
        .width(Length::Fill)
        .into(),
    );
    layers.push(
        container(center_overlay)
            .width(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center)
            .into(),
    );
    if model.show_debug_overlay {
        let overlay = container(
            container(text(model.debug_overlay_text).size(10))
                .padding(Padding::from([2, 8]))
                .style(move |_| container::Style {
                    background: Some(Background::Color(Color {
                        a: model.opacity,
                        ..Color::from_rgb8(0x1f, 0x23, 0x33)
                    })),
                    text_color: Some(Color::from_rgb8(0x7a, 0xa2, 0xf7)),
                    border: iced::Border {
                        radius: 8.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
        )
        .align_x(iced::alignment::Horizontal::Left)
        .align_y(iced::alignment::Vertical::Center)
        .padding(Padding::from([2, 8]));
        layers.push(overlay.into());
    }

    container(stack(layers))
        .width(Length::Fill)
        .height(Length::Fixed(model.bar_height))
        .style(|_| container::Style {
            background: Some(Background::Color(Color::TRANSPARENT)),
            ..Default::default()
        })
        .into()
}

fn audio_visualizer(
    model: &AudioVisualizerModel,
    opacity: f32,
    _bar_height: f32,
) -> iced::widget::Container<'static, Message> {
    let mut row = Row::new()
        .spacing(model.gap)
        .align_y(Alignment::End)
        .height(Length::Fixed(model.max_height));
    let inactive = !model.active || !model.bars.iter().any(|bar| *bar > 0);
    for &level in &model.bars {
        let normalized = (f32::from(level) / 100.0).clamp(0.0, 1.0);
        let height = model.min_height + normalized * (model.max_height - model.min_height);
        let color = if inactive {
            Color {
                a: opacity * 0.45,
                ..Color::from_rgb8(0x56, 0x5f, 0x89)
            }
        } else {
            audio_visualizer_color(model.color_profile, normalized, opacity)
        };
        row = row.push(
            container(Space::with_width(Length::Fixed(model.bar_width)))
                .width(Length::Fixed(model.bar_width))
                .height(Length::Fixed(height))
                .style(move |_| container::Style {
                    background: Some(Background::Color(color)),
                    border: iced::Border {
                        radius: 2.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
        );
    }

    container(row)
        .height(Length::Fixed(
            model.max_height + f32::from(model.padding_y) * 2.0,
        ))
        .padding(Padding::from([model.padding_y, model.padding_x]))
        .align_y(iced::alignment::Vertical::Center)
        .style(move |_| container::Style {
            background: Some(Background::Color(Color {
                a: opacity,
                ..Color::from_rgb8(0x29, 0x2e, 0x42)
            })),
            border: iced::Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
}

fn audio_visualizer_color(
    profile: crate::services::audio_visualizer::VisualizerColorProfile,
    normalized: f32,
    opacity: f32,
) -> Color {
    let base = match profile {
        crate::services::audio_visualizer::VisualizerColorProfile::Heat => {
            if normalized > 0.72 {
                Color::from_rgb8(0xf7, 0x76, 0x8e)
            } else if normalized > 0.42 {
                Color::from_rgb8(0xe0, 0xaf, 0x68)
            } else {
                Color::from_rgb8(0x7a, 0xa2, 0xf7)
            }
        }
        crate::services::audio_visualizer::VisualizerColorProfile::Accent => {
            if normalized > 0.72 {
                Color::from_rgb8(0x9e, 0xce, 0x6a)
            } else if normalized > 0.42 {
                Color::from_rgb8(0x7d, 0xcf, 0xff)
            } else {
                Color::from_rgb8(0x7a, 0xa2, 0xf7)
            }
        }
        crate::services::audio_visualizer::VisualizerColorProfile::Mono => {
            if normalized > 0.72 {
                Color::from_rgb8(0xc0, 0xca, 0xf5)
            } else if normalized > 0.42 {
                Color::from_rgb8(0x9a, 0xb0, 0xe6)
            } else {
                Color::from_rgb8(0x56, 0x5f, 0x89)
            }
        }
    };

    Color { a: opacity, ..base }
}

pub fn trunc_with_ellipsis(input: &str, max_chars: usize) -> String {
    let count = input.chars().count();
    if count <= max_chars {
        return input.to_string();
    }
    if max_chars <= 1 {
        return "…".to_string();
    }
    let mut out: String = input.chars().take(max_chars - 1).collect();
    out.push('…');
    out
}

pub fn scroll_direction(delta: mouse::ScrollDelta) -> i8 {
    let y = match delta {
        mouse::ScrollDelta::Lines { y, .. } | mouse::ScrollDelta::Pixels { y, .. } => y,
    };
    if y > 0.0 {
        1
    } else if y < 0.0 {
        -1
    } else {
        0
    }
}

pub fn temperature_summary(sys_data: &crate::modules::system::SysData) -> String {
    if sys_data.temp_str.trim().is_empty() {
        format!("{}°C", sys_data.temp.round() as i32)
    } else {
        sys_data.temp_str.clone()
    }
}

pub fn temperature_color(temp_c: i32, default: Color) -> Color {
    if temp_c >= 80 {
        Color::from_rgb8(0xf7, 0x76, 0x8e)
    } else if temp_c >= 60 {
        Color::from_rgb8(0xe0, 0xaf, 0x68)
    } else {
        default
    }
}

pub fn volume_icon(audio: &crate::services::controls::AudioInfo) -> &'static str {
    if audio.muted || audio.volume == 0 {
        "󰝟"
    } else {
        ""
    }
}

pub fn mic_icon(mic: &crate::services::controls::MicInfo) -> &'static str {
    if mic.muted || mic.volume == 0 {
        "󰍭"
    } else {
        ""
    }
}

pub fn wifi_icon(enabled: bool) -> &'static str {
    if enabled {
        "󰖩"
    } else {
        "󰖪"
    }
}

pub fn bluetooth_icon(enabled: bool) -> &'static str {
    if enabled {
        "󰂯"
    } else {
        "󰂲"
    }
}

pub fn wifi_label(enabled: bool, ssid: &str, max_chars: usize) -> String {
    if !enabled {
        return "Off".to_string();
    }

    let ssid = ssid.trim();
    if ssid.is_empty() || ssid == "Disconnected" || ssid == "Loading..." {
        "On".to_string()
    } else {
        trunc_with_ellipsis(ssid, max_chars)
    }
}

fn launcher_button(opacity: f32) -> iced::widget::Button<'static, Message> {
    button(text("").size(13))
        .padding(Padding::from([2, 8]))
        .on_press(Message::OpenLauncher)
        .style(move |_, _| iced::widget::button::Style {
            background: Some(Background::Color(Color {
                a: opacity,
                ..Color::from_rgb8(0x29, 0x2e, 0x42)
            })),
            text_color: Color::from_rgb8(0xc0, 0xca, 0xf5),
            border: iced::Border {
                radius: 10.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
}

fn workspace_row(workspaces: &[WorkspaceChip], opacity: f32) -> Row<'static, Message> {
    let mut row = Row::new().spacing(6).align_y(Alignment::Center);
    for ws in workspaces {
        let ws_id = ws.id;
        let ws_name = ws.name.clone();
        let is_active = ws.active;
        let is_special = ws.special;
        let btn = button(text(ws.name.clone()).size(12))
            .padding(Padding::from([1, 6]))
            .on_press(Message::SwitchWorkspace(ws_id, ws_name))
            .style(move |_, _| workspace_button_style(is_active, is_special, opacity));
        row = row.push(btn);
    }
    row
}

fn tray_row(items: &[TrayItemModel]) -> Row<'static, Message> {
    let mut row = Row::new().spacing(6).align_y(Alignment::Center);
    for item in items {
        let id_clone = item.id.clone();
        let id_right = item.id.clone();
        if let Some(handle) = &item.icon_handle {
            row = row.push(
                mouse_area(
                    image(handle.clone())
                        .width(Length::Fixed(16.0))
                        .height(Length::Fixed(16.0)),
                )
                .on_press(Message::TrayItemClicked(id_clone))
                .on_right_press(Message::TrayItemRightClicked(id_right)),
            );
        } else {
            row = row.push(
                mouse_area(
                    container(text(item.fallback_label.clone()).size(14))
                        .width(Length::Fixed(16.0))
                        .height(Length::Fixed(16.0))
                        .align_x(iced::alignment::Horizontal::Center),
                )
                .on_press(Message::TrayItemClicked(id_clone))
                .on_right_press(Message::TrayItemRightClicked(id_right)),
            );
        }
    }
    row
}

fn pill_button_style(
    bg: Color,
    fg: Color,
    radius: f32,
    opacity: f32,
) -> impl Fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style {
    move |_, status| {
        let active_bg = Color { a: opacity, ..bg };
        let (background, text_color) = match status {
            iced::widget::button::Status::Hovered => (
                Some(Background::Color(Color {
                    a: (opacity + 0.1).min(1.0),
                    ..bg
                })),
                fg,
            ),
            iced::widget::button::Status::Pressed => (
                Some(Background::Color(Color {
                    a: (opacity + 0.2).min(1.0),
                    ..bg
                })),
                fg,
            ),
            _ => (Some(Background::Color(active_bg)), fg),
        };

        iced::widget::button::Style {
            background,
            text_color,
            border: iced::Border {
                radius: radius.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

fn stats_pill(
    model: &StatsPillModel,
    pill_bg: Color,
    pill_fg: Color,
    radius: f32,
    opacity: f32,
) -> iced::widget::Button<'static, Message> {
    let cpu_summary = model.cpu_summary.clone();
    let temp_summary = model.temp_summary.clone();
    let temp_color = model.temp_color;
    let fan_speed = model.fan_speed.clone();
    button(
        Row::new()
            .spacing(6)
            .align_y(Alignment::Center)
            .push(text("").size(14))
            .push(text(cpu_summary).size(14))
            .push(
                text("")
                    .size(14)
                    .style(move |_| iced::widget::text::Style {
                        color: Some(temp_color),
                    }),
            )
            .push(
                text(temp_summary)
                    .size(14)
                    .style(move |_| iced::widget::text::Style {
                        color: Some(temp_color),
                    }),
            )
            .push(text("󰈐").size(14))
            .push(text(fan_speed).size(14)),
    )
    .padding(Padding::from([4, 12]))
    .on_press(Message::TogglePopup(Popup::Stats))
    .style(pill_button_style(pill_bg, pill_fg, radius, opacity))
}

fn controls_pill(
    model: &ControlsPillModel,
    pill_bg: Color,
    pill_fg: Color,
    radius: f32,
    opacity: f32,
) -> Element<'static, Message> {
    button(
        Row::new()
            .spacing(3)
            .align_y(Alignment::Center)
            .push(
                mouse_area(
                    Row::new()
                        .spacing(2)
                        .align_y(Alignment::Center)
                        .push(text("󰃠").size(14))
                        .push(text(model.brightness_label.clone()).size(14)),
                )
                .on_press(Message::TogglePopup(Popup::Controls))
                .on_scroll(|delta| Message::AdjustBrightnessBy(scroll_direction(delta))),
            )
            .push(separator_dot())
            .push(
                mouse_area(
                    Row::new()
                        .spacing(2)
                        .align_y(Alignment::Center)
                        .push(text(model.volume_icon).size(14))
                        .push(text(model.volume_label.clone()).size(14)),
                )
                .on_press(Message::TogglePopup(Popup::Controls))
                .on_scroll(|delta| Message::AdjustVolumeBy(scroll_direction(delta))),
            )
            .push(separator_dot())
            .push(
                mouse_area(
                    Row::new()
                        .spacing(2)
                        .align_y(Alignment::Center)
                        .push(text(model.mic_icon).size(14))
                        .push(text(model.mic_label.clone()).size(14)),
                )
                .on_press(Message::TogglePopup(Popup::Controls))
                .on_scroll(|delta| Message::AdjustMicVolumeBy(scroll_direction(delta))),
            ),
    )
    .padding(Padding::from([4, 8]))
    .on_press(Message::TogglePopup(Popup::Controls))
    .style(pill_button_style(pill_bg, pill_fg, radius, opacity))
    .into()
}

fn connectivity_pill(
    model: &ConnectivityPillModel,
    pill_bg: Color,
    pill_fg: Color,
    radius: f32,
    opacity: f32,
) -> iced::widget::Button<'static, Message> {
    button(
        Row::new()
            .spacing(6)
            .align_y(Alignment::Center)
            .push(text(model.wifi_icon).size(14))
            .push(text(model.wifi_label.clone()).size(14))
            .push(text(model.bluetooth_icon).size(14))
            .push_maybe(
                model
                    .bluetooth_label
                    .clone()
                    .map(|summary| text(summary).size(12)),
            ),
    )
    .padding(Padding::from([4, 12]))
    .on_press(Message::TogglePopup(Popup::Connectivity))
    .style(pill_button_style(pill_bg, pill_fg, radius, opacity))
}

fn battery_pill(
    model: &BatteryPillModel,
    pill_bg: Color,
    pill_fg: Color,
    radius: f32,
    opacity: f32,
) -> iced::widget::Button<'static, Message> {
    let battery_icon = model.battery_icon;
    let battery_color = model.battery_color;
    let battery_label = model.battery_label.clone();
    let power_profile_label = model.power_profile_label.clone();
    let power_profile_color = model.power_profile_color;
    let idle_enabled = model.idle_enabled;
    button({
        let mut battery_row = Row::new()
            .spacing(6)
            .align_y(Alignment::Center)
            .push(
                text(battery_icon)
                    .size(14)
                    .style(move |_| iced::widget::text::Style {
                        color: Some(battery_color),
                    }),
            )
            .push(
                text(battery_label)
                    .size(14)
                    .style(move |_| iced::widget::text::Style {
                        color: Some(battery_color),
                    }),
            )
            .push(
                container(text(power_profile_label).size(13).style(move |_| {
                    iced::widget::text::Style {
                        color: Some(power_profile_color),
                    }
                }))
                .height(Length::Fixed(16.0))
                .align_y(iced::alignment::Vertical::Center),
            );

        if idle_enabled {
            battery_row = battery_row.push(text("").size(13));
        }

        battery_row
    })
    .padding(Padding::from([4, 12]))
    .on_press(Message::TogglePopup(Popup::Power))
    .style(pill_button_style(pill_bg, pill_fg, radius, opacity))
}

fn plain_pill(
    label: impl Into<String>,
    font_size: u16,
    pill_bg: Color,
    pill_fg: Color,
    radius: f32,
    opacity: f32,
) -> iced::widget::Button<'static, Message> {
    plain_pill_with_color(
        label,
        font_size,
        pill_bg,
        pill_fg,
        radius,
        opacity,
        Padding::from([4, 12]),
    )
}

fn plain_pill_with_color(
    label: impl Into<String>,
    font_size: u16,
    pill_bg: Color,
    pill_fg: Color,
    radius: f32,
    opacity: f32,
    padding: Padding,
) -> iced::widget::Button<'static, Message> {
    button(text(label.into()).size(font_size))
        .padding(padding)
        .style(pill_button_style(pill_bg, pill_fg, radius, opacity))
}

fn separator_dot() -> iced::widget::Text<'static> {
    text("·").size(9).style(|_| iced::widget::text::Style {
        color: Some(Color::from_rgb8(0x56, 0x5f, 0x89)),
    })
}

fn workspace_button_style(
    active: bool,
    special: bool,
    opacity: f32,
) -> iced::widget::button::Style {
    if active {
        let (bg, fg) = if special {
            (
                Color::from_rgb8(0xff, 0xa0, 0x3d),
                Color::from_rgb8(0x1a, 0x1b, 0x26),
            )
        } else {
            (
                Color::from_rgb8(0x7a, 0xa2, 0xf7),
                Color::from_rgb8(0x1a, 0x1b, 0x26),
            )
        };
        iced::widget::button::Style {
            background: Some(Background::Color(Color { a: opacity, ..bg })),
            text_color: fg,
            border: iced::Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    } else {
        let (bg, fg) = if special {
            (
                Color::from_rgb8(0x5f, 0x3a, 0x1f),
                Color::from_rgb8(0xff, 0xd1, 0x9a),
            )
        } else {
            (
                Color::from_rgb8(0x29, 0x2e, 0x42),
                Color::from_rgb8(0xc0, 0xca, 0xf5),
            )
        };
        iced::widget::button::Style {
            background: Some(Background::Color(Color { a: opacity, ..bg })),
            text_color: fg,
            border: iced::Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

fn media_pill(
    model: &MediaPillModel,
    bg: Color,
    fg: Color,
    radius: f32,
    opacity: f32,
) -> Element<'static, Message> {
    let play_pause_icon = if model.playback_status == "Playing" {
        "󰏤"
    } else {
        "󰐊"
    };

    let controls = button(
        Row::new()
            .spacing(10)
            .align_y(Alignment::Center)
            .push(
                button(
                    text("󰒮")
                        .size(14)
                        .style(move |_| iced::widget::text::Style { color: Some(fg) }),
                )
                .padding(0)
                .on_press(Message::MediaCommand(
                    crate::services::media::MediaCommand::Previous,
                ))
                .style(move |_, _| button::Style::default()),
            )
            .push(
                button(
                    text(play_pause_icon)
                        .size(16)
                        .style(move |_| iced::widget::text::Style { color: Some(fg) }),
                )
                .padding(0)
                .on_press(Message::MediaCommand(
                    crate::services::media::MediaCommand::PlayPause,
                ))
                .style(move |_, _| button::Style::default()),
            )
            .push(
                button(
                    text("󰒭")
                        .size(14)
                        .style(move |_| iced::widget::text::Style { color: Some(fg) }),
                )
                .padding(0)
                .on_press(Message::MediaCommand(
                    crate::services::media::MediaCommand::Next,
                ))
                .style(move |_, _| button::Style::default()),
            ),
    )
    .padding(Padding::from([4, 12]))
    .style(pill_button_style(bg, fg, radius, opacity));

    let title_pill = button(
        text(model.title.clone())
            .size(11)
            .font(Font::MONOSPACE)
            .style(move |_| iced::widget::text::Style { color: Some(fg) }),
    )
    .padding(Padding::from([4, 12]))
    .on_press(Message::TogglePopup(Popup::Media))
    .style(pill_button_style(bg, fg, radius, opacity));

    Row::new()
        .spacing(4)
        .align_y(Alignment::Center)
        .push(controls)
        .push(title_pill)
        .into()
}

#[cfg(test)]
mod tests {
    use super::{
        audio_visualizer_color, scroll_direction, trunc_with_ellipsis, wifi_label,
        AudioVisualizerModel,
    };
    use crate::services::audio_visualizer::VisualizerColorProfile;
    use iced::mouse;

    #[test]
    fn trunc_with_ellipsis_shortens_only_when_needed() {
        assert_eq!(trunc_with_ellipsis("ThinkPadBar", 32), "ThinkPadBar");
        assert_eq!(trunc_with_ellipsis("abcdefghijkl", 5), "abcd…");
        assert_eq!(trunc_with_ellipsis("ab", 1), "…");
    }

    #[test]
    fn wifi_label_prefers_state_and_truncates_ssid() {
        assert_eq!(wifi_label(false, "ignored", 8), "Off");
        assert_eq!(wifi_label(true, "", 8), "On");
        assert_eq!(wifi_label(true, "Loading...", 8), "On");
        assert_eq!(wifi_label(true, "very-long-network-name", 8), "very-lo…");
    }

    #[test]
    fn scroll_direction_maps_line_and_pixel_delta() {
        assert_eq!(
            scroll_direction(mouse::ScrollDelta::Lines { x: 0.0, y: 1.0 }),
            1
        );
        assert_eq!(
            scroll_direction(mouse::ScrollDelta::Pixels { x: 0.0, y: -12.0 }),
            -1
        );
        assert_eq!(
            scroll_direction(mouse::ScrollDelta::Lines { x: 0.0, y: 0.0 }),
            0
        );
    }

    #[test]
    fn visualizer_visibility_requires_active_non_zero_levels() {
        assert!(!AudioVisualizerModel {
            enabled: false,
            bars: vec![0; 16],
            active: false,
            min_height: 4.0,
            max_height: 18.0,
            bar_width: 3.0,
            gap: 2,
            padding_x: 6,
            padding_y: 2,
            color_profile: VisualizerColorProfile::Heat,
        }
        .is_visible());
        assert!(AudioVisualizerModel {
            enabled: true,
            bars: vec![0; 16],
            active: true,
            min_height: 4.0,
            max_height: 18.0,
            bar_width: 3.0,
            gap: 2,
            padding_x: 6,
            padding_y: 2,
            color_profile: VisualizerColorProfile::Heat,
        }
        .is_visible());
        assert!(AudioVisualizerModel {
            enabled: true,
            bars: vec![5; 16],
            active: true,
            min_height: 4.0,
            max_height: 18.0,
            bar_width: 3.0,
            gap: 2,
            padding_x: 6,
            padding_y: 2,
            color_profile: VisualizerColorProfile::Heat,
        }
        .is_visible());
    }

    #[test]
    fn visualizer_color_profiles_map_to_distinct_palettes() {
        let heat = audio_visualizer_color(VisualizerColorProfile::Heat, 0.9, 0.85);
        let accent = audio_visualizer_color(VisualizerColorProfile::Accent, 0.9, 0.85);
        let mono = audio_visualizer_color(VisualizerColorProfile::Mono, 0.9, 0.85);

        assert_ne!(heat, accent);
        assert_ne!(accent, mono);
        assert_eq!(heat.a, 0.85);
    }
}
