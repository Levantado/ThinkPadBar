use std::sync::Arc;

pub use crate::modules::battery::BatteryInfo;
pub use crate::modules::fan::FanInfo;
pub use crate::modules::mic::MicInfo;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioInfo {
    pub volume: u32,
    pub muted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AudioRouteOrigin {
    Bluetooth,
    Usb,
    Internal,
    Hdmi,
    Virtual,
    #[default]
    Unknown,
}

impl AudioRouteOrigin {
    pub fn badge_label(self) -> &'static str {
        match self {
            Self::Bluetooth => "BT",
            Self::Usb => "USB",
            Self::Internal => "INTERNAL",
            Self::Hdmi => "HDMI",
            Self::Virtual => "VIRTUAL",
            Self::Unknown => "UNKNOWN",
        }
    }

    pub fn summary_label(self) -> &'static str {
        match self {
            Self::Bluetooth => "Bluetooth route",
            Self::Usb => "USB route",
            Self::Internal => "Internal route",
            Self::Hdmi => "Display/HDMI route",
            Self::Virtual => "Virtual route",
            Self::Unknown => "Unclassified route",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AudioRouteInfo {
    pub id: String,
    pub name: String,
    pub origin: AudioRouteOrigin,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AudioDeviceSummary {
    pub output_route: Option<String>,
    pub input_route: Option<String>,
    pub output_routes: Vec<AudioRouteInfo>,
    pub input_routes: Vec<AudioRouteInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BluetoothConnectedDevice {
    pub address: String,
    pub name: String,
    pub connected: bool,
    pub paired: bool,
    pub trusted: bool,
    pub battery_percent: Option<u8>,
    pub audio_profiles: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BluetoothDeviceSummary {
    pub connected_devices: Vec<String>,
    pub device_details: Vec<BluetoothConnectedDevice>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrightnessSnapshot {
    pub percent: u32,
    pub label: String,
}

impl Default for BrightnessSnapshot {
    fn default() -> Self {
        Self::from_percent(0)
    }
}

impl BrightnessSnapshot {
    pub fn from_percent(percent: u32) -> Self {
        let clamped = percent.clamp(0, 100);
        Self {
            percent: clamped,
            label: format!("{clamped}%"),
        }
    }

    #[cfg(test)]
    pub fn from_label(label: String) -> Self {
        let percent = label.trim_end_matches('%').parse::<u32>().unwrap_or(0);
        Self::from_percent(percent)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlsSnapshot {
    pub brightness: BrightnessSnapshot,
    pub audio: AudioInfo,
    pub audio_devices: AudioDeviceSummary,
    pub mic: MicInfo,
    pub fan: FanInfo,
    pub battery: BatteryInfo,
    pub power_profile: String,
    pub bluetooth_enabled: bool,
    pub bluetooth_devices: BluetoothDeviceSummary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlsDiagnostics {
    pub audio_backend: &'static str,
    pub audio_runtime: Option<String>,
    pub brightness_backend: &'static str,
    pub bluetooth_backend: &'static str,
    pub power_backend: &'static str,
    pub power_runtime: Option<String>,
}

impl ControlsDiagnostics {
    pub fn summary(&self) -> String {
        format!(
            "aud {} bri {} bt {} pwr {}",
            self.audio_backend, self.brightness_backend, self.bluetooth_backend, self.power_backend
        )
    }
}

impl Default for ControlsSnapshot {
    fn default() -> Self {
        Self {
            brightness: BrightnessSnapshot::default(),
            audio: AudioInfo {
                volume: 0,
                muted: false,
            },
            audio_devices: AudioDeviceSummary::default(),
            mic: MicInfo {
                volume: 0,
                muted: false,
            },
            fan: FanInfo {
                speed: "---".to_string(),
                level: "auto".to_string(),
            },
            battery: BatteryInfo {
                capacity: 0,
                status: "Unknown".to_string(),
                time_remaining: None,
                ac_online: None,
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
            bluetooth_enabled: false,
            bluetooth_devices: BluetoothDeviceSummary::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlsEvent {
    AudioServerChanged,
    PowerProfileChanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControlsRefreshKind {
    AudioMic,
    Brightness,
    Fan,
    BatteryPower,
    Power,
    Bluetooth,
    Slow,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ControlsRefresh {
    pub brightness: Option<BrightnessSnapshot>,
    pub audio: Option<AudioInfo>,
    pub audio_devices: Option<AudioDeviceSummary>,
    pub mic: Option<MicInfo>,
    pub fan: Option<FanInfo>,
    pub battery: Option<BatteryInfo>,
    pub power_profile: Option<String>,
    pub bluetooth_enabled: Option<bool>,
    pub bluetooth_devices: Option<BluetoothDeviceSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlsCommand {
    SetVolume(u32),
    ToggleAudioMute,
    SetAudioOutputRoute(String),
    SetMicVolume(u32),
    ToggleMicMute,
    SetAudioInputRoute(String),
    SetBrightness(u32),
    SetFanLevel(String),
    SetPowerProfile(String),
    ToggleBluetooth(bool),
    ScanBluetoothDevices,
    StopBluetoothScan,
    ConnectBluetoothDevice(String),
    DisconnectBluetoothDevice(String),
    PairBluetoothDevice(String),
    TrustBluetoothDevice(String),
    RemoveBluetoothDevice(String),
    OpenOverskride,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlsFollowUp {
    Refresh(ControlsRefreshKind),
    RefreshCompositor,
}

#[derive(Clone)]
pub struct ControlsService {
    snapshot: ControlsSnapshot,
    audio_backend: Arc<dyn crate::services::controls_backends::AudioBackend>,
    brightness_backend: Arc<dyn crate::services::controls_backends::BrightnessBackend>,
    bluetooth_backend: Arc<dyn crate::services::controls_backends::BluetoothBackend>,
    power_backend: Arc<dyn crate::services::controls_backends::PowerBackend>,
}

impl ControlsService {
    pub fn new() -> Self {
        Self::with_backends(
            Arc::new(crate::services::controls_backends::audio::WpctlAudioBackend::default()),
            Arc::new(crate::services::controls_backends::brightness::SysfsBrightnessBackend),
            Arc::new(crate::services::controls_backends::bluetooth::BluetoothCtlBackend),
            Arc::new(crate::services::controls_backends::power::PowerProfilesDaemonBackend),
        )
    }

    fn with_backends(
        audio_backend: Arc<dyn crate::services::controls_backends::AudioBackend>,
        brightness_backend: Arc<dyn crate::services::controls_backends::BrightnessBackend>,
        bluetooth_backend: Arc<dyn crate::services::controls_backends::BluetoothBackend>,
        power_backend: Arc<dyn crate::services::controls_backends::PowerBackend>,
    ) -> Self {
        Self {
            snapshot: ControlsSnapshot {
                brightness: brightness_backend.snapshot(),
                audio: audio_backend.audio_info(),
                audio_devices: audio_backend.device_summary(),
                mic: audio_backend.mic_info(),
                fan: crate::modules::fan::get_fan_info(),
                battery: crate::modules::battery::get_battery_info(),
                power_profile: power_backend.profile(),
                bluetooth_enabled: bluetooth_backend.enabled(),
                bluetooth_devices: bluetooth_backend.device_summary(),
            },
            audio_backend,
            brightness_backend,
            bluetooth_backend,
            power_backend,
        }
    }

    #[cfg(test)]
    pub fn with_snapshot_for_tests(snapshot: ControlsSnapshot) -> Self {
        Self {
            snapshot,
            audio_backend: Arc::new(NoopAudioBackend),
            brightness_backend: Arc::new(NoopBrightnessBackend),
            bluetooth_backend: Arc::new(NoopBluetoothBackend),
            power_backend: Arc::new(NoopPowerBackend),
        }
    }

    pub fn snapshot(&self) -> &ControlsSnapshot {
        &self.snapshot
    }

    pub fn diagnostics(&self) -> ControlsDiagnostics {
        ControlsDiagnostics {
            audio_backend: self.audio_backend.backend_name(),
            audio_runtime: self.audio_backend.diagnostics_summary(),
            brightness_backend: self.brightness_backend.backend_name(),
            bluetooth_backend: self.bluetooth_backend.backend_name(),
            power_backend: self.power_backend.backend_name(),
            power_runtime: self.power_backend.diagnostics_summary(),
        }
    }

    pub fn capability_statuses(&self) -> Vec<crate::services::capabilities::CapabilityStatus> {
        let mut statuses = vec![
            crate::services::capabilities::CapabilityStatus {
                key: "aud",
                label: "Audio",
                mode: self.audio_backend.capability_mode(),
                provider: self.audio_backend.backend_name().to_string(),
                detail: self.audio_backend.diagnostics_summary(),
            },
            crate::services::capabilities::CapabilityStatus {
                key: "bri",
                label: "Brightness",
                mode: self.brightness_backend.capability_mode(),
                provider: self.brightness_backend.backend_name().to_string(),
                detail: None,
            },
            crate::services::capabilities::CapabilityStatus {
                key: "bt",
                label: "Bluetooth",
                mode: self.bluetooth_backend.capability_mode(),
                provider: self.bluetooth_backend.backend_name().to_string(),
                detail: Some("cli/rfkill/sysfs control path".to_string()),
            },
            crate::services::capabilities::CapabilityStatus {
                key: "pwr",
                label: "Power Profile",
                mode: self.power_backend.capability_mode(),
                provider: self.power_backend.backend_name().to_string(),
                detail: self.power_backend.diagnostics_summary(),
            },
        ];

        let fan_available = std::path::Path::new("/proc/acpi/ibm/fan").exists();
        statuses.push(crate::services::capabilities::CapabilityStatus {
            key: "fan",
            label: "Fan Control",
            mode: if fan_available {
                crate::services::capabilities::CapabilityMode::Fallback
            } else {
                crate::services::capabilities::CapabilityMode::Unavailable
            },
            provider: "procfs+pkexec".to_string(),
            detail: (!fan_available).then(|| "thinkpad_acpi fan interface missing".to_string()),
        });

        let battery_care_exposed = self.snapshot.battery.charge_start_threshold.is_some()
            || self.snapshot.battery.charge_end_threshold.is_some();
        statuses.push(crate::services::capabilities::CapabilityStatus {
            key: "bat",
            label: "Battery Care",
            mode: if battery_care_exposed {
                crate::services::capabilities::CapabilityMode::ReadOnly
            } else {
                crate::services::capabilities::CapabilityMode::Unavailable
            },
            provider: "power_supply sysfs".to_string(),
            detail: if battery_care_exposed {
                Some("system-managed thresholds".to_string())
            } else {
                Some("threshold files unavailable".to_string())
            },
        });

        statuses
    }

    pub fn apply_refresh(&mut self, refresh: ControlsRefresh) {
        if let Some(brightness) = refresh.brightness {
            self.snapshot.brightness = brightness;
        }
        if let Some(audio) = refresh.audio {
            self.snapshot.audio = audio;
        }
        if let Some(audio_devices) = refresh.audio_devices {
            self.snapshot.audio_devices = audio_devices;
        }
        if let Some(mic) = refresh.mic {
            crate::modules::mic::update_led(mic.muted);
            self.snapshot.mic = mic;
        }
        if let Some(fan) = refresh.fan {
            self.snapshot.fan = fan;
        }
        if let Some(battery) = refresh.battery {
            self.snapshot.battery = battery;
        }
        if let Some(power_profile) = refresh.power_profile {
            self.snapshot.power_profile = power_profile;
        }
        if let Some(bluetooth_enabled) = refresh.bluetooth_enabled {
            self.snapshot.bluetooth_enabled = bluetooth_enabled;
        }
        if let Some(bluetooth_devices) = refresh.bluetooth_devices {
            self.snapshot.bluetooth_devices = bluetooth_devices;
        }
    }

    pub fn preview_command(&mut self, command: &ControlsCommand) {
        match command {
            ControlsCommand::SetVolume(volume) => {
                self.snapshot.audio.volume = *volume;
            }
            ControlsCommand::SetAudioOutputRoute(route_id) => {
                self.snapshot.audio_devices.output_route =
                    select_route_name(&self.snapshot.audio_devices.output_routes, route_id);
            }
            ControlsCommand::SetMicVolume(volume) => {
                self.snapshot.mic.volume = *volume;
            }
            ControlsCommand::SetAudioInputRoute(route_id) => {
                self.snapshot.audio_devices.input_route =
                    select_route_name(&self.snapshot.audio_devices.input_routes, route_id);
            }
            ControlsCommand::SetBrightness(percent) => {
                self.snapshot.brightness = BrightnessSnapshot::from_percent(*percent);
            }
            ControlsCommand::SetFanLevel(level) => {
                self.snapshot.fan.level = level.clone();
            }
            ControlsCommand::SetPowerProfile(profile) => {
                self.snapshot.power_profile = profile.clone();
            }
            ControlsCommand::ToggleBluetooth(enabled) => {
                self.snapshot.bluetooth_enabled = *enabled;
            }
            ControlsCommand::ScanBluetoothDevices | ControlsCommand::StopBluetoothScan => {}
            ControlsCommand::ConnectBluetoothDevice(address) => {
                update_bluetooth_device_connection_state(
                    &mut self.snapshot.bluetooth_devices,
                    address,
                    true,
                );
            }
            ControlsCommand::DisconnectBluetoothDevice(address) => {
                update_bluetooth_device_connection_state(
                    &mut self.snapshot.bluetooth_devices,
                    address,
                    false,
                );
            }
            ControlsCommand::PairBluetoothDevice(address) => {
                update_bluetooth_device_pair_state(
                    &mut self.snapshot.bluetooth_devices,
                    address,
                    true,
                );
            }
            ControlsCommand::TrustBluetoothDevice(address) => {
                update_bluetooth_device_trust_state(
                    &mut self.snapshot.bluetooth_devices,
                    address,
                    true,
                );
            }
            ControlsCommand::RemoveBluetoothDevice(address) => {
                remove_bluetooth_device(&mut self.snapshot.bluetooth_devices, address);
            }
            ControlsCommand::ToggleAudioMute
            | ControlsCommand::ToggleMicMute
            | ControlsCommand::OpenOverskride => {}
        }
    }

    pub async fn refresh(&self, kind: ControlsRefreshKind) -> ControlsRefresh {
        match kind {
            ControlsRefreshKind::AudioMic => ControlsRefresh {
                audio: Some(self.audio_backend.audio_info()),
                audio_devices: Some(self.audio_backend.device_summary()),
                mic: Some(self.audio_backend.mic_info()),
                ..ControlsRefresh::default()
            },
            ControlsRefreshKind::Brightness => ControlsRefresh {
                brightness: Some(self.brightness_backend.snapshot()),
                ..ControlsRefresh::default()
            },
            ControlsRefreshKind::Fan => ControlsRefresh {
                fan: Some(crate::modules::fan::get_fan_info()),
                ..ControlsRefresh::default()
            },
            ControlsRefreshKind::BatteryPower => ControlsRefresh {
                battery: Some(crate::modules::battery::get_battery_info()),
                power_profile: Some(self.power_backend.profile()),
                ..ControlsRefresh::default()
            },
            ControlsRefreshKind::Power => ControlsRefresh {
                power_profile: Some(self.power_backend.profile()),
                ..ControlsRefresh::default()
            },
            ControlsRefreshKind::Bluetooth => ControlsRefresh {
                bluetooth_enabled: Some(self.bluetooth_backend.enabled()),
                bluetooth_devices: Some(self.bluetooth_backend.device_summary()),
                ..ControlsRefresh::default()
            },
            ControlsRefreshKind::Slow => ControlsRefresh {
                brightness: Some(self.brightness_backend.snapshot()),
                battery: Some(crate::modules::battery::get_battery_info()),
                power_profile: Some(self.power_backend.profile()),
                bluetooth_enabled: Some(self.bluetooth_backend.enabled()),
                bluetooth_devices: Some(self.bluetooth_backend.device_summary()),
                ..ControlsRefresh::default()
            },
        }
    }

    pub async fn execute(&self, command: ControlsCommand) -> ControlsFollowUp {
        match command {
            ControlsCommand::SetVolume(volume) => {
                self.audio_backend.set_volume(volume).await;
                ControlsFollowUp::Refresh(ControlsRefreshKind::AudioMic)
            }
            ControlsCommand::ToggleAudioMute => {
                self.audio_backend.toggle_audio_mute().await;
                ControlsFollowUp::Refresh(ControlsRefreshKind::AudioMic)
            }
            ControlsCommand::SetAudioOutputRoute(route_id) => {
                let _ = self.audio_backend.set_output_route(route_id).await;
                ControlsFollowUp::Refresh(ControlsRefreshKind::AudioMic)
            }
            ControlsCommand::SetMicVolume(volume) => {
                self.audio_backend.set_mic_volume(volume).await;
                ControlsFollowUp::Refresh(ControlsRefreshKind::AudioMic)
            }
            ControlsCommand::ToggleMicMute => {
                self.audio_backend.toggle_mic_mute().await;
                ControlsFollowUp::Refresh(ControlsRefreshKind::AudioMic)
            }
            ControlsCommand::SetAudioInputRoute(route_id) => {
                let _ = self.audio_backend.set_input_route(route_id).await;
                ControlsFollowUp::Refresh(ControlsRefreshKind::AudioMic)
            }
            ControlsCommand::SetBrightness(percent) => {
                self.brightness_backend.set_brightness(percent);
                ControlsFollowUp::Refresh(ControlsRefreshKind::Brightness)
            }
            ControlsCommand::SetFanLevel(level) => {
                crate::modules::fan::set_fan_level(&level);
                ControlsFollowUp::Refresh(ControlsRefreshKind::Fan)
            }
            ControlsCommand::SetPowerProfile(profile) => {
                self.power_backend.set_profile(profile).await;
                ControlsFollowUp::Refresh(ControlsRefreshKind::Power)
            }
            ControlsCommand::ToggleBluetooth(enabled) => {
                let _ = self.bluetooth_backend.toggle(enabled);
                ControlsFollowUp::Refresh(ControlsRefreshKind::Bluetooth)
            }
            ControlsCommand::ScanBluetoothDevices => {
                let _ = self.bluetooth_backend.scan_devices().await;
                ControlsFollowUp::Refresh(ControlsRefreshKind::Bluetooth)
            }
            ControlsCommand::StopBluetoothScan => {
                let _ = self.bluetooth_backend.stop_scan_devices().await;
                ControlsFollowUp::Refresh(ControlsRefreshKind::Bluetooth)
            }
            ControlsCommand::ConnectBluetoothDevice(address) => {
                let _ = self.bluetooth_backend.connect_device(address).await;
                ControlsFollowUp::Refresh(ControlsRefreshKind::Bluetooth)
            }
            ControlsCommand::DisconnectBluetoothDevice(address) => {
                let _ = self.bluetooth_backend.disconnect_device(address).await;
                ControlsFollowUp::Refresh(ControlsRefreshKind::Bluetooth)
            }
            ControlsCommand::PairBluetoothDevice(address) => {
                let _ = self.bluetooth_backend.pair_device(address).await;
                ControlsFollowUp::Refresh(ControlsRefreshKind::Bluetooth)
            }
            ControlsCommand::TrustBluetoothDevice(address) => {
                let _ = self.bluetooth_backend.trust_device(address).await;
                ControlsFollowUp::Refresh(ControlsRefreshKind::Bluetooth)
            }
            ControlsCommand::RemoveBluetoothDevice(address) => {
                let _ = self.bluetooth_backend.remove_device(address).await;
                ControlsFollowUp::Refresh(ControlsRefreshKind::Bluetooth)
            }
            ControlsCommand::OpenOverskride => {
                let _ = self.bluetooth_backend.open_overskride();
                ControlsFollowUp::RefreshCompositor
            }
        }
    }

    pub fn subscription(&self) -> iced::Subscription<ControlsEvent> {
        iced::Subscription::batch([
            self.audio_backend.subscription(),
            self.power_backend.subscription(),
        ])
    }
}

fn select_route_name(routes: &[AudioRouteInfo], route_id: &str) -> Option<String> {
    routes
        .iter()
        .find(|route| route.id == route_id)
        .map(|route| route.name.clone())
}

fn update_bluetooth_device_connection_state(
    summary: &mut BluetoothDeviceSummary,
    address: &str,
    connected: bool,
) {
    for device in &mut summary.device_details {
        if device.address == address {
            device.connected = connected;
        }
    }

    summary.connected_devices = summary
        .device_details
        .iter()
        .filter(|device| device.connected)
        .map(|device| device.name.clone())
        .collect();
}

fn update_bluetooth_device_pair_state(
    summary: &mut BluetoothDeviceSummary,
    address: &str,
    paired: bool,
) {
    for device in &mut summary.device_details {
        if device.address == address {
            device.paired = paired;
        }
    }
}

fn update_bluetooth_device_trust_state(
    summary: &mut BluetoothDeviceSummary,
    address: &str,
    trusted: bool,
) {
    for device in &mut summary.device_details {
        if device.address == address {
            device.trusted = trusted;
        }
    }
}

fn remove_bluetooth_device(summary: &mut BluetoothDeviceSummary, address: &str) {
    summary
        .device_details
        .retain(|device| device.address != address);
    summary.connected_devices = summary
        .device_details
        .iter()
        .filter(|device| device.connected)
        .map(|device| device.name.clone())
        .collect();
}

impl Default for ControlsService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Copy)]
struct NoopAudioBackend;

#[cfg(test)]
impl crate::services::controls_backends::AudioBackend for NoopAudioBackend {
    fn backend_name(&self) -> &'static str {
        "noop-audio"
    }

    fn capability_mode(&self) -> crate::services::capabilities::CapabilityMode {
        crate::services::capabilities::CapabilityMode::Unavailable
    }

    fn audio_info(&self) -> AudioInfo {
        AudioInfo {
            volume: 0,
            muted: false,
        }
    }

    fn mic_info(&self) -> crate::modules::mic::MicInfo {
        crate::modules::mic::MicInfo {
            volume: 0,
            muted: false,
        }
    }

    fn device_summary(&self) -> AudioDeviceSummary {
        AudioDeviceSummary::default()
    }

    fn set_volume(
        &self,
        _percent: u32,
    ) -> crate::services::controls_backends::BackendFuture<'_, ()> {
        Box::pin(async {})
    }

    fn toggle_audio_mute(&self) -> crate::services::controls_backends::BackendFuture<'_, ()> {
        Box::pin(async {})
    }

    fn set_output_route(
        &self,
        _id: String,
    ) -> crate::services::controls_backends::BackendFuture<'_, bool> {
        Box::pin(async { false })
    }

    fn set_mic_volume(
        &self,
        _percent: u32,
    ) -> crate::services::controls_backends::BackendFuture<'_, ()> {
        Box::pin(async {})
    }

    fn toggle_mic_mute(&self) -> crate::services::controls_backends::BackendFuture<'_, ()> {
        Box::pin(async {})
    }

    fn set_input_route(
        &self,
        _id: String,
    ) -> crate::services::controls_backends::BackendFuture<'_, bool> {
        Box::pin(async { false })
    }

    fn subscription(&self) -> iced::Subscription<ControlsEvent> {
        iced::Subscription::none()
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Copy)]
struct NoopBrightnessBackend;

#[cfg(test)]
impl crate::services::controls_backends::BrightnessBackend for NoopBrightnessBackend {
    fn backend_name(&self) -> &'static str {
        "noop-brightness"
    }

    fn capability_mode(&self) -> crate::services::capabilities::CapabilityMode {
        crate::services::capabilities::CapabilityMode::Unavailable
    }

    fn snapshot(&self) -> BrightnessSnapshot {
        BrightnessSnapshot::default()
    }

    fn set_brightness(&self, _percent: u32) {}
}

#[cfg(test)]
#[derive(Debug, Clone, Copy)]
struct NoopBluetoothBackend;

#[cfg(test)]
impl crate::services::controls_backends::BluetoothBackend for NoopBluetoothBackend {
    fn backend_name(&self) -> &'static str {
        "noop-bluetooth"
    }

    fn capability_mode(&self) -> crate::services::capabilities::CapabilityMode {
        crate::services::capabilities::CapabilityMode::Unavailable
    }

    fn enabled(&self) -> bool {
        false
    }

    fn device_summary(&self) -> BluetoothDeviceSummary {
        BluetoothDeviceSummary::default()
    }

    fn toggle(&self, _enable: bool) -> bool {
        true
    }

    fn scan_devices(&self) -> crate::services::controls_backends::BackendFuture<'_, bool> {
        Box::pin(async { true })
    }

    fn stop_scan_devices(&self) -> crate::services::controls_backends::BackendFuture<'_, bool> {
        Box::pin(async { true })
    }

    fn connect_device(
        &self,
        _address: String,
    ) -> crate::services::controls_backends::BackendFuture<'_, bool> {
        Box::pin(async { true })
    }

    fn disconnect_device(
        &self,
        _address: String,
    ) -> crate::services::controls_backends::BackendFuture<'_, bool> {
        Box::pin(async { true })
    }

    fn pair_device(
        &self,
        _address: String,
    ) -> crate::services::controls_backends::BackendFuture<'_, bool> {
        Box::pin(async { true })
    }

    fn trust_device(
        &self,
        _address: String,
    ) -> crate::services::controls_backends::BackendFuture<'_, bool> {
        Box::pin(async { true })
    }

    fn remove_device(
        &self,
        _address: String,
    ) -> crate::services::controls_backends::BackendFuture<'_, bool> {
        Box::pin(async { true })
    }

    fn open_overskride(&self) -> bool {
        true
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Copy)]
struct NoopPowerBackend;

#[cfg(test)]
impl crate::services::controls_backends::PowerBackend for NoopPowerBackend {
    fn backend_name(&self) -> &'static str {
        "noop-power"
    }

    fn capability_mode(&self) -> crate::services::capabilities::CapabilityMode {
        crate::services::capabilities::CapabilityMode::Unavailable
    }

    fn diagnostics_summary(&self) -> Option<String> {
        None
    }

    fn profile(&self) -> String {
        "balanced".to_string()
    }

    fn set_profile(
        &self,
        _profile: String,
    ) -> crate::services::controls_backends::BackendFuture<'_, ()> {
        Box::pin(async {})
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AudioDeviceSummary, AudioInfo, AudioRouteInfo, AudioRouteOrigin, BluetoothConnectedDevice,
        BluetoothDeviceSummary, BrightnessSnapshot, ControlsCommand, ControlsRefresh,
        ControlsRefreshKind, ControlsService, ControlsSnapshot,
    };
    use crate::modules::mic::MicInfo;
    use std::sync::{Arc, Mutex};

    type SharedStringCalls = Arc<Mutex<Vec<String>>>;
    type SharedU32Calls = Arc<Mutex<Vec<u32>>>;
    type SharedBoolCalls = Arc<Mutex<Vec<bool>>>;
    type SharedCount = Arc<Mutex<u32>>;
    type SharedBluetoothCommandCalls = Arc<Mutex<Vec<String>>>;

    #[derive(Clone)]
    struct MockAudioBackend {
        audio: AudioInfo,
        devices: AudioDeviceSummary,
        mic: MicInfo,
        calls: SharedStringCalls,
    }

    impl crate::services::controls_backends::AudioBackend for MockAudioBackend {
        fn backend_name(&self) -> &'static str {
            "mock-audio"
        }

        fn capability_mode(&self) -> crate::services::capabilities::CapabilityMode {
            crate::services::capabilities::CapabilityMode::Hybrid
        }

        fn audio_info(&self) -> AudioInfo {
            self.audio.clone()
        }

        fn mic_info(&self) -> MicInfo {
            self.mic.clone()
        }

        fn device_summary(&self) -> AudioDeviceSummary {
            self.devices.clone()
        }

        fn set_volume(
            &self,
            percent: u32,
        ) -> crate::services::controls_backends::BackendFuture<'_, ()> {
            let calls = self.calls.clone();
            Box::pin(async move {
                calls.lock().unwrap().push(format!("set_volume:{percent}"));
            })
        }

        fn toggle_audio_mute(&self) -> crate::services::controls_backends::BackendFuture<'_, ()> {
            let calls = self.calls.clone();
            Box::pin(async move {
                calls.lock().unwrap().push("toggle_audio_mute".to_string());
            })
        }

        fn set_output_route(
            &self,
            id: String,
        ) -> crate::services::controls_backends::BackendFuture<'_, bool> {
            let calls = self.calls.clone();
            Box::pin(async move {
                calls.lock().unwrap().push(format!("set_output_route:{id}"));
                true
            })
        }

        fn set_mic_volume(
            &self,
            percent: u32,
        ) -> crate::services::controls_backends::BackendFuture<'_, ()> {
            let calls = self.calls.clone();
            Box::pin(async move {
                calls
                    .lock()
                    .unwrap()
                    .push(format!("set_mic_volume:{percent}"));
            })
        }

        fn toggle_mic_mute(&self) -> crate::services::controls_backends::BackendFuture<'_, ()> {
            let calls = self.calls.clone();
            Box::pin(async move {
                calls.lock().unwrap().push("toggle_mic_mute".to_string());
            })
        }

        fn set_input_route(
            &self,
            id: String,
        ) -> crate::services::controls_backends::BackendFuture<'_, bool> {
            let calls = self.calls.clone();
            Box::pin(async move {
                calls.lock().unwrap().push(format!("set_input_route:{id}"));
                true
            })
        }

        fn subscription(&self) -> iced::Subscription<super::ControlsEvent> {
            iced::Subscription::none()
        }
    }

    #[derive(Clone)]
    struct MockBrightnessBackend {
        snapshot: BrightnessSnapshot,
        calls: SharedU32Calls,
    }

    impl crate::services::controls_backends::BrightnessBackend for MockBrightnessBackend {
        fn backend_name(&self) -> &'static str {
            "mock-brightness"
        }

        fn capability_mode(&self) -> crate::services::capabilities::CapabilityMode {
            crate::services::capabilities::CapabilityMode::Fallback
        }

        fn snapshot(&self) -> BrightnessSnapshot {
            self.snapshot.clone()
        }

        fn set_brightness(&self, percent: u32) {
            self.calls.lock().unwrap().push(percent);
        }
    }

    #[derive(Clone)]
    struct MockBluetoothBackend {
        enabled: bool,
        devices: BluetoothDeviceSummary,
        toggle_calls: SharedBoolCalls,
        command_calls: SharedBluetoothCommandCalls,
        overskride_calls: SharedCount,
    }

    impl crate::services::controls_backends::BluetoothBackend for MockBluetoothBackend {
        fn backend_name(&self) -> &'static str {
            "mock-bluetooth"
        }

        fn capability_mode(&self) -> crate::services::capabilities::CapabilityMode {
            crate::services::capabilities::CapabilityMode::Fallback
        }

        fn enabled(&self) -> bool {
            self.enabled
        }

        fn device_summary(&self) -> BluetoothDeviceSummary {
            self.devices.clone()
        }

        fn toggle(&self, enable: bool) -> bool {
            self.toggle_calls.lock().unwrap().push(enable);
            true
        }

        fn scan_devices(&self) -> crate::services::controls_backends::BackendFuture<'_, bool> {
            let command_calls = self.command_calls.clone();
            Box::pin(async move {
                command_calls.lock().unwrap().push("scan".to_string());
                true
            })
        }

        fn stop_scan_devices(&self) -> crate::services::controls_backends::BackendFuture<'_, bool> {
            let command_calls = self.command_calls.clone();
            Box::pin(async move {
                command_calls.lock().unwrap().push("stop-scan".to_string());
                true
            })
        }

        fn connect_device(
            &self,
            address: String,
        ) -> crate::services::controls_backends::BackendFuture<'_, bool> {
            let command_calls = self.command_calls.clone();
            Box::pin(async move {
                command_calls
                    .lock()
                    .unwrap()
                    .push(format!("connect:{address}"));
                true
            })
        }

        fn disconnect_device(
            &self,
            address: String,
        ) -> crate::services::controls_backends::BackendFuture<'_, bool> {
            let command_calls = self.command_calls.clone();
            Box::pin(async move {
                command_calls
                    .lock()
                    .unwrap()
                    .push(format!("disconnect:{address}"));
                true
            })
        }

        fn pair_device(
            &self,
            address: String,
        ) -> crate::services::controls_backends::BackendFuture<'_, bool> {
            let command_calls = self.command_calls.clone();
            Box::pin(async move {
                command_calls
                    .lock()
                    .unwrap()
                    .push(format!("pair:{address}"));
                true
            })
        }

        fn trust_device(
            &self,
            address: String,
        ) -> crate::services::controls_backends::BackendFuture<'_, bool> {
            let command_calls = self.command_calls.clone();
            Box::pin(async move {
                command_calls
                    .lock()
                    .unwrap()
                    .push(format!("trust:{address}"));
                true
            })
        }

        fn remove_device(
            &self,
            address: String,
        ) -> crate::services::controls_backends::BackendFuture<'_, bool> {
            let command_calls = self.command_calls.clone();
            Box::pin(async move {
                command_calls
                    .lock()
                    .unwrap()
                    .push(format!("remove:{address}"));
                true
            })
        }

        fn open_overskride(&self) -> bool {
            let mut calls = self.overskride_calls.lock().unwrap();
            *calls += 1;
            true
        }
    }

    #[derive(Clone)]
    struct MockPowerBackend {
        profile: String,
        calls: SharedStringCalls,
    }

    impl crate::services::controls_backends::PowerBackend for MockPowerBackend {
        fn backend_name(&self) -> &'static str {
            "mock-power"
        }

        fn capability_mode(&self) -> crate::services::capabilities::CapabilityMode {
            crate::services::capabilities::CapabilityMode::Hybrid
        }

        fn diagnostics_summary(&self) -> Option<String> {
            Some("mock-power-runtime".to_string())
        }

        fn profile(&self) -> String {
            self.profile.clone()
        }

        fn set_profile(
            &self,
            profile: String,
        ) -> crate::services::controls_backends::BackendFuture<'_, ()> {
            let calls = self.calls.clone();
            Box::pin(async move {
                calls.lock().unwrap().push(profile);
            })
        }
    }

    type TestServiceParts = (
        ControlsService,
        SharedStringCalls,
        SharedU32Calls,
        SharedBoolCalls,
        SharedBluetoothCommandCalls,
        SharedCount,
        SharedStringCalls,
    );

    fn test_service() -> TestServiceParts {
        let audio_calls = Arc::new(Mutex::new(Vec::new()));
        let brightness_calls = Arc::new(Mutex::new(Vec::new()));
        let bluetooth_calls = Arc::new(Mutex::new(Vec::new()));
        let bluetooth_command_calls = Arc::new(Mutex::new(Vec::new()));
        let overskride_calls = Arc::new(Mutex::new(0));
        let power_calls = Arc::new(Mutex::new(Vec::new()));

        let service = ControlsService::with_backends(
            Arc::new(MockAudioBackend {
                audio: AudioInfo {
                    volume: 55,
                    muted: true,
                },
                devices: AudioDeviceSummary {
                    output_route: Some("Built-in Audio".to_string()),
                    input_route: Some("Internal Microphone".to_string()),
                    output_routes: vec![
                        AudioRouteInfo {
                            id: "52".to_string(),
                            name: "Built-in Audio".to_string(),
                            origin: AudioRouteOrigin::Internal,
                        },
                        AudioRouteInfo {
                            id: "77".to_string(),
                            name: "USB Audio DAC".to_string(),
                            origin: AudioRouteOrigin::Usb,
                        },
                    ],
                    input_routes: vec![
                        AudioRouteInfo {
                            id: "54".to_string(),
                            name: "Internal Microphone".to_string(),
                            origin: AudioRouteOrigin::Internal,
                        },
                        AudioRouteInfo {
                            id: "80".to_string(),
                            name: "USB Microphone".to_string(),
                            origin: AudioRouteOrigin::Usb,
                        },
                    ],
                },
                mic: MicInfo {
                    volume: 12,
                    muted: false,
                },
                calls: audio_calls.clone(),
            }),
            Arc::new(MockBrightnessBackend {
                snapshot: BrightnessSnapshot::from_percent(64),
                calls: brightness_calls.clone(),
            }),
            Arc::new(MockBluetoothBackend {
                enabled: true,
                devices: BluetoothDeviceSummary {
                    connected_devices: vec!["WH-1000XM5".to_string()],
                    device_details: vec![BluetoothConnectedDevice {
                        address: "AA:BB:CC:DD:EE:FF".to_string(),
                        name: "WH-1000XM5".to_string(),
                        connected: true,
                        paired: true,
                        trusted: true,
                        battery_percent: Some(90),
                        audio_profiles: vec!["A2DP".to_string(), "AVRCP".to_string()],
                    }],
                },
                toggle_calls: bluetooth_calls.clone(),
                command_calls: bluetooth_command_calls.clone(),
                overskride_calls: overskride_calls.clone(),
            }),
            Arc::new(MockPowerBackend {
                profile: "performance".to_string(),
                calls: power_calls.clone(),
            }),
        );

        (
            service,
            audio_calls,
            brightness_calls,
            bluetooth_calls,
            bluetooth_command_calls,
            overskride_calls,
            power_calls,
        )
    }

    #[test]
    fn brightness_snapshot_parses_label_to_percent() {
        let snapshot = BrightnessSnapshot::from_label("42%".to_string());
        assert_eq!(snapshot.percent, 42);
        assert_eq!(snapshot.label, "42%");
    }

    #[test]
    fn preview_command_updates_local_snapshot() {
        let mut service = ControlsService {
            snapshot: ControlsSnapshot {
                audio_devices: AudioDeviceSummary {
                    output_route: Some("Built-in Audio".to_string()),
                    input_route: Some("Internal Microphone".to_string()),
                    output_routes: vec![
                        AudioRouteInfo {
                            id: "52".to_string(),
                            name: "Built-in Audio".to_string(),
                            origin: AudioRouteOrigin::Internal,
                        },
                        AudioRouteInfo {
                            id: "77".to_string(),
                            name: "USB Audio DAC".to_string(),
                            origin: AudioRouteOrigin::Usb,
                        },
                    ],
                    input_routes: vec![
                        AudioRouteInfo {
                            id: "54".to_string(),
                            name: "Internal Microphone".to_string(),
                            origin: AudioRouteOrigin::Internal,
                        },
                        AudioRouteInfo {
                            id: "80".to_string(),
                            name: "USB Microphone".to_string(),
                            origin: AudioRouteOrigin::Usb,
                        },
                    ],
                },
                bluetooth_devices: BluetoothDeviceSummary {
                    connected_devices: vec!["WH-1000XM5".to_string()],
                    device_details: vec![BluetoothConnectedDevice {
                        address: "AA:BB:CC:DD:EE:FF".to_string(),
                        name: "WH-1000XM5".to_string(),
                        connected: true,
                        paired: false,
                        trusted: false,
                        battery_percent: Some(90),
                        audio_profiles: vec!["A2DP".to_string(), "AVRCP".to_string()],
                    }],
                },
                ..ControlsSnapshot::default()
            },
            audio_backend: Arc::new(MockAudioBackend {
                audio: AudioInfo {
                    volume: 0,
                    muted: false,
                },
                devices: AudioDeviceSummary::default(),
                mic: MicInfo {
                    volume: 0,
                    muted: false,
                },
                calls: Arc::new(Mutex::new(Vec::new())),
            }),
            brightness_backend: Arc::new(MockBrightnessBackend {
                snapshot: BrightnessSnapshot::default(),
                calls: Arc::new(Mutex::new(Vec::new())),
            }),
            bluetooth_backend: Arc::new(MockBluetoothBackend {
                enabled: false,
                devices: BluetoothDeviceSummary::default(),
                toggle_calls: Arc::new(Mutex::new(Vec::new())),
                command_calls: Arc::new(Mutex::new(Vec::new())),
                overskride_calls: Arc::new(Mutex::new(0)),
            }),
            power_backend: Arc::new(MockPowerBackend {
                profile: "balanced".to_string(),
                calls: Arc::new(Mutex::new(Vec::new())),
            }),
        };
        service.preview_command(&ControlsCommand::SetVolume(73));
        service.preview_command(&ControlsCommand::SetAudioOutputRoute("77".to_string()));
        service.preview_command(&ControlsCommand::SetBrightness(64));
        service.preview_command(&ControlsCommand::SetAudioInputRoute("80".to_string()));
        service.preview_command(&ControlsCommand::ScanBluetoothDevices);
        service.preview_command(&ControlsCommand::StopBluetoothScan);
        service.preview_command(&ControlsCommand::ConnectBluetoothDevice(
            "AA:BB:CC:DD:EE:FF".to_string(),
        ));
        service.preview_command(&ControlsCommand::DisconnectBluetoothDevice(
            "AA:BB:CC:DD:EE:FF".to_string(),
        ));
        service.preview_command(&ControlsCommand::PairBluetoothDevice(
            "AA:BB:CC:DD:EE:FF".to_string(),
        ));
        service.preview_command(&ControlsCommand::TrustBluetoothDevice(
            "AA:BB:CC:DD:EE:FF".to_string(),
        ));
        service.preview_command(&ControlsCommand::SetPowerProfile("performance".to_string()));

        assert_eq!(service.snapshot().audio.volume, 73);
        assert_eq!(
            service.snapshot().audio_devices.output_route.as_deref(),
            Some("USB Audio DAC")
        );
        assert_eq!(service.snapshot().brightness.percent, 64);
        assert_eq!(
            service.snapshot().audio_devices.input_route.as_deref(),
            Some("USB Microphone")
        );
        assert_eq!(service.snapshot().power_profile, "performance");
        assert!(service.snapshot().bluetooth_devices.device_details[0].paired);
        assert!(service.snapshot().bluetooth_devices.device_details[0].trusted);
    }

    #[test]
    fn apply_refresh_replaces_audio_and_mic_state() {
        let mut service = ControlsService {
            snapshot: ControlsSnapshot {
                bluetooth_devices: BluetoothDeviceSummary {
                    connected_devices: vec!["WH-1000XM5".to_string()],
                    device_details: vec![BluetoothConnectedDevice {
                        address: "AA:BB:CC:DD:EE:FF".to_string(),
                        name: "WH-1000XM5".to_string(),
                        connected: true,
                        paired: false,
                        trusted: false,
                        battery_percent: Some(90),
                        audio_profiles: vec!["A2DP".to_string(), "AVRCP".to_string()],
                    }],
                },
                ..ControlsSnapshot::default()
            },
            audio_backend: Arc::new(MockAudioBackend {
                audio: AudioInfo {
                    volume: 0,
                    muted: false,
                },
                devices: AudioDeviceSummary::default(),
                mic: MicInfo {
                    volume: 0,
                    muted: false,
                },
                calls: Arc::new(Mutex::new(Vec::new())),
            }),
            brightness_backend: Arc::new(MockBrightnessBackend {
                snapshot: BrightnessSnapshot::default(),
                calls: Arc::new(Mutex::new(Vec::new())),
            }),
            bluetooth_backend: Arc::new(MockBluetoothBackend {
                enabled: false,
                devices: BluetoothDeviceSummary::default(),
                toggle_calls: Arc::new(Mutex::new(Vec::new())),
                command_calls: Arc::new(Mutex::new(Vec::new())),
                overskride_calls: Arc::new(Mutex::new(0)),
            }),
            power_backend: Arc::new(MockPowerBackend {
                profile: "balanced".to_string(),
                calls: Arc::new(Mutex::new(Vec::new())),
            }),
        };
        service.apply_refresh(ControlsRefresh {
            audio: Some(AudioInfo {
                volume: 55,
                muted: false,
            }),
            mic: Some(MicInfo {
                volume: 12,
                muted: true,
            }),
            ..ControlsRefresh::default()
        });

        assert_eq!(service.snapshot().audio.volume, 55);
        assert_eq!(service.snapshot().mic.volume, 12);
        assert!(service.snapshot().mic.muted);
    }

    #[tokio::test]
    async fn refresh_uses_backend_snapshots_for_migrated_domains() {
        let (service, ..) = test_service();

        let audio_refresh = service.refresh(ControlsRefreshKind::AudioMic).await;
        let brightness_refresh = service.refresh(ControlsRefreshKind::Brightness).await;
        let battery_refresh = service.refresh(ControlsRefreshKind::BatteryPower).await;
        let power_refresh = service.refresh(ControlsRefreshKind::Power).await;
        let bluetooth_refresh = service.refresh(ControlsRefreshKind::Bluetooth).await;

        assert_eq!(audio_refresh.audio.unwrap().volume, 55);
        assert_eq!(
            audio_refresh.audio_devices.unwrap().output_route.as_deref(),
            Some("Built-in Audio")
        );
        assert_eq!(audio_refresh.mic.unwrap().volume, 12);
        assert_eq!(brightness_refresh.brightness.unwrap().percent, 64);
        assert_eq!(
            battery_refresh.power_profile.as_deref(),
            Some("performance")
        );
        assert!(battery_refresh.battery.is_some());
        assert_eq!(power_refresh.power_profile.unwrap(), "performance");
        assert!(bluetooth_refresh.bluetooth_enabled.unwrap());
        assert_eq!(
            bluetooth_refresh
                .bluetooth_devices
                .unwrap()
                .connected_devices,
            vec!["WH-1000XM5".to_string()]
        );
    }

    #[tokio::test]
    async fn execute_routes_commands_to_backends() {
        let (
            service,
            audio_calls,
            brightness_calls,
            bluetooth_calls,
            bluetooth_command_calls,
            overskride_calls,
            power_calls,
        ) = test_service();

        let follow_up = service.execute(ControlsCommand::SetVolume(77)).await;
        assert_eq!(
            follow_up,
            super::ControlsFollowUp::Refresh(ControlsRefreshKind::AudioMic)
        );
        let _ = service.execute(ControlsCommand::ToggleAudioMute).await;
        let _ = service
            .execute(ControlsCommand::SetAudioOutputRoute("77".to_string()))
            .await;
        let _ = service.execute(ControlsCommand::SetMicVolume(22)).await;
        let _ = service.execute(ControlsCommand::ToggleMicMute).await;
        let _ = service
            .execute(ControlsCommand::SetAudioInputRoute("80".to_string()))
            .await;
        let _ = service.execute(ControlsCommand::SetBrightness(31)).await;
        let _ = service
            .execute(ControlsCommand::SetPowerProfile("balanced".to_string()))
            .await;
        let _ = service
            .execute(ControlsCommand::ToggleBluetooth(false))
            .await;
        let _ = service.execute(ControlsCommand::ScanBluetoothDevices).await;
        let _ = service.execute(ControlsCommand::StopBluetoothScan).await;
        let _ = service
            .execute(ControlsCommand::ConnectBluetoothDevice(
                "AA:BB:CC:DD:EE:FF".to_string(),
            ))
            .await;
        let _ = service
            .execute(ControlsCommand::DisconnectBluetoothDevice(
                "AA:BB:CC:DD:EE:FF".to_string(),
            ))
            .await;
        let _ = service
            .execute(ControlsCommand::PairBluetoothDevice(
                "AA:BB:CC:DD:EE:FF".to_string(),
            ))
            .await;
        let _ = service
            .execute(ControlsCommand::TrustBluetoothDevice(
                "AA:BB:CC:DD:EE:FF".to_string(),
            ))
            .await;
        let _ = service
            .execute(ControlsCommand::RemoveBluetoothDevice(
                "AA:BB:CC:DD:EE:FF".to_string(),
            ))
            .await;
        let overskride_follow_up = service.execute(ControlsCommand::OpenOverskride).await;

        assert_eq!(
            audio_calls.lock().unwrap().as_slice(),
            [
                "set_volume:77",
                "toggle_audio_mute",
                "set_output_route:77",
                "set_mic_volume:22",
                "toggle_mic_mute",
                "set_input_route:80",
            ]
        );
        assert_eq!(brightness_calls.lock().unwrap().as_slice(), [31]);
        assert_eq!(power_calls.lock().unwrap().as_slice(), ["balanced"]);
        assert_eq!(bluetooth_calls.lock().unwrap().as_slice(), [false]);
        assert_eq!(
            bluetooth_command_calls.lock().unwrap().as_slice(),
            [
                "scan",
                "stop-scan",
                "connect:AA:BB:CC:DD:EE:FF",
                "disconnect:AA:BB:CC:DD:EE:FF",
                "pair:AA:BB:CC:DD:EE:FF",
                "trust:AA:BB:CC:DD:EE:FF",
                "remove:AA:BB:CC:DD:EE:FF",
            ]
        );
        assert_eq!(*overskride_calls.lock().unwrap(), 1);
        assert_eq!(
            overskride_follow_up,
            super::ControlsFollowUp::RefreshCompositor
        );
    }

    #[test]
    fn diagnostics_expose_backend_names() {
        let (service, ..) = test_service();
        let diagnostics = service.diagnostics();

        assert_eq!(diagnostics.audio_backend, "mock-audio");
        assert_eq!(diagnostics.audio_runtime, None);
        assert_eq!(diagnostics.brightness_backend, "mock-brightness");
        assert_eq!(diagnostics.bluetooth_backend, "mock-bluetooth");
        assert_eq!(diagnostics.power_backend, "mock-power");
        assert_eq!(
            diagnostics.power_runtime.as_deref(),
            Some("mock-power-runtime")
        );
        assert!(diagnostics.summary().contains("mock-audio"));
    }

    #[test]
    fn capability_statuses_surface_read_only_battery_care() {
        let mut snapshot = ControlsSnapshot::default();
        snapshot.battery.charge_start_threshold = Some(80);
        snapshot.battery.charge_end_threshold = Some(100);
        let service = ControlsService::with_snapshot_for_tests(snapshot);

        let statuses = service.capability_statuses();
        let battery = statuses.iter().find(|status| status.key == "bat").unwrap();
        let audio = statuses.iter().find(|status| status.key == "aud").unwrap();

        assert_eq!(
            battery.mode,
            crate::services::capabilities::CapabilityMode::ReadOnly
        );
        assert_eq!(battery.detail.as_deref(), Some("system-managed thresholds"));
        assert_eq!(
            audio.mode,
            crate::services::capabilities::CapabilityMode::Unavailable
        );
    }
}
