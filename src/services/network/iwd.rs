use zbus::Connection;

use crate::config::NetworkConfig;

use super::types::{WifiInfo, WifiNetwork};

#[derive(Debug, Clone)]
pub struct IwdBackend {
    adapter_path: String,
    station_path: String,
}

impl IwdBackend {
    pub fn new(config: &NetworkConfig) -> Self {
        Self {
            adapter_path: config.adapter_path.clone(),
            station_path: config.station_path.clone(),
        }
    }

    #[cfg(test)]
    pub fn adapter_path(&self) -> &str {
        &self.adapter_path
    }

    #[cfg(test)]
    pub fn station_path(&self) -> &str {
        &self.station_path
    }

    pub async fn get_wifi_info(&self, conn: &Connection) -> WifiInfo {
        crate::modules::wifi::get_wifi_info(conn, &self.adapter_path, &self.station_path).await
    }

    pub async fn scan_networks(&self, conn: &Connection) -> Vec<WifiNetwork> {
        crate::modules::wifi::scan_networks(conn, &self.station_path).await
    }

    pub async fn connect_network(
        &self,
        conn: &Connection,
        ssid: String,
        passphrase: Option<String>,
    ) -> bool {
        crate::modules::wifi::connect_network(conn, &self.station_path, ssid, passphrase).await
    }

    pub async fn toggle_wifi(&self, conn: &Connection, enable: bool) {
        crate::modules::wifi::toggle_wifi(conn, &self.adapter_path, &self.station_path, enable)
            .await;
    }
}
