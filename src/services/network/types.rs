#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WifiInfo {
    pub enabled: bool,
    pub ssid: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WifiNetwork {
    pub ssid: String,
    pub security: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NetworkBackendKind {
    #[default]
    Iwd,
    NetworkManager,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum NetworkStatus {
    #[default]
    Idle,
    Info(String),
    Scanning,
    AwaitingPassword(String),
    Connecting(String),
    Error(String),
}

impl NetworkStatus {
    pub fn message(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::Idle => None,
            Self::Info(message) | Self::Error(message) => Some(Cow::Borrowed(message.as_str())),
            Self::Scanning => Some(Cow::Borrowed("Сканирование сетей...")),
            Self::AwaitingPassword(_) => Some(Cow::Borrowed("Введите пароль и нажмите Connect")),
            Self::Connecting(ssid) => Some(Cow::Owned(format!("Подключение к {}...", ssid))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkSnapshot {
    pub wifi: WifiInfo,
    pub menu_open: bool,
    pub available_networks: Vec<WifiNetwork>,
    pub password_input: String,
    pub status: NetworkStatus,
    pub configured_backend: NetworkBackendKind,
    pub active_backend: NetworkBackendKind,
}

impl NetworkSnapshot {
    pub fn new(configured_backend: NetworkBackendKind, active_backend: NetworkBackendKind) -> Self {
        Self {
            wifi: WifiInfo {
                enabled: false,
                ssid: "Loading...".to_string(),
            },
            menu_open: false,
            available_networks: Vec::new(),
            password_input: String::new(),
            status: NetworkStatus::Idle,
            configured_backend,
            active_backend,
        }
    }

    pub fn status_message(&self) -> Option<Cow<'_, str>> {
        self.status.message()
    }

    pub fn awaiting_password_ssid(&self) -> Option<&str> {
        match &self.status {
            NetworkStatus::AwaitingPassword(ssid) => Some(ssid.as_str()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkCommand {
    ToggleMenu,
    SelectNetwork { ssid: String, security: String },
    UpdatePassword(String),
    SubmitPassword,
    CancelPassword,
    ToggleWifi(bool),
    CloseTransientUi,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkEvent {
    WifiInfoSynced(WifiInfo),
    ScanCompleted(Vec<WifiNetwork>),
    ConnectCompleted { ssid: String, success: bool },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkFollowUp {
    None,
    Scan,
    Connect {
        ssid: String,
        passphrase: Option<String>,
    },
    TogglePower(bool),
}
use std::borrow::Cow;
