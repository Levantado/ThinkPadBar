use std::sync::Arc;

pub use crate::modules::battery::BatteryInfo;
pub use crate::modules::fan::FanInfo;
pub use crate::modules::mic::MicInfo;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioInfo {
    pub volume: u32,
    pub muted: bool,
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
    pub mic: MicInfo,
    pub fan: FanInfo,
    pub battery: BatteryInfo,
    pub power_profile: String,
    pub bluetooth_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlsDiagnostics {
    pub audio_backend: &'static str,
    pub brightness_backend: &'static str,
    pub bluetooth_backend: &'static str,
    pub power_backend: &'static str,
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
            },
            power_profile: "balanced".to_string(),
            bluetooth_enabled: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlsEvent {
    AudioServerChanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlsRefreshKind {
    AudioMic,
    Brightness,
    Fan,
    Power,
    Bluetooth,
    Slow,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ControlsRefresh {
    pub brightness: Option<BrightnessSnapshot>,
    pub audio: Option<AudioInfo>,
    pub mic: Option<MicInfo>,
    pub fan: Option<FanInfo>,
    pub battery: Option<BatteryInfo>,
    pub power_profile: Option<String>,
    pub bluetooth_enabled: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlsCommand {
    SetVolume(u32),
    ToggleAudioMute,
    SetMicVolume(u32),
    ToggleMicMute,
    SetBrightness(u32),
    SetFanLevel(String),
    SetPowerProfile(String),
    ToggleBluetooth(bool),
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
            Arc::new(crate::services::controls_backends::audio::WpctlAudioBackend),
            Arc::new(crate::services::controls_backends::brightness::SysfsBrightnessBackend),
            Arc::new(crate::services::controls_backends::bluetooth::BluetoothCtlBackend),
            Arc::new(crate::services::controls_backends::power::PlatformProfilePowerBackend),
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
                mic: audio_backend.mic_info(),
                fan: crate::modules::fan::get_fan_info(),
                battery: crate::modules::battery::get_battery_info(),
                power_profile: power_backend.profile(),
                bluetooth_enabled: bluetooth_backend.enabled(),
            },
            audio_backend,
            brightness_backend,
            bluetooth_backend,
            power_backend,
        }
    }

    pub fn snapshot(&self) -> &ControlsSnapshot {
        &self.snapshot
    }

    pub fn diagnostics(&self) -> ControlsDiagnostics {
        ControlsDiagnostics {
            audio_backend: self.audio_backend.backend_name(),
            brightness_backend: self.brightness_backend.backend_name(),
            bluetooth_backend: self.bluetooth_backend.backend_name(),
            power_backend: self.power_backend.backend_name(),
        }
    }

    pub fn apply_refresh(&mut self, refresh: ControlsRefresh) {
        if let Some(brightness) = refresh.brightness {
            self.snapshot.brightness = brightness;
        }
        if let Some(audio) = refresh.audio {
            self.snapshot.audio = audio;
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
    }

    pub fn preview_command(&mut self, command: &ControlsCommand) {
        match command {
            ControlsCommand::SetVolume(volume) => {
                self.snapshot.audio.volume = *volume;
            }
            ControlsCommand::SetMicVolume(volume) => {
                self.snapshot.mic.volume = *volume;
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
            ControlsCommand::ToggleAudioMute
            | ControlsCommand::ToggleMicMute
            | ControlsCommand::OpenOverskride => {}
        }
    }

    pub async fn refresh(&self, kind: ControlsRefreshKind) -> ControlsRefresh {
        match kind {
            ControlsRefreshKind::AudioMic => ControlsRefresh {
                audio: Some(self.audio_backend.audio_info()),
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
            ControlsRefreshKind::Power => ControlsRefresh {
                power_profile: Some(self.power_backend.profile()),
                ..ControlsRefresh::default()
            },
            ControlsRefreshKind::Bluetooth => ControlsRefresh {
                bluetooth_enabled: Some(self.bluetooth_backend.enabled()),
                ..ControlsRefresh::default()
            },
            ControlsRefreshKind::Slow => ControlsRefresh {
                brightness: Some(self.brightness_backend.snapshot()),
                battery: Some(crate::modules::battery::get_battery_info()),
                power_profile: Some(self.power_backend.profile()),
                bluetooth_enabled: Some(self.bluetooth_backend.enabled()),
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
            ControlsCommand::SetMicVolume(volume) => {
                self.audio_backend.set_mic_volume(volume).await;
                ControlsFollowUp::Refresh(ControlsRefreshKind::AudioMic)
            }
            ControlsCommand::ToggleMicMute => {
                self.audio_backend.toggle_mic_mute().await;
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
            ControlsCommand::OpenOverskride => {
                let _ = self.bluetooth_backend.open_overskride();
                ControlsFollowUp::RefreshCompositor
            }
        }
    }

    pub fn subscription(&self) -> iced::Subscription<ControlsEvent> {
        self.audio_backend.subscription()
    }
}

impl Default for ControlsService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AudioInfo, BrightnessSnapshot, ControlsCommand, ControlsRefresh, ControlsRefreshKind,
        ControlsService, ControlsSnapshot,
    };
    use crate::modules::mic::MicInfo;
    use std::sync::{Arc, Mutex};

    type SharedStringCalls = Arc<Mutex<Vec<String>>>;
    type SharedU32Calls = Arc<Mutex<Vec<u32>>>;
    type SharedBoolCalls = Arc<Mutex<Vec<bool>>>;
    type SharedCount = Arc<Mutex<u32>>;

    #[derive(Clone)]
    struct MockAudioBackend {
        audio: AudioInfo,
        mic: MicInfo,
        calls: SharedStringCalls,
    }

    impl crate::services::controls_backends::AudioBackend for MockAudioBackend {
        fn backend_name(&self) -> &'static str {
            "mock-audio"
        }

        fn audio_info(&self) -> AudioInfo {
            self.audio.clone()
        }

        fn mic_info(&self) -> MicInfo {
            self.mic.clone()
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
        toggle_calls: SharedBoolCalls,
        overskride_calls: SharedCount,
    }

    impl crate::services::controls_backends::BluetoothBackend for MockBluetoothBackend {
        fn backend_name(&self) -> &'static str {
            "mock-bluetooth"
        }

        fn enabled(&self) -> bool {
            self.enabled
        }

        fn toggle(&self, enable: bool) -> bool {
            self.toggle_calls.lock().unwrap().push(enable);
            true
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
        SharedCount,
        SharedStringCalls,
    );

    fn test_service() -> TestServiceParts {
        let audio_calls = Arc::new(Mutex::new(Vec::new()));
        let brightness_calls = Arc::new(Mutex::new(Vec::new()));
        let bluetooth_calls = Arc::new(Mutex::new(Vec::new()));
        let overskride_calls = Arc::new(Mutex::new(0));
        let power_calls = Arc::new(Mutex::new(Vec::new()));

        let service = ControlsService::with_backends(
            Arc::new(MockAudioBackend {
                audio: AudioInfo {
                    volume: 55,
                    muted: true,
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
                toggle_calls: bluetooth_calls.clone(),
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
            snapshot: ControlsSnapshot::default(),
            audio_backend: Arc::new(MockAudioBackend {
                audio: AudioInfo {
                    volume: 0,
                    muted: false,
                },
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
                toggle_calls: Arc::new(Mutex::new(Vec::new())),
                overskride_calls: Arc::new(Mutex::new(0)),
            }),
            power_backend: Arc::new(MockPowerBackend {
                profile: "balanced".to_string(),
                calls: Arc::new(Mutex::new(Vec::new())),
            }),
        };
        service.preview_command(&ControlsCommand::SetVolume(73));
        service.preview_command(&ControlsCommand::SetBrightness(64));
        service.preview_command(&ControlsCommand::SetPowerProfile("performance".to_string()));

        assert_eq!(service.snapshot().audio.volume, 73);
        assert_eq!(service.snapshot().brightness.percent, 64);
        assert_eq!(service.snapshot().power_profile, "performance");
    }

    #[test]
    fn apply_refresh_replaces_audio_and_mic_state() {
        let mut service = ControlsService {
            snapshot: ControlsSnapshot::default(),
            audio_backend: Arc::new(MockAudioBackend {
                audio: AudioInfo {
                    volume: 0,
                    muted: false,
                },
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
                toggle_calls: Arc::new(Mutex::new(Vec::new())),
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
        let power_refresh = service.refresh(ControlsRefreshKind::Power).await;
        let bluetooth_refresh = service.refresh(ControlsRefreshKind::Bluetooth).await;

        assert_eq!(audio_refresh.audio.unwrap().volume, 55);
        assert_eq!(audio_refresh.mic.unwrap().volume, 12);
        assert_eq!(brightness_refresh.brightness.unwrap().percent, 64);
        assert_eq!(power_refresh.power_profile.unwrap(), "performance");
        assert!(bluetooth_refresh.bluetooth_enabled.unwrap());
    }

    #[tokio::test]
    async fn execute_routes_commands_to_backends() {
        let (
            service,
            audio_calls,
            brightness_calls,
            bluetooth_calls,
            overskride_calls,
            power_calls,
        ) = test_service();

        let follow_up = service.execute(ControlsCommand::SetVolume(77)).await;
        assert_eq!(
            follow_up,
            super::ControlsFollowUp::Refresh(ControlsRefreshKind::AudioMic)
        );
        let _ = service.execute(ControlsCommand::ToggleAudioMute).await;
        let _ = service.execute(ControlsCommand::SetMicVolume(22)).await;
        let _ = service.execute(ControlsCommand::ToggleMicMute).await;
        let _ = service.execute(ControlsCommand::SetBrightness(31)).await;
        let _ = service
            .execute(ControlsCommand::SetPowerProfile("balanced".to_string()))
            .await;
        let _ = service
            .execute(ControlsCommand::ToggleBluetooth(false))
            .await;
        let follow_up = service.execute(ControlsCommand::OpenOverskride).await;

        assert_eq!(
            audio_calls.lock().unwrap().as_slice(),
            [
                "set_volume:77",
                "toggle_audio_mute",
                "set_mic_volume:22",
                "toggle_mic_mute",
            ]
        );
        assert_eq!(brightness_calls.lock().unwrap().as_slice(), [31]);
        assert_eq!(power_calls.lock().unwrap().as_slice(), ["balanced"]);
        assert_eq!(bluetooth_calls.lock().unwrap().as_slice(), [false]);
        assert_eq!(*overskride_calls.lock().unwrap(), 1);
        assert_eq!(follow_up, super::ControlsFollowUp::RefreshCompositor);
    }

    #[test]
    fn diagnostics_expose_backend_names() {
        let (service, ..) = test_service();
        let diagnostics = service.diagnostics();

        assert_eq!(diagnostics.audio_backend, "mock-audio");
        assert_eq!(diagnostics.brightness_backend, "mock-brightness");
        assert_eq!(diagnostics.bluetooth_backend, "mock-bluetooth");
        assert_eq!(diagnostics.power_backend, "mock-power");
        assert!(diagnostics.summary().contains("mock-audio"));
    }
}
