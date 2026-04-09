use std::{future::Future, pin::Pin};

pub mod audio;
pub mod bluetooth;
pub mod brightness;
pub mod fan;
pub mod power;
pub mod privileged;

pub type BackendFuture<'a, T = ()> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait AudioBackend: Send + Sync {
    fn backend_name(&self) -> &'static str;
    fn capability_mode(&self) -> crate::services::capabilities::CapabilityMode;
    fn diagnostics_summary(&self) -> Option<String> {
        None
    }
    fn audio_info(&self) -> crate::services::controls::AudioInfo;
    fn mic_info(&self) -> crate::modules::mic::MicInfo;
    fn device_summary(&self) -> crate::services::controls::AudioDeviceSummary;
    fn set_volume(&self, percent: u32) -> BackendFuture<'_, ()>;
    fn toggle_audio_mute(&self) -> BackendFuture<'_, ()>;
    fn set_output_route(&self, id: String) -> BackendFuture<'_, bool>;
    fn set_mic_volume(&self, percent: u32) -> BackendFuture<'_, ()>;
    fn toggle_mic_mute(&self) -> BackendFuture<'_, ()>;
    fn set_input_route(&self, id: String) -> BackendFuture<'_, bool>;
    fn subscription(&self) -> iced::Subscription<crate::services::controls::ControlsEvent>;
}

pub trait BrightnessBackend: Send + Sync {
    fn backend_name(&self) -> &'static str;
    fn capability_mode(&self) -> crate::services::capabilities::CapabilityMode;
    fn snapshot(&self) -> crate::services::controls::BrightnessSnapshot;
    fn set_brightness(&self, percent: u32) -> BackendFuture<'_, ()>;
    fn subscription(&self) -> iced::Subscription<crate::services::controls::ControlsEvent> {
        iced::Subscription::none()
    }
}

pub trait BluetoothBackend: Send + Sync {
    fn backend_name(&self) -> &'static str;
    fn capability_mode(&self) -> crate::services::capabilities::CapabilityMode;
    fn diagnostics_summary(&self) -> Option<String> {
        None
    }
    fn enabled(&self) -> BackendFuture<'_, bool>;
    fn device_summary(
        &self,
    ) -> BackendFuture<'_, crate::services::controls::BluetoothDeviceSummary>;
    fn toggle(&self, enable: bool) -> BackendFuture<'_, bool>;
    fn scan_devices(&self) -> BackendFuture<'_, bool>;
    fn stop_scan_devices(&self) -> BackendFuture<'_, bool>;
    fn connect_device(&self, address: String) -> BackendFuture<'_, bool>;
    fn disconnect_device(&self, address: String) -> BackendFuture<'_, bool>;
    fn pair_device(&self, address: String) -> BackendFuture<'_, bool>;
    fn trust_device(&self, address: String) -> BackendFuture<'_, bool>;
    fn remove_device(&self, address: String) -> BackendFuture<'_, bool>;
    fn open_overskride(&self) -> bool;
    fn subscription(&self) -> iced::Subscription<crate::services::controls::ControlsEvent> {
        iced::Subscription::none()
    }
}

pub trait FanBackend: Send + Sync {
    fn backend_name(&self) -> &'static str;
    fn capability_mode(&self) -> crate::services::capabilities::CapabilityMode;
    fn diagnostics_summary(&self) -> Option<String> {
        None
    }
    fn info(&self) -> crate::services::controls::FanInfo;
    fn set_level(&self, level: &str) -> BackendFuture<'_, ()>;
}

pub trait PowerBackend: Send + Sync {
    fn backend_name(&self) -> &'static str;
    fn capability_mode(&self) -> crate::services::capabilities::CapabilityMode;
    fn diagnostics_summary(&self) -> Option<String> {
        None
    }
    fn profile(&self) -> String;
    fn set_profile(&self, profile: String) -> BackendFuture<'_, ()>;
    fn subscription(&self) -> iced::Subscription<crate::services::controls::ControlsEvent> {
        iced::Subscription::none()
    }
}
