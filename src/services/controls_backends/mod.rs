use std::{future::Future, pin::Pin};

pub mod audio;
pub mod bluetooth;
pub mod brightness;
pub mod power;

pub type BackendFuture<'a, T = ()> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait AudioBackend: Send + Sync {
    fn backend_name(&self) -> &'static str;
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
    fn snapshot(&self) -> crate::services::controls::BrightnessSnapshot;
    fn set_brightness(&self, percent: u32);
}

pub trait BluetoothBackend: Send + Sync {
    fn backend_name(&self) -> &'static str;
    fn enabled(&self) -> bool;
    fn device_summary(&self) -> crate::services::controls::BluetoothDeviceSummary;
    fn toggle(&self, enable: bool) -> bool;
    fn connect_device(&self, address: String) -> BackendFuture<'_, bool>;
    fn disconnect_device(&self, address: String) -> BackendFuture<'_, bool>;
    fn open_overskride(&self) -> bool;
}

pub trait PowerBackend: Send + Sync {
    fn backend_name(&self) -> &'static str;
    fn diagnostics_summary(&self) -> Option<String> {
        None
    }
    fn profile(&self) -> String;
    fn set_profile(&self, profile: String) -> BackendFuture<'_, ()>;
    fn set_battery_thresholds(
        &self,
        thresholds: crate::services::controls::BatteryThresholds,
    ) -> BackendFuture<'_, ()>;
}
