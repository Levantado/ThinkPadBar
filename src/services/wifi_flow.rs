use crate::services::network::{WifiInfo, WifiNetwork};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WifiFlowSnapshot {
    pub wifi: WifiInfo,
    pub menu_open: bool,
    pub available_networks: Vec<WifiNetwork>,
    pub password_input: String,
    pub selected_ssid: Option<String>,
    pub connecting_ssid: Option<String>,
    pub status_message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WifiFlowCommand {
    None,
    Scan,
    Connect {
        ssid: String,
        passphrase: Option<String>,
    },
    TogglePower(bool),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WifiFlowService {
    snapshot: WifiFlowSnapshot,
}

impl Default for WifiFlowService {
    fn default() -> Self {
        Self {
            snapshot: WifiFlowSnapshot {
                wifi: WifiInfo {
                    enabled: false,
                    ssid: "Loading...".to_string(),
                },
                menu_open: false,
                available_networks: Vec::new(),
                password_input: String::new(),
                selected_ssid: None,
                connecting_ssid: None,
                status_message: String::new(),
            },
        }
    }
}

impl WifiFlowService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> &WifiFlowSnapshot {
        &self.snapshot
    }

    pub fn sync_wifi_info(&mut self, info: WifiInfo) {
        self.snapshot.wifi = info;
        if self.snapshot.wifi.enabled {
            let ssid = self.snapshot.wifi.ssid.trim();
            if ssid.is_empty() || ssid == "Disconnected" || ssid == "Loading..." {
                self.snapshot.status_message = "Wi-Fi включен, сеть не определена".to_string();
            } else {
                self.snapshot.status_message = format!("Wi-Fi: {}", ssid);
            }
        } else {
            self.snapshot.status_message = "Wi-Fi выключен".to_string();
        }
    }

    pub fn toggle_menu(&mut self, dbus_available: bool) -> WifiFlowCommand {
        self.snapshot.menu_open = !self.snapshot.menu_open;
        if self.snapshot.menu_open {
            if dbus_available {
                self.snapshot.status_message = "Сканирование сетей...".to_string();
                self.snapshot.available_networks.clear();
                return WifiFlowCommand::Scan;
            }
            self.snapshot.status_message =
                "D-Bus недоступен: не удалось открыть system bus".to_string();
        }
        WifiFlowCommand::None
    }

    pub fn apply_scan_results(&mut self, networks: Vec<WifiNetwork>) {
        self.snapshot.available_networks = networks;
        if self.snapshot.available_networks.is_empty() {
            self.snapshot.status_message =
                "Сети не найдены или сканирование недоступно".to_string();
        } else {
            self.snapshot.status_message =
                format!("Найдено сетей: {}", self.snapshot.available_networks.len());
        }
    }

    pub fn select_network(
        &mut self,
        ssid: String,
        security: String,
        dbus_available: bool,
    ) -> WifiFlowCommand {
        if security == "open" {
            if dbus_available {
                self.snapshot.connecting_ssid = Some(ssid.clone());
                self.snapshot.status_message = format!("Подключение к {}...", ssid);
                return WifiFlowCommand::Connect {
                    ssid,
                    passphrase: None,
                };
            }
            self.snapshot.status_message = "D-Bus недоступен: подключение невозможно".to_string();
            return WifiFlowCommand::None;
        }
        self.snapshot.selected_ssid = Some(ssid);
        self.snapshot.password_input.clear();
        self.snapshot.status_message = "Введите пароль и нажмите Connect".to_string();
        WifiFlowCommand::None
    }

    pub fn update_password(&mut self, value: String) {
        self.snapshot.password_input = value;
    }

    pub fn submit_password(&mut self, dbus_available: bool) -> WifiFlowCommand {
        let Some(ssid) = self.snapshot.selected_ssid.clone() else {
            self.snapshot.status_message = "D-Bus недоступен: подключение невозможно".to_string();
            return WifiFlowCommand::None;
        };
        if !dbus_available {
            self.snapshot.status_message = "D-Bus недоступен: подключение невозможно".to_string();
            return WifiFlowCommand::None;
        }
        self.snapshot.connecting_ssid = Some(ssid.clone());
        self.snapshot.status_message = format!("Подключение к {}...", ssid);
        self.snapshot.selected_ssid = None;
        self.snapshot.menu_open = false;
        WifiFlowCommand::Connect {
            ssid,
            passphrase: Some(self.snapshot.password_input.clone()),
        }
    }

    pub fn cancel_password(&mut self) {
        self.snapshot.selected_ssid = None;
        self.snapshot.status_message = "Подключение отменено".to_string();
    }

    pub fn apply_connect_result(&mut self, success: bool) {
        let ssid = self
            .snapshot
            .connecting_ssid
            .take()
            .unwrap_or_else(|| "выбранной сети".to_string());
        if success {
            self.snapshot.status_message = format!("Подключено: {}", ssid);
        } else {
            self.snapshot.status_message = format!("Не удалось подключиться к {}", ssid);
        }
    }

    pub fn toggle_power(&mut self, enable: bool, dbus_available: bool) -> WifiFlowCommand {
        if !dbus_available {
            self.snapshot.status_message = "D-Bus недоступен: переключение невозможно".to_string();
            return WifiFlowCommand::None;
        }
        self.snapshot.status_message = if enable {
            "Включение Wi-Fi...".to_string()
        } else {
            "Отключение Wi-Fi...".to_string()
        };
        self.snapshot.wifi.enabled = enable;
        WifiFlowCommand::TogglePower(enable)
    }

    pub fn close_transient_ui(&mut self) {
        self.snapshot.menu_open = false;
        self.snapshot.selected_ssid = None;
    }
}

#[cfg(test)]
mod tests {
    use super::{WifiFlowCommand, WifiFlowService};
    use crate::services::network::WifiInfo;

    #[test]
    fn toggle_menu_requests_scan_when_dbus_available() {
        let mut service = WifiFlowService::new();
        assert_eq!(service.toggle_menu(true), WifiFlowCommand::Scan);
        assert!(service.snapshot().menu_open);
    }

    #[test]
    fn secure_network_selection_requires_password() {
        let mut service = WifiFlowService::new();
        assert_eq!(
            service.select_network("Home".to_string(), "psk".to_string(), true),
            WifiFlowCommand::None
        );
        assert_eq!(service.snapshot().selected_ssid.as_deref(), Some("Home"));
    }

    #[test]
    fn sync_wifi_info_updates_status_message() {
        let mut service = WifiFlowService::new();
        service.sync_wifi_info(WifiInfo {
            enabled: true,
            ssid: "TestNet".to_string(),
        });
        assert_eq!(service.snapshot().status_message, "Wi-Fi: TestNet");
    }
}
