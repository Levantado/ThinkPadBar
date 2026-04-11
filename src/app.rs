use crate::{
    popup_coordinator::{PopupTransitionPlan, PopupVisibilityAction},
    services::capabilities::RuntimeCapabilities,
    ui::{
        bar, popup_host,
        popups::{self},
        theme::ThemeTokens,
    },
    update_coordinator,
};
use chrono::Local;
use iced::{widget::Row, window::Id, Color, Element, Task, Theme};
use std::collections::HashSet;
use std::time::Duration;
use tracing::info;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Popup {
    None,
    Stats,
    Power,
    Controls,
    Connectivity,
    AudioRoutes,
    BluetoothDevices,
    SystemMonitor,
    Displays,
    Calendar,
    TrayMenu,
    Media,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg(test)]
struct PopupDomainState {
    is_power_popup: bool,
    is_controls_popup: bool,
    is_connectivity_popup: bool,
}

const BLUETOOTH_SCAN_WINDOW_SECS: u8 = 5;
const BLUETOOTH_SCAN_RESULT_WINDOW_SECS: u8 = 8;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) enum BluetoothScanState {
    #[default]
    Idle,
    Scanning {
        remaining_secs: u8,
        baseline_addresses: Vec<String>,
    },
    Completed {
        total_devices: usize,
        newly_discovered_addresses: Vec<String>,
        remaining_secs: u8,
    },
}

pub struct ThinkPadBar {
    config: crate::config::Config,
    dbus_conn: Option<zbus::Connection>,
    clock: String,
    controls: crate::services::controls::ControlsSnapshot,
    audio_visualizer: crate::services::audio_visualizer::AudioVisualizerSnapshot,
    network_service: crate::services::network::NetworkService,
    idle_inhibitor_service: crate::services::idle_inhibitor::IdleInhibitorService,
    wayland_runtime_service: crate::services::wayland_runtime::WaylandRuntimeService,
    popup: Popup,
    main_window_id: Option<Id>,
    popup_window_id: Option<Id>,
    calendar_offset: i32,
    compositor_service: crate::services::compositor::CompositorService,
    controls_service: crate::services::controls::ControlsService,
    popup_anchor_service: crate::services::popup_anchor::PopupAnchorService,
    session_service: crate::services::session::SessionService,
    system_info_service: crate::services::system_info::SystemInfoService,
    tray_ui_service: crate::services::tray_ui::TrayUiService,
    audio_visualizer_service: crate::services::audio_visualizer::AudioVisualizerService,
    media_snapshot: crate::services::media::MediaSnapshot,
    media_command_tx: tokio::sync::mpsc::Sender<crate::services::media::MediaCommand>,
    media_event_rx: tokio::sync::broadcast::Receiver<crate::services::media::MediaEvent>,
    start_time: std::time::Instant,
    bluetooth_scan_state: BluetoothScanState,
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
        Box<crate::services::controls::ControlsRefresh>,
    ),
    ControlsEvent(crate::services::controls::ControlsEvent),
    ControlsCommandCompleted(crate::services::controls::ControlsFollowUp),
    BackgroundWifiInfoSynced(crate::services::network::WifiInfo),
    AudioVisualizerFrame(crate::services::audio_visualizer::AudioVisualizerSnapshot),
    MediaEvent(crate::services::media::MediaEvent),
    MediaCommand(crate::services::media::MediaCommand),
    WaylandRuntimeEvent(crate::services::wayland_runtime::WaylandRuntimeEvent),
    RefreshCompositor,
    CompositorEvent(crate::services::compositor::CompositorEvent),
    CompositorRefreshed(crate::services::compositor::RefreshResult),
    SwitchWorkspace(i32, String),
    TogglePopup(Popup),
    SetVolume(u32),
    AdjustVolumeBy(i8),
    SetAudioOutputRoute(String),
    SetMicVolume(u32),
    AdjustMicVolumeBy(i8),
    SetAudioInputRoute(String),
    SetFanLevel(String),
    SetBrightness(u32),
    AdjustBrightnessBy(i8),
    FlushCoalescedControl(CoalescedControlKind, u64),
    FlushSlowTick(u64),
    SetPowerProfile(String),
    CyclePerformanceProfile,
    NetworkCommand(crate::services::network::NetworkCommand),
    NetworkEvent(crate::services::network::NetworkEvent),
    ToggleBluetooth(bool),
    ConnectBluetoothDevice(String),
    DisconnectBluetoothDevice(String),
    ScanBluetoothDevices,
    StopBluetoothScan,
    PairBluetoothDevice(String),
    TrustBluetoothDevice(String),
    RemoveBluetoothDevice(String),
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
    TrayMenuBack,
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

    fn pending_count(&self) -> usize {
        usize::from(self.volume.has_pending())
            + usize::from(self.mic_volume.has_pending())
            + usize::from(self.brightness.has_pending())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct AppCoalescingDiagnostics {
    pending_control_flushes: usize,
    pending_slow_tick: bool,
    inflight_control_refreshes: usize,
    queued_control_refreshes: usize,
    inflight_background_requests: usize,
    queued_background_requests: usize,
}

impl AppCoalescingDiagnostics {
    pub(crate) fn summary(&self) -> String {
        format!(
            "ctrl pending:{} refresh {}/{} bg {}/{} slow:{}",
            self.pending_control_flushes,
            self.inflight_control_refreshes,
            self.queued_control_refreshes,
            self.inflight_background_requests,
            self.queued_background_requests,
            self.pending_slow_tick
        )
    }
}

impl ThinkPadBar {
    const CONTROL_COALESCE_DELAY_MS: u64 = 75;
    const SLOW_TICK_COALESCE_DELAY_MS: u64 = 75;
    const PILL_SCROLL_STEP_PERCENT: u32 = 5;

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

    fn theme_tokens(&self) -> ThemeTokens {
        ThemeTokens::from_config(&self.config)
    }

    fn runtime_capabilities(&self) -> RuntimeCapabilities {
        let mut items = vec![
            self.compositor_service.capability_status(),
            self.wayland_runtime_service.capability_status(),
            self.idle_inhibitor_service.capability_status(),
            self.network_service.capability_status(),
            self.session_service.capability_status(),
        ];
        items.extend(self.controls_service.capability_statuses());
        RuntimeCapabilities::new(items)
    }

    fn popup_surface_kind(popup: &Popup) -> crate::services::popup_anchor::PopupSurfaceKind {
        match popup {
            Popup::None => crate::services::popup_anchor::PopupSurfaceKind::Hidden,
            Popup::Stats => crate::services::popup_anchor::PopupSurfaceKind::Stats,
            Popup::Power => crate::services::popup_anchor::PopupSurfaceKind::Power,
            Popup::Controls => crate::services::popup_anchor::PopupSurfaceKind::Controls,
            Popup::Connectivity => crate::services::popup_anchor::PopupSurfaceKind::Connectivity,
            Popup::AudioRoutes => crate::services::popup_anchor::PopupSurfaceKind::AudioRoutes,
            Popup::BluetoothDevices => {
                crate::services::popup_anchor::PopupSurfaceKind::BluetoothDevices
            }
            Popup::SystemMonitor => crate::services::popup_anchor::PopupSurfaceKind::SystemMonitor,
            Popup::Displays => crate::services::popup_anchor::PopupSurfaceKind::Displays,
            Popup::Calendar => crate::services::popup_anchor::PopupSurfaceKind::Calendar,
            Popup::TrayMenu => crate::services::popup_anchor::PopupSurfaceKind::TrayMenu,
            Popup::Media => crate::services::popup_anchor::PopupSurfaceKind::Media,
        }
    }

    #[cfg(test)]
    fn popup_domain_state(popup: &Popup) -> PopupDomainState {
        match popup {
            Popup::Power => PopupDomainState {
                is_power_popup: true,
                is_controls_popup: false,
                is_connectivity_popup: false,
            },
            Popup::Controls => PopupDomainState {
                is_power_popup: false,
                is_controls_popup: true,
                is_connectivity_popup: false,
            },
            Popup::Connectivity => PopupDomainState {
                is_power_popup: false,
                is_controls_popup: false,
                is_connectivity_popup: true,
            },
            _ => PopupDomainState {
                is_power_popup: false,
                is_controls_popup: false,
                is_connectivity_popup: false,
            },
        }
    }

    fn build_power_popup_model(&self) -> popups::power::PowerPopupModel {
        popups::power::PowerPopupModel::new(
            &self.controls,
            &self.config.performance,
            self.idle_inhibitor_service.snapshot(),
            self.session_service.snapshot().power_menu_open,
            self.config.appearance.opacity,
        )
    }

    fn build_controls_popup_model(&self) -> popups::controls::ControlsPopupModel {
        popups::controls::ControlsPopupModel::new(&self.controls, self.config.appearance.opacity)
    }

    fn build_stats_popup_model(&self) -> popups::stats::StatsPopupModel {
        let sys_data = self.system_info_service.snapshot();
        let cpu_summary = popups::power::cpu_usage_summary(sys_data);
        let mem_summary = if sys_data.mem_str.trim().is_empty() {
            if sys_data.mem_total > 0 {
                let percent =
                    (sys_data.mem_used as f64 / sys_data.mem_total as f64 * 100.0).round() as i32;
                format!("{percent}%")
            } else {
                "0%".to_string()
            }
        } else {
            sys_data.mem_str.clone()
        };
        let temp_summary = if sys_data.temp_str.trim().is_empty() {
            format!("{}°C", sys_data.temp.round() as i32)
        } else {
            sys_data.temp_str.clone()
        };

        popups::stats::StatsPopupModel::new(
            popups::stats::opaque_background_alpha(self.config.appearance.opacity),
            cpu_summary,
            mem_summary,
            temp_summary,
            popups::power::fan_runtime_summary(&self.controls.fan),
        )
    }

    fn build_displays_popup_model(
        &self,
        wayland_snapshot: &crate::services::wayland_runtime::WaylandRuntimeSnapshot,
    ) -> popups::displays::DisplaysPopupModel {
        popups::displays::DisplaysPopupModel::new(
            popups::displays::summary_rows(wayland_snapshot),
            wayland_snapshot.missing_capabilities(),
            popups::displays::output_cards(wayland_snapshot),
        )
    }

    fn build_audio_routes_popup_model(&self) -> popups::audio_routes::AudioRoutesPopupModel {
        let output_routes = popups::audio_routes::popup_items(
            &self.controls.audio_devices.output_routes,
            &self.controls.audio_devices.input_routes,
            self.controls.audio_devices.output_route.as_deref(),
            "SINK",
            "No output routes discovered",
        );
        let input_routes = popups::audio_routes::popup_items(
            &self.controls.audio_devices.input_routes,
            &self.controls.audio_devices.output_routes,
            self.controls.audio_devices.input_route.as_deref(),
            "SOURCE",
            "No input routes discovered",
        );

        popups::audio_routes::AudioRoutesPopupModel::new(
            popups::audio_routes::current_route_summary(
                &self.controls.audio_devices.output_routes,
                self.controls.audio_devices.output_route.as_deref(),
                "No output route selected",
            ),
            popups::audio_routes::current_route_summary(
                &self.controls.audio_devices.input_routes,
                self.controls.audio_devices.input_route.as_deref(),
                "No input route selected",
            ),
            output_routes,
            input_routes,
        )
    }

    fn build_bluetooth_devices_popup_model(
        &self,
    ) -> popups::bluetooth_devices::BluetoothDevicesPopupModel {
        popups::bluetooth_devices::BluetoothDevicesPopupModel::from_state(
            &self.controls,
            &self.bluetooth_scan_state,
        )
    }

    fn build_connectivity_popup_model(&self) -> popups::connectivity::ConnectivityPopupModel {
        popups::connectivity::ConnectivityPopupModel::new(
            self.network_service.snapshot(),
            &self.controls,
            self.config.appearance.opacity,
        )
    }

    fn build_tray_menu_popup_model(&self) -> popups::tray_menu::TrayMenuPopupModel {
        popups::tray_menu::TrayMenuPopupModel::from_owned_menu(
            self.tray_ui_service.open_menu(),
            self.tray_ui_service.open_menu_path(),
        )
    }

    fn build_calendar_popup_model(&self) -> Option<popups::calendar::CalendarPopupModel> {
        popups::calendar::CalendarPopupModel::from_offset(
            self.calendar_offset,
            chrono::Local::now(),
        )
    }

    fn build_system_info_popup_model(
        &self,
        compositor: &crate::services::compositor::CompositorSnapshot,
        wayland_snapshot: &crate::services::wayland_runtime::WaylandRuntimeSnapshot,
    ) -> popups::system_info::SystemInfoPopupModel {
        let sys_data = self.system_info_service.snapshot();
        let system_diagnostics = self.system_info_service.diagnostics();
        let compositor_diagnostics = self.compositor_service.diagnostics();
        let controls_diagnostics = self.controls_service.diagnostics();
        let network_diagnostics = self.network_service.diagnostics();
        let idle_snapshot = self.idle_inhibitor_service.snapshot();
        let tray_diagnostics = self.tray_ui_service.diagnostics();
        let coalescing_diagnostics = self.coalescing_diagnostics();
        let runtime_capabilities = self.runtime_capabilities();

        crate::ui::adapters::build_system_info_popup_model(
            crate::ui::adapters::SystemInfoPopupInputs {
                version: env!("CARGO_PKG_VERSION"),
                debug_ui_enabled: self.debug_ui_enabled,
                sys_data,
                battery: &self.controls.battery,
                power_profile: &self.controls.power_profile,
                fan: &self.controls.fan,
                idle_snapshot: &idle_snapshot,
                wayland_snapshot,
                system_diagnostics: &system_diagnostics,
                compositor_diagnostics: &compositor_diagnostics,
                controls_diagnostics: &controls_diagnostics,
                network_diagnostics: &network_diagnostics,
                tray_diagnostics: &tray_diagnostics,
                coalescing_diagnostics: &coalescing_diagnostics,
                audio_visualizer_runtime: self
                    .audio_visualizer_service
                    .diagnostics_summary()
                    .unwrap_or_else(|| "n/a".to_string()),
                runtime_capabilities_summary: runtime_capabilities.summary(),
                capability_providers_summary: runtime_capabilities.provider_summary(),
                capability_degradations_summary: runtime_capabilities
                    .degraded_summary()
                    .unwrap_or_else(|| "none".to_string()),
                service_backends_summary: format!(
                    "cmp {:?}->{:?} net {:?}->{:?}",
                    compositor.configured_backend,
                    compositor.active_backend,
                    self.network_service.configured_backend(),
                    self.network_service.active_backend()
                ),
            },
        )
    }

    fn build_popup_host_model(&self) -> popup_host::PopupHostModel {
        let compositor = self.compositor_service.snapshot();
        let wayland_snapshot = self.wayland_runtime_service.snapshot();

        match self.popup {
            Popup::None => popup_host::PopupHostModel::Hidden,
            Popup::TrayMenu => {
                popup_host::PopupHostModel::TrayMenu(self.build_tray_menu_popup_model())
            }
            Popup::Stats => popup_host::PopupHostModel::Stats(self.build_stats_popup_model()),
            Popup::Displays => popup_host::PopupHostModel::Displays(
                self.build_displays_popup_model(wayland_snapshot),
            ),
            Popup::AudioRoutes => {
                popup_host::PopupHostModel::AudioRoutes(self.build_audio_routes_popup_model())
            }
            Popup::BluetoothDevices => popup_host::PopupHostModel::BluetoothDevices(
                self.build_bluetooth_devices_popup_model(),
            ),
            Popup::Calendar => self
                .build_calendar_popup_model()
                .map(popup_host::PopupHostModel::Calendar)
                .unwrap_or(popup_host::PopupHostModel::Hidden),
            Popup::SystemMonitor => popup_host::PopupHostModel::SystemMonitor(
                self.build_system_info_popup_model(compositor, wayland_snapshot),
            ),
            Popup::Power => popup_host::PopupHostModel::Power(self.build_power_popup_model()),
            Popup::Connectivity => {
                popup_host::PopupHostModel::Connectivity(self.build_connectivity_popup_model())
            }
            Popup::Controls => {
                popup_host::PopupHostModel::Controls(self.build_controls_popup_model())
            }
            Popup::Media => popup_host::PopupHostModel::Media(self.build_media_popup_model()),
        }
    }

    fn build_media_popup_model(&self) -> popups::media::MediaPopupModel {
        popups::media::MediaPopupModel::new(&self.media_snapshot, self.config.appearance.opacity)
    }

    fn build_main_bar_model(&self) -> crate::ui::bar::MainBarModel {
        let compositor = self.compositor_service.snapshot();
        let sys_data = self.system_info_service.snapshot();
        let wifi_snapshot = self.network_service.snapshot();
        let idle_snapshot = self.idle_inhibitor_service.snapshot();
        let pill_fg = Color::from_rgb8(0xc0, 0xca, 0xf5);
        let temp_val = sys_data.temp.round() as i32;
        let (battery_icon, battery_color) =
            popups::power::battery_icon_and_color(&self.controls.battery);
        let (power_profile_label, power_profile_color) =
            popups::power::power_profile_visual(&self.controls.power_profile);

        bar::MainBarModel {
            opacity: self.config.appearance.opacity,
            bar_height: self.config.appearance.bar_height as f32,
            workspaces: compositor
                .workspaces
                .iter()
                .map(|ws| bar::WorkspaceChip {
                    id: ws.id,
                    name: ws.name.clone(),
                    active: ws.active,
                    special: Self::is_special_workspace(&ws.name),
                })
                .collect(),
            tray_items: self
                .tray_ui_service
                .items()
                .iter()
                .map(|(id, item)| bar::TrayItemModel {
                    id: id.clone(),
                    icon_handle: item.icon_handle.clone(),
                    fallback_label: item.fallback_label(),
                })
                .collect(),
            center_title: bar::trunc_with_ellipsis(compositor.active_window.as_str(), 34),
            special_workspace_visible: compositor.special_workspace_visible,
            visualizer: bar::AudioVisualizerModel {
                enabled: self.config.appearance.audio_visualizer.enabled,
                bars: self.audio_visualizer.bars[..usize::from(self.audio_visualizer.visible_bars)
                    .min(self.audio_visualizer.bars.len())]
                    .to_vec(),
                active: self.audio_visualizer.active,
                min_height: self
                    .config
                    .appearance
                    .audio_visualizer
                    .normalized_min_height(),
                max_height: self
                    .config
                    .appearance
                    .audio_visualizer
                    .normalized_max_height(),
                bar_width: self
                    .config
                    .appearance
                    .audio_visualizer
                    .normalized_bar_width(),
                gap: self.config.appearance.audio_visualizer.normalized_gap(),
                padding_x: self
                    .config
                    .appearance
                    .audio_visualizer
                    .normalized_padding_x(),
                padding_y: self
                    .config
                    .appearance
                    .audio_visualizer
                    .normalized_padding_y(),
                color_profile:
                    crate::services::audio_visualizer::VisualizerColorProfile::from_config(
                        self.config
                            .appearance
                            .audio_visualizer
                            .normalized_color_profile(),
                    ),
            },
            media: {
                let title_trimmed = self.media_snapshot.title.trim();
                let artist_trimmed = self.media_snapshot.artist.trim();
                let unknown_title =
                    title_trimmed.is_empty() || title_trimmed.eq_ignore_ascii_case("unknown");
                let unknown_artist = artist_trimmed.is_empty()
                    || artist_trimmed.eq_ignore_ascii_case("unknown artist");
                let meaningful_media = !(unknown_title && unknown_artist)
                    || self.media_snapshot.playback_status == "Playing";
                let full_text = format!(
                    "{} - {}",
                    self.media_snapshot.artist, self.media_snapshot.title
                );
                let display_text =
                    Self::calculate_marquee(&full_text, 25, self.start_time.elapsed().as_millis());

                bar::MediaPillModel {
                    title: display_text,
                    artist: String::new(), // Artist already merged into title for marquee
                    playback_status: self.media_snapshot.playback_status.clone(),
                    has_player: self.media_snapshot.has_player && meaningful_media,
                }
            },

            stats: bar::StatsPillModel {
                cpu_summary: popups::power::cpu_usage_summary(sys_data),
                temp_summary: bar::temperature_summary(sys_data),
                temp_color: bar::temperature_color(temp_val, pill_fg),
                fan_speed: self.controls.fan.speed.clone(),
            },
            controls: bar::ControlsPillModel {
                brightness_label: self.controls.brightness.label.clone(),
                volume_icon: bar::volume_icon(&self.controls.audio),
                volume_label: crate::ui::chrome::audio_percent_label(&self.controls.audio),
                mic_icon: bar::mic_icon(&self.controls.mic),
                mic_label: format!("{}%", self.controls.mic.volume),
            },
            connectivity: bar::ConnectivityPillModel {
                wifi_icon: bar::wifi_icon(wifi_snapshot.wifi.enabled),
                wifi_label: bar::wifi_label(
                    wifi_snapshot.wifi.enabled,
                    &wifi_snapshot.wifi.ssid,
                    8,
                ),
                bluetooth_icon: bar::bluetooth_icon(self.controls.bluetooth_enabled),
                bluetooth_label: crate::ui::chrome::bluetooth_pill_summary(&self.controls),
            },
            battery: bar::BatteryPillModel {
                battery_icon,
                battery_color,
                battery_label: crate::ui::chrome::battery_percent_label(&self.controls.battery),
                power_profile_label: power_profile_label.to_string(),
                power_profile_color,
                idle_enabled: idle_snapshot.enabled,
            },
            keyboard_layout: compositor.keyboard_layout.clone(),
            clock: self.clock.clone(),
            show_debug_toggle: self.debug_ui_enabled,
            show_debug_overlay: self.debug_ui_enabled && self.show_debug_overlay,
            debug_overlay_text: format!(
                "ws req:{} coal:{} last:{}ms avg:{}ms dbus ok/fail:{}/{}",
                self.perf.workspace_refresh_requested,
                self.perf.workspace_refresh_coalesced,
                self.perf.workspace_refresh_last_ms,
                self.perf.workspace_refresh_avg_ms(),
                self.perf.dbus_connect_successes,
                self.perf.dbus_connect_failures
            ),
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
            .map(|menu| menu.popup_height(self.tray_ui_service.open_menu_path()));
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

    fn apply_popup_transition_plan(&mut self, plan: PopupTransitionPlan) -> Vec<Task<Message>> {
        let mut tasks = Vec::new();

        for refresh_kind in plan.controls_refreshes {
            tasks.push(self.request_controls_refresh(refresh_kind));
        }

        if let Some(refresh_kind) = plan.system_info_refresh {
            self.system_info_service.refresh(refresh_kind);
        }

        self.popup = plan.next_popup;

        if plan.close_transient_ui {
            self.tray_ui_service.close_transient_ui();
            self.session_service.close_transient_ui();
            self.network_service.close_transient_ui();
        }

        if plan.reset_bluetooth_scan {
            self.bluetooth_scan_state = BluetoothScanState::Idle;
        }
        if plan.reset_calendar_offset {
            self.calendar_offset = 0;
        }

        match plan.visibility_action {
            PopupVisibilityAction::Hide => tasks.extend(self.popup_hide_tasks()),
            PopupVisibilityAction::Show(popup) => tasks.extend(self.popup_show_tasks(popup)),
        }

        tasks
    }

    fn is_special_workspace(name: &str) -> bool {
        let lower = name.to_ascii_lowercase();
        lower == "special" || lower.starts_with("special:")
    }

    fn step_adjust_percent(current: u32, direction: i8, min: u32, max: u32) -> u32 {
        let base = current.clamp(min, max);
        let step = Self::PILL_SCROLL_STEP_PERCENT;
        let next = if direction > 0 {
            base.saturating_add(step)
        } else if direction < 0 {
            base.saturating_sub(step)
        } else {
            base
        };
        next.clamp(min, max)
    }

    fn coalescing_diagnostics(&self) -> AppCoalescingDiagnostics {
        AppCoalescingDiagnostics {
            pending_control_flushes: self.controls_coalescing.pending_count(),
            pending_slow_tick: self.slow_tick_coalescing.has_pending(),
            inflight_control_refreshes: self.controls_refresh_coalescing.inflight_count(),
            queued_control_refreshes: self.controls_refresh_coalescing.queued_count(),
            inflight_background_requests: self.background_request_coalescing.inflight_count(),
            queued_background_requests: self.background_request_coalescing.queued_count(),
        }
    }

    fn spawn_controls_refresh(
        &self,
        kind: crate::services::controls::ControlsRefreshKind,
    ) -> Task<Message> {
        let controls_service = self.controls_service.clone();
        Task::perform(
            async move { controls_service.refresh(kind).await },
            move |refresh| Message::ControlsRefreshed(kind, Box::new(refresh)),
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

    fn preview_controls_command(&mut self, command: &crate::services::controls::ControlsCommand) {
        self.controls_service.preview_command(command);
        self.controls = self.controls_service.snapshot().clone();
    }

    fn preview_and_execute_controls_command(
        &mut self,
        command: crate::services::controls::ControlsCommand,
    ) -> Task<Message> {
        self.preview_controls_command(&command);
        self.execute_controls_command(command)
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

    pub fn calculate_marquee(full_text: &str, limit: usize, elapsed_ms: u128) -> String {
        if full_text.chars().count() <= limit {
            return full_text.to_string();
        }

        let chars: Vec<char> = full_text.chars().collect();
        let len = chars.len();
        let gap = 5;
        let total_len = len + gap;
        let speed_chars_per_sec = 2;
        let offset = ((elapsed_ms / (1000 / speed_chars_per_sec)) as usize) % total_len;

        let mut marquee_text = String::with_capacity(limit);
        for i in 0..limit {
            let idx = (offset + i) % total_len;
            if idx < len {
                marquee_text.push(chars[idx]);
            } else {
                marquee_text.push(' ');
            }
        }
        marquee_text
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
            let wayland_runtime_service =
                crate::services::wayland_runtime::WaylandRuntimeService::new();
            let popup_anchor_service =
                crate::services::popup_anchor::PopupAnchorService::new(cfg.appearance.bar_height);
            let session_service = crate::services::session::SessionService::new();
            let system_info_service = crate::services::system_info::SystemInfoService::new();
            let tray_ui_service = crate::services::tray_ui::TrayUiService::new();
            let audio_visualizer_service =
                crate::services::audio_visualizer::AudioVisualizerService::new(
                    crate::services::audio_visualizer::AudioVisualizerConfig::from_appearance(
                        &cfg.appearance.audio_visualizer,
                    ),
                );

            // Try to connect to D-Bus synchronously for initialization if possible,
            // or just let it be None and connect later.
            // Since we are in a tokio-enabled FnOnce, we can't easily await here without block_on.
            // But iced's run_with expects a Task.

            let (media_service, media_event_rx, media_command_tx) =
                crate::services::media::MediaService::new();

            let app = Self {
                config: cfg,
                dbus_conn: None,
                clock: Local::now().format("%a %d %b %H:%M").to_string(),
                controls: controls_snapshot,
                audio_visualizer:
                    crate::services::audio_visualizer::AudioVisualizerSnapshot::default(),
                network_service,
                idle_inhibitor_service,
                wayland_runtime_service,
                popup: Popup::None,
                main_window_id: Some(main_id),
                popup_window_id: Some(popup_id),
                calendar_offset: 0,
                compositor_service,
                controls_service,
                popup_anchor_service,
                session_service,
                system_info_service,
                tray_ui_service,
                audio_visualizer_service,
                media_snapshot: crate::services::media::MediaSnapshot::default(),
                media_command_tx,
                media_event_rx,
                start_time: std::time::Instant::now(),
                bluetooth_scan_state: BluetoothScanState::default(),
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

            let media_task = Task::perform(
                async move {
                    let _ = media_service.run().await;
                },
                |_| Message::Tick(Local::now()),
            ); // Dummy msg, we care about the side effect

            (app, Task::batch(vec![main_task, popup_task, media_task]))
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
                match &mut self.bluetooth_scan_state {
                    BluetoothScanState::Scanning { remaining_secs, .. } => {
                        *remaining_secs = remaining_secs.saturating_sub(1);
                    }
                    BluetoothScanState::Completed { remaining_secs, .. } => {
                        if *remaining_secs > 1 {
                            *remaining_secs -= 1;
                        } else {
                            self.bluetooth_scan_state = BluetoothScanState::Idle;
                        }
                    }
                    BluetoothScanState::Idle => {}
                }
                if self.system_info_fast_updates_enabled() {
                    self.system_info_service
                        .refresh(crate::services::system_info::SystemInfoRefreshKind::Fast);
                }
                return Task::none();
            }
            Message::RefreshControls(kind) => {
                return self.request_controls_refresh(kind);
            }
            Message::ControlsEvent(event) => {
                let kind = controls_event_refresh_kind(event);
                return self.request_controls_refresh(kind);
            }
            Message::ControlsRefreshed(kind, refresh) => {
                let previous_bluetooth_devices = (kind
                    == crate::services::controls::ControlsRefreshKind::Bluetooth)
                    .then(|| self.controls.bluetooth_devices.device_details.clone());
                self.controls_service.apply_refresh(*refresh);
                self.controls = self.controls_service.snapshot().clone();
                if kind == crate::services::controls::ControlsRefreshKind::Bluetooth
                    && matches!(
                        self.bluetooth_scan_state,
                        BluetoothScanState::Scanning { .. }
                    )
                {
                    let previous: HashSet<String> = match &self.bluetooth_scan_state {
                        BluetoothScanState::Scanning {
                            baseline_addresses, ..
                        } => baseline_addresses.iter().cloned().collect(),
                        _ => previous_bluetooth_devices
                            .unwrap_or_default()
                            .into_iter()
                            .map(|device| device.address)
                            .collect(),
                    };
                    let newly_discovered_addresses = self
                        .controls
                        .bluetooth_devices
                        .device_details
                        .iter()
                        .map(|device| device.address.clone())
                        .filter(|address| !previous.contains(address))
                        .collect::<Vec<_>>();
                    self.bluetooth_scan_state = BluetoothScanState::Completed {
                        total_devices: self.controls.bluetooth_devices.device_details.len(),
                        newly_discovered_addresses,
                        remaining_secs: BLUETOOTH_SCAN_RESULT_WINDOW_SECS,
                    };
                }
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
            Message::AudioVisualizerFrame(snapshot) => {
                self.audio_visualizer = snapshot;
            }
            Message::MediaEvent(event) => match event {
                crate::services::media::MediaEvent::SnapshotUpdated(snapshot) => {
                    self.media_snapshot = snapshot;
                    return Task::none();
                }
            },
            Message::MediaCommand(cmd) => {
                let tx = self.media_command_tx.clone();
                return Task::perform(
                    async move {
                        let _ = tx.send(cmd).await;
                    },
                    |_| Message::Tick(Local::now()),
                );
            }
            Message::WaylandRuntimeEvent(
                crate::services::wayland_runtime::WaylandRuntimeEvent::SnapshotUpdated(snapshot),
            ) => {
                self.wayland_runtime_service.apply_snapshot(snapshot);
            }
            Message::PopupWindowUnfocused(window_id) => {
                if Self::should_close_popup_on_unfocus(self.popup_window_id, &self.popup, window_id)
                {
                    return Task::batch(
                        self.apply_popup_transition_plan(PopupTransitionPlan::close_on_unfocus()),
                    );
                }
            }
            Message::OpenLauncher => {
                return update_coordinator::perform_session_command(
                    self.session_service.clone(),
                    crate::services::session::SessionCommand::OpenLauncher,
                );
            }
            Message::SessionCommandCompleted(follow_up) => {
                if update_coordinator::session_follow_up_requests_refresh(follow_up) {
                    return self.request_compositor_refresh();
                }
            }
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
                self.compositor_service.apply_refresh(refreshed);

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
                return Task::batch(
                    self.apply_popup_transition_plan(PopupTransitionPlan::toggle(
                        &self.popup,
                        target,
                    )),
                );
            }
            Message::SetVolume(val) => {
                let command = crate::services::controls::ControlsCommand::SetVolume(val);
                self.preview_controls_command(&command);
                let generation = self
                    .controls_coalescing
                    .push(CoalescedControlKind::Volume, val);
                return Self::schedule_coalesced_control_flush(
                    CoalescedControlKind::Volume,
                    generation,
                );
            }
            Message::AdjustVolumeBy(direction) => {
                let next = Self::step_adjust_percent(self.controls.audio.volume, direction, 0, 100);
                if next != self.controls.audio.volume {
                    return self.update(Message::SetVolume(next));
                }
            }
            Message::ToggleAudioMute => {
                return self.execute_controls_command(
                    crate::services::controls::ControlsCommand::ToggleAudioMute,
                );
            }
            Message::SetAudioOutputRoute(route_id) => {
                let command =
                    crate::services::controls::ControlsCommand::SetAudioOutputRoute(route_id);
                return self.preview_and_execute_controls_command(command);
            }
            Message::SetMicVolume(val) => {
                let command = crate::services::controls::ControlsCommand::SetMicVolume(val);
                self.preview_controls_command(&command);
                let generation = self
                    .controls_coalescing
                    .push(CoalescedControlKind::MicVolume, val);
                return Self::schedule_coalesced_control_flush(
                    CoalescedControlKind::MicVolume,
                    generation,
                );
            }
            Message::AdjustMicVolumeBy(direction) => {
                let next = Self::step_adjust_percent(self.controls.mic.volume, direction, 0, 100);
                if next != self.controls.mic.volume {
                    return self.update(Message::SetMicVolume(next));
                }
            }
            Message::ToggleMicMute => {
                return self.execute_controls_command(
                    crate::services::controls::ControlsCommand::ToggleMicMute,
                );
            }
            Message::SetAudioInputRoute(route_id) => {
                let command =
                    crate::services::controls::ControlsCommand::SetAudioInputRoute(route_id);
                return self.preview_and_execute_controls_command(command);
            }
            Message::SetBrightness(val) => {
                let command = crate::services::controls::ControlsCommand::SetBrightness(val);
                self.preview_controls_command(&command);
                let generation = self
                    .controls_coalescing
                    .push(CoalescedControlKind::Brightness, val);
                return Self::schedule_coalesced_control_flush(
                    CoalescedControlKind::Brightness,
                    generation,
                );
            }
            Message::AdjustBrightnessBy(direction) => {
                let next =
                    Self::step_adjust_percent(self.controls.brightness.percent, direction, 1, 100);
                if next != self.controls.brightness.percent {
                    return self.update(Message::SetBrightness(next));
                }
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
                return self.preview_and_execute_controls_command(command);
            }
            Message::SetPowerProfile(prof) => {
                let command = crate::services::controls::ControlsCommand::SetPowerProfile(prof);
                return self.preview_and_execute_controls_command(command);
            }
            Message::CyclePerformanceProfile => {
                self.config.performance.cycle_profile_runtime();
            }
            Message::NetworkCommand(command) => {
                let plan = update_coordinator::NetworkRuntimePlan::from(
                    self.network_service
                        .handle_command(command, self.dbus_conn.is_some()),
                );
                return update_coordinator::perform_network_plan(
                    self.network_service.clone(),
                    self.dbus_conn.clone(),
                    plan,
                );
            }
            Message::NetworkEvent(event) => {
                self.network_service.handle_event(event);
            }
            Message::ToggleBluetooth(enable) => {
                let command = crate::services::controls::ControlsCommand::ToggleBluetooth(enable);
                return self.preview_and_execute_controls_command(command);
            }
            Message::ConnectBluetoothDevice(address) => {
                let command =
                    crate::services::controls::ControlsCommand::ConnectBluetoothDevice(address);
                return self.preview_and_execute_controls_command(command);
            }
            Message::DisconnectBluetoothDevice(address) => {
                let command =
                    crate::services::controls::ControlsCommand::DisconnectBluetoothDevice(address);
                return self.preview_and_execute_controls_command(command);
            }
            Message::ScanBluetoothDevices => {
                let baseline_addresses = self
                    .controls
                    .bluetooth_devices
                    .device_details
                    .iter()
                    .map(|device| device.address.clone())
                    .collect::<Vec<_>>();
                self.bluetooth_scan_state = BluetoothScanState::Scanning {
                    remaining_secs: BLUETOOTH_SCAN_WINDOW_SECS,
                    baseline_addresses,
                };
                return self.execute_controls_command(
                    crate::services::controls::ControlsCommand::ScanBluetoothDevices,
                );
            }
            Message::StopBluetoothScan => {
                self.bluetooth_scan_state = BluetoothScanState::Idle;
                return self.execute_controls_command(
                    crate::services::controls::ControlsCommand::StopBluetoothScan,
                );
            }
            Message::PairBluetoothDevice(address) => {
                let command =
                    crate::services::controls::ControlsCommand::PairBluetoothDevice(address);
                return self.preview_and_execute_controls_command(command);
            }
            Message::TrustBluetoothDevice(address) => {
                let command =
                    crate::services::controls::ControlsCommand::TrustBluetoothDevice(address);
                return self.preview_and_execute_controls_command(command);
            }
            Message::RemoveBluetoothDevice(address) => {
                let command =
                    crate::services::controls::ControlsCommand::RemoveBluetoothDevice(address);
                return self.preview_and_execute_controls_command(command);
            }
            Message::ToggleIdleInhibitor => {
                self.idle_inhibitor_service.toggle();
            }
            Message::OpenOverskride => {
                return self.preview_and_execute_controls_command(
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
                let command = update_coordinator::session_command_from_power_action(action);
                let mut tasks =
                    self.apply_popup_transition_plan(PopupTransitionPlan::close_for_power_action());
                tasks.push(update_coordinator::perform_session_command(
                    self.session_service.clone(),
                    command,
                ));
                return Task::batch(tasks);
            }
            Message::TrayEvent(msg) => {
                if self.tray_ui_service.handle_runtime_message(msg) {
                    return Task::batch(
                        self.apply_popup_transition_plan(PopupTransitionPlan::close_popup()),
                    );
                }
            }
            Message::TrayItemClicked(id) => {
                if let Some(crate::services::tray_ui::TrayUiPrimaryAction::ResolveCandidates {
                    candidates,
                    ..
                }) = self.tray_ui_service.handle_primary_click(&id)
                {
                    return update_coordinator::perform_tray_candidate_resolution(
                        self.compositor_service.clone(),
                        candidates,
                        id,
                    );
                }
            }
            Message::TrayItemRightClicked(id) => {
                match update_coordinator::tray_popup_plan_from_secondary(
                    self.tray_ui_service.handle_secondary_click(
                        id.clone(),
                        self.compositor_service.cursor_position(),
                    ),
                ) {
                    update_coordinator::TrayPopupPlan::OpenMenu => {
                        return Task::batch(
                            self.apply_popup_transition_plan(PopupTransitionPlan::open_tray_menu()),
                        );
                    }
                    update_coordinator::TrayPopupPlan::CloseMenu => {
                        return Task::batch(
                            self.apply_popup_transition_plan(PopupTransitionPlan::close_popup()),
                        );
                    }
                    update_coordinator::TrayPopupPlan::None => {
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
                let cursor = self.tray_ui_service.menu_cursor();
                match update_coordinator::tray_popup_plan_from_selection(
                    self.tray_ui_service
                        .handle_menu_selection(menu_item_id, cursor),
                ) {
                    update_coordinator::TrayPopupPlan::CloseMenu => {
                        return Task::batch(
                            self.apply_popup_transition_plan(PopupTransitionPlan::close_popup()),
                        )
                    }
                    update_coordinator::TrayPopupPlan::None
                    | update_coordinator::TrayPopupPlan::OpenMenu => {}
                }
            }
            Message::TrayMenuBack => {
                match update_coordinator::tray_popup_plan_from_selection(
                    self.tray_ui_service.handle_menu_back(),
                ) {
                    update_coordinator::TrayPopupPlan::CloseMenu => {
                        return Task::batch(
                            self.apply_popup_transition_plan(PopupTransitionPlan::close_popup()),
                        )
                    }
                    update_coordinator::TrayPopupPlan::None
                    | update_coordinator::TrayPopupPlan::OpenMenu => {}
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
        crate::ui::bar::view(self.build_main_bar_model())
    }

    fn view_popup(&self) -> Element<'_, Message, Theme, iced::Renderer> {
        popup_host::view(
            self.theme_tokens(),
            self.config.appearance.opacity,
            self.build_popup_host_model(),
        )
    }

    pub fn theme(&self, _id: Id) -> Theme {
        Theme::Dark
    }

    fn audio_visualizer_updates_enabled(&self) -> bool {
        self.config.appearance.audio_visualizer.enabled
    }

    fn system_info_fast_updates_enabled(&self) -> bool {
        true
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        let (_, thermal_secs, slow_secs) = self.config.performance.effective_intervals();
        let audio_visualizer_subscription = if self.audio_visualizer_updates_enabled() {
            self.audio_visualizer_service
                .subscription()
                .map(Message::AudioVisualizerFrame)
        } else {
            iced::Subscription::none()
        };

        iced::Subscription::batch(vec![
            crate::modules::clock::tick(),
            iced::time::every(std::time::Duration::from_secs(thermal_secs)).map(|_| {
                Message::RefreshControls(crate::services::controls::ControlsRefreshKind::Fan)
            }),
            iced::time::every(std::time::Duration::from_secs(slow_secs))
                .map(|_| Message::TickSlow(chrono::Local::now())),
            self.compositor_service
                .subscription()
                .map(Message::CompositorEvent),
            crate::services::wayland_runtime::WaylandRuntimeService::subscription()
                .map(Message::WaylandRuntimeEvent),
            crate::services::tray_ui::TrayUiService::subscription().map(Message::TrayEvent),
            self.controls_service
                .subscription()
                .map(Message::ControlsEvent),
            crate::services::media::MediaService::subscription(self.media_event_rx.resubscribe())
                .map(Message::MediaEvent),
            audio_visualizer_subscription,
            iced::event::listen_with(|event, _status, window| match event {
                iced::Event::Window(iced::window::Event::Unfocused) => {
                    Some(Message::PopupWindowUnfocused(window))
                }
                _ => None,
            }),
        ])
    }
}

fn controls_event_refresh_kind(
    event: crate::services::controls::ControlsEvent,
) -> crate::services::controls::ControlsRefreshKind {
    match event {
        crate::services::controls::ControlsEvent::AudioServer => {
            crate::services::controls::ControlsRefreshKind::AudioMic
        }
        crate::services::controls::ControlsEvent::PowerProfile => {
            crate::services::controls::ControlsRefreshKind::Power
        }
        crate::services::controls::ControlsEvent::Bluetooth => {
            crate::services::controls::ControlsRefreshKind::Bluetooth
        }
        crate::services::controls::ControlsEvent::Brightness => {
            crate::services::controls::ControlsRefreshKind::Brightness
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        controls_event_refresh_kind, BackgroundRequestKind, CoalescedControlKind,
        ControlsCoalescing, Message, Popup, ThinkPadBar,
    };
    use crate::ui::{
        bar, chrome, popup_host,
        popups::{audio_routes, bluetooth_devices, displays, system_info},
    };
    use iced::window::Id;

    fn hermetic_bar() -> ThinkPadBar {
        ThinkPadBar {
            config: crate::config::Config::default(),
            dbus_conn: None,
            clock: String::new(),
            controls: crate::services::controls::ControlsSnapshot::default(),
            audio_visualizer: crate::services::audio_visualizer::AudioVisualizerSnapshot::default(),
            network_service: crate::services::network::NetworkService::new(
                &crate::config::NetworkConfig::default(),
            ),
            idle_inhibitor_service:
                crate::services::idle_inhibitor::IdleInhibitorService::unavailable_for_tests(),
            wayland_runtime_service:
                crate::services::wayland_runtime::WaylandRuntimeService::unavailable_for_tests(),
            popup: Popup::None,
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
            audio_visualizer_service:
                crate::services::audio_visualizer::AudioVisualizerService::new(
                    crate::services::audio_visualizer::AudioVisualizerConfig::from_appearance(
                        &crate::config::AudioVisualizerConfig {
                            enabled: false,
                            ..crate::config::AudioVisualizerConfig::default()
                        },
                    ),
                ),
            media_snapshot: crate::services::media::MediaSnapshot::default(),
            media_command_tx: tokio::sync::mpsc::channel(1).0,
            media_event_rx: tokio::sync::broadcast::channel(1).1,
            start_time: std::time::Instant::now(),
            bluetooth_scan_state: super::BluetoothScanState::default(),
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
            &Popup::Controls,
            popup_id,
        ));
    }

    #[test]
    fn popup_does_not_close_on_other_window_unfocus() {
        let popup_id = Id::unique();
        let other_id = Id::unique();
        assert!(!ThinkPadBar::should_close_popup_on_unfocus(
            Some(popup_id),
            &Popup::Controls,
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
    fn step_adjust_percent_enforces_step_and_bounds() {
        assert_eq!(ThinkPadBar::step_adjust_percent(40, 1, 0, 100), 45);
        assert_eq!(ThinkPadBar::step_adjust_percent(40, -1, 0, 100), 35);
        assert_eq!(ThinkPadBar::step_adjust_percent(100, 1, 0, 100), 100);
        assert_eq!(ThinkPadBar::step_adjust_percent(0, -1, 0, 100), 0);
        assert_eq!(ThinkPadBar::step_adjust_percent(1, -1, 1, 100), 1);
    }

    #[test]
    fn stats_popup_background_alpha_is_opaque() {
        assert_eq!(crate::ui::popups::stats::opaque_background_alpha(0.25), 1.0);
        assert_eq!(crate::ui::popups::stats::opaque_background_alpha(0.85), 1.0);
    }

    #[test]
    fn stats_row_value_never_returns_blank() {
        assert_eq!(crate::ui::popups::stats::normalize_value(""), "--");
        assert_eq!(crate::ui::popups::stats::normalize_value("   "), "--");
        assert_eq!(crate::ui::popups::stats::normalize_value("11%"), "11%");
        assert_eq!(crate::ui::popups::stats::normalize_value(" 34°C "), "34°C");
    }

    #[test]
    fn hardware_summary_rows_include_all_expected_lines() {
        let rows = system_info::hardware_rows(
            &crate::services::controls::BatteryInfo {
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
            },
            "balanced",
            &crate::services::controls::FanInfo {
                speed: "2700".to_string(),
                level: "auto".to_string(),
            },
            &crate::modules::system::SysData {
                temp: 56.0,
                temp_str: "56°C".to_string(),
                ..crate::modules::system::SysData::default()
            },
            &crate::services::idle_inhibitor::IdleInhibitorSnapshot {
                available: true,
                enabled: true,
                diagnostics: crate::services::idle_inhibitor::IdleInhibitorDiagnostics::default(),
            },
        );

        assert_eq!(rows.len(), 14);
        assert_eq!(
            rows.iter().map(|row| row.label).collect::<Vec<_>>(),
            vec![
                "Battery Runtime",
                "AC Adapter",
                "Battery Health",
                "Battery Wear",
                "Pack Capacity",
                "Pack Voltage",
                "Cycle Count",
                "Charge Thresholds",
                "Charge State",
                "Charge / Draw Power",
                "Power Profile",
                "Fan Runtime",
                "Thermal State",
                "Idle Inhibitor",
            ]
        );
        assert!(rows.iter().all(|row| !row.value.is_empty()));
    }

    #[test]
    fn control_center_power_items_surface_daily_battery_state() {
        let battery = crate::services::controls::BatteryInfo {
            capacity: 63,
            status: "Not charging".to_string(),
            time_remaining: Some("2h 06m remaining".to_string()),
            ac_online: Some(true),
            health_percent: Some(91),
            power_rate_mw: Some(11_800),
            pack_voltage_mv: None,
            cycle_count: None,
            full_charge_mwh: None,
            design_capacity_mwh: None,
            charge_start_threshold: Some(40),
            charge_end_threshold: Some(80),
        };

        let items = chrome::power_summary_items(&battery);
        let labels = items.iter().map(|(_, label, _)| *label).collect::<Vec<_>>();
        let values = items
            .iter()
            .map(|(_, _, value)| value.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            labels,
            vec![
                "Battery Runtime",
                "Charge State",
                "AC Adapter",
                "Charge / Draw Power",
                "Charge Thresholds",
            ]
        );
        assert!(values.contains(&"63% (2h 06m remaining)"));
        assert!(values.contains(&"Connected"));
        assert!(values.contains(&"11.8 W rate"));
        assert!(values.contains(&"40% -> 80%"));
    }

    #[test]
    fn control_center_device_items_surface_audio_mic_and_bluetooth_state() {
        let items = chrome::device_summary_items(&crate::services::controls::ControlsSnapshot {
            brightness: crate::services::controls::BrightnessSnapshot::from_percent(42),
            audio: crate::services::controls::AudioInfo {
                volume: 73,
                muted: false,
            },
            audio_devices: crate::services::controls::AudioDeviceSummary {
                output_route: Some("Built-in Audio".to_string()),
                input_route: Some("Internal Microphone".to_string()),
                output_routes: vec![
                    crate::services::controls::AudioRouteInfo {
                        id: "52".to_string(),
                        name: "Built-in Audio".to_string(),
                        origin: crate::services::controls::AudioRouteOrigin::Internal,
                    },
                    crate::services::controls::AudioRouteInfo {
                        id: "77".to_string(),
                        name: "USB DAC".to_string(),
                        origin: crate::services::controls::AudioRouteOrigin::Usb,
                    },
                ],
                input_routes: vec![
                    crate::services::controls::AudioRouteInfo {
                        id: "54".to_string(),
                        name: "Internal Microphone".to_string(),
                        origin: crate::services::controls::AudioRouteOrigin::Internal,
                    },
                    crate::services::controls::AudioRouteInfo {
                        id: "81".to_string(),
                        name: "USB Mic".to_string(),
                        origin: crate::services::controls::AudioRouteOrigin::Usb,
                    },
                ],
            },
            mic: crate::services::controls::MicInfo {
                volume: 18,
                muted: true,
            },
            fan: crate::services::controls::FanInfo {
                speed: "---".to_string(),
                level: "auto".to_string(),
            },
            battery: crate::services::controls::BatteryInfo {
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
                charge_start_threshold: None,
                charge_end_threshold: None,
            },
            power_profile: "balanced".to_string(),
            bluetooth_enabled: true,
            bluetooth_devices: crate::services::controls::BluetoothDeviceSummary {
                connected_devices: vec!["WH-1000XM5".to_string(), "MX Master 3S".to_string()],
                device_details: vec![
                    crate::services::controls::BluetoothConnectedDevice {
                        address: "AA:BB:CC:DD:EE:FF".to_string(),
                        name: "WH-1000XM5".to_string(),
                        connected: true,
                        paired: true,
                        trusted: true,
                        battery_percent: Some(90),
                        audio_profiles: vec!["A2DP".to_string(), "AVRCP".to_string()],
                    },
                    crate::services::controls::BluetoothConnectedDevice {
                        address: "11:22:33:44:55:66".to_string(),
                        name: "MX Master 3S".to_string(),
                        connected: true,
                        paired: true,
                        trusted: false,
                        battery_percent: Some(55),
                        audio_profiles: Vec::new(),
                    },
                ],
            },
        });

        assert_eq!(items[0].icon, "");
        assert_eq!(items[0].value, "73% output");
        assert_eq!(items[0].detail.as_deref(), Some("Built-in Audio"));
        assert_eq!(items[1].icon, "");
        assert_eq!(items[1].value, "Muted");
        assert_eq!(items[1].detail.as_deref(), Some("Internal Microphone"));
        assert_eq!(items[2].icon, "󰂯");
        assert_eq!(items[2].value, "2 connected");
        assert_eq!(items[2].detail.as_deref(), Some("WH-1000XM5, MX Master 3S"));
    }

    #[test]
    fn main_bar_model_preserves_top_bar_pill_state() {
        let mut bar_state = hermetic_bar();
        bar_state.clock = "Sat 28 Mar 00:32".to_string();
        bar_state.controls.brightness =
            crate::services::controls::BrightnessSnapshot::from_percent(100);
        bar_state.controls.audio = crate::services::controls::AudioInfo {
            volume: 40,
            muted: false,
        };
        bar_state.controls.mic = crate::services::controls::MicInfo {
            volume: 36,
            muted: true,
        };
        bar_state.controls.battery.capacity = 96;
        bar_state.controls.power_profile = "BAL".to_string();
        bar_state.controls.bluetooth_enabled = true;
        bar_state.controls.bluetooth_devices.connected_devices = vec!["WH-1000XM5".to_string()];
        bar_state.audio_visualizer = crate::services::audio_visualizer::AudioVisualizerSnapshot {
            bars: [8; 24],
            visible_bars: 16,
            active: true,
        };
        bar_state.system_info_service =
            crate::services::system_info::SystemInfoService::with_snapshot(
                crate::modules::system::SysData {
                    cpu_str: "12%".to_string(),
                    temp: 34.0,
                    temp_str: "34°C".to_string(),
                    ..crate::modules::system::SysData::default()
                },
            );
        bar_state.network_service.handle_event(
            crate::services::network::NetworkEvent::WifiInfoSynced(
                crate::services::network::WifiInfo {
                    enabled: true,
                    ssid: "y83Etz9_Long".to_string(),
                },
            ),
        );

        let model = bar_state.build_main_bar_model();

        assert_eq!(model.stats.cpu_summary, "12%");
        assert_eq!(model.controls.brightness_label, "100%");
        assert_eq!(model.controls.volume_label, "40%");
        assert_eq!(model.controls.mic_icon, "󰍭");
        assert_eq!(model.connectivity.wifi_label, "y83Etz9…");
        assert_eq!(model.connectivity.bluetooth_label, Some("1".to_string()));
        assert_eq!(model.battery.battery_label, "96%");
        assert!(model.visualizer.is_visible());
        assert_eq!(model.clock, "Sat 28 Mar 00:32");
        assert_eq!(bar::volume_icon(&bar_state.controls.audio), "");
    }

    #[test]
    fn calendar_popup_builder_preserves_month_navigation_shape() {
        let bar_state = hermetic_bar();
        let model = bar_state.build_calendar_popup_model().unwrap();

        assert_eq!(model.prev_icon, "<");
        assert_eq!(model.next_icon, ">");
        assert!(!model.month_name.is_empty());
        assert!(!model.weeks.is_empty());
    }

    #[test]
    fn tray_menu_popup_builder_keeps_nested_labels_and_separators() {
        let mut bar_state = hermetic_bar();
        bar_state.tray_ui_service.set_open_menu_for_tests(Some(
            crate::services::tray_menu::OwnedTrayMenu::new_for_tests(vec![
                crate::services::tray_menu::OwnedTrayMenuNode::Item(
                    crate::services::tray_menu::OwnedTrayMenuItem {
                        id: 1,
                        label: "Open".to_string(),
                        enabled: true,
                        activatable: true,
                        children: Vec::new(),
                    },
                ),
                crate::services::tray_menu::OwnedTrayMenuNode::Separator,
                crate::services::tray_menu::OwnedTrayMenuNode::Item(
                    crate::services::tray_menu::OwnedTrayMenuItem {
                        id: 2,
                        label: "Audio".to_string(),
                        enabled: true,
                        activatable: false,
                        children: vec![crate::services::tray_menu::OwnedTrayMenuNode::Item(
                            crate::services::tray_menu::OwnedTrayMenuItem {
                                id: 3,
                                label: "Headphones".to_string(),
                                enabled: true,
                                activatable: true,
                                children: Vec::new(),
                            },
                        )],
                    },
                ),
            ]),
        ));

        let model = bar_state.build_tray_menu_popup_model();

        assert_eq!(model.rows.len(), 3);
        assert!(matches!(
            model.rows[1],
            crate::ui::popups::tray_menu::TrayMenuRow::Separator
        ));
        assert!(matches!(
            &model.rows[2],
            crate::ui::popups::tray_menu::TrayMenuRow::Action(action) if action.label == "Audio" && action.has_children
        ));
    }

    #[test]
    fn bluetooth_device_cards_surface_battery_and_profiles() {
        let cards =
            crate::ui::popups::bluetooth_devices::BluetoothDevicesPopupModel::build_device_cards(
                &crate::services::controls::BluetoothDeviceSummary {
                    connected_devices: vec!["WH-1000XM5".to_string()],
                    device_details: vec![crate::services::controls::BluetoothConnectedDevice {
                        address: "AA:BB:CC:DD:EE:FF".to_string(),
                        name: "WH-1000XM5".to_string(),
                        connected: true,
                        paired: true,
                        trusted: true,
                        battery_percent: Some(90),
                        audio_profiles: vec!["A2DP".to_string(), "AVRCP".to_string()],
                    }],
                },
            );

        assert_eq!(
            cards,
            vec![crate::ui::popups::bluetooth_devices::BluetoothDeviceCard {
                address: "AA:BB:CC:DD:EE:FF".to_string(),
                label: "WH-1000XM5".to_string(),
                summary: "Connected • Battery 90%".to_string(),
                detail: Some("AA:BB:CC:DD:EE:FF • A2DP • AVRCP".to_string()),
                badges: vec![
                    "CONNECTED".to_string(),
                    "PAIRED".to_string(),
                    "TRUSTED".to_string(),
                    "BAT 90%".to_string(),
                ],
                connected: true,
                paired: true,
                trusted: true,
                is_new: false,
            }]
        );
    }

    #[test]
    fn audio_route_popup_items_mark_current_default() {
        let items = audio_routes::popup_items(
            &[
                crate::services::controls::AudioRouteInfo {
                    id: "52".to_string(),
                    name: "Built-in Audio".to_string(),
                    origin: crate::services::controls::AudioRouteOrigin::Internal,
                },
                crate::services::controls::AudioRouteInfo {
                    id: "77".to_string(),
                    name: "USB DAC".to_string(),
                    origin: crate::services::controls::AudioRouteOrigin::Usb,
                },
            ],
            &[],
            Some("USB DAC"),
            "SINK",
            "No output routes discovered",
        );

        assert_eq!(
            items,
            vec![
                audio_routes::AudioRoutePopupItem {
                    id: "52".to_string(),
                    label: "Built-in Audio".to_string(),
                    icon: "󰓃",
                    capability_label: "SINK",
                    origin_label: "INTERNAL",
                    profile_label: "ANALOG",
                    status_label: "AVAILABLE",
                    warning_label: None,
                    detail: "Internal route • Integrated device path".to_string(),
                    is_default: false,
                    available: true,
                },
                audio_routes::AudioRoutePopupItem {
                    id: "77".to_string(),
                    label: "USB DAC".to_string(),
                    icon: "󰕓",
                    capability_label: "SINK",
                    origin_label: "USB",
                    profile_label: "USB",
                    status_label: "ACTIVE",
                    warning_label: None,
                    detail: "USB route • Low-latency external path".to_string(),
                    is_default: true,
                    available: true,
                },
            ]
        );
    }

    #[test]
    fn audio_route_popup_items_surface_unavailable_capability() {
        let items = audio_routes::popup_items(
            &[],
            &[crate::services::controls::AudioRouteInfo {
                id: "77".to_string(),
                name: "WH-1000XM5 a2dp-sink".to_string(),
                origin: crate::services::controls::AudioRouteOrigin::Bluetooth,
            }],
            None,
            "SOURCE",
            "No input routes discovered",
        );

        assert_eq!(
            items,
            vec![audio_routes::AudioRoutePopupItem {
                id: String::new(),
                label: "No input routes discovered".to_string(),
                icon: "󰖪",
                capability_label: "SOURCE",
                origin_label: "N/A",
                profile_label: "N/A",
                status_label: "UNAVAILABLE",
                warning_label: Some("WHY"),
                detail: "SOURCE unavailable: Bluetooth media profile hides microphone path"
                    .to_string(),
                is_default: false,
                available: false,
            }]
        );
    }

    #[test]
    fn audio_route_popup_items_surface_origin_icons_and_status() {
        let items = audio_routes::popup_items(
            &[crate::services::controls::AudioRouteInfo {
                id: "77".to_string(),
                name: "WH-1000XM5 a2dp-sink".to_string(),
                origin: crate::services::controls::AudioRouteOrigin::Bluetooth,
            }],
            &[],
            None,
            "SINK",
            "No output routes discovered",
        );

        assert_eq!(items[0].icon, "󰂯");
        assert_eq!(items[0].origin_label, "BT");
        assert_eq!(items[0].profile_label, "A2DP");
        assert_eq!(items[0].status_label, "AVAILABLE");
        assert_eq!(items[0].warning_label, Some("NO MIC"));
        assert_eq!(
            items[0].detail,
            "Bluetooth route • Higher-latency media path • microphone path unavailable"
        );
    }

    #[test]
    fn audio_route_popup_items_surface_call_profile_conflict() {
        let items = audio_routes::popup_items(
            &[crate::services::controls::AudioRouteInfo {
                id: "80".to_string(),
                name: "WH-1000XM5 handsfree-head-unit".to_string(),
                origin: crate::services::controls::AudioRouteOrigin::Bluetooth,
            }],
            &[],
            Some("WH-1000XM5 handsfree-head-unit"),
            "SOURCE",
            "No input routes discovered",
        );

        assert_eq!(items[0].profile_label, "HFP");
        assert_eq!(items[0].warning_label, Some("LOW FIDELITY"));
        assert_eq!(
            items[0].detail,
            "Bluetooth route • Low-latency call path • reduced media quality"
        );
    }

    #[test]
    fn route_popup_items_group_by_origin_family_in_stable_order() {
        let items = vec![
            audio_routes::AudioRoutePopupItem {
                id: "1".to_string(),
                label: "WH-1000XM5".to_string(),
                icon: "󰂯",
                capability_label: "SINK",
                origin_label: "BT",
                profile_label: "BT",
                status_label: "ACTIVE",
                warning_label: None,
                detail: "Bluetooth route".to_string(),
                is_default: true,
                available: true,
            },
            audio_routes::AudioRoutePopupItem {
                id: "2".to_string(),
                label: "MX Keys".to_string(),
                icon: "󰂯",
                capability_label: "SOURCE",
                origin_label: "BT",
                profile_label: "BT",
                status_label: "AVAILABLE",
                warning_label: None,
                detail: "Bluetooth route".to_string(),
                is_default: false,
                available: true,
            },
            audio_routes::AudioRoutePopupItem {
                id: "3".to_string(),
                label: "USB DAC".to_string(),
                icon: "󰕓",
                capability_label: "SINK",
                origin_label: "USB",
                profile_label: "USB",
                status_label: "AVAILABLE",
                warning_label: None,
                detail: "USB route".to_string(),
                is_default: false,
                available: true,
            },
        ];

        let groups = audio_routes::group_by_origin(&items);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].0, "BT");
        assert_eq!(groups[0].1.len(), 2);
        assert_eq!(groups[1].0, "USB");
        assert_eq!(groups[1].1.len(), 1);
    }

    #[test]
    fn current_audio_route_summary_prefers_route_origin_when_known() {
        let routes = vec![
            crate::services::controls::AudioRouteInfo {
                id: "52".to_string(),
                name: "Built-in Audio".to_string(),
                origin: crate::services::controls::AudioRouteOrigin::Internal,
            },
            crate::services::controls::AudioRouteInfo {
                id: "77".to_string(),
                name: "WH-1000XM5".to_string(),
                origin: crate::services::controls::AudioRouteOrigin::Bluetooth,
            },
        ];

        assert_eq!(
            audio_routes::current_route_summary(
                &routes,
                Some("WH-1000XM5"),
                "No output route selected"
            ),
            "WH-1000XM5 • Bluetooth route • Wireless path"
        );
        assert_eq!(
            audio_routes::current_route_summary(&routes, None, "No output route selected"),
            "No output route selected"
        );
    }

    #[test]
    fn audio_route_button_label_reflects_availability_shape() {
        let mut controls = crate::services::controls::ControlsSnapshot::default();
        assert_eq!(
            audio_routes::route_button_label(&controls),
            "Routes Unavailable"
        );

        controls.audio_devices.output_routes = vec![crate::services::controls::AudioRouteInfo {
            id: "52".to_string(),
            name: "Built-in Audio".to_string(),
            origin: crate::services::controls::AudioRouteOrigin::Internal,
        }];
        assert_eq!(
            audio_routes::route_button_label(&controls),
            "Partial Routes"
        );

        controls.audio_devices.input_routes = vec![crate::services::controls::AudioRouteInfo {
            id: "54".to_string(),
            name: "Internal Microphone".to_string(),
            origin: crate::services::controls::AudioRouteOrigin::Internal,
        }];
        assert_eq!(audio_routes::route_button_label(&controls), "Audio Routes");
    }

    #[test]
    fn display_summary_rows_include_mode_and_hotplug_aware_state() {
        let rows =
            displays::summary_rows(&crate::services::wayland_runtime::WaylandRuntimeSnapshot {
                available: true,
                outputs: vec![
                    crate::services::wayland_runtime::WaylandOutputInfo {
                        global_name: 1,
                        version: 4,
                        name: Some("eDP-1".to_string()),
                        scale_factor: Some(2),
                        ..crate::services::wayland_runtime::WaylandOutputInfo::default()
                    },
                    crate::services::wayland_runtime::WaylandOutputInfo {
                        global_name: 2,
                        version: 4,
                        name: Some("DP-2".to_string()),
                        scale_factor: Some(1),
                        ..crate::services::wayland_runtime::WaylandOutputInfo::default()
                    },
                ],
                ..crate::services::wayland_runtime::WaylandRuntimeSnapshot::default()
            });

        assert_eq!(
            rows.iter().map(|row| row.icon).collect::<Vec<_>>(),
            vec!["DSP", "TOP", "SCL", "OUT"]
        );
        assert_eq!(
            rows.iter().map(|row| row.label).collect::<Vec<_>>(),
            vec![
                "Display Mode",
                "Display Topology",
                "Display Scale",
                "Display Outputs",
            ]
        );
        assert_eq!(rows[0].value, "Hybrid");
        assert_eq!(rows[1].value, "1 internal + 1 external");
        assert_eq!(rows[2].value, "eDP-1 2x, DP-2 1x");
        assert_eq!(rows[3].value, "2 outputs: eDP-1, DP-2");
    }

    #[test]
    fn display_popup_output_cards_include_status_badges() {
        let cards =
            displays::output_cards(&crate::services::wayland_runtime::WaylandRuntimeSnapshot {
                available: true,
                outputs: vec![
                    crate::services::wayland_runtime::WaylandOutputInfo {
                        global_name: 1,
                        version: 4,
                        name: Some("eDP-1".to_string()),
                        width: Some(1920),
                        height: Some(1200),
                        refresh_mhz: Some(60000),
                        scale_factor: Some(2),
                        ..crate::services::wayland_runtime::WaylandOutputInfo::default()
                    },
                    crate::services::wayland_runtime::WaylandOutputInfo {
                        global_name: 2,
                        version: 4,
                        name: Some("DP-2".to_string()),
                        width: Some(2560),
                        height: Some(1440),
                        refresh_mhz: Some(144000),
                        scale_factor: Some(1),
                        ..crate::services::wayland_runtime::WaylandOutputInfo::default()
                    },
                ],
                ..crate::services::wayland_runtime::WaylandRuntimeSnapshot::default()
            });

        assert_eq!(cards[0].label, "eDP-1");
        assert_eq!(cards[0].summary, "eDP-1 1920x1200 60Hz 2x");
        assert_eq!(
            cards[0].badges,
            vec![
                "INTERNAL".to_string(),
                "1920x1200".to_string(),
                "60Hz".to_string(),
                "2x SCALE".to_string(),
            ]
        );

        assert_eq!(cards[1].label, "DP-2");
        assert_eq!(
            cards[1].badges,
            vec![
                "EXTERNAL".to_string(),
                "2560x1440".to_string(),
                "144Hz".to_string(),
                "1x SCALE".to_string(),
            ]
        );
    }

    #[test]
    fn detail_popup_header_action_only_for_displays() {
        assert_eq!(
            crate::ui::chrome::detail_popup_header_action(&Popup::Displays).map(|action| (
                action.icon,
                action.label,
                action.target
            )),
            Some(("󰈈", "System Info", Popup::SystemMonitor))
        );
        assert_eq!(
            crate::ui::chrome::detail_popup_header_action(&Popup::AudioRoutes).map(|action| (
                action.icon,
                action.label,
                action.target
            )),
            None
        );
        assert_eq!(
            crate::ui::chrome::detail_popup_header_action(&Popup::BluetoothDevices).map(|action| (
                action.icon,
                action.label,
                action.target
            )),
            None
        );
    }

    #[test]
    fn domain_popup_nav_items_cover_variant_a_domains() {
        assert_eq!(
            crate::ui::chrome::domain_popup_nav_items(&Popup::Controls),
            [
                ("", "Stats", Popup::Stats, false),
                ("", "Power", Popup::Power, false),
                ("󰖀", "Controls", Popup::Controls, true),
                ("󰖩", "Connectivity", Popup::Connectivity, false),
            ]
        );
    }

    #[test]
    fn domain_nav_focus_maps_detail_popups_to_primary_domains() {
        assert_eq!(
            crate::ui::chrome::domain_nav_focus_popup(&Popup::AudioRoutes),
            Popup::Controls
        );
        assert_eq!(
            crate::ui::chrome::domain_nav_focus_popup(&Popup::Displays),
            Popup::Controls
        );
        assert_eq!(
            crate::ui::chrome::domain_nav_focus_popup(&Popup::BluetoothDevices),
            Popup::Connectivity
        );
        assert_eq!(
            crate::ui::chrome::domain_nav_focus_popup(&Popup::SystemMonitor),
            Popup::Stats
        );
    }

    #[test]
    fn display_pill_summary_uses_wayland_mode_and_hides_when_unavailable() {
        assert_eq!(
            chrome::display_pill_summary(
                &crate::services::wayland_runtime::WaylandRuntimeSnapshot {
                    available: false,
                    unavailable_reason: Some("no wayland".to_string()),
                    ..crate::services::wayland_runtime::WaylandRuntimeSnapshot::default()
                }
            ),
            None
        );

        assert_eq!(
            chrome::display_pill_summary(
                &crate::services::wayland_runtime::WaylandRuntimeSnapshot {
                    available: true,
                    outputs: vec![
                        crate::services::wayland_runtime::WaylandOutputInfo {
                            global_name: 1,
                            version: 4,
                            name: Some("eDP-1".to_string()),
                            ..crate::services::wayland_runtime::WaylandOutputInfo::default()
                        },
                        crate::services::wayland_runtime::WaylandOutputInfo {
                            global_name: 2,
                            version: 4,
                            name: Some("HDMI-A-1".to_string()),
                            ..crate::services::wayland_runtime::WaylandOutputInfo::default()
                        },
                    ],
                    ..crate::services::wayland_runtime::WaylandRuntimeSnapshot::default()
                }
            ),
            Some(("󰍺", "Hybrid".to_string()))
        );
    }

    #[test]
    fn bluetooth_pill_summary_hides_text_when_adapter_disabled() {
        let mut controls = crate::services::controls::ControlsSnapshot {
            bluetooth_enabled: false,
            ..crate::services::controls::ControlsSnapshot::default()
        };
        assert_eq!(chrome::bluetooth_pill_summary(&controls), None);

        controls.bluetooth_enabled = true;
        assert_eq!(chrome::bluetooth_pill_summary(&controls), None);

        controls.bluetooth_devices.connected_devices =
            vec!["WH-1000XM5".to_string(), "MX Anywhere 3".to_string()];
        assert_eq!(
            chrome::bluetooth_pill_summary(&controls),
            Some("2".to_string())
        );
    }

    #[test]
    fn controls_domain_state_marks_only_controls_domain() {
        let state = ThinkPadBar::popup_domain_state(&Popup::Controls);
        assert!(!state.is_power_popup);
        assert!(state.is_controls_popup);
        assert!(!state.is_connectivity_popup);
    }

    #[test]
    fn opening_controls_popup_requests_audio_refresh() {
        let mut bar = hermetic_bar();

        let _ = bar.update(Message::TogglePopup(Popup::Controls));

        assert!(bar
            .controls_refresh_coalescing
            .is_inflight(&crate::services::controls::ControlsRefreshKind::AudioMic));
        assert!(!bar
            .controls_refresh_coalescing
            .is_inflight(&crate::services::controls::ControlsRefreshKind::BatteryPower));
    }

    #[test]
    fn opening_power_popup_requests_battery_refresh() {
        let mut bar = hermetic_bar();

        let _ = bar.update(Message::TogglePopup(Popup::Power));

        assert!(bar
            .controls_refresh_coalescing
            .is_inflight(&crate::services::controls::ControlsRefreshKind::BatteryPower));
        assert!(!bar
            .controls_refresh_coalescing
            .is_inflight(&crate::services::controls::ControlsRefreshKind::Bluetooth));
    }

    #[test]
    fn power_popup_builder_preserves_power_domain_state() {
        let mut bar = hermetic_bar();
        bar.controls.battery.capacity = 82;
        bar.controls.power_profile = "low-power".to_string();
        bar.controls.fan = crate::services::controls::FanInfo {
            speed: "2800".to_string(),
            level: "2".to_string(),
        };

        let model = bar.build_power_popup_model();

        assert_eq!(model.battery.capacity, 82);
        assert_eq!(model.power_profile, "low-power");
        assert_eq!(model.fan.speed, "2800");
    }

    #[test]
    fn controls_popup_builder_preserves_controls_domain_state() {
        let mut bar = hermetic_bar();
        bar.controls.brightness = crate::services::controls::BrightnessSnapshot::from_percent(64);
        bar.controls.audio = crate::services::controls::AudioInfo {
            volume: 44,
            muted: false,
        };
        bar.controls.mic = crate::services::controls::MicInfo {
            volume: 19,
            muted: true,
        };

        let model = bar.build_controls_popup_model();

        assert_eq!(model.brightness.percent, 64);
        assert_eq!(model.audio.volume, 44);
        assert_eq!(model.mic.volume, 19);
        assert!(model.mic.muted);
    }

    #[test]
    fn preview_controls_command_syncs_app_controls_snapshot() {
        let mut bar = hermetic_bar();

        let command =
            crate::services::controls::ControlsCommand::SetPowerProfile("performance".to_string());
        bar.preview_controls_command(&command);

        assert_eq!(bar.controls.power_profile, "performance");
        assert_eq!(bar.controls_service.snapshot().power_profile, "performance");
    }

    #[test]
    fn popup_host_builder_routes_power_popup_to_power_variant() {
        let mut bar = hermetic_bar();
        bar.popup = Popup::Power;

        assert!(matches!(
            bar.build_popup_host_model(),
            popup_host::PopupHostModel::Power(_)
        ));
    }

    #[test]
    fn closing_bluetooth_popup_resets_scan_state() {
        let mut bar = hermetic_bar();
        bar.popup = Popup::BluetoothDevices;
        bar.bluetooth_scan_state = super::BluetoothScanState::Completed {
            total_devices: 2,
            newly_discovered_addresses: vec!["11:22:33:44:55:66".to_string()],
            remaining_secs: 5,
        };

        let _ = bar.update(Message::TogglePopup(Popup::BluetoothDevices));

        assert_eq!(bar.popup, Popup::None);
        assert_eq!(bar.bluetooth_scan_state, super::BluetoothScanState::Idle);
    }

    #[test]
    fn closing_calendar_popup_resets_offset() {
        let mut bar = hermetic_bar();
        bar.popup = Popup::Calendar;
        bar.calendar_offset = 3;

        let _ = bar.update(Message::TogglePopup(Popup::Calendar));

        assert_eq!(bar.popup, Popup::None);
        assert_eq!(bar.calendar_offset, 0);
    }

    #[test]
    fn stats_popup_builder_normalizes_runtime_summaries() {
        let mut bar = hermetic_bar();
        bar.system_info_service = crate::services::system_info::SystemInfoService::with_snapshot(
            crate::modules::system::SysData {
                cpu_usage: 12.4,
                cpu_str: " ".to_string(),
                mem_str: String::new(),
                mem_used: 4,
                mem_total: 8,
                temp: 39.6,
                temp_str: String::new(),
                ..crate::modules::system::SysData::default()
            },
        );
        bar.controls.fan = crate::services::controls::FanInfo {
            speed: "2700".to_string(),
            level: "2".to_string(),
        };

        let model = bar.build_stats_popup_model();

        assert_eq!(model.rows[0].value, "12%");
        assert_eq!(model.rows[1].value, "50%");
        assert_eq!(model.rows[2].value, "40°C");
        assert_eq!(model.rows[3].value, "2700 RPM (2)");
    }

    #[test]
    fn displays_popup_builder_preserves_summary_and_cards() {
        let bar = hermetic_bar();
        let snapshot = crate::services::wayland_runtime::WaylandRuntimeSnapshot {
            available: true,
            outputs: vec![crate::services::wayland_runtime::WaylandOutputInfo {
                global_name: 1,
                version: 4,
                name: Some("eDP-1".to_string()),
                description: None,
                make: None,
                model: None,
                width: Some(1920),
                height: Some(1200),
                refresh_mhz: Some(60000),
                scale_factor: Some(2),
            }],
            ..crate::services::wayland_runtime::WaylandRuntimeSnapshot::default()
        };

        let model = bar.build_displays_popup_model(&snapshot);

        assert!(!model.summary_rows.is_empty());
        assert_eq!(model.output_cards.len(), 1);
        assert_eq!(model.output_cards[0].label, "eDP-1");
    }

    #[test]
    fn audio_routes_popup_builder_preserves_active_route_summaries() {
        let mut bar = hermetic_bar();
        bar.controls.audio_devices.output_routes =
            vec![crate::services::controls::AudioRouteInfo {
                id: "52".to_string(),
                name: "Built-in Audio".to_string(),
                origin: crate::services::controls::AudioRouteOrigin::Internal,
            }];
        bar.controls.audio_devices.input_routes = vec![crate::services::controls::AudioRouteInfo {
            id: "54".to_string(),
            name: "Internal Microphone".to_string(),
            origin: crate::services::controls::AudioRouteOrigin::Internal,
        }];
        bar.controls.audio_devices.output_route = Some("Built-in Audio".to_string());
        bar.controls.audio_devices.input_route = Some("Internal Microphone".to_string());

        let model = bar.build_audio_routes_popup_model();

        assert!(model.output_summary.contains("Built-in Audio"));
        assert!(model.input_summary.contains("Internal Microphone"));
        assert_eq!(model.output_routes.len(), 1);
        assert_eq!(model.input_routes.len(), 1);
    }

    #[test]
    fn bluetooth_devices_popup_builder_marks_new_scan_results() {
        let mut bar = hermetic_bar();
        bar.controls.bluetooth_enabled = true;
        bar.controls.bluetooth_devices.connected_devices = vec!["WH-1000XM5".to_string()];
        bar.controls.bluetooth_devices.device_details =
            vec![crate::services::controls::BluetoothConnectedDevice {
                address: "AA:BB:CC:DD:EE:FF".to_string(),
                name: "WH-1000XM5".to_string(),
                connected: true,
                paired: true,
                trusted: true,
                battery_percent: Some(90),
                audio_profiles: vec!["A2DP".to_string()],
            }];
        bar.bluetooth_scan_state = super::BluetoothScanState::Completed {
            total_devices: 1,
            newly_discovered_addresses: vec!["AA:BB:CC:DD:EE:FF".to_string()],
            remaining_secs: 5,
        };

        let model = bar.build_bluetooth_devices_popup_model();

        assert_eq!(model.adapter_summary, "1 connected");
        assert_eq!(model.device_cards.len(), 1);
        assert!(model.device_cards[0].is_new);
    }

    #[test]
    fn connectivity_popup_builder_preserves_wifi_and_bluetooth_state() {
        let mut bar = hermetic_bar();
        bar.controls.bluetooth_enabled = true;
        bar.controls.bluetooth_devices.connected_devices = vec!["WH-1000XM5".to_string()];

        let model = bar.build_connectivity_popup_model();

        assert!(model.bluetooth_enabled);
        assert_eq!(model.bluetooth_devices.connected_devices.len(), 1);
    }

    #[test]
    fn system_info_popup_builder_includes_overview_and_hardware_rows() {
        let mut bar = hermetic_bar();
        bar.system_info_service = crate::services::system_info::SystemInfoService::with_snapshot(
            crate::modules::system::SysData {
                cpu_str: "11%".to_string(),
                mem_str: "26%".to_string(),
                temp_str: "40°C".to_string(),
                disk_root_str: "54%".to_string(),
                disk_boot_str: "50%".to_string(),
                ip_address: "192.168.100.13".to_string(),
                net_down_str: "0 KB/s".to_string(),
                net_up_str: "0 KB/s".to_string(),
                ..crate::modules::system::SysData::default()
            },
        );
        let compositor = crate::services::compositor::CompositorSnapshot {
            workspaces: Vec::new(),
            active_window: String::new(),
            special_workspace_visible: false,
            keyboard_layout: "US".to_string(),
            configured_backend: crate::services::compositor::CompositorBackendKind::Hyprland,
            active_backend: crate::services::compositor::CompositorBackendKind::Hyprland,
        };
        let wayland = crate::services::wayland_runtime::WaylandRuntimeSnapshot::default();

        let model = bar.build_system_info_popup_model(&compositor, &wayland);

        assert_eq!(model.version, env!("CARGO_PKG_VERSION"));
        assert!(!model.overview_rows.is_empty());
        assert!(!model.hardware_rows.is_empty());
    }

    #[test]
    fn opening_connectivity_popup_requests_bluetooth_refresh() {
        let mut bar = hermetic_bar();

        let _ = bar.update(Message::TogglePopup(Popup::Connectivity));

        assert!(bar
            .controls_refresh_coalescing
            .is_inflight(&crate::services::controls::ControlsRefreshKind::Bluetooth));
        assert!(!bar
            .controls_refresh_coalescing
            .is_inflight(&crate::services::controls::ControlsRefreshKind::BatteryPower));
    }

    #[test]
    fn opening_stats_popup_refreshes_system_info_fast() {
        let mut bar = hermetic_bar();
        assert_eq!(
            bar.system_info_service.diagnostics().last_refresh_kind,
            None
        );

        let _ = bar.update(Message::TogglePopup(Popup::Stats));

        assert_eq!(
            bar.system_info_service.diagnostics().last_refresh_kind,
            Some(crate::services::system_info::SystemInfoRefreshKind::Fast)
        );
    }

    #[test]
    fn opening_system_monitor_popup_refreshes_system_info_fast() {
        let mut bar = hermetic_bar();
        assert_eq!(
            bar.system_info_service.diagnostics().last_refresh_kind,
            None
        );

        let _ = bar.update(Message::TogglePopup(Popup::SystemMonitor));

        assert_eq!(
            bar.system_info_service.diagnostics().last_refresh_kind,
            Some(crate::services::system_info::SystemInfoRefreshKind::Fast)
        );
    }

    #[test]
    fn visualizer_updates_continue_while_popup_is_open() {
        let mut bar = hermetic_bar();
        assert!(bar.audio_visualizer_updates_enabled());

        bar.popup = Popup::SystemMonitor;
        // Visualizer should NOT pause anymore
        assert!(bar.audio_visualizer_updates_enabled());

        bar.popup = Popup::None;
        assert!(bar.audio_visualizer_updates_enabled());
    }

    #[test]
    fn tick_refreshes_system_info_fast_while_system_monitor_is_open() {
        let mut bar = hermetic_bar();
        bar.popup = Popup::SystemMonitor;

        let _ = bar.update(Message::Tick(chrono::Local::now()));

        assert_eq!(
            bar.system_info_service.diagnostics().last_refresh_kind,
            Some(crate::services::system_info::SystemInfoRefreshKind::Fast)
        );
    }

    #[test]
    fn opening_bluetooth_popup_requests_bluetooth_refresh() {
        let mut bar = hermetic_bar();

        let _ = bar.update(Message::TogglePopup(Popup::BluetoothDevices));

        assert!(bar
            .controls_refresh_coalescing
            .is_inflight(&crate::services::controls::ControlsRefreshKind::Bluetooth));
    }

    #[test]
    fn bluetooth_scan_state_transitions_to_completed_and_marks_new_devices() {
        let mut bar = hermetic_bar();
        bar.controls.bluetooth_devices = crate::services::controls::BluetoothDeviceSummary {
            connected_devices: vec!["WH-1000XM5".to_string()],
            device_details: vec![crate::services::controls::BluetoothConnectedDevice {
                address: "AA:BB:CC:DD:EE:FF".to_string(),
                name: "WH-1000XM5".to_string(),
                connected: true,
                paired: true,
                trusted: true,
                battery_percent: Some(90),
                audio_profiles: vec!["A2DP".to_string()],
            }],
        };
        bar.controls_service = crate::services::controls::ControlsService::with_snapshot_for_tests(
            bar.controls.clone(),
        );

        let _ = bar.update(Message::ScanBluetoothDevices);
        assert_eq!(
            bar.bluetooth_scan_state,
            super::BluetoothScanState::Scanning {
                remaining_secs: 5,
                baseline_addresses: vec!["AA:BB:CC:DD:EE:FF".to_string()],
            }
        );

        let refresh = crate::services::controls::ControlsRefresh {
            bluetooth_devices: Some(crate::services::controls::BluetoothDeviceSummary {
                connected_devices: vec!["WH-1000XM5".to_string()],
                device_details: vec![
                    crate::services::controls::BluetoothConnectedDevice {
                        address: "AA:BB:CC:DD:EE:FF".to_string(),
                        name: "WH-1000XM5".to_string(),
                        connected: true,
                        paired: true,
                        trusted: true,
                        battery_percent: Some(90),
                        audio_profiles: vec!["A2DP".to_string()],
                    },
                    crate::services::controls::BluetoothConnectedDevice {
                        address: "11:22:33:44:55:66".to_string(),
                        name: "New Keyboard".to_string(),
                        connected: false,
                        paired: false,
                        trusted: false,
                        battery_percent: None,
                        audio_profiles: vec![],
                    },
                ],
            }),
            ..crate::services::controls::ControlsRefresh::default()
        };

        let _ = bar.update(Message::ControlsRefreshed(
            crate::services::controls::ControlsRefreshKind::Bluetooth,
            Box::new(refresh),
        ));

        assert_eq!(
            bar.bluetooth_scan_state,
            super::BluetoothScanState::Completed {
                total_devices: 2,
                newly_discovered_addresses: vec!["11:22:33:44:55:66".to_string()],
                remaining_secs: super::BLUETOOTH_SCAN_RESULT_WINDOW_SECS,
            }
        );
        assert!(bluetooth_devices::bluetooth_device_is_new(
            &bar.bluetooth_scan_state,
            "11:22:33:44:55:66"
        ));
        assert_eq!(
            bluetooth_devices::scan_status_summary(&bar.bluetooth_scan_state),
            "2 devices known; 1 newly discovered • idle in 8s"
        );
    }

    #[test]
    fn bluetooth_scan_status_summary_counts_down_and_finishes() {
        assert_eq!(
            bluetooth_devices::scan_status_summary(&super::BluetoothScanState::Scanning {
                remaining_secs: super::BLUETOOTH_SCAN_WINDOW_SECS,
                baseline_addresses: vec![],
            }),
            "Scanning (5s left)"
        );
        assert_eq!(
            bluetooth_devices::scan_status_summary(&super::BluetoothScanState::Scanning {
                remaining_secs: 0,
                baseline_addresses: vec![],
            }),
            "Finishing scan..."
        );
        assert_eq!(
            bluetooth_devices::scan_status_summary(&super::BluetoothScanState::Completed {
                total_devices: 2,
                newly_discovered_addresses: vec!["11:22:33:44:55:66".to_string()],
                remaining_secs: super::BLUETOOTH_SCAN_RESULT_WINDOW_SECS,
            }),
            "2 devices known; 1 newly discovered • idle in 8s"
        );
    }

    #[test]
    fn current_audio_route_summary_surfaces_profile_and_latency() {
        let routes = vec![crate::services::controls::AudioRouteInfo {
            id: "77".to_string(),
            name: "WH-1000XM5 a2dp-sink".to_string(),
            origin: crate::services::controls::AudioRouteOrigin::Bluetooth,
        }];

        assert_eq!(
            audio_routes::current_route_summary(
                &routes,
                Some("WH-1000XM5 a2dp-sink"),
                "No output route selected"
            ),
            "WH-1000XM5 a2dp-sink • Bluetooth route • Higher-latency media path • microphone path unavailable"
        );
    }

    #[test]
    fn completed_bluetooth_scan_returns_to_idle_after_timeout() {
        let mut bar = hermetic_bar();
        bar.bluetooth_scan_state = super::BluetoothScanState::Completed {
            total_devices: 1,
            newly_discovered_addresses: vec!["11:22:33:44:55:66".to_string()],
            remaining_secs: 2,
        };

        let _ = bar.update(Message::Tick(chrono::Local::now()));
        assert_eq!(
            bar.bluetooth_scan_state,
            super::BluetoothScanState::Completed {
                total_devices: 1,
                newly_discovered_addresses: vec!["11:22:33:44:55:66".to_string()],
                remaining_secs: 1,
            }
        );

        let _ = bar.update(Message::Tick(chrono::Local::now()));
        assert_eq!(bar.bluetooth_scan_state, super::BluetoothScanState::Idle);
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
    fn compositor_refresh_does_not_close_open_popup_on_active_window_title_change() {
        let mut bar = hermetic_bar();
        bar.popup = Popup::Stats;

        let refresh = crate::services::compositor::RefreshResult {
            snapshot: crate::services::compositor::CompositorSnapshot {
                workspaces: Vec::new(),
                active_window: "terminal-progress-42%".to_string(),
                special_workspace_visible: false,
                keyboard_layout: "US".to_string(),
                configured_backend: crate::services::compositor::CompositorBackendKind::Hyprland,
                active_backend: crate::services::compositor::CompositorBackendKind::Hyprland,
            },
            elapsed_ms: 3,
        };

        let _ = bar.update(Message::CompositorRefreshed(refresh));

        assert_eq!(bar.popup, Popup::Stats);
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
    fn wayland_runtime_event_updates_snapshot() {
        let mut bar = hermetic_bar();
        let snapshot = crate::services::wayland_runtime::WaylandRuntimeSnapshot {
            available: true,
            outputs: vec![crate::services::wayland_runtime::WaylandOutputInfo {
                global_name: 1,
                version: 4,
                name: Some("eDP-1".to_string()),
                scale_factor: Some(2),
                ..crate::services::wayland_runtime::WaylandOutputInfo::default()
            }],
            ..crate::services::wayland_runtime::WaylandRuntimeSnapshot::default()
        };

        let _ = bar.update(Message::WaylandRuntimeEvent(
            crate::services::wayland_runtime::WaylandRuntimeEvent::SnapshotUpdated(
                snapshot.clone(),
            ),
        ));

        assert_eq!(bar.wayland_runtime_service.snapshot(), &snapshot);
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
    fn app_coalescing_diagnostics_reports_pending_work() {
        let mut bar = hermetic_bar();
        let _ = bar
            .controls_coalescing
            .push(CoalescedControlKind::Volume, 33);
        let _ = bar.slow_tick_coalescing.push(());
        assert!(bar
            .controls_refresh_coalescing
            .request(crate::services::controls::ControlsRefreshKind::Brightness));
        assert!(!bar
            .controls_refresh_coalescing
            .request(crate::services::controls::ControlsRefreshKind::Brightness));
        assert!(bar
            .background_request_coalescing
            .request(BackgroundRequestKind::DbusConnect));

        let diagnostics = bar.coalescing_diagnostics();
        assert_eq!(diagnostics.pending_control_flushes, 1);
        assert!(diagnostics.pending_slow_tick);
        assert_eq!(diagnostics.inflight_control_refreshes, 1);
        assert_eq!(diagnostics.queued_control_refreshes, 1);
        assert_eq!(diagnostics.inflight_background_requests, 1);
        assert_eq!(diagnostics.queued_background_requests, 0);
    }

    #[test]
    fn app_coalescing_diagnostics_summary_is_stable() {
        let mut bar = hermetic_bar();
        let _ = bar
            .controls_coalescing
            .push(CoalescedControlKind::Brightness, 77);
        let _ = bar.slow_tick_coalescing.push(());

        assert_eq!(
            bar.coalescing_diagnostics().summary(),
            "ctrl pending:1 refresh 0/0 bg 0/0 slow:true"
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

        let _ = bar.update(super::Message::ControlsRefreshed(kind, Box::default()));
        assert!(bar.controls_refresh_coalescing.is_inflight(&kind));
        assert!(!bar.controls_refresh_coalescing.is_queued(&kind));

        let _ = bar.update(super::Message::ControlsRefreshed(kind, Box::default()));
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
    fn power_profile_controls_event_triggers_power_refresh_flow() {
        let kind = crate::services::controls::ControlsRefreshKind::Power;
        let mut bar = hermetic_bar();

        let _ = bar.update(super::Message::ControlsEvent(
            crate::services::controls::ControlsEvent::PowerProfile,
        ));

        assert!(bar.controls_refresh_coalescing.is_inflight(&kind));
        assert!(!bar.controls_refresh_coalescing.is_queued(&kind));
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

    #[test]
    fn controls_event_refresh_kind_maps_bluetooth_event_to_bluetooth_refresh() {
        assert_eq!(
            controls_event_refresh_kind(crate::services::controls::ControlsEvent::Bluetooth),
            crate::services::controls::ControlsRefreshKind::Bluetooth
        );
    }

    #[test]
    fn calculate_marquee_returns_static_for_short_text() {
        let text = "Short Text";
        let res = ThinkPadBar::calculate_marquee(text, 25, 1000);
        assert_eq!(res, "Short Text");
    }

    #[test]
    fn calculate_marquee_shifts_long_text_cyclically() {
        let text = "This is a very long text that should marquee";
        let res0 = ThinkPadBar::calculate_marquee(text, 25, 0);
        assert_eq!(res0.chars().count(), 25);
        assert!(res0.starts_with("This is"));

        let res1 = ThinkPadBar::calculate_marquee(text, 25, 1500); // 3 chars shift (2 chars/sec)
        assert!(res1.starts_with("s is a ve"));
    }

    #[test]
    fn media_event_updates_snapshot() {
        let mut bar = hermetic_bar();
        let new_snap = crate::services::media::MediaSnapshot {
            title: "New Track".to_string(),
            artist: "Artist".to_string(),
            has_player: true,
            ..crate::services::media::MediaSnapshot::default()
        };

        let _ = bar.update(Message::MediaEvent(
            crate::services::media::MediaEvent::SnapshotUpdated(new_snap.clone()),
        ));

        assert_eq!(bar.media_snapshot.title, "New Track");
        assert!(bar.media_snapshot.has_player);
    }

    #[test]
    fn controls_event_refresh_kind_maps_brightness_event_to_brightness_refresh() {
        assert_eq!(
            controls_event_refresh_kind(crate::services::controls::ControlsEvent::Brightness),
            crate::services::controls::ControlsRefreshKind::Brightness
        );
    }
}
