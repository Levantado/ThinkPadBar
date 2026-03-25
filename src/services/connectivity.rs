use zbus::Connection;

pub use crate::services::network::{NetworkBackendKind, WifiInfo, WifiNetwork};
pub use crate::services::wifi_flow::WifiFlowSnapshot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectivityRequest {
    None,
    Scan,
    Connect {
        ssid: String,
        passphrase: Option<String>,
    },
    TogglePower(bool),
}

impl From<crate::services::wifi_flow::WifiFlowCommand> for ConnectivityRequest {
    fn from(value: crate::services::wifi_flow::WifiFlowCommand) -> Self {
        match value {
            crate::services::wifi_flow::WifiFlowCommand::None => Self::None,
            crate::services::wifi_flow::WifiFlowCommand::Scan => Self::Scan,
            crate::services::wifi_flow::WifiFlowCommand::Connect { ssid, passphrase } => {
                Self::Connect { ssid, passphrase }
            }
            crate::services::wifi_flow::WifiFlowCommand::TogglePower(enabled) => {
                Self::TogglePower(enabled)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConnectivityService {
    network: crate::services::network::NetworkService,
    wifi_flow: crate::services::wifi_flow::WifiFlowService,
}

impl ConnectivityService {
    pub fn new(config: &crate::config::NetworkConfig) -> Self {
        Self {
            network: crate::services::network::NetworkService::new(config),
            wifi_flow: crate::services::wifi_flow::WifiFlowService::new(),
        }
    }

    pub fn snapshot(&self) -> &WifiFlowSnapshot {
        self.wifi_flow.snapshot()
    }

    pub fn configured_backend(&self) -> NetworkBackendKind {
        self.network.configured_backend()
    }

    pub fn active_backend(&self) -> NetworkBackendKind {
        self.network.active_backend()
    }

    pub fn close_transient_ui(&mut self) {
        self.wifi_flow.close_transient_ui();
    }

    pub fn toggle_menu(&mut self, dbus_available: bool) -> ConnectivityRequest {
        self.wifi_flow.toggle_menu(dbus_available).into()
    }

    pub fn apply_scan_results(&mut self, networks: Vec<WifiNetwork>) {
        self.wifi_flow.apply_scan_results(networks);
    }

    pub fn select_network(
        &mut self,
        ssid: String,
        security: String,
        dbus_available: bool,
    ) -> ConnectivityRequest {
        self.wifi_flow
            .select_network(ssid, security, dbus_available)
            .into()
    }

    pub fn update_password(&mut self, value: String) {
        self.wifi_flow.update_password(value);
    }

    pub fn submit_password(&mut self, dbus_available: bool) -> ConnectivityRequest {
        self.wifi_flow.submit_password(dbus_available).into()
    }

    pub fn cancel_password(&mut self) {
        self.wifi_flow.cancel_password();
    }

    pub fn apply_connect_result(&mut self, success: bool) {
        self.wifi_flow.apply_connect_result(success);
    }

    pub fn toggle_power(&mut self, enable: bool, dbus_available: bool) -> ConnectivityRequest {
        self.wifi_flow.toggle_power(enable, dbus_available).into()
    }

    pub fn sync_wifi_info(&mut self, info: WifiInfo) {
        self.wifi_flow.sync_wifi_info(info);
    }

    pub async fn get_wifi_info(&self, conn: &Connection) -> WifiInfo {
        self.network.get_wifi_info(conn).await
    }

    pub async fn scan_networks(&self, conn: &Connection) -> Vec<WifiNetwork> {
        self.network.scan_networks(conn).await
    }

    pub async fn connect_network(
        &self,
        conn: &Connection,
        ssid: String,
        passphrase: Option<String>,
    ) -> bool {
        self.network.connect_network(conn, ssid, passphrase).await
    }

    pub async fn toggle_wifi(&self, conn: &Connection, enable: bool) {
        self.network.toggle_wifi(conn, enable).await
    }
}

#[cfg(test)]
mod tests {
    use super::{ConnectivityRequest, ConnectivityService, NetworkBackendKind};

    #[test]
    fn connectivity_service_exposes_network_backend_state() {
        let service = ConnectivityService::new(&crate::config::NetworkConfig::default());
        assert_eq!(service.configured_backend(), NetworkBackendKind::Iwd);
        assert_eq!(service.active_backend(), NetworkBackendKind::Iwd);
    }

    #[test]
    fn toggle_menu_returns_scan_request_when_dbus_available() {
        let mut service = ConnectivityService::new(&crate::config::NetworkConfig::default());
        assert_eq!(service.toggle_menu(true), ConnectivityRequest::Scan);
        assert!(service.snapshot().menu_open);
    }

    #[test]
    fn secure_network_selection_stays_local_until_password_submit() {
        let mut service = ConnectivityService::new(&crate::config::NetworkConfig::default());
        assert_eq!(
            service.select_network("Home".to_string(), "psk".to_string(), true),
            ConnectivityRequest::None
        );
        assert_eq!(service.snapshot().selected_ssid.as_deref(), Some("Home"));
    }
}
