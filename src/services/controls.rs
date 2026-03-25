pub use crate::modules::audio::AudioInfo;
pub use crate::modules::battery::BatteryInfo;
pub use crate::modules::fan::FanInfo;
pub use crate::modules::mic::MicInfo;

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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ControlsService {
    snapshot: ControlsSnapshot,
}

impl ControlsService {
    pub fn new() -> Self {
        Self {
            snapshot: ControlsSnapshot {
                brightness: BrightnessSnapshot::from_label(
                    crate::modules::brightness::get_brightness(),
                ),
                audio: crate::modules::audio::get_info(),
                mic: crate::modules::mic::get_info(),
                fan: crate::modules::fan::get_fan_info(),
                battery: crate::modules::battery::get_battery_info(),
                power_profile: crate::modules::power::get_profile(),
                bluetooth_enabled: crate::modules::bluetooth::get_bluetooth_info(),
            },
        }
    }

    pub fn snapshot(&self) -> &ControlsSnapshot {
        &self.snapshot
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
                audio: Some(crate::modules::audio::get_info()),
                mic: Some(crate::modules::mic::get_info()),
                ..ControlsRefresh::default()
            },
            ControlsRefreshKind::Brightness => ControlsRefresh {
                brightness: Some(BrightnessSnapshot::from_label(
                    crate::modules::brightness::get_brightness(),
                )),
                ..ControlsRefresh::default()
            },
            ControlsRefreshKind::Fan => ControlsRefresh {
                fan: Some(crate::modules::fan::get_fan_info()),
                ..ControlsRefresh::default()
            },
            ControlsRefreshKind::Power => ControlsRefresh {
                power_profile: Some(crate::modules::power::get_profile()),
                ..ControlsRefresh::default()
            },
            ControlsRefreshKind::Bluetooth => ControlsRefresh {
                bluetooth_enabled: Some(crate::modules::bluetooth::get_bluetooth_info()),
                ..ControlsRefresh::default()
            },
            ControlsRefreshKind::Slow => ControlsRefresh {
                brightness: Some(BrightnessSnapshot::from_label(
                    crate::modules::brightness::get_brightness(),
                )),
                battery: Some(crate::modules::battery::get_battery_info()),
                power_profile: Some(crate::modules::power::get_profile()),
                bluetooth_enabled: Some(crate::modules::bluetooth::get_bluetooth_info()),
                ..ControlsRefresh::default()
            },
        }
    }

    pub async fn execute(&self, command: ControlsCommand) -> ControlsFollowUp {
        match command {
            ControlsCommand::SetVolume(volume) => {
                crate::modules::audio::set_volume(volume).await;
                ControlsFollowUp::Refresh(ControlsRefreshKind::AudioMic)
            }
            ControlsCommand::ToggleAudioMute => {
                crate::modules::audio::toggle_mute().await;
                ControlsFollowUp::Refresh(ControlsRefreshKind::AudioMic)
            }
            ControlsCommand::SetMicVolume(volume) => {
                crate::modules::mic::set_volume(volume).await;
                ControlsFollowUp::Refresh(ControlsRefreshKind::AudioMic)
            }
            ControlsCommand::ToggleMicMute => {
                crate::modules::mic::toggle_mute().await;
                ControlsFollowUp::Refresh(ControlsRefreshKind::AudioMic)
            }
            ControlsCommand::SetBrightness(percent) => {
                crate::modules::brightness::set_brightness(percent);
                ControlsFollowUp::Refresh(ControlsRefreshKind::Brightness)
            }
            ControlsCommand::SetFanLevel(level) => {
                crate::modules::fan::set_fan_level(&level);
                ControlsFollowUp::Refresh(ControlsRefreshKind::Fan)
            }
            ControlsCommand::SetPowerProfile(profile) => {
                crate::modules::power::set_profile(&profile).await;
                ControlsFollowUp::Refresh(ControlsRefreshKind::Power)
            }
            ControlsCommand::ToggleBluetooth(enabled) => {
                let _ = crate::modules::bluetooth::toggle_bluetooth(enabled);
                ControlsFollowUp::Refresh(ControlsRefreshKind::Bluetooth)
            }
            ControlsCommand::OpenOverskride => {
                let _ = crate::modules::bluetooth::open_overskride();
                ControlsFollowUp::RefreshCompositor
            }
        }
    }

    pub fn subscription() -> iced::Subscription<ControlsEvent> {
        crate::modules::audio::subscription()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BrightnessSnapshot, ControlsCommand, ControlsRefresh, ControlsService, ControlsSnapshot,
    };
    use crate::modules::audio::AudioInfo;
    use crate::modules::mic::MicInfo;

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
        assert!(service.snapshot().mic.muted);
        assert_eq!(service.snapshot().mic.volume, 12);
    }
}
