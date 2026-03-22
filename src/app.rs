use iced::{widget::{button, container, text, Row, Column, slider, mouse_area, text_input, scrollable, stack, Space, image}, Element, Task, Theme, Color, Length, Alignment, Padding, window::Id};
use chrono::Local;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Popup {
    None,
    ControlCenter,
    SystemMonitor,
    Calendar,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PowerAction {
    Lock,
    Sleep,
    Hibernate,
    Restart,
    Shutdown,
    Logout,
}

pub struct ThinkPadBar {
    workspaces: Vec<crate::modules::workspaces::WorkspaceInfo>,
    active_window: String,
    clock: String,
    brightness: String,
    audio: crate::modules::audio::AudioInfo,
    mic: crate::modules::mic::MicInfo,
    fan: crate::modules::fan::FanInfo,
    battery: (u8, String),
    power_profile: String,
    wifi: crate::modules::wifi::WifiInfo,
    show_wifi_menu: bool,
    show_power_menu: bool,
    keyboard_layout: String,
    available_networks: Vec<crate::modules::wifi::WifiNetwork>,
    wifi_password_input: String,
    wifi_selected_ssid: Option<String>,
    bluetooth: bool,
    popup: Popup,
    volume_level: u32,
    mic_volume_level: u32,
    brightness_level: u32,
    main_window_id: Option<Id>,
    popup_window_id: Option<Id>,
    sys_monitor: crate::modules::system::SysMonitor,
    sys_data: crate::modules::system::SysData,
    calendar_offset: i32,
    tray: crate::modules::tray::Tray,
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick(chrono::DateTime<chrono::Local>),
    UpdateWorkspaces,
    SwitchWorkspace(i32),
    TogglePopup(Popup),
    SetVolume(u32),
    SetMicVolume(u32),
    SetFanLevel(String),
    SetBrightness(u32),
    SetPowerProfile(String),
    ToggleWifiMenu,
    NetworksScanned(Vec<crate::modules::wifi::WifiNetwork>),
    SelectWifiNetwork(String, String),
    WifiPasswordChanged(String),
    SubmitWifiPassword,
    CancelWifiPassword,
    WifiConnectResult(bool),
    ToggleWifi(bool),
    ToggleBluetooth(bool),
    UpdateKeyboardLayout,
    NextKeyboardLayout,
    TogglePowerMenu,
    PowerAction(PowerAction),
    ToggleAudioMute,
    ToggleMicMute,
    CalendarPrevMonth,
    CalendarNextMonth,
    TrayMessage(crate::modules::tray::TrayMessage),
    TrayItemClicked(String),
}

impl ThinkPadBar {
    pub fn new() -> impl FnOnce() -> (Self, Task<Message>) {
        move || {
            let main_id = Id::unique();
            let popup_id = Id::unique();

            use iced::platform_specific::shell::commands::layer_surface::{
                Anchor, KeyboardInteractivity, Layer, get_layer_surface,
            };
            use iced::runtime::platform_specific::wayland::layer_surface::{SctkLayerSurfaceSettings, IcedMargin};

            let main_task = get_layer_surface(SctkLayerSurfaceSettings {
                id: main_id,
                namespace: "thinkpadbar-main".to_string(),
                size: Some((None, Some(32))),
                layer: Layer::Top,
                keyboard_interactivity: KeyboardInteractivity::None,
                exclusive_zone: 32,
                anchor: Anchor::TOP | Anchor::LEFT | Anchor::RIGHT,
                margin: IcedMargin { top: 0, right: 4, bottom: 0, left: 4 },
                ..Default::default()
            });

            let popup_task = get_layer_surface(SctkLayerSurfaceSettings {
                id: popup_id,
                namespace: "thinkpadbar-popup".to_string(),
                size: Some((None, Some(600))), 
                layer: Layer::Background,
                keyboard_interactivity: KeyboardInteractivity::None,
                anchor: Anchor::TOP | Anchor::LEFT | Anchor::RIGHT,
                margin: IcedMargin { top: 24, right: 0, bottom: 0, left: 0 },
                ..Default::default()
            });

            let mut monitor = crate::modules::system::SysMonitor::new();
            let initial_sys_data = monitor.update();

            (
                Self {
                    clock: Local::now().format("%a %d %b %H:%M").to_string(),
                    workspaces: crate::modules::workspaces::get_workspaces(),
                    active_window: crate::modules::workspaces::get_active_window_title(),
                    brightness: crate::modules::brightness::get_brightness(),
                    audio: crate::modules::audio::get_info(),
                    mic: crate::modules::mic::get_info(),
                    fan: crate::modules::fan::get_fan_info(),
                    battery: crate::modules::battery::get_battery_info(),
                    power_profile: crate::modules::power::get_profile(),
                    wifi: crate::modules::wifi::get_wifi_info(),
                    bluetooth: crate::modules::bluetooth::get_bluetooth_info(),
                    show_wifi_menu: false,
                    show_power_menu: false,
                    keyboard_layout: crate::modules::keyboard::get_layout(),
                    available_networks: Vec::new(),
                    wifi_password_input: String::new(),
                    wifi_selected_ssid: None,
                    popup: Popup::None,
                    volume_level: 0,
                    mic_volume_level: 0,
                    brightness_level: 50,
                    main_window_id: Some(main_id),
                    popup_window_id: Some(popup_id),
                    sys_monitor: monitor,
                    sys_data: initial_sys_data,
                    calendar_offset: 0,
                    tray: crate::modules::tray::Tray::new(),
                },
                Task::batch(vec![main_task, popup_task]),
            )
        }
    }

    pub fn title(&self, _id: Id) -> String {
        "thinkpadbar".to_string()
    }

    pub fn style(&self, _theme: &Theme) -> iced::daemon::Appearance {
        iced::daemon::Appearance {
            background_color: Color::TRANSPARENT,
            text_color: Color::WHITE,
            icon_color: Color::WHITE,
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick(now) => {
                self.clock = now.format("%a %d %b %H:%M").to_string();
                self.brightness = crate::modules::brightness::get_brightness();
                self.audio = crate::modules::audio::get_info();
                self.mic = crate::modules::mic::get_info();
                self.volume_level = self.audio.volume;
                self.mic_volume_level = self.mic.volume;
                self.fan = crate::modules::fan::get_fan_info();
                self.battery = crate::modules::battery::get_battery_info();
                self.power_profile = crate::modules::power::get_profile();
                self.wifi = crate::modules::wifi::get_wifi_info();
                self.bluetooth = crate::modules::bluetooth::get_bluetooth_info();
                self.keyboard_layout = crate::modules::keyboard::get_layout();
                self.sys_data = self.sys_monitor.update();
                
                // No longer need complex string parsing
            }
            Message::UpdateWorkspaces => {
                self.workspaces = crate::modules::workspaces::get_workspaces();
                self.active_window = crate::modules::workspaces::get_active_window_title();
            }
            Message::SwitchWorkspace(w_id) => {
                crate::modules::workspaces::switch_workspace(w_id);
                for ws in &mut self.workspaces {
                    ws.active = ws.id == w_id;
                }
            }
            Message::TogglePopup(target) => {
                use iced::platform_specific::shell::commands::layer_surface::{set_layer, set_keyboard_interactivity, Layer, KeyboardInteractivity};
                if self.popup == target {
                    self.popup = Popup::None;
                    if target == Popup::Calendar {
                        self.calendar_offset = 0;
                    }
                    if let Some(pid) = self.popup_window_id {
                        return Task::batch(vec![
                            set_layer(pid, Layer::Background),
                            set_keyboard_interactivity(pid, KeyboardInteractivity::None),
                        ]);
                    }
                } else {
                    let is_calendar = target == Popup::Calendar;
                    self.popup = target;
                    if is_calendar {
                        self.calendar_offset = 0;
                    }
                    if let Some(pid) = self.popup_window_id {
                        return Task::batch(vec![
                            set_layer(pid, Layer::Top),
                            set_keyboard_interactivity(pid, KeyboardInteractivity::OnDemand),
                        ]);
                    }
                }
            }
            Message::SetVolume(val) => {
                self.volume_level = val;
                crate::modules::audio::set_volume(val);
                self.audio.volume = val;
            }
            Message::ToggleAudioMute => {
                crate::modules::audio::toggle_mute();
                self.audio = crate::modules::audio::get_info();
            }
            Message::SetMicVolume(val) => {
                self.mic_volume_level = val;
                crate::modules::mic::set_volume(val);
                self.mic.volume = val;
            }
            Message::ToggleMicMute => {
                crate::modules::mic::toggle_mute();
                self.mic = crate::modules::mic::get_info();
            }
            Message::SetBrightness(val) => {
                self.brightness_level = val;
                crate::modules::brightness::set_brightness(val);
                self.brightness = format!("󰃠  {}%", val);
            }
            Message::SetFanLevel(level) => {
                crate::modules::fan::set_fan_level(&level);
                self.fan.level = level;
            }
            Message::SetPowerProfile(prof) => {
                crate::modules::power::set_profile(&prof);
                self.power_profile = prof;
            }
            Message::ToggleWifiMenu => {
                self.show_wifi_menu = !self.show_wifi_menu;
                if self.show_wifi_menu {
                    return Task::perform(crate::modules::wifi::scan_networks(), Message::NetworksScanned);
                }
            }
            Message::NetworksScanned(networks) => {
                self.available_networks = networks;
            }
            Message::SelectWifiNetwork(ssid, sec) => {
                if sec == "open" {
                    return Task::perform(crate::modules::wifi::connect_network(ssid, None), Message::WifiConnectResult);
                } else {
                    self.wifi_selected_ssid = Some(ssid);
                    self.wifi_password_input = String::new();
                }
            }
            Message::WifiPasswordChanged(val) => {
                self.wifi_password_input = val;
            }
            Message::SubmitWifiPassword => {
                if let Some(ssid) = self.wifi_selected_ssid.clone() {
                    let pass = self.wifi_password_input.clone();
                    self.wifi_selected_ssid = None;
                    self.show_wifi_menu = false;
                    return Task::perform(crate::modules::wifi::connect_network(ssid, Some(pass)), Message::WifiConnectResult);
                }
            }
            Message::CancelWifiPassword => {
                self.wifi_selected_ssid = None;
            }
            Message::WifiConnectResult(_success) => {}
            Message::ToggleWifi(enable) => {
                crate::modules::wifi::toggle_wifi(enable);
                self.wifi.enabled = enable;
            }
            Message::ToggleBluetooth(enable) => {
                crate::modules::bluetooth::toggle_bluetooth(enable);
                self.bluetooth = enable;
            }
            Message::UpdateKeyboardLayout => {
                self.keyboard_layout = crate::modules::keyboard::get_layout();
            }
            Message::NextKeyboardLayout => {
                crate::modules::keyboard::next_layout();
                self.keyboard_layout = crate::modules::keyboard::get_layout();
            }
            Message::TogglePowerMenu => {
                self.show_power_menu = !self.show_power_menu;
            }
            Message::CalendarPrevMonth => {
                self.calendar_offset -= 1;
            }
            Message::CalendarNextMonth => {
                self.calendar_offset += 1;
            }
            Message::PowerAction(action) => {
                let cmd = match action {
                    PowerAction::Lock => "hyprlock",
                    PowerAction::Sleep => "systemctl suspend",
                    PowerAction::Hibernate => "systemctl hibernate",
                    PowerAction::Restart => "systemctl reboot",
                    PowerAction::Shutdown => "systemctl poweroff",
                    PowerAction::Logout => "hyprctl dispatch exit",
                };
                let mut args = cmd.split_whitespace();
                let bin = args.next().unwrap();
                let _ = std::process::Command::new(bin).args(args).spawn();
                self.popup = Popup::None; // Close after action if didn't quit app
            }
            Message::TrayMessage(msg) => {
                self.tray.update(msg);
            }
            Message::TrayItemClicked(id) => {
                if let Some(item) = self.tray.items.get(&id) {
                    let name = item.title.clone()
                        .or_else(|| item.icon_name.clone())
                        .or_else(|| id.split('.').last().map(|s| s.to_string()))
                        .unwrap_or_default();
                    
                    self.tray.update(crate::modules::tray::TrayMessage::ActivateItem(id));
                    return Task::perform(crate::modules::workspaces::find_and_switch_to_app(name), |_| Message::UpdateWorkspaces);
                }
            }
        }
        Task::none()
    }

    pub fn view(&self, id: Id) -> Element<'_, Message, Theme, iced::Renderer> {
        if Some(id) == self.main_window_id {
            self.view_main_bar()
        } else if Some(id) == self.popup_window_id {
            if self.popup == Popup::None {
                iced::widget::Space::new(Length::Fill, Length::Fixed(450.0)).into()
            } else {
                let popup_layer = container(self.view_popup())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(Padding::from([0, 16]))
                    .align_x(match self.popup {
                        Popup::SystemMonitor => iced::alignment::Horizontal::Center,
                        _ => iced::alignment::Horizontal::Right,
                    });
                
                mouse_area(popup_layer)
                    .on_press(Message::TogglePopup(Popup::None))
                    .into()
            }
        } else {
            Row::new().into()
        }
    }

    fn view_main_bar(&self) -> Element<'_, Message, Theme, iced::Renderer> {
        // Build Workspaces widget
        let mut ws_row = Row::new().spacing(6).align_y(Alignment::Center);
        for ws in &self.workspaces {
            let ws_id = ws.id;
            let is_active = ws.active;
            
            let btn = button(text(ws.name.clone()).size(12))
                .padding(Padding::from([1, 6]))
                .on_press(Message::SwitchWorkspace(ws_id))
                .style(move |_, _| {
                    if is_active {
                        iced::widget::button::Style {
                            background: Some(iced::Background::Color(Color { a: 0.85, ..Color::from_rgb8(0x7a, 0xa2, 0xf7) })), // Tokyo Night Blue
                            text_color: Color::from_rgb8(0x1a, 0x1b, 0x26),
                            border: iced::Border { radius: 8.0.into(), ..Default::default() },
                            ..Default::default()
                        }
                    } else {
                        iced::widget::button::Style {
                            background: Some(iced::Background::Color(Color { a: 0.85, ..Color::from_rgb8(0x29, 0x2e, 0x42) })),
                            text_color: Color::from_rgb8(0xc0, 0xca, 0xf5),
                            border: iced::Border { radius: 8.0.into(), ..Default::default() },
                            ..Default::default()
                        }
                    }
                });
            ws_row = ws_row.push(btn);
        }
        let mut tray_row = Row::new().spacing(6).align_y(Alignment::Center);
        for (id, item) in self.tray.items.iter() {
            let id_clone = id.clone();
            if let Some(handle) = &item.icon_handle {
                tray_row = tray_row.push(
                    mouse_area(
                        image(handle.clone())
                            .width(Length::Fixed(16.0))
                            .height(Length::Fixed(16.0))
                    ).on_press(Message::TrayItemClicked(id_clone))
                );
            } else if let Some(name) = &item.icon_name {
                tray_row = tray_row.push(
                    mouse_area(
                        container(text(name.chars().next().unwrap_or('?').to_string()).size(14))
                            .width(Length::Fixed(16.0))
                            .height(Length::Fixed(16.0))
                            .align_x(iced::alignment::Horizontal::Center)
                    ).on_press(Message::TrayItemClicked(id_clone))
                );
            } else {
                tray_row = tray_row.push(
                    mouse_area(
                        container(text("?").size(14))
                            .width(Length::Fixed(16.0))
                            .height(Length::Fixed(16.0))
                            .align_x(iced::alignment::Horizontal::Center)
                    ).on_press(Message::TrayItemClicked(id_clone))
                );
            }
        }

        let left = Row::new().spacing(12).align_y(Alignment::Center)
            .push(container(ws_row).width(Length::Fixed(280.0)));

        // Center: Active Window Title
        let center = container(
            container(
                text(&self.active_window)
                    .size(11)
                    .style(|_| iced::widget::text::Style { color: Some(Color::from_rgb8(0xc0, 0xca, 0xf5)) })
            )
            .padding(Padding::from([2, 12]))
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(Color { a: 0.85, ..Color::from_rgb8(0x29, 0x2e, 0x42) })),
                border: iced::Border { radius: 12.0.into(), ..Default::default() },
                ..Default::default()
            })
        ).width(Length::Shrink);

        // Right side: Pills
        let pill_bg = Color { a: 0.85, ..Color::from_rgb8(0x29, 0x2e, 0x42) };
        let pill_fg = Color::from_rgb8(0xc0, 0xca, 0xf5);
        let pill_border_radius = 12.0;

        let cpu_str = format!("{}%", self.sys_data.cpu_usage.round() as u64);
        let mem_str = if self.sys_data.mem_total > 0 { 
            format!("{}%", (self.sys_data.mem_used as f64 / self.sys_data.mem_total as f64 * 100.0).round() as u64) 
        } else { "0%".to_string() };
        let temp_val = self.sys_data.temp.round() as i32;
        let temp_str = format!("{}°C", temp_val);
        let temp_color = if temp_val >= 80 {
            Color::from_rgb8(0xf7, 0x76, 0x8e) // Red
        } else if temp_val >= 60 {
            Color::from_rgb8(0xe0, 0xaf, 0x68) // Yellow
        } else {
            pill_fg
        };

        // 1. System Pill
        let sys_pill = container(
            Row::new().spacing(4).align_y(Alignment::Center)
                .push(text("󰍹").size(14))
                .push(text(cpu_str).size(14))
                .push(iced::widget::Space::with_width(4))
                .push(text("󰘚").size(14))
                .push(text(mem_str).size(14))
                .push(iced::widget::Space::with_width(4))
                .push(text("").size(14).style(move |_| iced::widget::text::Style { color: Some(temp_color) }))
                .push(text(temp_str).size(14).style(move |_| iced::widget::text::Style { color: Some(temp_color) }))
        )
        .padding(Padding::from([4, 12]))
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(pill_bg)),
            text_color: Some(pill_fg),
            border: iced::Border { radius: pill_border_radius.into(), ..Default::default() },
            ..Default::default()
        });

        let mic_icon = if self.mic.muted || self.mic.volume == 0 { "󰍭" } else { "" };
        let audio_icon = if self.audio.muted { "󰝟" } else { "" };
        let wifi_icon = if self.wifi.enabled { "󰖩" } else { "󰖪" };
        let bt_icon = if self.bluetooth { "󰂯" } else { "󰂲" };
        let (bat_cap, bat_status) = &self.battery;
        let bat_icon = if bat_status.contains("Charging") { "󰂄" } else { "󰁹" };

        let combined_pill = container(
            Row::new().spacing(6).align_y(Alignment::Center)
                .push(text(wifi_icon).size(14))
                .push(text(bt_icon).size(14))
                .push(text(bat_icon).size(14))
                .push(text(format!("{}%", bat_cap)).size(14))
        )
        .padding(Padding::from([4, 12]))
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(pill_bg)),
            text_color: Some(pill_fg),
            border: iced::Border { radius: pill_border_radius.into(), ..Default::default() },
            ..Default::default()
        });

        // 3. Hardware Pill (Brightness, Audio/Mic, and Fan)
        let hw_pill = container(
            Row::new().spacing(10).align_y(Alignment::Center)
                .push(text(&self.brightness).size(14))
                .push(
                    Row::new().spacing(4).align_y(Alignment::Center)
                        .push(text(mic_icon).size(14))
                        .push(text(format!("{} {}%", audio_icon, self.audio.volume)).size(14))
                )
                .push(text(format!(" {}", self.fan.speed)).size(14))
        )
        .padding(Padding::from([4, 12]))
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(pill_bg)),
            text_color: Some(pill_fg),
            border: iced::Border { radius: pill_border_radius.into(), ..Default::default() },
            ..Default::default()
        });

        // 4. Keyboard Layout Pill
        let kbd_pill = container(
            text(&self.keyboard_layout).size(14)
        )
        .padding(Padding::from([4, 12]))
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(pill_bg)),
            text_color: Some(pill_fg),
            border: iced::Border { radius: pill_border_radius.into(), ..Default::default() },
            ..Default::default()
        });

        // 5. Clock Pill
        let clock_pill = container(
            text(&self.clock).size(14)
        )
        .padding(Padding::from([4, 12]))
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(pill_bg)),
            text_color: Some(pill_fg),
            border: iced::Border { radius: pill_border_radius.into(), ..Default::default() },
            ..Default::default()
        });

        let right = container(
            Row::new().spacing(4).align_y(Alignment::Center)
                .push(tray_row)
                .push(mouse_area(sys_pill).on_press(Message::TogglePopup(Popup::SystemMonitor)))
                .push(mouse_area(hw_pill).on_press(Message::TogglePopup(Popup::ControlCenter)))
                .push(mouse_area(combined_pill).on_press(Message::TogglePopup(Popup::ControlCenter)))
                .push(mouse_area(kbd_pill).on_press(Message::NextKeyboardLayout))
                .push(mouse_area(clock_pill).on_press(Message::TogglePopup(Popup::Calendar)))
        ).width(Length::Shrink);

        container(
            stack![
                container(
                    Row::new()
                        .push(left)
                        .push(Space::with_width(Length::Fill))
                        .push(right)
                )
                .width(Length::Fill),
                container(center)
                    .width(Length::Fill)
                    .align_x(iced::alignment::Horizontal::Center)
                    .align_y(iced::alignment::Vertical::Center)
            ]
        )
        .width(Length::Fill)
        .height(Length::Fixed(24.0))
        .style(|_| container::Style {
            background: Some(iced::Background::Color(Color::TRANSPARENT)),
            ..Default::default()
        })
        .into()
    }

    fn view_popup(&self) -> Element<'_, Message, Theme, iced::Renderer> {
        if self.popup == Popup::Calendar {
            let now = chrono::Local::now();
            
            // Calculate displayed month/year based on offset
            use chrono::{Datelike, TimeZone};
            let mut display_month = now.month() as i32 + self.calendar_offset;
            let mut display_year = now.year();
            
            while display_month > 12 {
                display_month -= 12;
                display_year += 1;
            }
            while display_month < 1 {
                display_month += 12;
                display_year -= 1;
            }
            
            let display_date = chrono::Local.with_ymd_and_hms(display_year, display_month as u32, 1, 0, 0, 0).unwrap();
            let month_name = display_date.format("%B %Y").to_string();
            
            let current_day = if display_year == now.year() && display_month as u32 == now.month() { Some(now.day()) } else { None };
            
            // Calculate first day of month and days in month
            let weekday_offset = (display_date.weekday().number_from_monday() - 1) as usize;
            
            let days_in_month = match display_date.month() {
                1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
                4 | 6 | 9 | 11 => 30,
                2 => if display_date.year() % 4 == 0 && (display_date.year() % 100 != 0 || display_date.year() % 400 == 0) { 29 } else { 28 },
                _ => 30,
            };

            let title_row = Row::new().spacing(10).align_y(Alignment::Center)
                .push(button(text("󰅀").size(20)).on_press(Message::CalendarPrevMonth).style(|_, _| button::Style { text_color: Color::from_rgb8(0x7a, 0xa2, 0xf7), ..Default::default() }))
                .push(text(month_name).size(18).width(Length::Fill).align_x(iced::alignment::Horizontal::Center))
                .push(button(text("󰅂").size(20)).on_press(Message::CalendarNextMonth).style(|_, _| button::Style { text_color: Color::from_rgb8(0x7a, 0xa2, 0xf7), ..Default::default() }));

            let mut header_row = Row::new().spacing(0);
            for day in ["Пн", "Вт", "Ср", "Чт", "Пт", "Сб", "Вс"] {
                header_row = header_row.push(
                    container(text(day).size(12).style(|_| iced::widget::text::Style { color: Some(Color::from_rgb8(0x56, 0x5f, 0x89)) }))
                        .width(Length::Fill)
                        .align_x(iced::alignment::Horizontal::Center)
                );
            }

            let mut days_col = Column::new().spacing(8);
            let mut current_row = Row::new().spacing(0);
            
            // Empty spaces for offset
            for _ in 0..weekday_offset {
                current_row = current_row.push(Space::with_width(Length::Fill));
            }
            
            for d in 1..=days_in_month {
                let is_today = Some(d) == current_day;
                let day_btn = container(text(d.to_string()).size(14))
                    .width(Length::Fill)
                    .padding(8)
                    .align_x(iced::alignment::Horizontal::Center)
                    .style(move |_| {
                        if is_today {
                            container::Style {
                                background: Some(iced::Background::Color(Color::from_rgb8(0x7a, 0xa2, 0xf7))),
                                text_color: Some(Color::from_rgb8(0x1a, 0x1b, 0x26)),
                                border: iced::Border { radius: 8.0.into(), ..Default::default() },
                                ..Default::default()
                            }
                        } else {
                            container::Style::default()
                        }
                    });
                
                current_row = current_row.push(day_btn);
                
                if (weekday_offset + (d as usize)) % 7 == 0 || d == days_in_month {
                    // Fill the last row if needed
                    if d == days_in_month {
                        let remaining = 7 - ((weekday_offset + (d as usize)) % 7);
                        if remaining < 7 {
                            for _ in 0..remaining {
                                current_row = current_row.push(Space::with_width(Length::Fill));
                            }
                        }
                    }
                    days_col = days_col.push(current_row);
                    current_row = Row::new().spacing(0);
                }
            }

            let content = Column::new().spacing(16)
                .push(title_row)
                .push(header_row)
                .push(days_col);

            return container(
                container(content)
                    .width(Length::Fixed(360.0))
                    .padding(24)
                    .style(|_| container::Style {
                        background: Some(iced::Background::Color(Color { a: 0.85, ..Color::from_rgb8(0x11, 0x12, 0x1d) })),
                        text_color: Some(Color::from_rgb8(0xc0, 0xca, 0xf5)),
                        border: iced::Border { radius: 12.0.into(), ..Default::default() },
                        ..Default::default()
                    })
            )
            .width(Length::Fill)
            .align_x(iced::alignment::Horizontal::Right)
            .padding(Padding::from([0, 16]))
            .into();
        }

        if self.popup == Popup::SystemMonitor {
            let item = |icon: &str, label: &str, val: String| -> Element<'_, Message, Theme, iced::Renderer> {
                Row::new().spacing(12).align_y(Alignment::Center)
                    .push(text(icon.to_string()).size(16))
                    .push(text(label.to_string()).size(13).width(Length::Fill))
                    .push(text(val).size(13))
                    .into()
            };

            let mem_pct = if self.sys_data.mem_total > 0 { (self.sys_data.mem_used as f64 / self.sys_data.mem_total as f64 * 100.0).round() as u64 } else { 0 };
            let swap_pct = if self.sys_data.swap_total > 0 { (self.sys_data.swap_used as f64 / self.sys_data.swap_total as f64 * 100.0).round() as u64 } else { 0 };
            let disk_root_pct = if self.sys_data.disk_root_total > 0 { (self.sys_data.disk_root_used as f64 / self.sys_data.disk_root_total as f64 * 100.0).round() as u64 } else { 0 };
            let disk_boot_pct = if self.sys_data.disk_boot_total > 0 { (self.sys_data.disk_boot_used as f64 / self.sys_data.disk_boot_total as f64 * 100.0).round() as u64 } else { 0 };

            let format_net = |bytes: u64| -> String {
                if bytes > 1024 * 1024 {
                    format!("{:.1} MB/s", bytes as f64 / (1024.0 * 1024.0))
                } else {
                    format!("{} KB/s", bytes / 1024)
                }
            };

            let col = Column::new().spacing(12)
                .push(text("System Info").size(18).style(move |_| iced::widget::text::Style { color: Some(Color::from_rgb8(0xc0, 0xca, 0xf5)) }))
                .push(item("", "CPU Usage", format!("{}%", self.sys_data.cpu_usage.round() as u64)))
                .push(item("󰍛", "Memory Usage", format!("{}%", mem_pct)))
                .push(item("󰍛", "Swap Usage", format!("{}%", swap_pct)))
                .push(item("", "Temperature", format!("{}°C", self.sys_data.temp.round() as u64)))
                .push(item("💿", "Disk Usage /", format!("{}%", disk_root_pct)))
                .push(item("💿", "Disk Usage /boot", format!("{}%", disk_boot_pct)))
                .push(item("🌐", "IP Address", self.sys_data.ip_address.clone()))
                .push(item("⬇", "Download Speed", format_net(self.sys_data.net_down)))
                .push(item("⬆", "Upload Speed", format_net(self.sys_data.net_up)));
                
            return container(
                container(col)
                    .width(Length::Fixed(360.0))
                    .padding(Padding::from([20, 24]))
                    .style(|_| container::Style {
                        background: Some(iced::Background::Color(Color { a: 0.85, ..Color::from_rgb8(0x11, 0x12, 0x1d) })), // Slightly darker but transparent
                        text_color: Some(Color::from_rgb8(0xc0, 0xca, 0xf5)),
                        border: iced::Border { radius: 12.0.into(), ..Default::default() },
                        ..Default::default()
                    })
            )
            .width(Length::Fill)
            .align_x(iced::alignment::Horizontal::Right)
            .padding(Padding::from([0, 16]))
            .into();
        }

        // Control Center Popup
        let vol_muted = self.audio.muted;
        let vol_row = Row::new().spacing(12).align_y(Alignment::Center)
            .push(
                button(
                    container(text(if vol_muted { "󰝟" } else { "" }).size(18))
                        .width(28)
                        .height(28)
                        .align_x(iced::alignment::Horizontal::Center)
                        .align_y(iced::alignment::Vertical::Center)
                )
                .on_press(Message::ToggleAudioMute)
                .style(move |_, _| if vol_muted {
                    iced::widget::button::Style { text_color: Color::from_rgb8(0x56, 0x5f, 0x89), ..Default::default() }
                } else {
                    iced::widget::button::Style { text_color: Color::WHITE, ..Default::default() }
                })
            )
            .push(
                slider(0..=100, self.volume_level, Message::SetVolume)
                    .width(Length::Fill)
                    .style(move |_, _| if vol_muted {
                        iced::widget::slider::Style {
                            rail: iced::widget::slider::Rail { 
                                backgrounds: (iced::Background::Color(Color::from_rgb8(0x41, 0x48, 0x68)), iced::Background::Color(Color::from_rgb8(0x29, 0x2e, 0x42))), 
                                width: 4.0,
                                border: iced::Border { radius: 2.0.into(), width: 0.0, color: Color::TRANSPARENT } 
                            },
                            handle: iced::widget::slider::Handle { 
                                shape: iced::widget::slider::HandleShape::Circle { radius: 6.0 }, 
                                background: iced::Background::Color(Color::from_rgb8(0x56, 0x5f, 0x89)), 
                                border_width: 0.0, 
                                border_color: Color::TRANSPARENT 
                            },
                            breakpoint: iced::widget::slider::Breakpoint {
                                color: Color::TRANSPARENT,
                            },
                        }
                    } else {
                        iced::widget::slider::Style {
                            rail: iced::widget::slider::Rail { 
                                backgrounds: (iced::Background::Color(Color::from_rgb8(0x7a, 0xa2, 0xf7)), iced::Background::Color(Color::from_rgb8(0x29, 0x2e, 0x42))), 
                                width: 4.0,
                                border: iced::Border { radius: 2.0.into(), width: 0.0, color: Color::TRANSPARENT } 
                            },
                            handle: iced::widget::slider::Handle { 
                                shape: iced::widget::slider::HandleShape::Circle { radius: 8.0 }, 
                                background: iced::Background::Color(Color::from_rgb8(0x7a, 0xa2, 0xf7)), 
                                border_width: 0.0, 
                                border_color: Color::TRANSPARENT 
                            },
                            breakpoint: iced::widget::slider::Breakpoint {
                                color: Color::TRANSPARENT,
                            },
                        }
                    })
            );

        let mic_muted = self.mic.muted;
        let mic_row = Row::new().spacing(12).align_y(Alignment::Center)
            .push(
                button(
                    container(text(if mic_muted || self.mic.volume == 0 { "󰍭" } else { "" }).size(18))
                        .width(28)
                        .height(28)
                        .align_x(iced::alignment::Horizontal::Center)
                        .align_y(iced::alignment::Vertical::Center)
                )
                .on_press(Message::ToggleMicMute)
                .style(move |_, _| if mic_muted {
                    iced::widget::button::Style { text_color: Color::from_rgb8(0x56, 0x5f, 0x89), ..Default::default() }
                } else {
                    iced::widget::button::Style { text_color: Color::WHITE, ..Default::default() }
                })
            )
            .push(
                slider(0..=100, self.mic_volume_level, Message::SetMicVolume)
                    .width(Length::Fill)
                    .style(move |_, _| if mic_muted {
                        iced::widget::slider::Style {
                            rail: iced::widget::slider::Rail { 
                                backgrounds: (iced::Background::Color(Color::from_rgb8(0x41, 0x48, 0x68)), iced::Background::Color(Color::from_rgb8(0x29, 0x2e, 0x42))), 
                                width: 4.0,
                                border: iced::Border { radius: 2.0.into(), width: 0.0, color: Color::TRANSPARENT } 
                            },
                            handle: iced::widget::slider::Handle { 
                                shape: iced::widget::slider::HandleShape::Circle { radius: 6.0 }, 
                                background: iced::Background::Color(Color::from_rgb8(0x56, 0x5f, 0x89)), 
                                border_width: 0.0, 
                                border_color: Color::TRANSPARENT 
                            },
                            breakpoint: iced::widget::slider::Breakpoint {
                                color: Color::TRANSPARENT,
                            },
                        }
                    } else {
                        iced::widget::slider::Style {
                            rail: iced::widget::slider::Rail { 
                                backgrounds: (iced::Background::Color(Color::from_rgb8(0x7a, 0xa2, 0xf7)), iced::Background::Color(Color::from_rgb8(0x29, 0x2e, 0x42))), 
                                width: 4.0,
                                border: iced::Border { radius: 2.0.into(), width: 0.0, color: Color::TRANSPARENT } 
                            },
                            handle: iced::widget::slider::Handle { 
                                shape: iced::widget::slider::HandleShape::Circle { radius: 8.0 }, 
                                background: iced::Background::Color(Color::from_rgb8(0x7a, 0xa2, 0xf7)), 
                                border_width: 0.0, 
                                border_color: Color::TRANSPARENT 
                            },
                            breakpoint: iced::widget::slider::Breakpoint {
                                color: Color::TRANSPARENT,
                            },
                        }
                    })
            );
        let brt_row = Row::new().spacing(12).align_y(Alignment::Center)
            .push(text("󰃠").size(20).width(Length::Fixed(24.0)))
            .push(slider(1..=100, self.brightness_level, Message::SetBrightness).width(Length::Fill));

        let mut fan_row = Row::new().width(Length::Shrink).spacing(4);
        for l in ["1", "2", "3", "4", "5", "6", "7", "auto", "max"].iter() {
            let lvl = if *l == "max" { "full-speed".to_string() } else { l.to_string() };
            let current_level = self.fan.level.trim();
            let is_active = current_level == lvl || (lvl == "full-speed" && current_level == "disengaged");
            
            let btn_width = if *l == "auto" || *l == "max" { Length::Fixed(42.0) } else { Length::Fixed(26.0) };

            let btn = button(text(*l).size(11).align_x(iced::alignment::Horizontal::Center))
                .on_press(Message::SetFanLevel(lvl.clone()))
                .width(btn_width)
                .height(Length::Fixed(26.0))
                .padding(Padding::from([2, 0]))
                .style(move |_, _| if is_active {
                    iced::widget::button::Style {
                        background: Some(iced::Background::Color(Color::from_rgb8(0x7a, 0xa2, 0xf7))),
                        text_color: Color::from_rgb8(0x1a, 0x1b, 0x26),
                        border: iced::Border { radius: 6.0.into(), ..Default::default() },
                        ..Default::default()
                    }
                } else {
                    iced::widget::button::Style {
                        background: Some(iced::Background::Color(Color::from_rgb8(0x41, 0x48, 0x68))),
                        text_color: Color::from_rgb8(0xc0, 0xca, 0xf5),
                        border: iced::Border { radius: 6.0.into(), ..Default::default() },
                        ..Default::default()
                    }
                });
            fan_row = fan_row.push(btn);
        }

        let mut prof_row = Row::new().width(Length::Fill).spacing(8);
        for (vid, label) in [("low-power", "LOW"), ("balanced", "BAL"), ("performance", "HIGH"), ("auto-tlp", "󰒓 AUTO")].iter() {
            let is_active = self.power_profile == *vid;
            let vid_str = vid.to_string();
            let btn = button(text(label.to_string()).size(11).align_x(iced::alignment::Horizontal::Center))
                .width(Length::FillPortion(1))
                .height(Length::Fixed(32.0))
                .on_press(Message::SetPowerProfile(vid_str))
                .style(move |_, _| if is_active {
                    iced::widget::button::Style {
                        background: Some(iced::Background::Color(Color::from_rgb8(0x7a, 0xa2, 0xf7))),
                        text_color: Color::from_rgb8(0x1a, 0x1b, 0x26),
                        border: iced::Border { radius: 8.0.into(), ..Default::default() },
                        ..Default::default()
                    }
                } else {
                    iced::widget::button::Style {
                        background: Some(iced::Background::Color(Color::from_rgb8(0x29, 0x2e, 0x42))),
                        text_color: Color::from_rgb8(0xc0, 0xca, 0xf5),
                        border: iced::Border { radius: 8.0.into(), ..Default::default() },
                        ..Default::default()
                    }
                });
            prof_row = prof_row.push(btn);
        }

        let wifi_is_active = self.wifi.enabled;
        let wifi_label = if wifi_is_active { 
            if self.wifi.ssid.len() > 10 { format!("{}...", self.wifi.ssid.chars().take(8).collect::<String>()) } else { self.wifi.ssid.clone() }
        } else { "Off".to_string() };
        let wifi_btn = button(
            Row::new().spacing(4).align_y(Alignment::Center)
                .push(text(if wifi_is_active { "󰖩" } else { "󰖪" }).size(18))
                .push(text(wifi_label).size(12))
        )
        .width(Length::FillPortion(1))
        .padding(Padding::from([12, 12]))
        .on_press(Message::ToggleWifiMenu)
        .style(move |_, _| if wifi_is_active {
            iced::widget::button::Style {
                background: Some(iced::Background::Color(Color::from_rgb8(0x7a, 0xa2, 0xf7))),
                text_color: Color::from_rgb8(0x1a, 0x1b, 0x26),
                border: iced::Border { radius: 16.0.into(), ..Default::default() },
                ..Default::default()
            }
        } else {
            iced::widget::button::Style {
                background: Some(iced::Background::Color(Color::from_rgb8(0x29, 0x2e, 0x42))),
                text_color: Color::from_rgb8(0xc0, 0xca, 0xf5),
                border: iced::Border { radius: 16.0.into(), ..Default::default() },
                ..Default::default()
            }
        });

        let bt_is_active = self.bluetooth;
        let bt_label = if bt_is_active { "On" } else { "Off" };
        let bt_btn = button(
            Row::new().spacing(4).align_y(Alignment::Center)
                .push(text(if bt_is_active { "󰂯" } else { "󰂲" }).size(18))
                .push(text(bt_label).size(12))
        )
        .width(Length::FillPortion(1))
        .padding(Padding::from([12, 12]))
        .on_press(Message::ToggleBluetooth(!bt_is_active))
        .style(move |_, _| if bt_is_active {
            iced::widget::button::Style {
                background: Some(iced::Background::Color(Color::from_rgb8(0x7a, 0xa2, 0xf7))),
                text_color: Color::from_rgb8(0x1a, 0x1b, 0x26),
                border: iced::Border { radius: 16.0.into(), ..Default::default() },
                ..Default::default()
            }
        } else {
            iced::widget::button::Style {
                background: Some(iced::Background::Color(Color::from_rgb8(0x29, 0x2e, 0x42))),
                text_color: Color::from_rgb8(0xc0, 0xca, 0xf5),
                border: iced::Border { radius: 16.0.into(), ..Default::default() },
                ..Default::default()
            }
        });

        let (bat_cap, bat_status) = &self.battery;
        let bat_icon = if bat_status.contains("Charging") { "󰂄" } else { "󰁹" };

        let circular_btn_style = |_: &Theme, _: iced::widget::button::Status| iced::widget::button::Style {
            background: Some(iced::Background::Color(Color::from_rgb8(0x29, 0x2e, 0x42))),
            text_color: Color::from_rgb8(0xc0, 0xca, 0xf5),
            border: iced::Border { radius: 24.0.into(), ..Default::default() },
            ..Default::default()
        };

        let top_row = Row::new().align_y(Alignment::Center)
            .push(Row::new().spacing(8).align_y(Alignment::Center).push(text(bat_icon).size(16)).push(text(format!("{}%", bat_cap)).size(14)))
            .push(iced::widget::Space::with_width(Length::Fill))
            .push(button(text("󰌾").size(16)).padding(8).on_press(Message::PowerAction(PowerAction::Lock)).style(circular_btn_style))
            .push(button(text("").size(16)).padding(8).on_press(Message::TogglePowerMenu).style(circular_btn_style));

        if self.show_power_menu {
            let power_action_btn = |label: &str, icon: &str, action: PowerAction| {
                button(
                    Row::new().spacing(8).align_y(Alignment::Center)
                        .push(text(icon.to_string()).size(18))
                        .push(text(label.to_string()).size(14))
                )
                .width(Length::Fill)
                .padding(12)
                .on_press(Message::PowerAction(action))
                .style(move |_, _| iced::widget::button::Style {
                    background: Some(iced::Background::Color(Color::TRANSPARENT)),
                    text_color: Color::from_rgb8(0xc0, 0xca, 0xf5),
                    border: iced::Border { radius: 8.0.into(), ..Default::default() },
                    ..Default::default()
                })
            };

            let separator = container(iced::widget::Space::with_height(1))
                .width(Length::Fill)
                .style(|_| container::Style {
                    background: Some(iced::Background::Color(Color::from_rgb8(0x29, 0x2e, 0x42))),
                    ..Default::default()
                });

            let power_col = Column::new().spacing(4)
                .push(Row::new().spacing(12).align_y(Alignment::Center).push(text("Power Menu").size(18).width(Length::Fill)).push(button(text("󰁝 Back").size(14)).on_press(Message::TogglePowerMenu).padding(8)))
                .push(iced::widget::Space::with_height(12))
                .push(power_action_btn("Suspend", "󰒲", PowerAction::Sleep))
                .push(power_action_btn("Hibernate", "󰖕", PowerAction::Hibernate))
                .push(power_action_btn("Reboot", "󰑓", PowerAction::Restart))
                .push(power_action_btn("Shutdown", "", PowerAction::Shutdown))
                .push(iced::widget::Space::with_height(8))
                .push(separator)
                .push(iced::widget::Space::with_height(8))
                .push(power_action_btn("Logout", "󰍃", PowerAction::Logout));

            return container(power_col)
                .padding(24)
                .width(Length::Fixed(440.0))
                .style(|_| container::Style {
                    background: Some(iced::Background::Color(Color::from_rgb8(0x1a, 0x1b, 0x26))),
                    border: iced::Border { radius: 16.0.into(), color: Color::from_rgb8(0x29, 0x2e, 0x42), width: 1.5 },
                    ..Default::default()
                })
                .into();
        }

        let sliders_col = Column::new()
            .spacing(8)
            .push(vol_row)
            .push(brt_row)
            .push(mic_row);

        let mut container_col = Column::new()
            .spacing(20)
            .push(top_row)
            .push(sliders_col)
            .push(Row::new().spacing(16).push(wifi_btn).push(bt_btn));

        if self.show_wifi_menu {
            let mut inner_col = Column::new().spacing(8);
            let toggle_power_btn = button(text(if wifi_is_active { "Отключить Wi-Fi" } else { "Включить Wi-Fi" }).size(14))
                .on_press(Message::ToggleWifi(!wifi_is_active))
                .width(Length::Fill).padding(8)
                .style(|_, _| iced::widget::button::Style {
                    background: Some(iced::Background::Color(Color::from_rgb8(0x41, 0x48, 0x68))),
                    text_color: Color::from_rgb8(0xc0, 0xca, 0xf5),
                    border: iced::Border { radius: 8.0.into(), ..Default::default() },
                    ..Default::default()
                });
            inner_col = inner_col.push(toggle_power_btn);

            if let Some(ref ssid) = self.wifi_selected_ssid {
                let input = text_input("Enter password...", &self.wifi_password_input)
                    .on_input(Message::WifiPasswordChanged)
                    .on_submit(Message::SubmitWifiPassword)
                    .secure(true).padding(10);
                let actions = Row::new().spacing(8)
                    .push(button(text("Connect")).on_press(Message::SubmitWifiPassword).padding(8))
                    .push(button(text("Cancel")).on_press(Message::CancelWifiPassword).padding(8));
                inner_col = inner_col.push(text(format!("Connect to {}", ssid))).push(input).push(actions);
            } else {
                let mut net_list = Column::new().spacing(4);
                for net in &self.available_networks {
                    net_list = net_list.push(button(text(net.ssid.clone())).width(Length::Fill).on_press(Message::SelectWifiNetwork(net.ssid.clone(), net.security.clone())).style(|_, _| iced::widget::button::Style { background: Some(iced::Background::Color(Color::TRANSPARENT)), text_color: Color::from_rgb8(0xc0, 0xca, 0xf5), ..Default::default() }));
                }
                inner_col = inner_col.push(scrollable(net_list).height(Length::Fixed(150.0)));
            }
            container_col = container_col.push(container(inner_col).padding(16).style(|_| container::Style { background: Some(iced::Background::Color(Color::from_rgb8(0x29, 0x2e, 0x42))), border: iced::Border { radius: 12.0.into(), ..Default::default() }, ..Default::default() }));
        }

        container_col = container_col
            .push(
                Column::new().spacing(8)
                    .push(Row::new().spacing(8).align_y(Alignment::Center).push(text("󰒓").size(16)).push(text("Power Profiles (TLP)").size(14)))
                    .push(container(prof_row).width(Length::Fill).align_x(iced::alignment::Horizontal::Center))
            )
            .push(
                Column::new().spacing(8)
                    .push(Row::new().spacing(8).align_y(Alignment::Center).push(text("󰈐").size(16)).push(text(format!("Fan Control: {} RPM", self.fan.speed)).size(14)))
                    .push(container(fan_row).width(Length::Fill).align_x(iced::alignment::Horizontal::Center))
            );

        container(
            container(container_col)
                .padding(24)
                .width(Length::Fixed(360.0))
                .style(|_| container::Style {
                    background: Some(iced::Background::Color(Color { a: 0.85, ..Color::from_rgb8(0x11, 0x12, 0x1d) })),
                    border: iced::Border { radius: 16.0.into(), color: Color::from_rgb8(0x29, 0x2e, 0x42), width: 1.5 },
                    ..Default::default()
                })
        )
        .width(Length::Fill)
        .align_x(iced::alignment::Horizontal::Right)
        .padding(Padding::from([0, 16]))
        .into()
    }

    pub fn theme(&self, _id: Id) -> Theme {
        Theme::Dark
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        iced::Subscription::batch(vec![
            crate::modules::clock::tick(),
            crate::modules::workspaces::tick(),
            iced::time::every(std::time::Duration::from_millis(500))
                .map(|_| Message::UpdateKeyboardLayout),
            crate::modules::tray::Tray::subscription().map(Message::TrayMessage),
        ])
    }
}
