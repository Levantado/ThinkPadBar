use zbus::Connection;

pub use crate::modules::wifi::{WifiInfo, WifiNetwork};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NetworkBackendKind {
    #[default]
    Iwd,
    NetworkManager,
}

#[derive(Debug, Clone)]
pub struct NetworkService {
    adapter_path: String,
    station_path: String,
    configured_backend: NetworkBackendKind,
    active_backend: NetworkBackendKind,
}

impl NetworkService {
    pub fn new(config: &crate::config::NetworkConfig) -> Self {
        let configured_backend = match config.backend.trim().to_ascii_lowercase().as_str() {
            "networkmanager" => NetworkBackendKind::NetworkManager,
            _ => NetworkBackendKind::Iwd,
        };
        Self {
            adapter_path: config.adapter_path.clone(),
            station_path: config.station_path.clone(),
            configured_backend,
            active_backend: NetworkBackendKind::Iwd,
        }
    }

    pub async fn get_wifi_info(&self, conn: &Connection) -> WifiInfo {
        match self.active_backend {
            NetworkBackendKind::Iwd | NetworkBackendKind::NetworkManager => {
                crate::modules::wifi::get_wifi_info(conn, &self.adapter_path, &self.station_path)
                    .await
            }
        }
    }

    pub async fn scan_networks(&self, conn: &Connection) -> Vec<WifiNetwork> {
        match self.active_backend {
            NetworkBackendKind::Iwd | NetworkBackendKind::NetworkManager => {
                crate::modules::wifi::scan_networks(conn, &self.station_path).await
            }
        }
    }

    pub async fn connect_network(
        &self,
        conn: &Connection,
        ssid: String,
        passphrase: Option<String>,
    ) -> bool {
        match self.active_backend {
            NetworkBackendKind::Iwd | NetworkBackendKind::NetworkManager => {
                crate::modules::wifi::connect_network(conn, &self.station_path, ssid, passphrase)
                    .await
            }
        }
    }

    pub async fn toggle_wifi(&self, conn: &Connection, enable: bool) {
        match self.active_backend {
            NetworkBackendKind::Iwd | NetworkBackendKind::NetworkManager => {
                crate::modules::wifi::toggle_wifi(
                    conn,
                    &self.adapter_path,
                    &self.station_path,
                    enable,
                )
                .await
            }
        }
    }

    pub fn configured_backend(&self) -> NetworkBackendKind {
        self.configured_backend
    }

    pub fn active_backend(&self) -> NetworkBackendKind {
        self.active_backend
    }
}

#[cfg(test)]
mod tests {
    use super::{NetworkBackendKind, NetworkService};

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
        assert_eq!(service.adapter_path, "/a");
        assert_eq!(service.station_path, "/b");
    }

    #[test]
    fn networkmanager_config_falls_back_to_iwd_runtime() {
        let cfg = crate::config::NetworkConfig {
            backend: "networkmanager".to_string(),
            adapter_path: "/a".to_string(),
            station_path: "/b".to_string(),
        };
        let service = NetworkService::new(&cfg);
        assert_eq!(
            service.configured_backend(),
            NetworkBackendKind::NetworkManager
        );
        assert_eq!(service.active_backend(), NetworkBackendKind::Iwd);
    }
}
