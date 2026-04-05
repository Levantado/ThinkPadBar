mod iwd;
pub mod types;

use zbus::Connection;

pub use types::{
    NetworkBackendKind, NetworkCommand, NetworkEvent, NetworkFollowUp, NetworkSnapshot,
    NetworkStatus, WifiInfo, WifiNetwork,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkDiagnostics {
    pub configured_backend: NetworkBackendKind,
    pub active_backend: NetworkBackendKind,
    pub fallback_path: Option<String>,
    pub unavailable_reason: Option<String>,
    pub last_error: Option<String>,
}

impl NetworkDiagnostics {
    pub fn summary(&self) -> String {
        let fallback = self.fallback_path.as_deref().unwrap_or("-");
        let unavailable = self.unavailable_reason.as_deref().unwrap_or("-");
        let error = self.last_error.as_deref().unwrap_or("-");
        format!(
            "cfg {:?} act {:?} fb {} why {} err {}",
            self.configured_backend, self.active_backend, fallback, unavailable, error
        )
    }
}

trait NetworkBackend {
    fn kind(&self) -> NetworkBackendKind;
    async fn get_wifi_info(&self, conn: &Connection) -> WifiInfo;
    async fn scan_networks(&self, conn: &Connection) -> Vec<WifiNetwork>;
    async fn connect_network(
        &self,
        conn: &Connection,
        ssid: String,
        passphrase: Option<String>,
    ) -> bool;
    async fn toggle_wifi(&self, conn: &Connection, enable: bool);
}

impl NetworkBackend for iwd::IwdBackend {
    fn kind(&self) -> NetworkBackendKind {
        NetworkBackendKind::Iwd
    }

    async fn get_wifi_info(&self, conn: &Connection) -> WifiInfo {
        self.get_wifi_info(conn).await
    }

    async fn scan_networks(&self, conn: &Connection) -> Vec<WifiNetwork> {
        self.scan_networks(conn).await
    }

    async fn connect_network(
        &self,
        conn: &Connection,
        ssid: String,
        passphrase: Option<String>,
    ) -> bool {
        self.connect_network(conn, ssid, passphrase).await
    }

    async fn toggle_wifi(&self, conn: &Connection, enable: bool) {
        self.toggle_wifi(conn, enable).await
    }
}

#[derive(Debug, Clone)]
pub struct NetworkService {
    snapshot: NetworkSnapshot,
    backend: iwd::IwdBackend,
}

impl NetworkService {
    pub fn new(config: &crate::config::NetworkConfig) -> Self {
        let configured_backend = NetworkBackendKind::Iwd;
        let backend = iwd::IwdBackend::new(config);
        let active_backend = backend.kind();

        Self {
            snapshot: NetworkSnapshot::new(configured_backend, active_backend),
            backend,
        }
    }

    pub fn snapshot(&self) -> &NetworkSnapshot {
        &self.snapshot
    }

    pub fn configured_backend(&self) -> NetworkBackendKind {
        self.snapshot.configured_backend
    }

    pub fn active_backend(&self) -> NetworkBackendKind {
        self.snapshot.active_backend
    }

    pub fn diagnostics(&self) -> NetworkDiagnostics {
        let backend_diagnostics = self.backend.diagnostics();
        let status_error = match &self.snapshot.status {
            NetworkStatus::Error(error) => Some(error.clone()),
            _ => None,
        };

        let backend_unavailable =
            (self.snapshot.configured_backend != self.snapshot.active_backend).then(|| {
                format!(
                    "configured {:?}, runtime {:?}: backend fallback active",
                    self.snapshot.configured_backend, self.snapshot.active_backend
                )
            });

        NetworkDiagnostics {
            configured_backend: self.snapshot.configured_backend,
            active_backend: self.snapshot.active_backend,
            fallback_path: backend_diagnostics.last_fallback_path,
            unavailable_reason: backend_unavailable.or(backend_diagnostics.unavailable_reason),
            last_error: status_error.or(backend_diagnostics.last_error),
        }
    }

    pub fn capability_status(&self) -> crate::services::capabilities::CapabilityStatus {
        crate::services::capabilities::CapabilityStatus {
            key: "net",
            label: "Network",
            mode: if self.snapshot.configured_backend == self.snapshot.active_backend {
                crate::services::capabilities::CapabilityMode::Native
            } else {
                crate::services::capabilities::CapabilityMode::Fallback
            },
            provider: format!(
                "{:?}->{:?}",
                self.snapshot.configured_backend, self.snapshot.active_backend
            ),
            detail: (self.snapshot.configured_backend != self.snapshot.active_backend)
                .then(|| "configured backend is not implemented at runtime".to_string()),
        }
    }

    pub fn handle_command(
        &mut self,
        command: NetworkCommand,
        dbus_available: bool,
    ) -> NetworkFollowUp {
        match command {
            NetworkCommand::ToggleMenu => {
                self.snapshot.menu_open = !self.snapshot.menu_open;
                if self.snapshot.menu_open {
                    if dbus_available {
                        self.snapshot.status = NetworkStatus::Scanning;
                        self.snapshot.available_networks.clear();
                        return NetworkFollowUp::Scan;
                    }
                    self.snapshot.status = NetworkStatus::Error(
                        "D-Bus недоступен: не удалось открыть system bus".to_string(),
                    );
                }
                NetworkFollowUp::None
            }
            NetworkCommand::SelectNetwork { ssid, security } => {
                if security == "open" {
                    if dbus_available {
                        self.snapshot.status = NetworkStatus::Connecting(ssid.clone());
                        return NetworkFollowUp::Connect {
                            ssid,
                            passphrase: None,
                        };
                    }
                    self.snapshot.status = NetworkStatus::Error(
                        "D-Bus недоступен: подключение невозможно".to_string(),
                    );
                    return NetworkFollowUp::None;
                }

                self.snapshot.password_input.clear();
                self.snapshot.status = NetworkStatus::AwaitingPassword(ssid);
                NetworkFollowUp::None
            }
            NetworkCommand::UpdatePassword(value) => {
                self.snapshot.password_input = value;
                NetworkFollowUp::None
            }
            NetworkCommand::SubmitPassword => {
                let Some(ssid) = self
                    .snapshot
                    .awaiting_password_ssid()
                    .map(|ssid| ssid.to_string())
                else {
                    self.snapshot.status =
                        NetworkStatus::Error("Пароль не требуется для выбранной сети".to_string());
                    return NetworkFollowUp::None;
                };

                if !dbus_available {
                    self.snapshot.status = NetworkStatus::Error(
                        "D-Bus недоступен: подключение невозможно".to_string(),
                    );
                    return NetworkFollowUp::None;
                }

                self.snapshot.status = NetworkStatus::Connecting(ssid.clone());
                self.snapshot.menu_open = false;
                NetworkFollowUp::Connect {
                    ssid,
                    passphrase: Some(self.snapshot.password_input.clone()),
                }
            }
            NetworkCommand::CancelPassword => {
                self.snapshot.status = NetworkStatus::Info("Подключение отменено".to_string());
                NetworkFollowUp::None
            }
            NetworkCommand::ToggleWifi(enable) => {
                if !dbus_available {
                    self.snapshot.status = NetworkStatus::Error(
                        "D-Bus недоступен: переключение невозможно".to_string(),
                    );
                    return NetworkFollowUp::None;
                }

                self.snapshot.wifi.enabled = enable;
                self.snapshot.status = NetworkStatus::Info(if enable {
                    "Включение Wi-Fi...".to_string()
                } else {
                    "Отключение Wi-Fi...".to_string()
                });
                NetworkFollowUp::TogglePower(enable)
            }
            NetworkCommand::CloseTransientUi => {
                self.snapshot.menu_open = false;
                if matches!(self.snapshot.status, NetworkStatus::AwaitingPassword(_)) {
                    self.snapshot.status = NetworkStatus::Idle;
                }
                NetworkFollowUp::None
            }
        }
    }

    pub fn handle_event(&mut self, event: NetworkEvent) {
        match event {
            NetworkEvent::WifiInfoSynced(info) => {
                self.snapshot.wifi = info;
                if matches!(
                    self.snapshot.status,
                    NetworkStatus::Scanning
                        | NetworkStatus::Connecting(_)
                        | NetworkStatus::AwaitingPassword(_)
                ) {
                    return;
                }

                self.snapshot.status = if self.snapshot.wifi.enabled {
                    let ssid = self.snapshot.wifi.ssid.trim();
                    if ssid.is_empty() || ssid == "Disconnected" || ssid == "Loading..." {
                        NetworkStatus::Info("Wi-Fi включен, сеть не определена".to_string())
                    } else {
                        NetworkStatus::Info(format!("Wi-Fi: {}", ssid))
                    }
                } else {
                    NetworkStatus::Info("Wi-Fi выключен".to_string())
                };
            }
            NetworkEvent::ScanCompleted(networks) => {
                self.snapshot.available_networks = networks;
                self.snapshot.status = if self.snapshot.available_networks.is_empty() {
                    NetworkStatus::Info("Сети не найдены или сканирование недоступно".to_string())
                } else {
                    NetworkStatus::Info(format!(
                        "Найдено сетей: {}",
                        self.snapshot.available_networks.len()
                    ))
                };
            }
            NetworkEvent::ConnectCompleted { ssid, success } => {
                self.snapshot.password_input.clear();
                self.snapshot.status = if success {
                    NetworkStatus::Info(format!("Подключено: {}", ssid))
                } else {
                    NetworkStatus::Error(format!("Не удалось подключиться к {}", ssid))
                };
            }
        }
    }

    pub fn close_transient_ui(&mut self) {
        let _ = self.handle_command(NetworkCommand::CloseTransientUi, false);
    }

    pub async fn get_wifi_info(&self, conn: &Connection) -> WifiInfo {
        <iwd::IwdBackend as NetworkBackend>::get_wifi_info(&self.backend, conn).await
    }

    pub async fn scan_networks(&self, conn: &Connection) -> Vec<WifiNetwork> {
        <iwd::IwdBackend as NetworkBackend>::scan_networks(&self.backend, conn).await
    }

    pub async fn connect_network(
        &self,
        conn: &Connection,
        ssid: String,
        passphrase: Option<String>,
    ) -> bool {
        <iwd::IwdBackend as NetworkBackend>::connect_network(&self.backend, conn, ssid, passphrase)
            .await
    }

    pub async fn toggle_wifi(&self, conn: &Connection, enable: bool) {
        <iwd::IwdBackend as NetworkBackend>::toggle_wifi(&self.backend, conn, enable).await;
    }
}

#[cfg(test)]
mod tests {
    use super::{
        types::NetworkStatus, NetworkBackendKind, NetworkCommand, NetworkEvent, NetworkFollowUp,
        NetworkService, WifiInfo,
    };

    #[test]
    fn network_service_uses_config_paths() {
        let cfg = crate::config::NetworkConfig {
            backend: "iwd".to_string(),
            adapter_path: "/a".to_string(),
            station_path: "/b".to_string(),
        };
        let service = NetworkService::new(&cfg);
        assert_eq!(service.configured_backend(), NetworkBackendKind::Iwd);
        assert_eq!(service.active_backend(), NetworkBackendKind::Iwd);
        assert_eq!(service.backend.adapter_path(), "/a");
        assert_eq!(service.backend.station_path(), "/b");
    }

    #[test]
    fn toggle_menu_requests_scan_when_dbus_available() {
        let mut service = NetworkService::new(&crate::config::NetworkConfig::default());
        assert_eq!(
            service.handle_command(NetworkCommand::ToggleMenu, true),
            NetworkFollowUp::Scan
        );
        assert!(service.snapshot().menu_open);
        assert_eq!(service.snapshot().status, NetworkStatus::Scanning);
    }

    #[test]
    fn secure_network_selection_requires_password() {
        let mut service = NetworkService::new(&crate::config::NetworkConfig::default());
        assert_eq!(
            service.handle_command(
                NetworkCommand::SelectNetwork {
                    ssid: "Home".to_string(),
                    security: "psk".to_string(),
                },
                true
            ),
            NetworkFollowUp::None
        );
        assert_eq!(service.snapshot().awaiting_password_ssid(), Some("Home"));
    }

    #[test]
    fn submit_password_emits_connect_follow_up() {
        let mut service = NetworkService::new(&crate::config::NetworkConfig::default());
        let _ = service.handle_command(
            NetworkCommand::SelectNetwork {
                ssid: "Home".to_string(),
                security: "psk".to_string(),
            },
            true,
        );
        let _ = service.handle_command(NetworkCommand::UpdatePassword("secret".to_string()), true);

        assert_eq!(
            service.handle_command(NetworkCommand::SubmitPassword, true),
            NetworkFollowUp::Connect {
                ssid: "Home".to_string(),
                passphrase: Some("secret".to_string()),
            }
        );
        assert!(!service.snapshot().menu_open);
    }

    #[test]
    fn connect_result_updates_status() {
        let mut service = NetworkService::new(&crate::config::NetworkConfig::default());
        service.handle_event(NetworkEvent::ConnectCompleted {
            ssid: "Home".to_string(),
            success: true,
        });
        assert_eq!(
            service.snapshot().status,
            NetworkStatus::Info("Подключено: Home".to_string())
        );
    }

    #[test]
    fn wifi_sync_does_not_override_connecting_status() {
        let mut service = NetworkService::new(&crate::config::NetworkConfig::default());
        let _ = service.handle_command(
            NetworkCommand::SelectNetwork {
                ssid: "Home".to_string(),
                security: "open".to_string(),
            },
            true,
        );

        service.handle_event(NetworkEvent::WifiInfoSynced(WifiInfo {
            enabled: true,
            ssid: "Other".to_string(),
        }));

        assert_eq!(
            service.snapshot().status,
            NetworkStatus::Connecting("Home".to_string())
        );
    }

    #[test]
    fn diagnostics_surface_status_errors() {
        let mut service = NetworkService::new(&crate::config::NetworkConfig::default());
        let _ = service.handle_command(NetworkCommand::ToggleMenu, false);
        let diagnostics = service.diagnostics();
        assert_eq!(
            diagnostics.last_error.as_deref(),
            Some("D-Bus недоступен: не удалось открыть system bus")
        );
    }
}
