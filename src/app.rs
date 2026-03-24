use chrono::Local;
use iced::{
    widget::{
        button, container, image, mouse_area, scrollable, slider, stack, text, text_input, Column,
        Row, Space,
    },
    window::Id,
    Alignment, Color, Element, Length, Padding, Task, Theme,
};
use std::fmt::Write as _;
use tracing::info;

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
    config: crate::config::Config,
    dbus_conn: Option<zbus::Connection>,
    workspaces: Vec<crate::modules::workspaces::WorkspaceInfo>,
    special_workspace_visible: bool,
    active_window: String,
    clock: String,
    brightness: String,
    audio: crate::modules::audio::AudioInfo,
    mic: crate::modules::mic::MicInfo,
    fan: crate::modules::fan::FanInfo,
    battery: crate::modules::battery::BatteryInfo,
    power_profile: String,
    wifi: crate::modules::wifi::WifiInfo,
    show_wifi_menu: bool,
    show_power_menu: bool,
    keyboard_layout: String,
    available_networks: Vec<crate::modules::wifi::WifiNetwork>,
    wifi_password_input: String,
    wifi_selected_ssid: Option<String>,
    wifi_connecting_ssid: Option<String>,
    wifi_status_message: String,
    bluetooth: bool,
    popup: Popup,
    volume_level: u32,
    mic_volume_level: u32,
    brightness_level: u32,
    battery_str: String,
    audio_str: String,
    main_window_id: Option<Id>,
    popup_window_id: Option<Id>,
    sys_monitor: crate::modules::system::SysMonitor,
    sys_data: crate::modules::system::SysData,
    calendar_offset: i32,
    tray: crate::modules::tray::Tray,
    workspace_refresh_inflight: bool,
    workspace_refresh_queued: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick(chrono::DateTime<chrono::Local>),
    TickBrightness(chrono::DateTime<chrono::Local>),
    TickThermal(chrono::DateTime<chrono::Local>),
    TickSlow(chrono::DateTime<chrono::Local>),
    RefreshAudioMic,
    UpdateWorkspaces,
    WorkspacesUpdated(
        Vec<crate::modules::workspaces::WorkspaceInfo>,
        String,
        String,
        bool,
    ),
    SwitchWorkspace(i32, String),
    TogglePopup(Popup),
    SetVolume(u32),
    SetMicVolume(u32),
    SetFanLevel(String),
    SetBrightness(u32),
    SetPowerProfile(String),
    CyclePerformanceProfile,
    ToggleWifiMenu,
    NetworksScanned(Vec<crate::modules::wifi::WifiNetwork>),
    SelectWifiNetwork(String, String),
    WifiPasswordChanged(String),
    SubmitWifiPassword,
    CancelWifiPassword,
    WifiConnectResult(bool),
    ToggleWifi(bool),
    ToggleBluetooth(bool),
    NextKeyboardLayout,
    TogglePowerMenu,
    PowerAction(PowerAction),
    ToggleAudioMute,
    ToggleMicMute,
    CalendarPrevMonth,
    CalendarNextMonth,
    TrayMessage(crate::modules::tray::TrayMessage),
    TrayItemClicked(String),
    TrayItemRightClicked(String),
    TrayItemClickResolved(String, bool),
    AudioUpdated(crate::modules::audio::AudioInfo),
    MicUpdated(crate::modules::mic::MicInfo),
    BrightnessUpdated(String),
    FanUpdated(crate::modules::fan::FanInfo),
    BatteryUpdated(crate::modules::battery::BatteryInfo),
    PowerProfileUpdated(String),
    WifiUpdated(crate::modules::wifi::WifiInfo),
    BluetoothUpdated(bool),
    OpenOverskride,
    DBusConnected(zbus::Connection),
    PopupWindowUnfocused(Id),
    OpenLauncher,
}

impl ThinkPadBar {
    fn trunc_with_ellipsis(input: &str, max_chars: usize) -> String {
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

    fn popup_size_for(&self, popup: Popup) -> (u32, u32) {
        match popup {
            Popup::None => (1, 1),
            Popup::Calendar => (400, 420),
            Popup::SystemMonitor => (400, 520),
            Popup::ControlCenter => (420, 760),
        }
    }

    fn popup_hide_tasks(&self) -> Vec<Task<Message>> {
        use iced::platform_specific::shell::commands::layer_surface::{
            set_exclusive_zone, set_keyboard_interactivity, set_layer, set_size,
            KeyboardInteractivity, Layer,
        };

        let mut tasks = Vec::new();
        if let Some(pid) = self.popup_window_id {
            tasks.push(set_exclusive_zone(pid, 0));
            tasks.push(set_layer(pid, Layer::Background));
            tasks.push(set_keyboard_interactivity(pid, KeyboardInteractivity::None));
            tasks.push(set_size(pid, Some(1), Some(1)));
        }
        tasks
    }

    fn popup_show_tasks(&self, popup: Popup) -> Vec<Task<Message>> {
        use iced::platform_specific::shell::commands::layer_surface::{
            set_exclusive_zone, set_keyboard_interactivity, set_layer, set_size,
            KeyboardInteractivity, Layer,
        };

        let mut tasks = Vec::new();
        let (width, height) = self.popup_size_for(popup);
        if let Some(pid) = self.popup_window_id {
            tasks.push(set_exclusive_zone(pid, 0));
            tasks.push(set_layer(pid, Layer::Top));
            tasks.push(set_keyboard_interactivity(
                pid,
                KeyboardInteractivity::OnDemand,
            ));
            tasks.push(set_size(pid, Some(width), Some(height)));
        }
        tasks
    }

    fn should_close_popup_on_unfocus(
        popup_window_id: Option<Id>,
        popup: &Popup,
        unfocused_window: Id,
    ) -> bool {
        *popup != Popup::None && popup_window_id.is_some_and(|id| id == unfocused_window)
    }

    fn launcher_command() -> (&'static str, &'static [&'static str]) {
        ("rofi", &["-replace", "-show", "drun"])
    }

    fn is_special_workspace(name: &str) -> bool {
        let lower = name.to_ascii_lowercase();
        lower == "special" || lower.starts_with("special:")
    }

    fn set_brightness_percent_string(&mut self, value: u32) {
        self.brightness.clear();
        let _ = write!(&mut self.brightness, "{}%", value);
    }

    fn set_battery_percent_string(&mut self, value: u8) {
        self.battery_str.clear();
        let _ = write!(&mut self.battery_str, "{}%", value);
    }

    fn set_audio_summary_string(&mut self) {
        self.audio_str.clear();
        if self.audio.muted {
            self.audio_str.push_str("󰝟 ");
        } else {
            self.audio_str.push_str(" ");
        }
        let _ = write!(&mut self.audio_str, "{}%", self.audio.volume);
    }

    fn request_workspace_refresh(&mut self) -> bool {
        if self.workspace_refresh_inflight {
            self.workspace_refresh_queued = true;
            return false;
        }
        self.workspace_refresh_inflight = true;
        true
    }

    fn complete_workspace_refresh(&mut self) -> bool {
        self.workspace_refresh_inflight = false;
        if self.workspace_refresh_queued {
            self.workspace_refresh_queued = false;
            return true;
        }
        false
    }

    fn spawn_command_and_reap(bin: &str, args: &[&str]) {
        if let Ok(mut child) = std::process::Command::new(bin)
            .args(args)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            // Reap the child process to avoid zombies when launcher exits.
            std::mem::drop(std::thread::spawn(move || {
                let _ = child.wait();
            }));
        }
    }

    fn tray_search_candidates(item: &crate::modules::tray::TrayItem, id: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut push_candidate = |raw: &str| {
            let s = raw.trim().to_ascii_lowercase();
            if s.len() >= 2 && !out.iter().any(|e| e == &s) {
                out.push(s);
            }
        };

        if let Some(title) = &item.title {
            push_candidate(title);
        }
        if let Some(icon_name) = &item.icon_name {
            push_candidate(icon_name);
            let icon_base = icon_name
                .rsplit('/')
                .next()
                .unwrap_or(icon_name)
                .trim_end_matches(".svg")
                .trim_end_matches(".png")
                .trim_end_matches(".xpm")
                .trim_end_matches("-symbolic")
                .trim_end_matches("-panel");
            if !icon_base.is_empty() {
                push_candidate(icon_base);
            }
        }

        push_candidate(id);
        for token in id
            .split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|t| t.len() >= 3)
        {
            if !matches!(
                token,
                "statusnotifieritem"
                    | "status"
                    | "notifier"
                    | "item"
                    | "org"
                    | "kde"
                    | "github"
                    | "com"
            ) {
                push_candidate(token);
            }
        }
        out
    }

    pub fn new(config: crate::config::Config) -> impl FnOnce() -> (Self, Task<Message>) {
        let cfg = config.clone();
        move || {
            let main_id = Id::unique();
            let popup_id = Id::unique();

            use iced::platform_specific::shell::commands::layer_surface::{
                get_layer_surface, Anchor, KeyboardInteractivity, Layer,
            };
            use iced::runtime::platform_specific::wayland::layer_surface::{
                IcedMargin, SctkLayerSurfaceSettings,
            };

            let bar_h = cfg.appearance.bar_height;

            let main_task = get_layer_surface(SctkLayerSurfaceSettings {
                id: main_id,
                namespace: "thinkpadbar-main".to_string(),
                size: Some((None, Some(bar_h))),
                layer: Layer::Top,
                keyboard_interactivity: KeyboardInteractivity::None,
                exclusive_zone: bar_h as i32,
                anchor: Anchor::TOP | Anchor::LEFT | Anchor::RIGHT,
                margin: IcedMargin {
                    top: 0,
                    right: 4,
                    bottom: 0,
                    left: 4,
                },
                ..Default::default()
            });

            let popup_task = get_layer_surface(SctkLayerSurfaceSettings {
                id: popup_id,
                namespace: "thinkpadbar-popup".to_string(),
                // Keep popup surface effectively hidden until user opens a popup.
                size: Some((Some(1), Some(1))),
                exclusive_zone: 0,
                layer: Layer::Background,
                keyboard_interactivity: KeyboardInteractivity::None,
                anchor: Anchor::TOP | Anchor::RIGHT,
                margin: IcedMargin {
                    top: bar_h as i32,
                    right: 8,
                    bottom: 0,
                    left: 0,
                },
                ..Default::default()
            });

            let mut monitor = crate::modules::system::SysMonitor::new();
            let initial_sys_data = monitor.update(false);

            // Try to connect to D-Bus synchronously for initialization if possible,
            // or just let it be None and connect later.
            // Since we are in a tokio-enabled FnOnce, we can't easily await here without block_on.
            // But iced's run_with expects a Task.

            (
                Self {
                    config: cfg,
                    dbus_conn: None, // Will be initialized on first tick or via Task
                    clock: Local::now().format("%a %d %b %H:%M").to_string(),
                    workspaces: crate::modules::workspaces::get_workspaces(),
                    special_workspace_visible:
                        crate::modules::workspaces::is_special_workspace_visible(),
                    active_window: crate::modules::workspaces::get_active_window_title(),
                    brightness: crate::modules::brightness::get_brightness(),
                    audio: crate::modules::audio::get_info(),
                    mic: crate::modules::mic::get_info(),
                    fan: crate::modules::fan::get_fan_info(),
                    battery: crate::modules::battery::get_battery_info(),
                    power_profile: crate::modules::power::get_profile(),
                    wifi: crate::modules::wifi::WifiInfo {
                        enabled: false,
                        ssid: "Loading...".to_string(),
                    },
                    bluetooth: crate::modules::bluetooth::get_bluetooth_info(),
                    show_wifi_menu: false,
                    show_power_menu: false,
                    keyboard_layout: crate::modules::keyboard::get_layout(),
                    available_networks: Vec::new(),
                    wifi_password_input: String::new(),
                    wifi_selected_ssid: None,
                    wifi_connecting_ssid: None,
                    wifi_status_message: String::new(),
                    popup: Popup::None,
                    volume_level: 0,
                    mic_volume_level: 0,
                    brightness_level: 50,
                    battery_str: String::new(),
                    audio_str: String::new(),
                    main_window_id: Some(main_id),
                    popup_window_id: Some(popup_id),
                    sys_monitor: monitor,
                    sys_data: initial_sys_data,
                    calendar_offset: 0,
                    tray: crate::modules::tray::Tray::new(),
                    workspace_refresh_inflight: false,
                    workspace_refresh_queued: false,
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
                self.sys_data = self.sys_monitor.update(true);
                return Task::none();
            }
            Message::RefreshAudioMic => {
                return Task::batch(vec![
                    Task::perform(
                        async { crate::modules::audio::get_info() },
                        Message::AudioUpdated,
                    ),
                    Task::perform(
                        async { crate::modules::mic::get_info() },
                        Message::MicUpdated,
                    ),
                ]);
            }
            Message::TickBrightness(_now) => {
                return Task::perform(
                    async { crate::modules::brightness::get_brightness() },
                    Message::BrightnessUpdated,
                );
            }
            Message::TickThermal(_now) => {
                if let Some(temp) = crate::modules::system::read_temperature_celsius() {
                    self.sys_data.temp = temp;
                    self.sys_data.temp_str = format!("{}°C", temp.round() as u64);
                }
                return Task::perform(
                    async { crate::modules::fan::get_fan_info() },
                    Message::FanUpdated,
                );
            }
            Message::TickSlow(_now) => {
                self.sys_data = self.sys_monitor.update(false);

                let mut tasks = vec![
                    Task::perform(
                        async { crate::modules::brightness::get_brightness() },
                        Message::BrightnessUpdated,
                    ),
                    Task::perform(
                        async { crate::modules::battery::get_battery_info() },
                        Message::BatteryUpdated,
                    ),
                    Task::perform(
                        async { crate::modules::power::get_profile() },
                        Message::PowerProfileUpdated,
                    ),
                    Task::perform(
                        async { crate::modules::bluetooth::get_bluetooth_info() },
                        Message::BluetoothUpdated,
                    ),
                ];

                if let Some(conn) = &self.dbus_conn {
                    let conn = conn.clone();
                    let a_path = self.config.network.adapter_path.clone();
                    let s_path = self.config.network.station_path.clone();
                    tasks.push(Task::perform(
                        async move {
                            crate::modules::wifi::get_wifi_info(&conn, &a_path, &s_path).await
                        },
                        Message::WifiUpdated,
                    ));
                } else {
                    tasks.push(Task::perform(
                        async { zbus::Connection::system().await },
                        |res| {
                            match res {
                                Ok(conn) => Message::DBusConnected(conn),
                                Err(_) => Message::TickSlow(chrono::Local::now()), // Retry
                            }
                        },
                    ));
                }

                return Task::batch(tasks);
            }
            Message::DBusConnected(conn) => {
                info!("Successfully connected to system D-Bus");
                self.dbus_conn = Some(conn);
                return Task::perform(async { chrono::Local::now() }, Message::TickSlow);
            }
            Message::PopupWindowUnfocused(window_id) => {
                if Self::should_close_popup_on_unfocus(self.popup_window_id, &self.popup, window_id)
                {
                    self.popup = Popup::None;
                    self.show_wifi_menu = false;
                    self.show_power_menu = false;
                    self.wifi_selected_ssid = None;
                    self.calendar_offset = 0;
                    return Task::batch(self.popup_hide_tasks());
                }
            }
            Message::OpenLauncher => {
                let (bin, args) = Self::launcher_command();
                Self::spawn_command_and_reap(bin, args);
            }
            Message::UpdateWorkspaces => {
                if !self.request_workspace_refresh() {
                    return Task::none();
                }
                return Task::perform(
                    async {
                        let ws = crate::modules::workspaces::get_workspaces();
                        let special_visible =
                            crate::modules::workspaces::is_special_workspace_visible();
                        let title = crate::modules::workspaces::get_active_window_title();
                        let layout = crate::modules::keyboard::get_layout();
                        (ws, title, layout, special_visible)
                    },
                    |(ws, title, layout, special_visible)| {
                        Message::WorkspacesUpdated(ws, title, layout, special_visible)
                    },
                );
            }
            Message::WorkspacesUpdated(workspaces, title, layout, special_visible) => {
                let active_window_changed = self.active_window != title;

                if self.workspaces != workspaces {
                    self.workspaces = workspaces;
                }
                if self.special_workspace_visible != special_visible {
                    self.special_workspace_visible = special_visible;
                }
                if active_window_changed {
                    self.active_window = title;
                }
                if self.keyboard_layout != layout {
                    self.keyboard_layout = layout;
                }

                if self.popup != Popup::None && active_window_changed {
                    self.popup = Popup::None;
                    self.show_wifi_menu = false;
                    self.show_power_menu = false;
                    self.wifi_selected_ssid = None;
                    self.calendar_offset = 0;
                    let mut tasks = self.popup_hide_tasks();
                    if self.complete_workspace_refresh() {
                        tasks.push(Task::perform(async {}, |_| Message::UpdateWorkspaces));
                    }
                    return Task::batch(tasks);
                }

                if self.complete_workspace_refresh() {
                    return Task::perform(async {}, |_| Message::UpdateWorkspaces);
                }
            }
            Message::SwitchWorkspace(w_id, ws_name) => {
                return Task::perform(
                    async move { crate::modules::workspaces::switch_workspace(w_id, &ws_name) },
                    |_| Message::UpdateWorkspaces,
                );
            }
            Message::TogglePopup(target) => {
                let mut tasks = Vec::new();

                // Refresh audio when opening ControlCenter
                if target == Popup::ControlCenter && self.popup != target {
                    tasks.push(Task::perform(
                        async { crate::modules::audio::get_info() },
                        Message::AudioUpdated,
                    ));
                    tasks.push(Task::perform(
                        async { crate::modules::mic::get_info() },
                        Message::MicUpdated,
                    ));
                }

                if self.popup == target {
                    self.popup = Popup::None;
                    if target == Popup::Calendar {
                        self.calendar_offset = 0;
                    }
                    tasks.extend(self.popup_hide_tasks());
                } else {
                    let is_calendar = target == Popup::Calendar;
                    self.popup = target.clone();
                    if is_calendar {
                        self.calendar_offset = 0;
                    }
                    tasks.extend(self.popup_show_tasks(target));
                }
                return Task::batch(tasks);
            }
            Message::SetVolume(val) => {
                self.volume_level = val;
                self.audio.volume = val;
                return Task::perform(
                    async move { crate::modules::audio::set_volume(val) },
                    |_| Message::Tick(chrono::Local::now()),
                );
            }
            Message::ToggleAudioMute => {
                return Task::perform(async { crate::modules::audio::toggle_mute() }, |_| {
                    Message::Tick(chrono::Local::now())
                });
            }
            Message::SetMicVolume(val) => {
                self.mic_volume_level = val;
                self.mic.volume = val;
                return Task::perform(async move { crate::modules::mic::set_volume(val) }, |_| {
                    Message::Tick(chrono::Local::now())
                });
            }
            Message::ToggleMicMute => {
                return Task::perform(async { crate::modules::mic::toggle_mute() }, |_| {
                    Message::Tick(chrono::Local::now())
                });
            }
            Message::SetBrightness(val) => {
                self.brightness_level = val;
                self.set_brightness_percent_string(val);
                return Task::perform(
                    async move { crate::modules::brightness::set_brightness(val) },
                    |_| Message::TickSlow(chrono::Local::now()),
                );
            }
            Message::SetFanLevel(level) => {
                self.fan.level = level.clone();
                return Task::perform(
                    async move { crate::modules::fan::set_fan_level(&level) },
                    |_| Message::TickSlow(chrono::Local::now()),
                );
            }
            Message::SetPowerProfile(prof) => {
                self.power_profile = prof.clone();
                return Task::perform(
                    async move { crate::modules::power::set_profile(&prof).await },
                    |_| Message::TickSlow(chrono::Local::now()),
                );
            }
            Message::CyclePerformanceProfile => {
                self.config.performance.cycle_profile_runtime();
            }
            Message::ToggleWifiMenu => {
                self.show_wifi_menu = !self.show_wifi_menu;
                if self.show_wifi_menu {
                    if let Some(conn) = &self.dbus_conn {
                        self.wifi_status_message = "Сканирование сетей...".to_string();
                        self.available_networks.clear();
                        let conn = conn.clone();
                        let s_path = self.config.network.station_path.clone();
                        return Task::perform(
                            async move { crate::modules::wifi::scan_networks(&conn, &s_path).await },
                            Message::NetworksScanned,
                        );
                    }
                    self.wifi_status_message =
                        "D-Bus недоступен: не удалось открыть system bus".to_string();
                }
            }
            Message::NetworksScanned(networks) => {
                self.available_networks = networks;
                if self.available_networks.is_empty() {
                    self.wifi_status_message =
                        "Сети не найдены или сканирование недоступно".to_string();
                } else {
                    self.wifi_status_message =
                        format!("Найдено сетей: {}", self.available_networks.len());
                }
            }
            Message::SelectWifiNetwork(ssid, sec) => {
                if sec == "open" {
                    if let Some(conn) = &self.dbus_conn {
                        self.wifi_connecting_ssid = Some(ssid.clone());
                        self.wifi_status_message = format!("Подключение к {}...", ssid);
                        let conn = conn.clone();
                        let s_path = self.config.network.station_path.clone();
                        return Task::perform(
                            async move {
                                crate::modules::wifi::connect_network(&conn, &s_path, ssid, None)
                                    .await
                            },
                            Message::WifiConnectResult,
                        );
                    }
                    self.wifi_status_message =
                        "D-Bus недоступен: подключение невозможно".to_string();
                } else {
                    self.wifi_selected_ssid = Some(ssid);
                    self.wifi_password_input = String::new();
                    self.wifi_status_message = "Введите пароль и нажмите Connect".to_string();
                }
            }
            Message::WifiPasswordChanged(val) => {
                self.wifi_password_input = val;
            }
            Message::SubmitWifiPassword => {
                if let (Some(ssid), Some(conn)) = (self.wifi_selected_ssid.clone(), &self.dbus_conn)
                {
                    self.wifi_connecting_ssid = Some(ssid.clone());
                    self.wifi_status_message = format!("Подключение к {}...", ssid);
                    let conn = conn.clone();
                    let pass = self.wifi_password_input.clone();
                    self.wifi_selected_ssid = None;
                    self.show_wifi_menu = false;
                    let s_path = self.config.network.station_path.clone();
                    return Task::perform(
                        async move {
                            crate::modules::wifi::connect_network(&conn, &s_path, ssid, Some(pass))
                                .await
                        },
                        Message::WifiConnectResult,
                    );
                }
                self.wifi_status_message = "D-Bus недоступен: подключение невозможно".to_string();
            }
            Message::CancelWifiPassword => {
                self.wifi_selected_ssid = None;
                self.wifi_status_message = "Подключение отменено".to_string();
            }
            Message::WifiConnectResult(success) => {
                let ssid = self
                    .wifi_connecting_ssid
                    .take()
                    .unwrap_or_else(|| "выбранной сети".to_string());
                if success {
                    self.wifi_status_message = format!("Подключено: {}", ssid);
                } else {
                    self.wifi_status_message = format!("Не удалось подключиться к {}", ssid);
                }
            }
            Message::ToggleWifi(enable) => {
                if let Some(conn) = &self.dbus_conn {
                    let conn = conn.clone();
                    let a_path = self.config.network.adapter_path.clone();
                    let s_path = self.config.network.station_path.clone();
                    self.wifi_status_message = if enable {
                        "Включение Wi-Fi...".to_string()
                    } else {
                        "Отключение Wi-Fi...".to_string()
                    };
                    self.wifi.enabled = enable;
                    return Task::perform(
                        async move {
                            crate::modules::wifi::toggle_wifi(&conn, &a_path, &s_path, enable).await
                        },
                        |_| Message::TickSlow(chrono::Local::now()),
                    );
                }
                self.wifi_status_message = "D-Bus недоступен: переключение невозможно".to_string();
            }
            Message::ToggleBluetooth(enable) => {
                return Task::perform(
                    async move {
                        let _ = crate::modules::bluetooth::toggle_bluetooth(enable);
                    },
                    |_| Message::TickSlow(chrono::Local::now()),
                );
            }
            Message::OpenOverskride => {
                return Task::perform(
                    async {
                        let _ = crate::modules::bluetooth::open_overskride();
                    },
                    |_| Message::UpdateWorkspaces,
                );
            }
            Message::NextKeyboardLayout => {
                return Task::perform(async { crate::modules::keyboard::next_layout() }, |_| {
                    Message::UpdateWorkspaces
                });
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
                let Some(bin) = args.next().map(|s| s.to_string()) else {
                    return Task::none();
                };
                let args_vec: Vec<String> = args.map(|s| s.to_string()).collect();

                self.popup = Popup::None;
                let mut tasks = self.popup_hide_tasks();
                tasks.push(Task::perform(
                    async move {
                        let _ = std::process::Command::new(bin).args(args_vec).spawn();
                    },
                    |_| Message::UpdateWorkspaces,
                ));
                return Task::batch(tasks);
            }
            Message::TrayMessage(msg) => {
                self.tray.update(msg);
            }
            Message::TrayItemClicked(id) => {
                if let Some(item) = self.tray.items.get(&id) {
                    let candidates = Self::tray_search_candidates(item, &id);
                    let id_for_result = id.clone();
                    return Task::perform(
                        async move {
                            for c in candidates {
                                if crate::modules::workspaces::find_and_switch_to_app(c).await {
                                    return true;
                                }
                            }
                            false
                        },
                        move |found| Message::TrayItemClickResolved(id_for_result.clone(), found),
                    );
                }
            }
            Message::TrayItemRightClicked(id) => {
                self.tray
                    .update(crate::modules::tray::TrayMessage::ActivateItemSecondary(id));
                return Task::none();
            }
            Message::TrayItemClickResolved(id, found) => {
                if !found {
                    self.tray
                        .update(crate::modules::tray::TrayMessage::ActivateItem(id));
                }
                return Task::perform(async {}, |_| Message::UpdateWorkspaces);
            }
            Message::AudioUpdated(info) => {
                let audio_changed = self.audio != info;
                self.audio = info;
                self.volume_level = self.audio.volume;
                if audio_changed {
                    self.set_audio_summary_string();
                }
            }
            Message::MicUpdated(info) => {
                self.mic = info.clone();
                self.mic_volume_level = self.mic.volume;
                crate::modules::mic::update_led(info.muted);
            }
            Message::BrightnessUpdated(val) => {
                if let Ok(parsed) = val.trim_end_matches('%').parse::<u32>() {
                    self.brightness_level = parsed.clamp(1, 100);
                }
                if self.brightness != val {
                    self.brightness = val;
                }
            }
            Message::FanUpdated(info) => {
                self.fan = info;
            }
            Message::BatteryUpdated(info) => {
                let battery_changed = self.battery != info;
                self.battery = info;
                if battery_changed {
                    self.set_battery_percent_string(self.battery.capacity);
                }
            }
            Message::PowerProfileUpdated(prof) => {
                self.power_profile = prof;
            }
            Message::WifiUpdated(info) => {
                self.wifi = info;
                if self.wifi.enabled {
                    let ssid = self.wifi.ssid.trim();
                    if ssid.is_empty() || ssid == "Disconnected" || ssid == "Loading..." {
                        self.wifi_status_message = "Wi-Fi включен, сеть не определена".to_string();
                    } else {
                        self.wifi_status_message = format!("Wi-Fi: {}", ssid);
                    }
                } else {
                    self.wifi_status_message = "Wi-Fi выключен".to_string();
                }
            }
            Message::BluetoothUpdated(val) => {
                self.bluetooth = val;
            }
        }
        Task::none()
    }

    pub fn view(&self, id: Id) -> Element<'_, Message, Theme, iced::Renderer> {
        if Some(id) == self.main_window_id {
            self.view_main_bar()
        } else if Some(id) == self.popup_window_id {
            if self.popup == Popup::None {
                iced::widget::Space::new(0, 0).into()
            } else {
                self.view_popup()
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
            let ws_name = ws.name.clone();
            let is_active = ws.active;
            let is_special = Self::is_special_workspace(&ws.name);

            let btn = button(text(ws.name.clone()).size(12))
                .padding(Padding::from([1, 6]))
                .on_press(Message::SwitchWorkspace(ws_id, ws_name))
                .style(move |_, _| {
                    if is_active {
                        let (bg, fg) = if is_special {
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
                            background: Some(iced::Background::Color(Color {
                                a: self.config.appearance.opacity,
                                ..bg
                            })), // Tokyo Night Blue
                            text_color: fg,
                            border: iced::Border {
                                radius: 8.0.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        }
                    } else {
                        let (bg, fg) = if is_special {
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
                            background: Some(iced::Background::Color(Color {
                                a: self.config.appearance.opacity,
                                ..bg
                            })),
                            text_color: fg,
                            border: iced::Border {
                                radius: 8.0.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        }
                    }
                });
            ws_row = ws_row.push(btn);
        }
        let mut tray_row = Row::new().spacing(6).align_y(Alignment::Center);
        for (id, item) in self.tray.items.iter() {
            let id_clone = id.clone();
            let id_right = id.clone();
            if let Some(handle) = &item.icon_handle {
                tray_row = tray_row.push(
                    mouse_area(
                        image(handle.clone())
                            .width(Length::Fixed(16.0))
                            .height(Length::Fixed(16.0)),
                    )
                    .on_press(Message::TrayItemClicked(id_clone))
                    .on_right_press(Message::TrayItemRightClicked(id_right)),
                );
            } else if let Some(name) = &item.icon_name {
                let id_right = id.clone();
                tray_row = tray_row.push(
                    mouse_area(
                        container(text(name.chars().next().unwrap_or('?').to_string()).size(14))
                            .width(Length::Fixed(16.0))
                            .height(Length::Fixed(16.0))
                            .align_x(iced::alignment::Horizontal::Center),
                    )
                    .on_press(Message::TrayItemClicked(id_clone))
                    .on_right_press(Message::TrayItemRightClicked(id_right)),
                );
            } else {
                let id_right = id.clone();
                tray_row = tray_row.push(
                    mouse_area(
                        container(text("?").size(14))
                            .width(Length::Fixed(16.0))
                            .height(Length::Fixed(16.0))
                            .align_x(iced::alignment::Horizontal::Center),
                    )
                    .on_press(Message::TrayItemClicked(id_clone))
                    .on_right_press(Message::TrayItemRightClicked(id_right)),
                );
            }
        }

        let left = Row::new()
            .spacing(12)
            .align_y(Alignment::Center)
            .push(
                button(text("").size(13))
                    .padding(Padding::from([2, 8]))
                    .on_press(Message::OpenLauncher)
                    .style(|_, _| iced::widget::button::Style {
                        background: Some(iced::Background::Color(Color {
                            a: self.config.appearance.opacity,
                            ..Color::from_rgb8(0x29, 0x2e, 0x42)
                        })),
                        text_color: Color::from_rgb8(0xc0, 0xca, 0xf5),
                        border: iced::Border {
                            radius: 10.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
            )
            .push(container(ws_row).width(Length::Fixed(280.0)));

        // Center: Active Window Title
        let center_title = Self::trunc_with_ellipsis(self.active_window.as_str(), 34);
        let center_bg = if self.special_workspace_visible {
            Color::from_rgb8(0x64, 0x2f, 0x37)
        } else {
            Color::from_rgb8(0x29, 0x2e, 0x42)
        };
        let center = container(
            container(
                text(center_title)
                    .size(11)
                    .style(|_| iced::widget::text::Style {
                        color: Some(Color::from_rgb8(0xc0, 0xca, 0xf5)),
                    }),
            )
            .padding(Padding::from([2, 12]))
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(Color {
                    a: self.config.appearance.opacity,
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

        // Right side: Pills
        let pill_bg = Color {
            a: self.config.appearance.opacity,
            ..Color::from_rgb8(0x29, 0x2e, 0x42)
        };
        let pill_fg = Color::from_rgb8(0xc0, 0xca, 0xf5);
        let pill_border_radius = 12.0;

        let cpu_str = &self.sys_data.cpu_str;
        let mem_str = &self.sys_data.mem_str;
        let temp_str = &self.sys_data.temp_str;
        let temp_val = self.sys_data.temp.round() as i32;
        let temp_color = if temp_val >= 80 {
            Color::from_rgb8(0xf7, 0x76, 0x8e) // Red
        } else if temp_val >= 60 {
            Color::from_rgb8(0xe0, 0xaf, 0x68) // Yellow
        } else {
            pill_fg
        };

        // 1. System Pill
        let sys_pill = container(
            Row::new()
                .spacing(4)
                .align_y(Alignment::Center)
                .push(text("󰍹").size(14))
                .push(text(cpu_str).size(14))
                .push(iced::widget::Space::with_width(4))
                .push(text("󰘚").size(14))
                .push(text(mem_str).size(14))
                .push(iced::widget::Space::with_width(4))
                .push(
                    text("")
                        .size(14)
                        .style(move |_| iced::widget::text::Style {
                            color: Some(temp_color),
                        }),
                )
                .push(
                    text(temp_str)
                        .size(14)
                        .style(move |_| iced::widget::text::Style {
                            color: Some(temp_color),
                        }),
                ),
        )
        .padding(Padding::from([4, 12]))
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(pill_bg)),
            text_color: Some(pill_fg),
            border: iced::Border {
                radius: pill_border_radius.into(),
                ..Default::default()
            },
            ..Default::default()
        });

        let mic_icon = if self.mic.muted || self.mic.volume == 0 {
            "󰍭"
        } else {
            ""
        };
        let wifi_icon = if self.wifi.enabled { "󰖩" } else { "󰖪" };
        let bt_icon = if self.bluetooth { "󰂯" } else { "󰂲" };

        let bat_cap = self.battery.capacity;
        let bat_status = &self.battery.status;

        let (bat_icon, bat_color) = if bat_status.contains("Charging") {
            ("󰂄", Color::from_rgb8(0x9e, 0xce, 0x6a)) // Green
        } else if bat_status.contains("Full") || bat_status.contains("Not charging") {
            ("", pill_fg) // Plug icon, default color
        } else {
            // Discharging
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
                Color::from_rgb8(0xf7, 0x76, 0x8e) // Red
            } else if bat_cap <= 20 {
                Color::from_rgb8(0xe0, 0xaf, 0x68) // Yellow
            } else {
                pill_fg
            };
            (icon, color)
        };

        let combined_pill = container(
            Row::new()
                .spacing(6)
                .align_y(Alignment::Center)
                .push(text(wifi_icon).size(14))
                .push(text(bt_icon).size(14))
                .push(
                    text(bat_icon)
                        .size(14)
                        .style(move |_| iced::widget::text::Style {
                            color: Some(bat_color),
                        }),
                )
                .push(text(self.battery_str.as_str()).size(14).style(move |_| {
                    iced::widget::text::Style {
                        color: Some(bat_color),
                    }
                })),
        )
        .padding(Padding::from([4, 12]))
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(pill_bg)),
            text_color: Some(pill_fg),
            border: iced::Border {
                radius: pill_border_radius.into(),
                ..Default::default()
            },
            ..Default::default()
        });

        // 3. Hardware Pill (Brightness, Audio/Mic, and Fan)
        let hw_pill = container(
            Row::new()
                .spacing(10)
                .align_y(Alignment::Center)
                .push(
                    Row::new()
                        .spacing(4)
                        .align_y(Alignment::Center)
                        .push(text("󰃠").size(14))
                        .push(text(self.brightness.as_str()).size(14)),
                )
                .push(
                    Row::new()
                        .spacing(4)
                        .align_y(Alignment::Center)
                        .push(text(mic_icon).size(14))
                        .push(text(self.audio_str.as_str()).size(14)),
                )
                .push(text(format!(" {}", self.fan.speed)).size(14)),
        )
        .padding(Padding::from([4, 12]))
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(pill_bg)),
            text_color: Some(pill_fg),
            border: iced::Border {
                radius: pill_border_radius.into(),
                ..Default::default()
            },
            ..Default::default()
        });

        // 4. Keyboard Layout Pill
        let kbd_pill = container(text(self.keyboard_layout.as_str()).size(14))
            .padding(Padding::from([4, 12]))
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(pill_bg)),
                text_color: Some(pill_fg),
                border: iced::Border {
                    radius: pill_border_radius.into(),
                    ..Default::default()
                },
                ..Default::default()
            });

        // 5. Clock Pill
        let clock_pill = container(text(self.clock.as_str()).size(14))
            .padding(Padding::from([4, 12]))
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(pill_bg)),
                text_color: Some(pill_fg),
                border: iced::Border {
                    radius: pill_border_radius.into(),
                    ..Default::default()
                },
                ..Default::default()
            });

        let right = container(
            Row::new()
                .spacing(4)
                .align_y(Alignment::Center)
                .push(tray_row)
                .push(mouse_area(sys_pill).on_press(Message::TogglePopup(Popup::SystemMonitor)))
                .push(mouse_area(hw_pill).on_press(Message::TogglePopup(Popup::ControlCenter)))
                .push(
                    mouse_area(combined_pill).on_press(Message::TogglePopup(Popup::ControlCenter)),
                )
                .push(mouse_area(kbd_pill).on_press(Message::NextKeyboardLayout))
                .push(mouse_area(clock_pill).on_press(Message::TogglePopup(Popup::Calendar))),
        )
        .width(Length::Shrink);

        let center_overlay = container(center)
            .width(Length::Fixed(340.0))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center);

        container(stack![
            container(
                Row::new()
                    .align_y(Alignment::Center)
                    .push(left)
                    .push(Space::with_width(Length::Fill))
                    .push(right)
            )
            .width(Length::Fill),
            container(center_overlay)
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center)
        ])
        .width(Length::Fill)
        .height(Length::Fixed(self.config.appearance.bar_height as f32))
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

            let Some(display_date) = chrono::Local
                .with_ymd_and_hms(display_year, display_month as u32, 1, 0, 0, 0)
                .single()
            else {
                return container(Space::with_width(Length::Shrink)).into();
            };
            let month_name = display_date.format("%B %Y").to_string();

            let current_day = if display_year == now.year() && display_month as u32 == now.month() {
                Some(now.day())
            } else {
                None
            };

            // Calculate first day of month and days in month
            let weekday_offset = (display_date.weekday().number_from_monday() - 1) as usize;

            let days_in_month = match display_date.month() {
                1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
                4 | 6 | 9 | 11 => 30,
                2 => {
                    if display_date.year() % 4 == 0
                        && (display_date.year() % 100 != 0 || display_date.year() % 400 == 0)
                    {
                        29
                    } else {
                        28
                    }
                }
                _ => 30,
            };

            let title_row = Row::new()
                .spacing(10)
                .align_y(Alignment::Center)
                .push(
                    button(text("󰅀").size(20))
                        .on_press(Message::CalendarPrevMonth)
                        .style(|_, _| button::Style {
                            text_color: Color::from_rgb8(0x7a, 0xa2, 0xf7),
                            ..Default::default()
                        }),
                )
                .push(
                    text(month_name)
                        .size(18)
                        .width(Length::Fill)
                        .align_x(iced::alignment::Horizontal::Center),
                )
                .push(
                    button(text("󰅂").size(20))
                        .on_press(Message::CalendarNextMonth)
                        .style(|_, _| button::Style {
                            text_color: Color::from_rgb8(0x7a, 0xa2, 0xf7),
                            ..Default::default()
                        }),
                );

            let mut header_row = Row::new().spacing(0);
            for day in ["Пн", "Вт", "Ср", "Чт", "Пт", "Сб", "Вс"] {
                header_row = header_row.push(
                    container(text(day).size(12).style(|_| iced::widget::text::Style {
                        color: Some(Color::from_rgb8(0x56, 0x5f, 0x89)),
                    }))
                    .width(Length::Fill)
                    .align_x(iced::alignment::Horizontal::Center),
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
                                background: Some(iced::Background::Color(Color::from_rgb8(
                                    0x7a, 0xa2, 0xf7,
                                ))),
                                text_color: Some(Color::from_rgb8(0x1a, 0x1b, 0x26)),
                                border: iced::Border {
                                    radius: 8.0.into(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            }
                        } else {
                            container::Style::default()
                        }
                    });

                current_row = current_row.push(day_btn);

                if (weekday_offset + (d as usize)).is_multiple_of(7) || d == days_in_month {
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

            let content = Column::new()
                .spacing(16)
                .push(title_row)
                .push(header_row)
                .push(days_col);

            return container(
                container(content)
                    .width(Length::Fill)
                    .padding(24)
                    .style(|_| container::Style {
                        background: Some(iced::Background::Color(Color {
                            a: self.config.appearance.opacity,
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
            .into();
        }

        if self.popup == Popup::SystemMonitor {
            let item = |icon: &str,
                        label: &str,
                        val: String|
             -> Element<'_, Message, Theme, iced::Renderer> {
                Row::new()
                    .spacing(12)
                    .align_y(Alignment::Center)
                    .push(text(icon.to_string()).size(16))
                    .push(text(label.to_string()).size(13).width(Length::Fill))
                    .push(text(val).size(13))
                    .into()
            };

            let col = Column::new()
                .spacing(12)
                .push(
                    Row::new()
                        .align_y(Alignment::Center)
                        .push(text("System Info").size(18).style(move |_| {
                            iced::widget::text::Style {
                                color: Some(Color::from_rgb8(0xc0, 0xca, 0xf5)),
                            }
                        }))
                        .push(Space::with_width(Length::Fill))
                        .push(
                            text(concat!("ver ", env!("CARGO_PKG_VERSION")))
                                .size(10)
                                .style(move |_| iced::widget::text::Style {
                                    color: Some(Color::from_rgb8(0x56, 0x5f, 0x89)),
                                }),
                        ),
                )
                .push(item("", "CPU Usage", self.sys_data.cpu_str.clone()))
                .push(item("󰍛", "Memory Usage", self.sys_data.mem_str.clone()))
                .push(item("󰍛", "Swap Usage", self.sys_data.swap_str.clone()))
                .push(item("", "Temperature", self.sys_data.temp_str.clone()))
                .push(item(
                    "💿",
                    "Disk Usage /",
                    self.sys_data.disk_root_str.clone(),
                ))
                .push(item(
                    "💿",
                    "Disk Usage /boot",
                    self.sys_data.disk_boot_str.clone(),
                ))
                .push(item("🌐", "IP Address", self.sys_data.ip_address.clone()))
                .push(item(
                    "⬇",
                    "Download Speed",
                    self.sys_data.net_down_str.clone(),
                ))
                .push(item("⬆", "Upload Speed", self.sys_data.net_up_str.clone()));

            return container(
                container(col)
                    .width(Length::Fill)
                    .padding(Padding::from([20, 24]))
                    .style(|_| container::Style {
                        background: Some(iced::Background::Color(Color {
                            a: self.config.appearance.opacity,
                            ..Color::from_rgb8(0x11, 0x12, 0x1d)
                        })), // Slightly darker but transparent
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
            .into();
        }

        // Control Center Popup
        let vol_muted = self.audio.muted;
        let vol_row = Row::new()
            .spacing(12)
            .align_y(Alignment::Center)
            .push(
                button(
                    container(text(if vol_muted { "󰝟" } else { "" }).size(18))
                        .width(28)
                        .height(28)
                        .align_x(iced::alignment::Horizontal::Center)
                        .align_y(iced::alignment::Vertical::Center),
                )
                .on_press(Message::ToggleAudioMute)
                .style(move |_, _| {
                    if vol_muted {
                        iced::widget::button::Style {
                            text_color: Color::from_rgb8(0x56, 0x5f, 0x89),
                            ..Default::default()
                        }
                    } else {
                        iced::widget::button::Style {
                            text_color: Color::WHITE,
                            ..Default::default()
                        }
                    }
                }),
            )
            .push(
                slider(0..=100, self.volume_level, Message::SetVolume)
                    .width(Length::Fill)
                    .style(move |_, _| {
                        if vol_muted {
                            iced::widget::slider::Style {
                                rail: iced::widget::slider::Rail {
                                    backgrounds: (
                                        iced::Background::Color(Color::from_rgb8(0x41, 0x48, 0x68)),
                                        iced::Background::Color(Color::from_rgb8(0x29, 0x2e, 0x42)),
                                    ),
                                    width: 4.0,
                                    border: iced::Border {
                                        radius: 2.0.into(),
                                        width: 0.0,
                                        color: Color::TRANSPARENT,
                                    },
                                },
                                handle: iced::widget::slider::Handle {
                                    shape: iced::widget::slider::HandleShape::Circle {
                                        radius: 6.0,
                                    },
                                    background: iced::Background::Color(Color::from_rgb8(
                                        0x56, 0x5f, 0x89,
                                    )),
                                    border_width: 0.0,
                                    border_color: Color::TRANSPARENT,
                                },
                                breakpoint: iced::widget::slider::Breakpoint {
                                    color: Color::TRANSPARENT,
                                },
                            }
                        } else {
                            iced::widget::slider::Style {
                                rail: iced::widget::slider::Rail {
                                    backgrounds: (
                                        iced::Background::Color(Color::from_rgb8(0x7a, 0xa2, 0xf7)),
                                        iced::Background::Color(Color::from_rgb8(0x29, 0x2e, 0x42)),
                                    ),
                                    width: 4.0,
                                    border: iced::Border {
                                        radius: 2.0.into(),
                                        width: 0.0,
                                        color: Color::TRANSPARENT,
                                    },
                                },
                                handle: iced::widget::slider::Handle {
                                    shape: iced::widget::slider::HandleShape::Circle {
                                        radius: 8.0,
                                    },
                                    background: iced::Background::Color(Color::from_rgb8(
                                        0x7a, 0xa2, 0xf7,
                                    )),
                                    border_width: 0.0,
                                    border_color: Color::TRANSPARENT,
                                },
                                breakpoint: iced::widget::slider::Breakpoint {
                                    color: Color::TRANSPARENT,
                                },
                            }
                        }
                    }),
            );

        let mic_muted = self.mic.muted;
        let mic_row = Row::new()
            .spacing(12)
            .align_y(Alignment::Center)
            .push(
                button(
                    container(
                        text(if mic_muted || self.mic.volume == 0 {
                            "󰍭"
                        } else {
                            ""
                        })
                        .size(18),
                    )
                    .width(28)
                    .height(28)
                    .align_x(iced::alignment::Horizontal::Center)
                    .align_y(iced::alignment::Vertical::Center),
                )
                .on_press(Message::ToggleMicMute)
                .style(move |_, _| {
                    if mic_muted {
                        iced::widget::button::Style {
                            text_color: Color::from_rgb8(0x56, 0x5f, 0x89),
                            ..Default::default()
                        }
                    } else {
                        iced::widget::button::Style {
                            text_color: Color::WHITE,
                            ..Default::default()
                        }
                    }
                }),
            )
            .push(
                slider(0..=100, self.mic_volume_level, Message::SetMicVolume)
                    .width(Length::Fill)
                    .style(move |_, _| {
                        if mic_muted {
                            iced::widget::slider::Style {
                                rail: iced::widget::slider::Rail {
                                    backgrounds: (
                                        iced::Background::Color(Color::from_rgb8(0x41, 0x48, 0x68)),
                                        iced::Background::Color(Color::from_rgb8(0x29, 0x2e, 0x42)),
                                    ),
                                    width: 4.0,
                                    border: iced::Border {
                                        radius: 2.0.into(),
                                        width: 0.0,
                                        color: Color::TRANSPARENT,
                                    },
                                },
                                handle: iced::widget::slider::Handle {
                                    shape: iced::widget::slider::HandleShape::Circle {
                                        radius: 6.0,
                                    },
                                    background: iced::Background::Color(Color::from_rgb8(
                                        0x56, 0x5f, 0x89,
                                    )),
                                    border_width: 0.0,
                                    border_color: Color::TRANSPARENT,
                                },
                                breakpoint: iced::widget::slider::Breakpoint {
                                    color: Color::TRANSPARENT,
                                },
                            }
                        } else {
                            iced::widget::slider::Style {
                                rail: iced::widget::slider::Rail {
                                    backgrounds: (
                                        iced::Background::Color(Color::from_rgb8(0x7a, 0xa2, 0xf7)),
                                        iced::Background::Color(Color::from_rgb8(0x29, 0x2e, 0x42)),
                                    ),
                                    width: 4.0,
                                    border: iced::Border {
                                        radius: 2.0.into(),
                                        width: 0.0,
                                        color: Color::TRANSPARENT,
                                    },
                                },
                                handle: iced::widget::slider::Handle {
                                    shape: iced::widget::slider::HandleShape::Circle {
                                        radius: 8.0,
                                    },
                                    background: iced::Background::Color(Color::from_rgb8(
                                        0x7a, 0xa2, 0xf7,
                                    )),
                                    border_width: 0.0,
                                    border_color: Color::TRANSPARENT,
                                },
                                breakpoint: iced::widget::slider::Breakpoint {
                                    color: Color::TRANSPARENT,
                                },
                            }
                        }
                    }),
            );
        let brt_row = Row::new()
            .spacing(12)
            .align_y(Alignment::Center)
            .push(
                button(
                    container(text("󰃠").size(18))
                        .width(28)
                        .height(28)
                        .align_x(iced::alignment::Horizontal::Center)
                        .align_y(iced::alignment::Vertical::Center),
                )
                .on_press(Message::TickSlow(chrono::Local::now()))
                .style(move |_, _| iced::widget::button::Style {
                    text_color: Color::WHITE,
                    ..Default::default()
                }),
            )
            .push(
                slider(1..=100, self.brightness_level, Message::SetBrightness)
                    .width(Length::Fill)
                    .style(move |_, _| iced::widget::slider::Style {
                        rail: iced::widget::slider::Rail {
                            backgrounds: (
                                iced::Background::Color(Color::from_rgb8(0x7a, 0xa2, 0xf7)),
                                iced::Background::Color(Color::from_rgb8(0x29, 0x2e, 0x42)),
                            ),
                            width: 4.0,
                            border: iced::Border {
                                radius: 2.0.into(),
                                width: 0.0,
                                color: Color::TRANSPARENT,
                            },
                        },
                        handle: iced::widget::slider::Handle {
                            shape: iced::widget::slider::HandleShape::Circle { radius: 8.0 },
                            background: iced::Background::Color(Color::from_rgb8(0x7a, 0xa2, 0xf7)),
                            border_width: 0.0,
                            border_color: Color::TRANSPARENT,
                        },
                        breakpoint: iced::widget::slider::Breakpoint {
                            color: Color::TRANSPARENT,
                        },
                    }),
            )
            .push(
                text(self.brightness.as_str())
                    .size(13)
                    .width(Length::Fixed(44.0))
                    .align_x(iced::alignment::Horizontal::Right),
            );

        let mut fan_row = Row::new().width(Length::Shrink).spacing(4);
        for l in ["1", "2", "3", "4", "5", "6", "7", "auto", "max"].iter() {
            let lvl = if *l == "max" {
                "full-speed".to_string()
            } else {
                l.to_string()
            };
            let current_level = self.fan.level.trim();
            let is_active =
                current_level == lvl || (lvl == "full-speed" && current_level == "disengaged");

            let btn_width = if *l == "auto" || *l == "max" {
                Length::Fixed(42.0)
            } else {
                Length::Fixed(26.0)
            };

            let btn = button(
                text(*l)
                    .size(11)
                    .align_x(iced::alignment::Horizontal::Center),
            )
            .on_press(Message::SetFanLevel(lvl.clone()))
            .width(btn_width)
            .height(Length::Fixed(26.0))
            .padding(Padding::from([2, 0]))
            .style(move |_, _| {
                if is_active {
                    iced::widget::button::Style {
                        background: Some(iced::Background::Color(Color::from_rgb8(
                            0x7a, 0xa2, 0xf7,
                        ))),
                        text_color: Color::from_rgb8(0x1a, 0x1b, 0x26),
                        border: iced::Border {
                            radius: 6.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                } else {
                    iced::widget::button::Style {
                        background: Some(iced::Background::Color(Color::from_rgb8(
                            0x41, 0x48, 0x68,
                        ))),
                        text_color: Color::from_rgb8(0xc0, 0xca, 0xf5),
                        border: iced::Border {
                            radius: 6.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                }
            });
            fan_row = fan_row.push(btn);
        }

        let mut prof_row = Row::new().width(Length::Fill).spacing(8);
        for (vid, label) in [
            ("low-power", "LOW"),
            ("balanced", "BAL"),
            ("performance", "HIGH"),
            ("auto-tlp", "󰒓 AUTO"),
        ]
        .iter()
        {
            let is_active = self.power_profile == *vid;
            let vid_str = vid.to_string();
            let btn = button(
                text(label.to_string())
                    .size(11)
                    .align_x(iced::alignment::Horizontal::Center),
            )
            .width(Length::FillPortion(1))
            .height(Length::Fixed(32.0))
            .on_press(Message::SetPowerProfile(vid_str))
            .style(move |_, _| {
                if is_active {
                    iced::widget::button::Style {
                        background: Some(iced::Background::Color(Color::from_rgb8(
                            0x7a, 0xa2, 0xf7,
                        ))),
                        text_color: Color::from_rgb8(0x1a, 0x1b, 0x26),
                        border: iced::Border {
                            radius: 8.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                } else {
                    iced::widget::button::Style {
                        background: Some(iced::Background::Color(Color::from_rgb8(
                            0x29, 0x2e, 0x42,
                        ))),
                        text_color: Color::from_rgb8(0xc0, 0xca, 0xf5),
                        border: iced::Border {
                            radius: 8.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                }
            });
            prof_row = prof_row.push(btn);
        }

        let wifi_is_active = self.wifi.enabled;
        let ssid = self.wifi.ssid.trim();
        let has_real_ssid =
            !ssid.is_empty() && ssid != "Disconnected" && ssid != "Loading..." && ssid != "Unknown";
        let wifi_label = if wifi_is_active {
            if has_real_ssid {
                if ssid.len() > 10 {
                    format!("{}...", ssid.chars().take(8).collect::<String>())
                } else {
                    ssid.to_string()
                }
            } else {
                "On".to_string()
            }
        } else {
            "Off".to_string()
        };
        let wifi_btn = button(
            Row::new()
                .spacing(4)
                .align_y(Alignment::Center)
                .push(text(if wifi_is_active { "󰖩" } else { "󰖪" }).size(18))
                .push(text(wifi_label).size(12)),
        )
        .width(Length::FillPortion(1))
        .padding(Padding::from([12, 12]))
        .on_press(Message::ToggleWifiMenu)
        .style(move |_, _| {
            if wifi_is_active {
                iced::widget::button::Style {
                    background: Some(iced::Background::Color(Color::from_rgb8(0x7a, 0xa2, 0xf7))),
                    text_color: Color::from_rgb8(0x1a, 0x1b, 0x26),
                    border: iced::Border {
                        radius: 16.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            } else {
                iced::widget::button::Style {
                    background: Some(iced::Background::Color(Color::from_rgb8(0x29, 0x2e, 0x42))),
                    text_color: Color::from_rgb8(0xc0, 0xca, 0xf5),
                    border: iced::Border {
                        radius: 16.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            }
        });

        let bt_is_active = self.bluetooth;
        let bt_label = if bt_is_active { "On" } else { "Off" };
        let bt_btn = button(
            Row::new()
                .spacing(4)
                .align_y(Alignment::Center)
                .push(text(if bt_is_active { "󰂯" } else { "󰂲" }).size(18))
                .push(text(bt_label).size(12)),
        )
        .width(Length::FillPortion(1))
        .padding(Padding::from([12, 12]))
        .on_press(Message::ToggleBluetooth(!bt_is_active))
        .style(move |_, _| {
            if bt_is_active {
                iced::widget::button::Style {
                    background: Some(iced::Background::Color(Color::from_rgb8(0x7a, 0xa2, 0xf7))),
                    text_color: Color::from_rgb8(0x1a, 0x1b, 0x26),
                    border: iced::Border {
                        radius: 16.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            } else {
                iced::widget::button::Style {
                    background: Some(iced::Background::Color(Color::from_rgb8(0x29, 0x2e, 0x42))),
                    text_color: Color::from_rgb8(0xc0, 0xca, 0xf5),
                    border: iced::Border {
                        radius: 16.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            }
        });
        let bt_app_btn = button(
            Row::new()
                .spacing(4)
                .align_y(Alignment::Center)
                .push(text("󰳋").size(16))
                .push(text("Overskride").size(11)),
        )
        .width(Length::FillPortion(1))
        .padding(Padding::from([8, 10]))
        .on_press(Message::OpenOverskride)
        .style(|_, _| iced::widget::button::Style {
            background: Some(iced::Background::Color(Color::from_rgb8(0x41, 0x48, 0x68))),
            text_color: Color::from_rgb8(0xc0, 0xca, 0xf5),
            border: iced::Border {
                radius: 12.0.into(),
                ..Default::default()
            },
            ..Default::default()
        });

        let bat_cap = self.battery.capacity;
        let bat_status = &self.battery.status;
        let (bat_icon, bat_color) = if bat_status.contains("Charging") {
            ("󰂄", Color::from_rgb8(0x9e, 0xce, 0x6a))
        } else if bat_status.contains("Full") || bat_status.contains("Not charging") {
            ("", Color::from_rgb8(0xc0, 0xca, 0xf5))
        } else {
            let icon = if bat_cap >= 90 {
                "󰁹"
            } else if bat_cap >= 20 {
                "󰁼"
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
        };

        let circular_btn_style =
            |_: &Theme, _: iced::widget::button::Status| iced::widget::button::Style {
                background: Some(iced::Background::Color(Color::from_rgb8(0x29, 0x2e, 0x42))),
                text_color: Color::from_rgb8(0xc0, 0xca, 0xf5),
                border: iced::Border {
                    radius: 24.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            };
        let (perf_b, perf_t, perf_s) = self.config.performance.effective_intervals();

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
                    .push(text(format!("{}%", bat_cap)).size(14).style(move |_| {
                        iced::widget::text::Style {
                            color: Some(bat_color),
                        }
                    }))
                    .push(iced::widget::Space::with_width(8))
                    .push(
                        text(self.battery.time_remaining.as_deref().unwrap_or(""))
                            .size(12)
                            .style(move |_| iced::widget::text::Style {
                                color: Some(Color::from_rgb8(0x56, 0x5f, 0x89)),
                            }),
                    ),
            )
            .push(iced::widget::Space::with_width(Length::Fill))
            .push(
                button(
                    text(format!(
                        "Perf {} {}/{}/{}",
                        self.config.performance.profile_badge(),
                        perf_b,
                        perf_t,
                        perf_s
                    ))
                    .size(12),
                )
                .padding(8)
                .on_press(Message::CyclePerformanceProfile)
                .style(circular_btn_style),
            )
            .push(
                button(text("󰌾").size(16))
                    .padding(8)
                    .on_press(Message::PowerAction(PowerAction::Lock))
                    .style(circular_btn_style),
            )
            .push(
                button(text("").size(16))
                    .padding(8)
                    .on_press(Message::TogglePowerMenu)
                    .style(circular_btn_style),
            );

        if self.show_power_menu {
            let power_action_btn = |label: &str, icon: &str, action: PowerAction| {
                button(
                    Row::new()
                        .spacing(8)
                        .align_y(Alignment::Center)
                        .push(text(icon.to_string()).size(18))
                        .push(text(label.to_string()).size(14)),
                )
                .width(Length::Fill)
                .padding(12)
                .on_press(Message::PowerAction(action))
                .style(move |_, _| iced::widget::button::Style {
                    background: Some(iced::Background::Color(Color::TRANSPARENT)),
                    text_color: Color::from_rgb8(0xc0, 0xca, 0xf5),
                    border: iced::Border {
                        radius: 8.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
            };

            let separator = container(iced::widget::Space::with_height(1))
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
                    border: iced::Border {
                        radius: 16.0.into(),
                        color: Color::from_rgb8(0x29, 0x2e, 0x42),
                        width: 1.5,
                    },
                    ..Default::default()
                })
                .into();
        }

        let sliders_col = Column::new()
            .spacing(8)
            .push(brt_row)
            .push(vol_row)
            .push(mic_row);

        let mut container_col = Column::new()
            .spacing(20)
            .push(top_row)
            .push(sliders_col)
            .push(Row::new().spacing(16).push(wifi_btn).push(bt_btn))
            .push(Row::new().spacing(16).push(bt_app_btn));

        if self.show_wifi_menu {
            let mut inner_col = Column::new().spacing(8);
            if !self.wifi_status_message.is_empty() {
                inner_col =
                    inner_col.push(text(self.wifi_status_message.as_str()).size(12).style(|_| {
                        iced::widget::text::Style {
                            color: Some(Color::from_rgb8(0x9a, 0xb0, 0xe6)),
                        }
                    }));
            }
            let toggle_power_btn = button(
                text(if wifi_is_active {
                    "Отключить Wi-Fi"
                } else {
                    "Включить Wi-Fi"
                })
                .size(14),
            )
            .on_press(Message::ToggleWifi(!wifi_is_active))
            .width(Length::Fill)
            .padding(8)
            .style(|_, _| iced::widget::button::Style {
                background: Some(iced::Background::Color(Color::from_rgb8(0x41, 0x48, 0x68))),
                text_color: Color::from_rgb8(0xc0, 0xca, 0xf5),
                border: iced::Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            });
            inner_col = inner_col.push(toggle_power_btn);

            if let Some(ref ssid) = self.wifi_selected_ssid {
                let input = text_input("Enter password...", &self.wifi_password_input)
                    .on_input(Message::WifiPasswordChanged)
                    .on_submit(Message::SubmitWifiPassword)
                    .secure(true)
                    .padding(10);
                let actions = Row::new()
                    .spacing(8)
                    .push(
                        button(text("Connect"))
                            .on_press(Message::SubmitWifiPassword)
                            .padding(8),
                    )
                    .push(
                        button(text("Cancel"))
                            .on_press(Message::CancelWifiPassword)
                            .padding(8),
                    );
                inner_col = inner_col
                    .push(text(format!("Connect to {}", ssid)))
                    .push(input)
                    .push(actions);
            } else {
                let mut net_list = Column::new().spacing(4);
                for net in &self.available_networks {
                    net_list = net_list.push(
                        button(text(net.ssid.clone()))
                            .width(Length::Fill)
                            .on_press(Message::SelectWifiNetwork(
                                net.ssid.clone(),
                                net.security.clone(),
                            ))
                            .style(|_, _| iced::widget::button::Style {
                                background: Some(iced::Background::Color(Color::TRANSPARENT)),
                                text_color: Color::from_rgb8(0xc0, 0xca, 0xf5),
                                ..Default::default()
                            }),
                    );
                }
                inner_col = inner_col.push(scrollable(net_list).height(Length::Fixed(150.0)));
            }
            container_col =
                container_col.push(
                    container(inner_col)
                        .padding(16)
                        .style(|_| container::Style {
                            background: Some(iced::Background::Color(Color::from_rgb8(
                                0x29, 0x2e, 0x42,
                            ))),
                            border: iced::Border {
                                radius: 12.0.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        }),
                );
        }

        container_col = container_col
            .push(
                Column::new()
                    .spacing(8)
                    .push(
                        Row::new()
                            .spacing(8)
                            .align_y(Alignment::Center)
                            .push(text("󰒓").size(16))
                            .push(text("Power Profiles (TLP)").size(14)),
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
                            .push(text(format!("Fan Control: {} RPM", self.fan.speed)).size(14)),
                    )
                    .push(
                        container(fan_row)
                            .width(Length::Fill)
                            .align_x(iced::alignment::Horizontal::Center),
                    ),
            );

        container(
            container(container_col)
                .padding(24)
                .width(Length::Fill)
                .style(|_| container::Style {
                    background: Some(iced::Background::Color(Color {
                        a: self.config.appearance.opacity,
                        ..Color::from_rgb8(0x11, 0x12, 0x1d)
                    })),
                    border: iced::Border {
                        radius: 16.0.into(),
                        color: Color::from_rgb8(0x29, 0x2e, 0x42),
                        width: 1.5,
                    },
                    ..Default::default()
                }),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    pub fn theme(&self, _id: Id) -> Theme {
        Theme::Dark
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        let (brightness_secs, thermal_secs, slow_secs) =
            self.config.performance.effective_intervals();

        iced::Subscription::batch(vec![
            crate::modules::clock::tick(),
            iced::time::every(std::time::Duration::from_secs(brightness_secs))
                .map(|_| Message::TickBrightness(chrono::Local::now())),
            iced::time::every(std::time::Duration::from_secs(thermal_secs))
                .map(|_| Message::TickThermal(chrono::Local::now())),
            iced::time::every(std::time::Duration::from_secs(slow_secs))
                .map(|_| Message::TickSlow(chrono::Local::now())),
            crate::modules::workspaces::subscription(),
            crate::modules::tray::Tray::subscription().map(Message::TrayMessage),
            crate::modules::audio::subscription(),
            iced::event::listen_with(|event, _status, window| match event {
                iced::Event::Window(iced::window::Event::Unfocused) => {
                    Some(Message::PopupWindowUnfocused(window))
                }
                _ => None,
            }),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::{Popup, ThinkPadBar};
    use iced::window::Id;

    #[test]
    fn popup_closes_when_popup_window_unfocused() {
        let popup_id = Id::unique();
        assert!(ThinkPadBar::should_close_popup_on_unfocus(
            Some(popup_id),
            &Popup::ControlCenter,
            popup_id,
        ));
    }

    #[test]
    fn popup_does_not_close_on_other_window_unfocus() {
        let popup_id = Id::unique();
        let other_id = Id::unique();
        assert!(!ThinkPadBar::should_close_popup_on_unfocus(
            Some(popup_id),
            &Popup::ControlCenter,
            other_id,
        ));
        assert!(!ThinkPadBar::should_close_popup_on_unfocus(
            Some(popup_id),
            &Popup::None,
            popup_id,
        ));
    }

    #[test]
    fn launcher_command_points_to_rofi_replace_drun() {
        let (bin, args) = ThinkPadBar::launcher_command();
        assert_eq!(bin, "rofi");
        assert_eq!(args, &["-replace", "-show", "drun"]);
    }

    #[test]
    fn special_workspace_name_detection_handles_prefix() {
        assert!(ThinkPadBar::is_special_workspace("special"));
        assert!(ThinkPadBar::is_special_workspace("special:term"));
        assert!(ThinkPadBar::is_special_workspace("SPECIAL:tools"));
        assert!(!ThinkPadBar::is_special_workspace("1"));
        assert!(!ThinkPadBar::is_special_workspace("dev"));
    }

    #[test]
    fn tray_candidate_generation_is_generic_and_normalized() {
        let item = crate::modules::tray::TrayItem {
            _id: "irrelevant".to_string(),
            title: Some("My App".to_string()),
            icon_name: Some("org.example.myapp-panel-symbolic".to_string()),
            icon_handle: None,
        };
        let c = ThinkPadBar::tray_search_candidates(&item, "org.kde.StatusNotifierItem-1234");
        assert!(c.iter().any(|v| v == "my app"));
        assert!(c.iter().any(|v| v == "org.example.myapp-panel-symbolic"));
        assert!(c.iter().any(|v| v == "org.example.myapp"));
        assert!(c.iter().any(|v| v == "1234"));
    }

    #[test]
    fn workspace_refresh_coalesces_while_inflight() {
        let mut bar = ThinkPadBar {
            config: crate::config::Config::default(),
            dbus_conn: None,
            workspaces: Vec::new(),
            special_workspace_visible: false,
            active_window: String::new(),
            clock: String::new(),
            brightness: String::new(),
            audio: crate::modules::audio::AudioInfo {
                volume: 0,
                muted: false,
            },
            mic: crate::modules::mic::MicInfo {
                volume: 0,
                muted: false,
            },
            fan: crate::modules::fan::FanInfo {
                speed: "0".to_string(),
                level: "auto".to_string(),
            },
            battery: crate::modules::battery::BatteryInfo {
                capacity: 0,
                status: "Unknown".to_string(),
                time_remaining: None,
            },
            power_profile: String::new(),
            wifi: crate::modules::wifi::WifiInfo {
                enabled: false,
                ssid: String::new(),
            },
            show_wifi_menu: false,
            show_power_menu: false,
            keyboard_layout: String::new(),
            available_networks: Vec::new(),
            wifi_password_input: String::new(),
            wifi_selected_ssid: None,
            wifi_connecting_ssid: None,
            wifi_status_message: String::new(),
            bluetooth: false,
            popup: Popup::None,
            volume_level: 0,
            mic_volume_level: 0,
            brightness_level: 0,
            battery_str: String::new(),
            audio_str: String::new(),
            main_window_id: None,
            popup_window_id: None,
            sys_monitor: crate::modules::system::SysMonitor::new(),
            sys_data: crate::modules::system::SysData::default(),
            calendar_offset: 0,
            tray: crate::modules::tray::Tray::new(),
            workspace_refresh_inflight: false,
            workspace_refresh_queued: false,
        };

        assert!(bar.request_workspace_refresh());
        assert!(!bar.request_workspace_refresh());
        assert!(bar.workspace_refresh_inflight);
        assert!(bar.workspace_refresh_queued);

        assert!(bar.complete_workspace_refresh());
        assert!(!bar.workspace_refresh_inflight);
        assert!(!bar.workspace_refresh_queued);

        assert!(!bar.complete_workspace_refresh());
    }
}
