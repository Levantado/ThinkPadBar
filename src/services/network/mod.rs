use zbus::Connection;

pub use crate::modules::wifi::{WifiInfo, WifiNetwork};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NetworkBackendKind {
    #[default]
    Iwd,
}

#[derive(Debug, Clone)]
pub struct NetworkService {
    adapter_path: String,
    station_path: String,
    backend: NetworkBackendKind,
}

impl NetworkService {
    pub fn new(config: &crate::config::NetworkConfig) -> Self {
        Self {
            adapter_path: config.adapter_path.clone(),
            station_path: config.station_path.clone(),
            backend: NetworkBackendKind::Iwd,
        }
    }

    pub async fn get_wifi_info(&self, conn: &Connection) -> WifiInfo {
        match self.backend {
            NetworkBackendKind::Iwd => {
                crate::modules::wifi::get_wifi_info(conn, &self.adapter_path, &self.station_path)
                    .await
            }
        }
    }

    pub async fn scan_networks(&self, conn: &Connection) -> Vec<WifiNetwork> {
        match self.backend {
            NetworkBackendKind::Iwd => {
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
        match self.backend {
            NetworkBackendKind::Iwd => {
                crate::modules::wifi::connect_network(conn, &self.station_path, ssid, passphrase)
                    .await
            }
        }
    }

    pub async fn toggle_wifi(&self, conn: &Connection, enable: bool) {
        match self.backend {
            NetworkBackendKind::Iwd => {
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
}

#[cfg(test)]
mod tests {
    use super::{NetworkBackendKind, NetworkService};

    #[test]
    fn network_service_uses_config_paths() {
        let cfg = crate::config::NetworkConfig {
            adapter_path: "/a".to_string(),
            station_path: "/b".to_string(),
        };
        let service = NetworkService::new(&cfg);
        assert_eq!(service.backend, NetworkBackendKind::Iwd);
        assert_eq!(service.adapter_path, "/a");
        assert_eq!(service.station_path, "/b");
    }
}
