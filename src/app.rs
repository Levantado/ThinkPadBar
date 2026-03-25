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
use std::time::Duration;
use tracing::info;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Popup {
    None,
    ControlCenter,
    SystemMonitor,
    Calendar,
    TrayMenu,
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
    clock: String,
    controls: crate::services::controls::ControlsSnapshot,
    network_service: crate::services::network::NetworkService,
    idle_inhibitor_service: crate::services::idle_inhibitor::IdleInhibitorService,
    popup: Popup,
    battery_str: String,
    audio_str: String,
    main_window_id: Option<Id>,
    popup_window_id: Option<Id>,
    calendar_offset: i32,
    compositor_service: crate::services::compositor::CompositorService,
    controls_service: crate::services::controls::ControlsService,
    popup_anchor_service: crate::services::popup_anchor::PopupAnchorService,
    session_service: crate::services::session::SessionService,
    system_info_service: crate::services::system_info::SystemInfoService,
    tray_ui_service: crate::services::tray_ui::TrayUiService,
    controls_coalescing: ControlsCoalescing,
    controls_refresh_coalescing: crate::services::coalescing::RequestCoalescer<
        crate::services::controls::ControlsRefreshKind,
    >,
    slow_tick_coalescing: crate::services::coalescing::ValueCoalescer<()>,
    background_request_coalescing:
        crate::services::coalescing::RequestCoalescer<BackgroundRequestKind>,
    perf: PerfCounters,
    show_debug_overlay: bool,
    debug_ui_enabled: bool,
}

#[derive(Debug, Clone, Default)]
struct PerfCounters {
    workspace_refresh_requested: u64,
    workspace_refresh_coalesced: u64,
    workspace_refresh_completed: u64,
    workspace_refresh_last_ms: u64,
    workspace_refresh_total_ms: u128,
    dbus_connect_attempts: u64,
    dbus_connect_successes: u64,
    dbus_connect_failures: u64,
}

impl PerfCounters {
    fn workspace_refresh_avg_ms(&self) -> u64 {
        if self.workspace_refresh_completed == 0 {
            return 0;
        }
        (self.workspace_refresh_total_ms / self.workspace_refresh_completed as u128) as u64
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick(chrono::DateTime<chrono::Local>),
    TickSlow(chrono::DateTime<chrono::Local>),
    RefreshControls(crate::services::controls::ControlsRefreshKind),
    ControlsRefreshed(
        crate::services::controls::ControlsRefreshKind,
        crate::services::controls::ControlsRefresh,
    ),
    ControlsEvent(crate::services::controls::ControlsEvent),
    ControlsCommandCompleted(crate::services::controls::ControlsFollowUp),
    BackgroundWifiInfoSynced(crate::services::network::WifiInfo),
    RefreshCompositor,
    CompositorEvent(crate::services::compositor::CompositorEvent),
    CompositorRefreshed(crate::services::compositor::RefreshResult),
    SwitchWorkspace(i32, String),
    TogglePopup(Popup),
    SetVolume(u32),
    SetMicVolume(u32),
    SetFanLevel(String),
    SetBrightness(u32),
    FlushCoalescedControl(CoalescedControlKind, u64),
    FlushSlowTick(u64),
    SetPowerProfile(String),
    CyclePerformanceProfile,
    NetworkCommand(crate::services::network::NetworkCommand),
    NetworkEvent(crate::services::network::NetworkEvent),
    ToggleBluetooth(bool),
    ToggleIdleInhibitor,
    NextKeyboardLayout,
    TogglePowerMenu,
    PowerAction(PowerAction),
    ToggleAudioMute,
    ToggleMicMute,
    CalendarPrevMonth,
    CalendarNextMonth,
    TrayEvent(crate::services::tray_ui::TrayRuntimeEvent),
    TrayItemClicked(String),
    TrayItemRightClicked(String),
    TrayItemClickResolved(String, bool),
    TrayMenuItemSelected(i32),
    OpenOverskride,
    SessionCommandCompleted(crate::services::session::SessionFollowUp),
    DBusConnected(zbus::Connection),
    DBusConnectAttempted(Option<zbus::Connection>),
    PopupWindowUnfocused(Id),
    OpenLauncher,
    ToggleDebugOverlay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoalescedControlKind {
    Volume,
    MicVolume,
    Brightness,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum BackgroundRequestKind {
    DbusConnect,
    WifiInfo,
}

#[derive(Debug, Clone, Default)]
struct ControlsCoalescing {
    volume: crate::services::coalescing::ValueCoalescer<u32>,
    mic_volume: crate::services::coalescing::ValueCoalescer<u32>,
    brightness: crate::services::coalescing::ValueCoalescer<u32>,
}

impl ControlsCoalescing {
    fn push(&mut self, kind: CoalescedControlKind, value: u32) -> u64 {
        match kind {
            CoalescedControlKind::Volume => self.volume.push(value),
            CoalescedControlKind::MicVolume => self.mic_volume.push(value),
            CoalescedControlKind::Brightness => self.brightness.push(value),
        }
    }

    fn take_command_if_current(
        &mut self,
        kind: CoalescedControlKind,
        generation: u64,
    ) -> Option<crate::services::controls::ControlsCommand> {
        match kind {
            CoalescedControlKind::Volume => self
                .volume
                .take_if_current(generation)
                .map(crate::services::controls::ControlsCommand::SetVolume),
            CoalescedControlKind::MicVolume => self
                .mic_volume
                .take_if_current(generation)
                .map(crate::services::controls::ControlsCommand::SetMicVolume),
            CoalescedControlKind::Brightness => self
                .brightness
                .take_if_current(generation)
                .map(crate::services::controls::ControlsCommand::SetBrightness),
        }
    }
}

impl ThinkPadBar {
    const CONTROL_COALESCE_DELAY_MS: u64 = 75;
    const SLOW_TICK_COALESCE_DELAY_MS: u64 = 75;

    fn debug_ui_enabled_from_rust_log(raw: Option<&str>) -> bool {
        let Some(raw) = raw else {
            return false;
        };
        for directive in raw.split(',').map(str::trim).filter(|d| !d.is_empty()) {
            if let Some((target, level)) = directive.split_once('=') {
                let level = level.trim().to_ascii_lowercase();
                let is_debug_level = level == "debug" || level == "trace";
                if !is_debug_level {
                    continue;
                }
                let target = target.trim();
                if target.is_empty()
                    || target == "thinkpadbar"
                    || target.starts_with("thinkpadbar::")
                {
                    return true;
                }
            } else {
                let level = directive.to_ascii_lowercase();
                if level == "debug" || level == "trace" {
                    return true;
                }
            }
        }
        false
    }

    fn debug_ui_enabled() -> bool {
        Self::debug_ui_enabled_from_rust_log(std::env::var("RUST_LOG").ok().as_deref())
    }

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

    fn popup_surface_kind(popup: &Popup) -> crate::services::popup_anchor::PopupSurfaceKind {
        match popup {
            Popup::None => crate::services::popup_anchor::PopupSurfaceKind::Hidden,
            Popup::ControlCenter => crate::services::popup_anchor::PopupSurfaceKind::ControlCenter,
            Popup::SystemMonitor => crate::services::popup_anchor::PopupSurfaceKind::SystemMonitor,
            Popup::Calendar => crate::services::popup_anchor::PopupSurfaceKind::Calendar,
            Popup::TrayMenu => crate::services::popup_anchor::PopupSurfaceKind::TrayMenu,
        }
    }

    fn popup_hide_tasks(&self) -> Vec<Task<Message>> {
        use iced::platform_specific::shell::commands::layer_surface::{
            set_anchor, set_exclusive_zone, set_keyboard_interactivity, set_layer, set_margin,
            set_size, KeyboardInteractivity, Layer,
        };

        let mut tasks = Vec::new();
        let plan = self.popup_anchor_service.plan(
            crate::services::popup_anchor::PopupSurfaceKind::Hidden,
            None,
            None,
        );
        if let Some(pid) = self.popup_window_id {
            tasks.push(set_exclusive_zone(pid, 0));
            tasks.push(set_layer(pid, Layer::Background));
            tasks.push(set_anchor(pid, plan.anchor));
            tasks.push(set_margin(
                pid,
                plan.margin.0,
                plan.margin.1,
                plan.margin.2,
                plan.margin.3,
            ));
            tasks.push(set_keyboard_interactivity(pid, KeyboardInteractivity::None));
            tasks.push(set_size(pid, Some(plan.width), Some(plan.height)));
        }
        tasks
    }

    fn popup_show_tasks(&self, popup: Popup) -> Vec<Task<Message>> {
        use iced::platform_specific::shell::commands::layer_surface::{
            set_anchor, set_exclusive_zone, set_keyboard_interactivity, set_layer, set_margin,
            set_size, KeyboardInteractivity, Layer,
        };

        let mut tasks = Vec::new();
        let tray_menu_height = self
            .tray_ui_service
            .open_menu()
            .map(crate::services::tray_menu::OwnedTrayMenu::popup_height);
        let plan = self.popup_anchor_service.plan(
            Self::popup_surface_kind(&popup),
            self.tray_ui_service.menu_cursor(),
            tray_menu_height,
        );
        if let Some(pid) = self.popup_window_id {
            tasks.push(set_exclusive_zone(pid, 0));
            tasks.push(set_layer(pid, Layer::Top));
            tasks.push(set_anchor(pid, plan.anchor));
            tasks.push(set_margin(
                pid,
                plan.margin.0,
                plan.margin.1,
                plan.margin.2,
                plan.margin.3,
            ));
            tasks.push(set_keyboard_interactivity(
                pid,
                KeyboardInteractivity::OnDemand,
            ));
            tasks.push(set_size(pid, Some(plan.width), Some(plan.height)));
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

    fn is_special_workspace(name: &str) -> bool {
        let lower = name.to_ascii_lowercase();
        lower == "special" || lower.starts_with("special:")
    }

    fn battery_runtime_summary(battery: &crate::services::controls::BatteryInfo) -> String {
        match battery.time_remaining.as_deref() {
            Some(time_remaining) => {
                format!(
                    "{}% {} ({time_remaining})",
                    battery.capacity, battery.status
                )
            }
            None => format!("{}% {}", battery.capacity, battery.status),
        }
    }

    fn fan_runtime_summary(fan: &crate::services::controls::FanInfo) -> String {
        format!("{} RPM ({})", fan.speed, fan.level)
    }

    fn set_battery_percent_string(&mut self, value: u8) {
        self.battery_str.clear();
        let _ = write!(&mut self.battery_str, "{}%", value);
    }

    fn set_audio_summary_string(&mut self) {
        self.audio_str.clear();
        if self.controls.audio.muted {
            self.audio_str.push_str("󰝟 ");
        } else {
            self.audio_str.push_str(" ");
        }
        let _ = write!(&mut self.audio_str, "{}%", self.controls.audio.volume);
    }

    fn sync_control_summary_strings(&mut self) {
        self.set_battery_percent_string(self.controls.battery.capacity);
        self.set_audio_summary_string();
    }

    fn spawn_controls_refresh(
        &self,
        kind: crate::services::controls::ControlsRefreshKind,
    ) -> Task<Message> {
        let controls_service = self.controls_service.clone();
        Task::perform(
            async move { controls_service.refresh(kind).await },
            move |refresh| Message::ControlsRefreshed(kind, refresh),
        )
    }

    fn request_controls_refresh(
        &mut self,
        kind: crate::services::controls::ControlsRefreshKind,
    ) -> Task<Message> {
        if !self.controls_refresh_coalescing.request(kind) {
            return Task::none();
        }
        self.spawn_controls_refresh(kind)
    }

    fn execute_controls_command(
        &self,
        command: crate::services::controls::ControlsCommand,
    ) -> Task<Message> {
        let controls_service = self.controls_service.clone();
        Task::perform(
            async move { controls_service.execute(command).await },
            Message::ControlsCommandCompleted,
        )
    }

    fn schedule_coalesced_control_flush(
        kind: CoalescedControlKind,
        generation: u64,
    ) -> Task<Message> {
        Task::perform(
            async move {
                tokio::time::sleep(Duration::from_millis(Self::CONTROL_COALESCE_DELAY_MS)).await;
                (kind, generation)
            },
            |(kind, generation)| Message::FlushCoalescedControl(kind, generation),
        )
    }

    fn schedule_slow_tick_flush(generation: u64) -> Task<Message> {
        Task::perform(
            async move {
                tokio::time::sleep(Duration::from_millis(Self::SLOW_TICK_COALESCE_DELAY_MS)).await;
                generation
            },
            Message::FlushSlowTick,
        )
    }

    fn request_compositor_refresh(&mut self) -> Task<Message> {
        self.perf.workspace_refresh_requested =
            self.perf.workspace_refresh_requested.saturating_add(1);
        if !self.compositor_service.request_refresh() {
            self.perf.workspace_refresh_coalesced =
                self.perf.workspace_refresh_coalesced.saturating_add(1);
            return Task::none();
        }
        let compositor_service = self.compositor_service.clone();
        Task::perform(
            async move { compositor_service.refresh().await },
            Message::CompositorRefreshed,
        )
    }

    fn spawn_dbus_connect() -> Task<Message> {
        Task::perform(
            async { zbus::Connection::system().await.ok() },
            Message::DBusConnectAttempted,
        )
    }

    fn request_dbus_connect(&mut self) -> Task<Message> {
        if !self
            .background_request_coalescing
            .request(BackgroundRequestKind::DbusConnect)
        {
            return Task::none();
        }

        Self::spawn_dbus_connect()
    }

    fn spawn_wifi_info_sync(&self, conn: zbus::Connection) -> Task<Message> {
        let network_service = self.network_service.clone();
        Task::perform(
            async move { network_service.get_wifi_info(&conn).await },
            Message::BackgroundWifiInfoSynced,
        )
    }

    fn request_wifi_info_sync(&mut self) -> Task<Message> {
        let Some(conn) = self.dbus_conn.clone() else {
            return self.request_dbus_connect();
        };

        if !self
            .background_request_coalescing
            .request(BackgroundRequestKind::WifiInfo)
        {
            return Task::none();
        }

        self.spawn_wifi_info_sync(conn)
    }

    fn perform_slow_tick(&mut self) -> Task<Message> {
        self.system_info_service
            .refresh(crate::services::system_info::SystemInfoRefreshKind::Thermal);
        self.system_info_service
            .refresh(crate::services::system_info::SystemInfoRefreshKind::Slow);

        Task::batch(vec![
            self.request_controls_refresh(crate::services::controls::ControlsRefreshKind::Fan),
            self.request_controls_refresh(crate::services::controls::ControlsRefreshKind::Slow),
            self.request_wifi_info_sync(),
        ])
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

            let compositor_service =
                crate::services::compositor::CompositorService::new(&cfg.compositor);
            let controls_service = crate::services::controls::ControlsService::new();
            let controls_snapshot = controls_service.snapshot().clone();
            let network_service = crate::services::network::NetworkService::new(&cfg.network);
            let idle_inhibitor_service =
                crate::services::idle_inhibitor::IdleInhibitorService::new();
            let popup_anchor_service =
                crate::services::popup_anchor::PopupAnchorService::new(cfg.appearance.bar_height);
            let session_service = crate::services::session::SessionService::new();
            let system_info_service = crate::services::system_info::SystemInfoService::new();
            let tray_ui_service = crate::services::tray_ui::TrayUiService::new();

            // Try to connect to D-Bus synchronously for initialization if possible,
            // or just let it be None and connect later.
            // Since we are in a tokio-enabled FnOnce, we can't easily await here without block_on.
            // But iced's run_with expects a Task.

            let mut app = Self {
                config: cfg,
                dbus_conn: None,
                clock: Local::now().format("%a %d %b %H:%M").to_string(),
                controls: controls_snapshot,
                network_service,
                idle_inhibitor_service,
                popup: Popup::None,
                battery_str: String::new(),
                audio_str: String::new(),
                main_window_id: Some(main_id),
                popup_window_id: Some(popup_id),
                calendar_offset: 0,
                compositor_service,
                controls_service,
                popup_anchor_service,
                session_service,
                system_info_service,
                tray_ui_service,
                controls_coalescing: ControlsCoalescing::default(),
                controls_refresh_coalescing: crate::services::coalescing::RequestCoalescer::default(
                ),
                slow_tick_coalescing: crate::services::coalescing::ValueCoalescer::default(),
                background_request_coalescing:
                    crate::services::coalescing::RequestCoalescer::default(),
                perf: PerfCounters::default(),
                show_debug_overlay: false,
                debug_ui_enabled: Self::debug_ui_enabled(),
            };
            app.sync_control_summary_strings();

            (app, Task::batch(vec![main_task, popup_task]))
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
                self.system_info_service
                    .refresh(crate::services::system_info::SystemInfoRefreshKind::Fast);
                return Task::none();
            }
            Message::RefreshControls(kind) => {
                return self.request_controls_refresh(kind);
            }
            Message::ControlsEvent(
                crate::services::controls::ControlsEvent::AudioServerChanged,
            ) => {
                return self.request_controls_refresh(
                    crate::services::controls::ControlsRefreshKind::AudioMic,
                );
            }
            Message::ControlsRefreshed(kind, refresh) => {
                self.controls_service.apply_refresh(refresh);
                self.controls = self.controls_service.snapshot().clone();
                self.sync_control_summary_strings();
                if self.controls_refresh_coalescing.complete(&kind) {
                    return self.spawn_controls_refresh(kind);
                }
            }
            Message::ControlsCommandCompleted(follow_up) => match follow_up {
                crate::services::controls::ControlsFollowUp::Refresh(kind) => {
                    return self.request_controls_refresh(kind);
                }
                crate::services::controls::ControlsFollowUp::RefreshCompositor => {
                    return self.request_compositor_refresh();
                }
            },
            Message::CompositorEvent(
                crate::services::compositor::CompositorEvent::StateChanged,
            ) => {
                return self.request_compositor_refresh();
            }
            Message::RefreshCompositor => {
                return self.request_compositor_refresh();
            }
            Message::TickSlow(_now) => {
                let generation = self.slow_tick_coalescing.push(());
                return Self::schedule_slow_tick_flush(generation);
            }
            Message::DBusConnectAttempted(conn) => {
                self.perf.dbus_connect_attempts = self.perf.dbus_connect_attempts.saturating_add(1);
                if let Some(conn) = conn {
                    self.background_request_coalescing
                        .clear(&BackgroundRequestKind::DbusConnect);
                    return Task::perform(async move { conn }, Message::DBusConnected);
                }

                if self
                    .background_request_coalescing
                    .complete(&BackgroundRequestKind::DbusConnect)
                {
                    return Self::spawn_dbus_connect();
                }

                self.perf.dbus_connect_failures = self.perf.dbus_connect_failures.saturating_add(1);
            }
            Message::DBusConnected(conn) => {
                info!("Successfully connected to system D-Bus");
                self.perf.dbus_connect_successes =
                    self.perf.dbus_connect_successes.saturating_add(1);
                self.dbus_conn = Some(conn);
                return Task::perform(async { chrono::Local::now() }, Message::TickSlow);
            }
            Message::BackgroundWifiInfoSynced(info) => {
                self.network_service
                    .handle_event(crate::services::network::NetworkEvent::WifiInfoSynced(info));
                if self
                    .background_request_coalescing
                    .complete(&BackgroundRequestKind::WifiInfo)
                {
                    return self.request_wifi_info_sync();
                }
            }
            Message::PopupWindowUnfocused(window_id) => {
                if Self::should_close_popup_on_unfocus(self.popup_window_id, &self.popup, window_id)
                {
                    self.popup = Popup::None;
                    self.tray_ui_service.close_transient_ui();
                    self.session_service.close_transient_ui();
                    self.network_service.close_transient_ui();
                    self.calendar_offset = 0;
                    return Task::batch(self.popup_hide_tasks());
                }
            }
            Message::OpenLauncher => {
                let session_service = self.session_service.clone();
                return Task::perform(
                    async move {
                        session_service
                            .execute(crate::services::session::SessionCommand::OpenLauncher)
                            .await
                    },
                    Message::SessionCommandCompleted,
                );
            }
            Message::SessionCommandCompleted(follow_up) => match follow_up {
                crate::services::session::SessionFollowUp::None => {}
                crate::services::session::SessionFollowUp::RefreshCompositor => {
                    return self.request_compositor_refresh();
                }
            },
            Message::ToggleDebugOverlay => {
                if self.debug_ui_enabled {
                    self.show_debug_overlay = !self.show_debug_overlay;
                } else {
                    self.show_debug_overlay = false;
                }
            }
            Message::CompositorRefreshed(refreshed) => {
                self.perf.workspace_refresh_completed =
                    self.perf.workspace_refresh_completed.saturating_add(1);
                self.perf.workspace_refresh_last_ms = refreshed.elapsed_ms;
                self.perf.workspace_refresh_total_ms = self
                    .perf
                    .workspace_refresh_total_ms
                    .saturating_add(refreshed.elapsed_ms as u128);
                let active_window_changed = self.compositor_service.snapshot().active_window
                    != refreshed.snapshot.active_window;
                self.compositor_service.apply_refresh(refreshed);

                if self.popup != Popup::None && active_window_changed {
                    self.popup = Popup::None;
                    self.tray_ui_service.close_transient_ui();
                    self.session_service.close_transient_ui();
                    self.network_service.close_transient_ui();
                    self.calendar_offset = 0;
                    let mut tasks = self.popup_hide_tasks();
                    if self.compositor_service.complete_refresh() {
                        tasks.push(Task::perform(async {}, |_| Message::RefreshCompositor));
                    }
                    return Task::batch(tasks);
                }

                if self.compositor_service.complete_refresh() {
                    return Task::perform(async {}, |_| Message::RefreshCompositor);
                }
            }
            Message::SwitchWorkspace(w_id, ws_name) => {
                let compositor_service = self.compositor_service.clone();
                return Task::perform(
                    async move { compositor_service.switch_workspace(w_id, &ws_name) },
                    |_| Message::RefreshCompositor,
                );
            }
            Message::TogglePopup(target) => {
                let mut tasks = Vec::new();

                // Refresh audio when opening ControlCenter
                if target == Popup::ControlCenter && self.popup != target {
                    tasks.push(self.request_controls_refresh(
                        crate::services::controls::ControlsRefreshKind::AudioMic,
                    ));
                }

                if self.popup == target {
                    self.popup = Popup::None;
                    self.tray_ui_service.close_transient_ui();
                    self.session_service.close_transient_ui();
                    self.network_service.close_transient_ui();
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
                let command = crate::services::controls::ControlsCommand::SetVolume(val);
                self.controls_service.preview_command(&command);
                self.controls = self.controls_service.snapshot().clone();
                self.sync_control_summary_strings();
                let generation = self
                    .controls_coalescing
                    .push(CoalescedControlKind::Volume, val);
                return Self::schedule_coalesced_control_flush(
                    CoalescedControlKind::Volume,
                    generation,
                );
            }
            Message::ToggleAudioMute => {
                return self.execute_controls_command(
                    crate::services::controls::ControlsCommand::ToggleAudioMute,
                );
            }
            Message::SetMicVolume(val) => {
                let command = crate::services::controls::ControlsCommand::SetMicVolume(val);
                self.controls_service.preview_command(&command);
                self.controls = self.controls_service.snapshot().clone();
                let generation = self
                    .controls_coalescing
                    .push(CoalescedControlKind::MicVolume, val);
                return Self::schedule_coalesced_control_flush(
                    CoalescedControlKind::MicVolume,
                    generation,
                );
            }
            Message::ToggleMicMute => {
                return self.execute_controls_command(
                    crate::services::controls::ControlsCommand::ToggleMicMute,
                );
            }
            Message::SetBrightness(val) => {
                let command = crate::services::controls::ControlsCommand::SetBrightness(val);
                self.controls_service.preview_command(&command);
                self.controls = self.controls_service.snapshot().clone();
                let generation = self
                    .controls_coalescing
                    .push(CoalescedControlKind::Brightness, val);
                return Self::schedule_coalesced_control_flush(
                    CoalescedControlKind::Brightness,
                    generation,
                );
            }
            Message::FlushCoalescedControl(kind, generation) => {
                if let Some(command) = self
                    .controls_coalescing
                    .take_command_if_current(kind, generation)
                {
                    return self.execute_controls_command(command);
                }
            }
            Message::FlushSlowTick(generation) => {
                if self
                    .slow_tick_coalescing
                    .take_if_current(generation)
                    .is_some()
                {
                    return self.perform_slow_tick();
                }
            }
            Message::SetFanLevel(level) => {
                let command = crate::services::controls::ControlsCommand::SetFanLevel(level);
                self.controls_service.preview_command(&command);
                self.controls = self.controls_service.snapshot().clone();
                return self.execute_controls_command(command);
            }
            Message::SetPowerProfile(prof) => {
                let command = crate::services::controls::ControlsCommand::SetPowerProfile(prof);
                self.controls_service.preview_command(&command);
                self.controls = self.controls_service.snapshot().clone();
                return self.execute_controls_command(command);
            }
            Message::CyclePerformanceProfile => {
                self.config.performance.cycle_profile_runtime();
            }
            Message::NetworkCommand(command) => {
                match self
                    .network_service
                    .handle_command(command, self.dbus_conn.is_some())
                {
                    crate::services::network::NetworkFollowUp::Scan => {
                        if let Some(conn) = &self.dbus_conn {
                            let conn = conn.clone();
                            let network_service = self.network_service.clone();
                            return Task::perform(
                                async move { network_service.scan_networks(&conn).await },
                                |networks| {
                                    Message::NetworkEvent(
                                        crate::services::network::NetworkEvent::ScanCompleted(
                                            networks,
                                        ),
                                    )
                                },
                            );
                        }
                    }
                    crate::services::network::NetworkFollowUp::Connect { ssid, passphrase } => {
                        if let Some(conn) = &self.dbus_conn {
                            let conn = conn.clone();
                            let network_service = self.network_service.clone();
                            return Task::perform(
                                async move {
                                    let success = network_service
                                        .connect_network(&conn, ssid.clone(), passphrase)
                                        .await;
                                    crate::services::network::NetworkEvent::ConnectCompleted {
                                        ssid,
                                        success,
                                    }
                                },
                                Message::NetworkEvent,
                            );
                        }
                    }
                    crate::services::network::NetworkFollowUp::TogglePower(enable) => {
                        if let Some(conn) = &self.dbus_conn {
                            let conn = conn.clone();
                            let network_service = self.network_service.clone();
                            return Task::perform(
                                async move {
                                    network_service.toggle_wifi(&conn, enable).await;
                                    network_service.get_wifi_info(&conn).await
                                },
                                |info| {
                                    Message::NetworkEvent(
                                        crate::services::network::NetworkEvent::WifiInfoSynced(
                                            info,
                                        ),
                                    )
                                },
                            );
                        }
                    }
                    crate::services::network::NetworkFollowUp::None => {}
                }
            }
            Message::NetworkEvent(event) => {
                self.network_service.handle_event(event);
            }
            Message::ToggleBluetooth(enable) => {
                let command = crate::services::controls::ControlsCommand::ToggleBluetooth(enable);
                self.controls_service.preview_command(&command);
                self.controls = self.controls_service.snapshot().clone();
                return self.execute_controls_command(command);
            }
            Message::ToggleIdleInhibitor => {
                self.idle_inhibitor_service.toggle();
            }
            Message::OpenOverskride => {
                return self.execute_controls_command(
                    crate::services::controls::ControlsCommand::OpenOverskride,
                );
            }
            Message::NextKeyboardLayout => {
                let compositor_service = self.compositor_service.clone();
                return Task::perform(
                    async move { compositor_service.next_keyboard_layout() },
                    |_| Message::RefreshCompositor,
                );
            }
            Message::TogglePowerMenu => {
                self.session_service.toggle_power_menu();
            }
            Message::CalendarPrevMonth => {
                self.calendar_offset -= 1;
            }
            Message::CalendarNextMonth => {
                self.calendar_offset += 1;
            }
            Message::PowerAction(action) => {
                let command = match action {
                    PowerAction::Lock => crate::services::session::SessionCommand::Lock,
                    PowerAction::Sleep => crate::services::session::SessionCommand::Sleep,
                    PowerAction::Hibernate => crate::services::session::SessionCommand::Hibernate,
                    PowerAction::Restart => crate::services::session::SessionCommand::Restart,
                    PowerAction::Shutdown => crate::services::session::SessionCommand::Shutdown,
                    PowerAction::Logout => crate::services::session::SessionCommand::Logout,
                };
                self.popup = Popup::None;
                self.session_service.close_transient_ui();
                let mut tasks = self.popup_hide_tasks();
                let session_service = self.session_service.clone();
                tasks.push(Task::perform(
                    async move { session_service.execute(command).await },
                    Message::SessionCommandCompleted,
                ));
                return Task::batch(tasks);
            }
            Message::TrayEvent(msg) => {
                if self.tray_ui_service.handle_runtime_message(msg) {
                    self.popup = Popup::None;
                    return Task::batch(self.popup_hide_tasks());
                }
            }
            Message::TrayItemClicked(id) => {
                if let Some(crate::services::tray_ui::TrayUiPrimaryAction::ResolveCandidates {
                    candidates,
                    ..
                }) = self.tray_ui_service.handle_primary_click(&id)
                {
                    let id_for_result = id.clone();
                    let compositor_service = self.compositor_service.clone();
                    return Task::perform(
                        async move {
                            for c in candidates {
                                if compositor_service.find_and_switch_to_app(c).await {
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
                match self
                    .tray_ui_service
                    .handle_secondary_click(id.clone(), self.compositor_service.cursor_position())
                {
                    crate::services::tray_ui::TrayUiSecondaryAction::OpenMenu => {
                        self.popup = Popup::TrayMenu;
                        return Task::batch(self.popup_show_tasks(self.popup.clone()));
                    }
                    crate::services::tray_ui::TrayUiSecondaryAction::CloseMenu => {
                        self.popup = Popup::None;
                        return Task::batch(self.popup_hide_tasks());
                    }
                    crate::services::tray_ui::TrayUiSecondaryAction::ActivateSecondary => {
                        return Task::none();
                    }
                }
            }
            Message::TrayItemClickResolved(id, found) => {
                if self.tray_ui_service.handle_click_resolved(id, found) {
                    return Task::perform(async {}, |_| Message::RefreshCompositor);
                }
            }
            Message::TrayMenuItemSelected(menu_item_id) => {
                match self.tray_ui_service.handle_menu_selection(menu_item_id) {
                    crate::services::tray_ui::TrayUiSelectionAction::CloseMenu => {
                        self.popup = Popup::None;
                        return Task::batch(self.popup_hide_tasks());
                    }
                    crate::services::tray_ui::TrayUiSelectionAction::ActivateMenuItem {
                        ..
                    } => {
                        self.popup = Popup::None;
                        return Task::batch(self.popup_hide_tasks());
                    }
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
                iced::widget::Space::new(0, 0).into()
            } else {
                self.view_popup()
            }
        } else {
            Row::new().into()
        }
    }

    fn view_main_bar(&self) -> Element<'_, Message, Theme, iced::Renderer> {
        let compositor = self.compositor_service.snapshot();
        // Build Workspaces widget
        let mut ws_row = Row::new().spacing(6).align_y(Alignment::Center);
        for ws in &compositor.workspaces {
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
        for (id, item) in self.tray_ui_service.items() {
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
            } else {
                let label = item.fallback_label();
                tray_row = tray_row.push(
                    mouse_area(
                        container(text(label).size(14))
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
        let center_title = Self::trunc_with_ellipsis(compositor.active_window.as_str(), 34);
        let center_bg = if compositor.special_workspace_visible {
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

        let sys_data = self.system_info_service.snapshot();
        let cpu_str = &sys_data.cpu_str;
        let mem_str = &sys_data.mem_str;
        let temp_str = &sys_data.temp_str;
        let temp_val = sys_data.temp.round() as i32;
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

        let mic_icon = if self.controls.mic.muted || self.controls.mic.volume == 0 {
            "󰍭"
        } else {
            ""
        };
        let wifi_icon = if self.network_service.snapshot().wifi.enabled {
            "󰖩"
        } else {
            "󰖪"
        };
        let bt_icon = if self.controls.bluetooth_enabled {
            "󰂯"
        } else {
            "󰂲"
        };
        let idle_snapshot = self.idle_inhibitor_service.snapshot();

        let bat_cap = self.controls.battery.capacity;
        let bat_status = &self.controls.battery.status;

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

        let mut combined_pill_row = Row::new()
            .spacing(6)
            .align_y(Alignment::Center)
            .push(text(wifi_icon).size(14))
            .push(text(bt_icon).size(14));
        if idle_snapshot.enabled {
            combined_pill_row = combined_pill_row.push(text("").size(14));
        }
        let combined_pill = container(
            combined_pill_row
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
                        .push(text(self.controls.brightness.label.as_str()).size(14)),
                )
                .push(
                    Row::new()
                        .spacing(4)
                        .align_y(Alignment::Center)
                        .push(text(mic_icon).size(14))
                        .push(text(self.audio_str.as_str()).size(14)),
                )
                .push(text(format!(" {}", self.controls.fan.speed)).size(14)),
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
        let kbd_pill = container(text(compositor.keyboard_layout.as_str()).size(14))
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

        let mut right_row = Row::new()
            .spacing(4)
            .align_y(Alignment::Center)
            .push(tray_row)
            .push(mouse_area(sys_pill).on_press(Message::TogglePopup(Popup::SystemMonitor)))
            .push(mouse_area(hw_pill).on_press(Message::TogglePopup(Popup::ControlCenter)))
            .push(mouse_area(combined_pill).on_press(Message::TogglePopup(Popup::ControlCenter)))
            .push(mouse_area(kbd_pill).on_press(Message::NextKeyboardLayout));

        if self.debug_ui_enabled {
            // Debug toggle pill (shown only in debug mode).
            let dbg_pill = container(text("DBG").size(12))
                .padding(Padding::from([4, 10]))
                .style(move |_| container::Style {
                    background: Some(iced::Background::Color(pill_bg)),
                    text_color: Some(Color::from_rgb8(0x7a, 0xa2, 0xf7)),
                    border: iced::Border {
                        radius: pill_border_radius.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                });
            right_row = right_row.push(mouse_area(dbg_pill).on_press(Message::ToggleDebugOverlay));
        }

        right_row =
            right_row.push(mouse_area(clock_pill).on_press(Message::TogglePopup(Popup::Calendar)));

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
        if self.debug_ui_enabled && self.show_debug_overlay {
            let overlay = container(
                container(
                    text(format!(
                        "ws req:{} coal:{} last:{}ms avg:{}ms dbus ok/fail:{}/{}",
                        self.perf.workspace_refresh_requested,
                        self.perf.workspace_refresh_coalesced,
                        self.perf.workspace_refresh_last_ms,
                        self.perf.workspace_refresh_avg_ms(),
                        self.perf.dbus_connect_successes,
                        self.perf.dbus_connect_failures
                    ))
                    .size(10),
                )
                .padding(Padding::from([2, 8]))
                .style(move |_| container::Style {
                    background: Some(iced::Background::Color(Color {
                        a: self.config.appearance.opacity,
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
            .height(Length::Fixed(self.config.appearance.bar_height as f32))
            .style(|_| container::Style {
                background: Some(iced::Background::Color(Color::TRANSPARENT)),
                ..Default::default()
            })
            .into()
    }

    fn view_popup(&self) -> Element<'_, Message, Theme, iced::Renderer> {
        let compositor = self.compositor_service.snapshot();
        if let Popup::TrayMenu = &self.popup {
            let mut content = Column::new()
                .spacing(6)
                .push(
                    text("Tray Menu")
                        .size(16)
                        .style(|_| iced::widget::text::Style {
                            color: Some(Color::from_rgb8(0xc0, 0xca, 0xf5)),
                        }),
                );
            if let Some(menu) = self.tray_ui_service.open_menu() {
                for node in menu.nodes() {
                    match node {
                        crate::services::tray_menu::OwnedTrayMenuNode::Separator => {
                            content = content.push(iced::widget::horizontal_rule(1));
                        }
                        crate::services::tray_menu::OwnedTrayMenuNode::Action(action) => {
                            let mut label = String::new();
                            for _ in 0..action.depth {
                                label.push_str("  ");
                            }
                            label.push_str(&action.label);
                            if !action.activatable {
                                label.push_str("  ›");
                            }

                            let mut btn = button(text(label).size(13))
                                .width(Length::Fill)
                                .padding(Padding::from([4, 8]));
                            if action.enabled && action.activatable {
                                btn = btn.on_press(Message::TrayMenuItemSelected(action.id));
                            }
                            content = content.push(btn);
                        }
                    }
                }
            }
            return container(
                container(scrollable(content))
                    .width(Length::Fill)
                    .padding(Padding::from([12, 12]))
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

            let mut col =
                Column::new().spacing(12).push(
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
                );
            let sys_data = self.system_info_service.snapshot();
            let controls_diagnostics = self.controls_service.diagnostics();
            let idle_snapshot = self.idle_inhibitor_service.snapshot();
            let tray_diagnostics = self.tray_ui_service.diagnostics();
            col = col
                .push(item("", "CPU Usage", sys_data.cpu_str.clone()))
                .push(item("󰍛", "Memory Usage", sys_data.mem_str.clone()))
                .push(item("󰍛", "Swap Usage", sys_data.swap_str.clone()))
                .push(item("", "Temperature", sys_data.temp_str.clone()))
                .push(item("💿", "Disk Usage /", sys_data.disk_root_str.clone()))
                .push(item(
                    "💿",
                    "Disk Usage /boot",
                    sys_data.disk_boot_str.clone(),
                ))
                .push(item("🌐", "IP Address", sys_data.ip_address.clone()))
                .push(item("⬇", "Download Speed", sys_data.net_down_str.clone()))
                .push(item("⬆", "Upload Speed", sys_data.net_up_str.clone()))
                .push(Space::with_height(Length::Fixed(8.0)))
                .push(text("ThinkPad Hardware").size(14).style(move |_| {
                    iced::widget::text::Style {
                        color: Some(Color::from_rgb8(0x9e, 0xce, 0x6a)),
                    }
                }))
                .push(item(
                    "󰁹",
                    "Battery Runtime",
                    Self::battery_runtime_summary(&self.controls.battery),
                ))
                .push(item(
                    "󰾆",
                    "Power Profile",
                    self.controls.power_profile.clone(),
                ))
                .push(item(
                    "󰈐",
                    "Fan Runtime",
                    Self::fan_runtime_summary(&self.controls.fan),
                ))
                .push(item(
                    "",
                    "Idle Inhibitor",
                    idle_snapshot.label().to_string(),
                ));

            if self.debug_ui_enabled {
                col = col
                    .push(Space::with_height(Length::Fixed(8.0)))
                    .push(text("Observability").size(14).style(move |_| {
                        iced::widget::text::Style {
                            color: Some(Color::from_rgb8(0x7a, 0xa2, 0xf7)),
                        }
                    }))
                    .push(item(
                        "⏱",
                        "Workspace Refresh (last/avg ms)",
                        format!(
                            "{}/{}",
                            self.perf.workspace_refresh_last_ms,
                            self.perf.workspace_refresh_avg_ms()
                        ),
                    ))
                    .push(item(
                        "🔁",
                        "Workspace Refresh (req/coalesced)",
                        format!(
                            "{}/{}",
                            self.perf.workspace_refresh_requested,
                            self.perf.workspace_refresh_coalesced
                        ),
                    ))
                    .push(item(
                        "🚌",
                        "D-Bus Connect (ok/fail)",
                        format!(
                            "{}/{}",
                            self.perf.dbus_connect_successes, self.perf.dbus_connect_failures
                        ),
                    ))
                    .push(item("🧩", "Built-in Modules", {
                        let modules = crate::modules::capabilities::built_in_modules();
                        let modules_count = modules.len();
                        let capability_links: usize =
                            modules.iter().map(|m| m.capabilities.len()).sum();
                        let names_total_len: usize = modules.iter().map(|m| m.name.len()).sum();
                        format!(
                            "{} modules / {} caps / name-bytes {}",
                            modules_count, capability_links, names_total_len
                        )
                    }))
                    .push(item(
                        "🛠",
                        "Runtime Contract",
                        format!(
                            "{} events:{} cmds:{} impl:{}",
                            crate::modules::runtime::contract_version(),
                            crate::modules::runtime::canonical_events().len(),
                            crate::modules::runtime::canonical_commands().len(),
                            crate::modules::runtime::noop_runtime_descriptor_name()
                        ),
                    ))
                    .push(item(
                        "🧭",
                        "Service Backends",
                        format!(
                            "cmp {:?}->{:?} net {:?}->{:?}",
                            compositor.configured_backend,
                            compositor.active_backend,
                            self.network_service.configured_backend(),
                            self.network_service.active_backend()
                        ),
                    ))
                    .push(item(
                        "🎛",
                        "Controls Backends",
                        controls_diagnostics.summary(),
                    ))
                    .push(item(
                        "🔊",
                        "Audio Runtime",
                        controls_diagnostics
                            .audio_runtime
                            .clone()
                            .unwrap_or_else(|| "n/a".to_string()),
                    ))
                    .push(item(
                        "☕",
                        "Idle Inhibitor Runtime",
                        idle_snapshot.debug_summary(),
                    ))
                    .push(item("🖼", "Tray Icons", tray_diagnostics.summary()));

                if let Some(last_unresolved) = tray_diagnostics.last_unresolved_item {
                    col = col.push(item("⚠", "Tray Icon Last Unresolved", last_unresolved));
                }
            }

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
        let vol_muted = self.controls.audio.muted;
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
                slider(0..=100, self.controls.audio.volume, Message::SetVolume)
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

        let mic_muted = self.controls.mic.muted;
        let mic_row = Row::new()
            .spacing(12)
            .align_y(Alignment::Center)
            .push(
                button(
                    container(
                        text(if mic_muted || self.controls.mic.volume == 0 {
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
                slider(0..=100, self.controls.mic.volume, Message::SetMicVolume)
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
                .on_press(Message::RefreshControls(
                    crate::services::controls::ControlsRefreshKind::Brightness,
                ))
                .style(move |_, _| iced::widget::button::Style {
                    text_color: Color::WHITE,
                    ..Default::default()
                }),
            )
            .push(
                slider(
                    1..=100,
                    self.controls.brightness.percent.clamp(1, 100),
                    Message::SetBrightness,
                )
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
                text(self.controls.brightness.label.as_str())
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
            let current_level = self.controls.fan.level.trim();
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
            let is_active = self.controls.power_profile == *vid;
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

        let wifi_snapshot = self.network_service.snapshot();
        let wifi_is_active = wifi_snapshot.wifi.enabled;
        let ssid = wifi_snapshot.wifi.ssid.trim();
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
        .on_press(Message::NetworkCommand(
            crate::services::network::NetworkCommand::ToggleMenu,
        ))
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

        let bt_is_active = self.controls.bluetooth_enabled;
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
        let idle_snapshot = self.idle_inhibitor_service.snapshot();
        let idle_btn = {
            let mut btn = button(
                Row::new()
                    .spacing(4)
                    .align_y(Alignment::Center)
                    .push(text("").size(18))
                    .push(text(idle_snapshot.label()).size(12)),
            )
            .width(Length::FillPortion(1))
            .padding(Padding::from([12, 12]))
            .style(move |_, _| {
                if idle_snapshot.enabled {
                    iced::widget::button::Style {
                        background: Some(iced::Background::Color(Color::from_rgb8(
                            0x7a, 0xa2, 0xf7,
                        ))),
                        text_color: Color::from_rgb8(0x1a, 0x1b, 0x26),
                        border: iced::Border {
                            radius: 16.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                } else if idle_snapshot.available {
                    iced::widget::button::Style {
                        background: Some(iced::Background::Color(Color::from_rgb8(
                            0x29, 0x2e, 0x42,
                        ))),
                        text_color: Color::from_rgb8(0xc0, 0xca, 0xf5),
                        border: iced::Border {
                            radius: 16.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                } else {
                    iced::widget::button::Style {
                        background: Some(iced::Background::Color(Color::from_rgb8(
                            0x23, 0x27, 0x38,
                        ))),
                        text_color: Color::from_rgb8(0x56, 0x5f, 0x89),
                        border: iced::Border {
                            radius: 16.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                }
            });
            if idle_snapshot.available {
                btn = btn.on_press(Message::ToggleIdleInhibitor);
            }
            btn
        };
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

        let bat_cap = self.controls.battery.capacity;
        let bat_status = &self.controls.battery.status;
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
                        text(
                            self.controls
                                .battery
                                .time_remaining
                                .as_deref()
                                .unwrap_or(""),
                        )
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

        if self.session_service.snapshot().power_menu_open {
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
            .push(
                Row::new()
                    .spacing(16)
                    .push(wifi_btn)
                    .push(bt_btn)
                    .push(idle_btn),
            )
            .push(Row::new().spacing(16).push(bt_app_btn));

        if wifi_snapshot.menu_open {
            let mut inner_col = Column::new().spacing(8);
            if let Some(status_message) = wifi_snapshot.status_message() {
                inner_col = inner_col.push(text(status_message).size(12).style(|_| {
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
            .on_press(Message::NetworkCommand(
                crate::services::network::NetworkCommand::ToggleWifi(!wifi_is_active),
            ))
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

            if let Some(ssid) = wifi_snapshot.awaiting_password_ssid() {
                let input = text_input("Enter password...", &wifi_snapshot.password_input)
                    .on_input(|value| {
                        Message::NetworkCommand(
                            crate::services::network::NetworkCommand::UpdatePassword(value),
                        )
                    })
                    .on_submit(Message::NetworkCommand(
                        crate::services::network::NetworkCommand::SubmitPassword,
                    ))
                    .secure(true)
                    .padding(10);
                let actions = Row::new()
                    .spacing(8)
                    .push(
                        button(text("Connect"))
                            .on_press(Message::NetworkCommand(
                                crate::services::network::NetworkCommand::SubmitPassword,
                            ))
                            .padding(8),
                    )
                    .push(
                        button(text("Cancel"))
                            .on_press(Message::NetworkCommand(
                                crate::services::network::NetworkCommand::CancelPassword,
                            ))
                            .padding(8),
                    );
                inner_col = inner_col
                    .push(text(format!("Connect to {}", ssid)))
                    .push(input)
                    .push(actions);
            } else {
                let mut net_list = Column::new().spacing(4);
                for net in &wifi_snapshot.available_networks {
                    net_list = net_list.push(
                        button(text(net.ssid.clone()))
                            .width(Length::Fill)
                            .on_press(Message::NetworkCommand(
                                crate::services::network::NetworkCommand::SelectNetwork {
                                    ssid: net.ssid.clone(),
                                    security: net.security.clone(),
                                },
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
                            .push(
                                text(format!("Fan Control: {} RPM", self.controls.fan.speed))
                                    .size(14),
                            ),
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
            iced::time::every(std::time::Duration::from_secs(brightness_secs)).map(|_| {
                Message::RefreshControls(crate::services::controls::ControlsRefreshKind::Brightness)
            }),
            iced::time::every(std::time::Duration::from_secs(thermal_secs)).map(|_| {
                Message::RefreshControls(crate::services::controls::ControlsRefreshKind::Fan)
            }),
            iced::time::every(std::time::Duration::from_secs(slow_secs))
                .map(|_| Message::TickSlow(chrono::Local::now())),
            self.compositor_service
                .subscription()
                .map(Message::CompositorEvent),
            crate::services::tray_ui::TrayUiService::subscription().map(Message::TrayEvent),
            self.controls_service
                .subscription()
                .map(Message::ControlsEvent),
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
    use super::{
        BackgroundRequestKind, CoalescedControlKind, ControlsCoalescing, Popup, ThinkPadBar,
    };
    use iced::window::Id;

    fn hermetic_bar() -> ThinkPadBar {
        ThinkPadBar {
            config: crate::config::Config::default(),
            dbus_conn: None,
            clock: String::new(),
            controls: crate::services::controls::ControlsSnapshot::default(),
            network_service: crate::services::network::NetworkService::new(
                &crate::config::NetworkConfig::default(),
            ),
            idle_inhibitor_service:
                crate::services::idle_inhibitor::IdleInhibitorService::unavailable_for_tests(),
            popup: Popup::None,
            battery_str: String::new(),
            audio_str: String::new(),
            main_window_id: None,
            popup_window_id: None,
            calendar_offset: 0,
            compositor_service: crate::services::compositor::CompositorService::hermetic_for_tests(
                crate::services::compositor::CompositorBackendKind::Hyprland,
                crate::services::compositor::CompositorBackendKind::Hyprland,
            ),
            controls_service: crate::services::controls::ControlsService::with_snapshot_for_tests(
                crate::services::controls::ControlsSnapshot::default(),
            ),
            popup_anchor_service: crate::services::popup_anchor::PopupAnchorService::new(24),
            session_service: crate::services::session::SessionService::default(),
            system_info_service: crate::services::system_info::SystemInfoService::with_snapshot(
                crate::modules::system::SysData::default(),
            ),
            tray_ui_service: crate::services::tray_ui::TrayUiService::new(),
            controls_coalescing: ControlsCoalescing::default(),
            controls_refresh_coalescing: crate::services::coalescing::RequestCoalescer::default(),
            slow_tick_coalescing: crate::services::coalescing::ValueCoalescer::default(),
            background_request_coalescing: crate::services::coalescing::RequestCoalescer::default(),
            perf: super::PerfCounters::default(),
            show_debug_overlay: false,
            debug_ui_enabled: false,
        }
    }

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
    fn special_workspace_name_detection_handles_prefix() {
        assert!(ThinkPadBar::is_special_workspace("special"));
        assert!(ThinkPadBar::is_special_workspace("special:term"));
        assert!(ThinkPadBar::is_special_workspace("SPECIAL:tools"));
        assert!(!ThinkPadBar::is_special_workspace("1"));
        assert!(!ThinkPadBar::is_special_workspace("dev"));
    }

    #[test]
    fn battery_runtime_summary_includes_time_when_present() {
        assert_eq!(
            ThinkPadBar::battery_runtime_summary(&crate::services::controls::BatteryInfo {
                capacity: 64,
                status: "Discharging".to_string(),
                time_remaining: Some("2h 6m remaining".to_string()),
            }),
            "64% Discharging (2h 6m remaining)"
        );
        assert_eq!(
            ThinkPadBar::battery_runtime_summary(&crate::services::controls::BatteryInfo {
                capacity: 100,
                status: "Full".to_string(),
                time_remaining: None,
            }),
            "100% Full"
        );
    }

    #[test]
    fn fan_runtime_summary_formats_speed_and_level() {
        assert_eq!(
            ThinkPadBar::fan_runtime_summary(&crate::services::controls::FanInfo {
                speed: "2700".to_string(),
                level: "auto".to_string(),
            }),
            "2700 RPM (auto)"
        );
    }

    #[test]
    fn workspace_refresh_coalesces_while_inflight() {
        let mut bar = hermetic_bar();

        assert!(bar.compositor_service.request_refresh());
        assert!(!bar.compositor_service.request_refresh());
        assert!(bar.compositor_service.complete_refresh());
        assert!(!bar.compositor_service.complete_refresh());
    }

    #[test]
    fn debug_ui_enabled_detects_only_debug_or_trace_levels() {
        assert!(!ThinkPadBar::debug_ui_enabled_from_rust_log(None));
        assert!(!ThinkPadBar::debug_ui_enabled_from_rust_log(Some("info")));
        assert!(!ThinkPadBar::debug_ui_enabled_from_rust_log(Some(
            "thinkpadbar=info"
        )));
        assert!(ThinkPadBar::debug_ui_enabled_from_rust_log(Some("debug")));
        assert!(ThinkPadBar::debug_ui_enabled_from_rust_log(Some("trace")));
        assert!(ThinkPadBar::debug_ui_enabled_from_rust_log(Some(
            "thinkpadbar=debug"
        )));
        assert!(ThinkPadBar::debug_ui_enabled_from_rust_log(Some(
            "thinkpadbar::modules::tray=trace"
        )));
        assert!(!ThinkPadBar::debug_ui_enabled_from_rust_log(Some(
            "hyper=debug,thinkpadbar=info"
        )));
    }

    #[test]
    fn control_coalescing_keeps_only_latest_generation_per_kind() {
        let mut coalescing = ControlsCoalescing::default();
        let first = coalescing.push(CoalescedControlKind::Volume, 15);
        let second = coalescing.push(CoalescedControlKind::Volume, 42);

        assert_eq!(
            coalescing.take_command_if_current(CoalescedControlKind::Volume, first),
            None
        );
        assert_eq!(
            coalescing.take_command_if_current(CoalescedControlKind::Volume, second),
            Some(crate::services::controls::ControlsCommand::SetVolume(42))
        );
    }

    #[test]
    fn control_coalescing_tracks_each_kind_independently() {
        let mut coalescing = ControlsCoalescing::default();
        let volume = coalescing.push(CoalescedControlKind::Volume, 33);
        let brightness = coalescing.push(CoalescedControlKind::Brightness, 77);

        assert_eq!(
            coalescing.take_command_if_current(CoalescedControlKind::Brightness, brightness),
            Some(crate::services::controls::ControlsCommand::SetBrightness(
                77
            ))
        );
        assert_eq!(
            coalescing.take_command_if_current(CoalescedControlKind::Volume, volume),
            Some(crate::services::controls::ControlsCommand::SetVolume(33))
        );
    }

    #[test]
    fn controls_refresh_coalesces_same_kind_until_completion() {
        let kind = crate::services::controls::ControlsRefreshKind::Brightness;
        let mut bar = hermetic_bar();

        let _ = bar.update(super::Message::RefreshControls(kind));
        assert!(bar.controls_refresh_coalescing.is_inflight(&kind));
        assert!(!bar.controls_refresh_coalescing.is_queued(&kind));

        let _ = bar.update(super::Message::RefreshControls(kind));
        assert!(bar.controls_refresh_coalescing.is_inflight(&kind));
        assert!(bar.controls_refresh_coalescing.is_queued(&kind));

        let _ = bar.update(super::Message::ControlsRefreshed(
            kind,
            crate::services::controls::ControlsRefresh::default(),
        ));
        assert!(bar.controls_refresh_coalescing.is_inflight(&kind));
        assert!(!bar.controls_refresh_coalescing.is_queued(&kind));

        let _ = bar.update(super::Message::ControlsRefreshed(
            kind,
            crate::services::controls::ControlsRefresh::default(),
        ));
        assert!(!bar.controls_refresh_coalescing.is_inflight(&kind));
        assert!(!bar.controls_refresh_coalescing.is_queued(&kind));
    }

    #[test]
    fn controls_follow_up_refresh_queues_when_kind_is_inflight() {
        let kind = crate::services::controls::ControlsRefreshKind::Brightness;
        let mut bar = hermetic_bar();

        let _ = bar.update(super::Message::RefreshControls(kind));
        let _ = bar.update(super::Message::ControlsCommandCompleted(
            crate::services::controls::ControlsFollowUp::Refresh(kind),
        ));

        assert!(bar.controls_refresh_coalescing.is_inflight(&kind));
        assert!(bar.controls_refresh_coalescing.is_queued(&kind));
    }

    #[test]
    fn dbus_connect_requests_coalesce_until_failure_completion() {
        let mut bar = hermetic_bar();

        let _ = bar.request_dbus_connect();
        assert!(bar
            .background_request_coalescing
            .is_inflight(&BackgroundRequestKind::DbusConnect));
        assert!(!bar
            .background_request_coalescing
            .is_queued(&BackgroundRequestKind::DbusConnect));

        let _ = bar.request_dbus_connect();
        assert!(bar
            .background_request_coalescing
            .is_queued(&BackgroundRequestKind::DbusConnect));

        let _ = bar.update(super::Message::DBusConnectAttempted(None));
        assert!(bar
            .background_request_coalescing
            .is_inflight(&BackgroundRequestKind::DbusConnect));
        assert!(!bar
            .background_request_coalescing
            .is_queued(&BackgroundRequestKind::DbusConnect));

        let _ = bar.update(super::Message::DBusConnectAttempted(None));
        assert!(!bar
            .background_request_coalescing
            .is_inflight(&BackgroundRequestKind::DbusConnect));
        assert!(!bar
            .background_request_coalescing
            .is_queued(&BackgroundRequestKind::DbusConnect));
    }

    #[test]
    fn slow_tick_reuses_single_background_connect_request_without_bus() {
        let mut bar = hermetic_bar();

        let _ = bar.perform_slow_tick();
        assert!(bar
            .background_request_coalescing
            .is_inflight(&BackgroundRequestKind::DbusConnect));
        assert!(!bar
            .background_request_coalescing
            .is_queued(&BackgroundRequestKind::DbusConnect));

        let _ = bar.perform_slow_tick();
        assert!(bar
            .background_request_coalescing
            .is_inflight(&BackgroundRequestKind::DbusConnect));
        assert!(bar
            .background_request_coalescing
            .is_queued(&BackgroundRequestKind::DbusConnect));
    }
}
