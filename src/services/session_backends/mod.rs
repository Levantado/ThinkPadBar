use crate::services::capabilities::CapabilityMode;

pub mod systemd;
pub mod rofi;
pub mod hyprland;

pub trait PowerSessionBackend: Send + Sync {
    fn backend_name(&self) -> &'static str;
    fn capability_mode(&self) -> CapabilityMode;
    fn lock(&self);
    fn logout(&self);
    fn suspend(&self);
    fn hibernate(&self);
    fn reboot(&self);
    fn shutdown(&self);
}

pub trait LauncherBackend: Send + Sync {
    fn backend_name(&self) -> &'static str;
    fn toggle_launcher(&self) -> bool;
}
