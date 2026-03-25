use std::collections::HashSet;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use zbus::{proxy, Connection};

use crate::config::NetworkConfig;

use super::types::{WifiInfo, WifiNetwork};

const DISCOVERY_CACHE_TTL: Duration = Duration::from_secs(30);

#[proxy(
    interface = "net.connman.iwd.Station",
    default_service = "net.connman.iwd"
)]
trait Station {
    #[zbus(property)]
    fn connected_network(&self) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;

    fn scan(&self) -> zbus::Result<()>;
    fn get_ordered_networks(&self) -> zbus::Result<Vec<(zbus::zvariant::OwnedObjectPath, i16)>>;
}

#[proxy(
    interface = "net.connman.iwd.Network",
    default_service = "net.connman.iwd"
)]
trait Network {
    #[zbus(property)]
    fn name(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn type_(&self) -> zbus::Result<String>;

    fn connect(&self) -> zbus::Result<()>;
}

#[proxy(
    interface = "net.connman.iwd.Adapter",
    default_service = "net.connman.iwd"
)]
trait Adapter {
    #[zbus(property)]
    fn powered(&self) -> zbus::Result<bool>;

    #[zbus(property, name = "Powered")]
    fn set_powered(&self, value: bool) -> zbus::Result<()>;
}

#[derive(Debug, Clone)]
struct PathCache {
    value: Option<(String, String)>,
    at: Instant,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IwdBackendDiagnostics {
    pub last_fallback_path: Option<String>,
    pub last_error: Option<String>,
    pub unavailable_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct IwdBackend {
    adapter_path: String,
    station_path: String,
    discovery_cache: Arc<Mutex<PathCache>>,
    diagnostics: Arc<Mutex<IwdBackendDiagnostics>>,
}

impl IwdBackend {
    pub fn new(config: &NetworkConfig) -> Self {
        Self {
            adapter_path: config.adapter_path.clone(),
            station_path: config.station_path.clone(),
            discovery_cache: Arc::new(Mutex::new(PathCache {
                value: None,
                at: Instant::now() - DISCOVERY_CACHE_TTL,
            })),
            diagnostics: Arc::new(Mutex::new(IwdBackendDiagnostics::default())),
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

    fn is_valid_object_path(path: &str) -> bool {
        zbus::zvariant::ObjectPath::try_from(path).is_ok()
    }

    fn are_config_paths_valid(adapter_path: &str, station_path: &str) -> bool {
        Self::is_valid_object_path(adapter_path) && Self::is_valid_object_path(station_path)
    }

    fn parse_ssid_from_iw_dev(output: &str) -> Option<String> {
        output.lines().find_map(|line| {
            let trimmed = line.trim_start();
            trimmed.strip_prefix("ssid ").and_then(|value| {
                let ssid = value.trim();
                if ssid.is_empty() {
                    None
                } else {
                    Some(ssid.to_string())
                }
            })
        })
    }

    fn fallback_current_ssid() -> Option<String> {
        let iwgetid = Command::new("iwgetid")
            .args(["-r"])
            .output()
            .ok()
            .and_then(|output| {
                if !output.status.success() {
                    return None;
                }
                let ssid = String::from_utf8(output.stdout).ok()?.trim().to_string();
                if ssid.is_empty() {
                    None
                } else {
                    Some(ssid)
                }
            });
        if iwgetid.is_some() {
            return iwgetid;
        }

        Command::new("iw")
            .args(["dev"])
            .output()
            .ok()
            .and_then(|output| {
                if !output.status.success() {
                    return None;
                }
                let stdout = String::from_utf8(output.stdout).ok()?;
                Self::parse_ssid_from_iw_dev(&stdout)
            })
    }

    fn parse_discovered_paths(stdout: &str) -> Option<(String, String)> {
        let mut paths = Vec::new();
        for line in stdout.lines() {
            if let Some(path) = line
                .split_whitespace()
                .find(|candidate| candidate.starts_with("/net/connman/iwd/"))
            {
                paths.push(path.to_string());
            }
        }
        if paths.is_empty() {
            return None;
        }

        for station in &paths {
            let depth = station
                .split('/')
                .filter(|segment| !segment.is_empty())
                .count();
            if depth != 5 {
                continue;
            }
            let Some(last) = station.rsplit('/').next() else {
                continue;
            };
            if last.contains("_psk")
                || last.contains("_open")
                || last.contains("_wep")
                || last.contains("_8021x")
            {
                continue;
            }

            let mut parts: Vec<&str> = station.split('/').collect();
            parts.pop();
            let adapter = parts.join("/");
            if Self::are_config_paths_valid(&adapter, station) {
                return Some((adapter, station.clone()));
            }
        }

        for adapter in &paths {
            let depth = adapter
                .split('/')
                .filter(|segment| !segment.is_empty())
                .count();
            if depth == 4 {
                let station_candidate = format!("{}/wlan0", adapter);
                if Self::are_config_paths_valid(adapter, &station_candidate) {
                    return Some((adapter.clone(), station_candidate));
                }
            }
        }

        None
    }

    fn discover_iwd_paths(&self) -> Option<(String, String)> {
        let output = Command::new("busctl")
            .args(["tree", "net.connman.iwd"])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let stdout = String::from_utf8(output.stdout).ok()?;
        Self::parse_discovered_paths(&stdout)
    }

    fn discover_iwd_paths_cached(&self) -> Option<(String, String)> {
        if let Ok(guard) = self.discovery_cache.lock() {
            if guard.at.elapsed() < DISCOVERY_CACHE_TTL {
                return guard.value.clone();
            }
        }

        let refreshed = self.discover_iwd_paths();
        if let Ok(mut guard) = self.discovery_cache.lock() {
            guard.value = refreshed.clone();
            guard.at = Instant::now();
        }
        refreshed
    }

    pub fn diagnostics(&self) -> IwdBackendDiagnostics {
        self.diagnostics
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    fn update_diagnostics(&self, apply: impl FnOnce(&mut IwdBackendDiagnostics)) {
        if let Ok(mut guard) = self.diagnostics.lock() {
            apply(&mut guard);
        }
    }

    fn record_fallback_path(&self, path: &'static str) {
        self.update_diagnostics(|diagnostics| {
            diagnostics.last_fallback_path = Some(path.to_string());
        });
    }

    fn record_error(&self, error: impl Into<String>) {
        let error = error.into();
        self.update_diagnostics(|diagnostics| {
            diagnostics.last_error = Some(error);
        });
    }

    fn clear_error(&self) {
        self.update_diagnostics(|diagnostics| diagnostics.last_error = None);
    }

    fn set_unavailable_reason(&self, reason: impl Into<String>) {
        let reason = reason.into();
        self.update_diagnostics(|diagnostics| {
            diagnostics.unavailable_reason = Some(reason);
        });
    }

    fn clear_unavailable_reason(&self) {
        self.update_diagnostics(|diagnostics| diagnostics.unavailable_reason = None);
    }

    fn candidate_paths(&self) -> Vec<(String, String)> {
        let mut out = Vec::new();
        let mut seen = HashSet::new();

        let push_unique = |adapter: String,
                           station: String,
                           out: &mut Vec<(String, String)>,
                           seen: &mut HashSet<String>| {
            let key = format!("{}|{}", adapter, station);
            if seen.insert(key) {
                out.push((adapter, station));
            }
        };

        if let Some((adapter, station)) = self.discover_iwd_paths_cached() {
            push_unique(adapter, station, &mut out, &mut seen);
        }
        if Self::are_config_paths_valid(&self.adapter_path, &self.station_path) {
            push_unique(
                self.adapter_path.clone(),
                self.station_path.clone(),
                &mut out,
                &mut seen,
            );
        }

        out
    }

    fn candidate_station_paths(&self) -> Vec<String> {
        let mut out = Vec::new();
        let mut seen = HashSet::new();

        if Self::is_valid_object_path(&self.station_path) && seen.insert(self.station_path.clone())
        {
            out.push(self.station_path.clone());
        }
        if let Some((_, station)) = self.discover_iwd_paths_cached() {
            if seen.insert(station.clone()) {
                out.push(station);
            }
        }

        out
    }

    fn strip_ansi_csi(input: &str) -> String {
        let bytes = input.as_bytes();
        let mut out = String::with_capacity(input.len());
        let mut index = 0usize;
        while index < bytes.len() {
            if bytes[index] == 0x1b && index + 1 < bytes.len() && bytes[index + 1] == b'[' {
                index += 2;
                while index < bytes.len() {
                    let byte = bytes[index];
                    if (0x40..=0x7e).contains(&byte) {
                        index += 1;
                        break;
                    }
                    index += 1;
                }
                continue;
            }
            out.push(bytes[index] as char);
            index += 1;
        }
        out
    }

    fn parse_networks_from_iwctl(output: &str) -> Vec<WifiNetwork> {
        let mut networks = Vec::new();
        for raw_line in output.lines() {
            let clean = Self::strip_ansi_csi(raw_line);
            let line = clean.trim().trim_start_matches(['>', '*', ' ']).trim();
            if line.is_empty() || line.contains("Available networks") || line.starts_with("- ") {
                continue;
            }
            if line.chars().all(|ch| ch == '-') {
                continue;
            }

            let lower = line.to_lowercase();
            if lower.contains("connected")
                || lower.contains("known")
                || lower.contains("network name")
                || lower.contains("security")
                || lower.contains("signal")
            {
                continue;
            }

            let tokens: Vec<&str> = line.split_whitespace().collect();
            if tokens.is_empty() {
                continue;
            }

            let is_meta_token = |token: &str| {
                let lower = token.to_lowercase();
                lower == "open"
                    || lower == "psk"
                    || lower.starts_with("wpa")
                    || lower == "wep"
                    || lower == "802.1x"
                    || lower == "connected"
                    || lower == "known"
            };
            let cutoff = tokens
                .iter()
                .position(|token| is_meta_token(token))
                .unwrap_or(tokens.len());
            let ssid = tokens[..cutoff].join(" ");

            if ssid.is_empty() || ssid.eq_ignore_ascii_case("SSID") {
                continue;
            }

            let security = if lower.contains("open") {
                "open"
            } else if lower.contains("wpa") || lower.contains("psk") || lower.contains("802.1x") {
                "psk"
            } else {
                "unknown"
            };

            if !networks
                .iter()
                .any(|network: &WifiNetwork| network.ssid == ssid)
            {
                networks.push(WifiNetwork {
                    ssid: ssid.to_string(),
                    security: security.to_string(),
                });
            }
        }
        networks
    }

    fn iface_from_station_path(station_path: &str) -> Option<String> {
        station_path
            .split('/')
            .rfind(|segment| !segment.is_empty())
            .map(ToString::to_string)
    }

    fn build_iwctl_connect_args(iface: &str, ssid: &str, passphrase: Option<&str>) -> Vec<String> {
        let mut args = Vec::new();
        if let Some(passphrase) = passphrase {
            if !passphrase.is_empty() {
                args.push("--passphrase".to_string());
                args.push(passphrase.to_string());
            }
        }
        args.push("station".to_string());
        args.push(iface.to_string());
        args.push("connect".to_string());
        args.push(ssid.to_string());
        args
    }

    fn fallback_connect_with_iwctl(
        &self,
        station_path: &str,
        ssid: &str,
        passphrase: Option<&str>,
    ) -> bool {
        let Some(iface) = Self::iface_from_station_path(station_path) else {
            return false;
        };
        if iface.is_empty() {
            return false;
        }

        Command::new("iwctl")
            .args(Self::build_iwctl_connect_args(&iface, ssid, passphrase))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    fn fallback_scan_networks(&self, station_path: &str) -> Vec<WifiNetwork> {
        let Some(iface) = Self::iface_from_station_path(station_path) else {
            return Vec::new();
        };

        let Ok(output) = Command::new("iwctl")
            .args(["station", iface.as_str(), "get-networks"])
            .output()
        else {
            return Vec::new();
        };

        if !output.status.success() {
            return Vec::new();
        }

        let Ok(stdout) = String::from_utf8(output.stdout) else {
            return Vec::new();
        };

        Self::parse_networks_from_iwctl(&stdout)
    }

    pub async fn get_wifi_info(&self, conn: &Connection) -> WifiInfo {
        self.clear_error();
        let mut enabled = false;
        let mut ssid = String::from("Disconnected");
        let candidate_paths = self.candidate_paths();

        if candidate_paths.is_empty() {
            self.set_unavailable_reason("no usable IWD adapter/station paths");
        } else {
            self.clear_unavailable_reason();
        }

        for (adapter_path, station_path) in candidate_paths {
            enabled = false;
            ssid = String::from("Disconnected");

            if let Ok(builder) = AdapterProxy::builder(conn).path(adapter_path.as_str()) {
                if let Ok(proxy) = builder.build().await {
                    enabled = proxy.powered().await.unwrap_or(false);
                }
            }

            if enabled {
                if let Ok(station_builder) = StationProxy::builder(conn).path(station_path.as_str())
                {
                    if let Ok(station) = station_builder.build().await {
                        if let Ok(path) = station.connected_network().await {
                            if let Ok(network_builder) = NetworkProxy::builder(conn).path(path) {
                                if let Ok(network) = network_builder.build().await {
                                    if let Ok(name) = network.name().await {
                                        ssid = name;
                                    }
                                }
                            }
                        }
                    }
                }

                if ssid == "Disconnected" || ssid.is_empty() {
                    if let Some(found) = Self::fallback_current_ssid() {
                        self.record_fallback_path("ssid:iwgetid/iw");
                        ssid = found;
                    }
                }
                return WifiInfo { enabled, ssid };
            }
        }

        if let Some(found) = Self::fallback_current_ssid() {
            self.record_fallback_path("ssid:iwgetid/iw");
            ssid = found;
        }

        WifiInfo { enabled, ssid }
    }

    pub async fn scan_networks(&self, conn: &Connection) -> Vec<WifiNetwork> {
        self.clear_error();
        let mut networks = Vec::new();
        let station_paths = self.candidate_station_paths();

        if station_paths.is_empty() {
            self.set_unavailable_reason("no usable IWD station paths");
        } else {
            self.clear_unavailable_reason();
        }

        for station_path in station_paths {
            networks.clear();
            if let Ok(station_builder) = StationProxy::builder(conn).path(station_path.as_str()) {
                if let Ok(station) = station_builder.build().await {
                    let _ = station.scan().await;

                    let mut ordered_networks = Vec::new();
                    for _ in 0..10 {
                        if let Ok(ordered) = station.get_ordered_networks().await {
                            if !ordered.is_empty() {
                                ordered_networks = ordered;
                                break;
                            }
                        }
                        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
                    }

                    for (path, _) in ordered_networks {
                        if let Ok(network_builder) = NetworkProxy::builder(conn).path(path) {
                            if let Ok(network) = network_builder.build().await {
                                if let (Ok(name), Ok(security)) =
                                    (network.name().await, network.type_().await)
                                {
                                    if !networks
                                        .iter()
                                        .any(|network: &WifiNetwork| network.ssid == name)
                                    {
                                        networks.push(WifiNetwork {
                                            ssid: name,
                                            security,
                                        });
                                    }
                                }
                            }
                        }
                    }
                    if networks.is_empty() {
                        networks = self.fallback_scan_networks(station_path.as_str());
                        if !networks.is_empty() {
                            self.record_fallback_path("scan:iwctl");
                        }
                    }
                    if !networks.is_empty() {
                        return networks;
                    }
                }
            }
        }

        networks
    }

    pub async fn connect_network(
        &self,
        conn: &Connection,
        ssid: String,
        passphrase: Option<String>,
    ) -> bool {
        self.clear_error();
        let station_paths = self.candidate_station_paths();
        if station_paths.is_empty() {
            self.set_unavailable_reason("no usable IWD station paths");
        } else {
            self.clear_unavailable_reason();
        }

        for station_path in station_paths {
            if let Ok(station_builder) = StationProxy::builder(conn).path(station_path.as_str()) {
                if let Ok(station) = station_builder.build().await {
                    if let Ok(ordered) = station.get_ordered_networks().await {
                        for (path, _) in ordered {
                            if let Ok(network_builder) = NetworkProxy::builder(conn).path(path) {
                                if let Ok(network) = network_builder.build().await {
                                    if let Ok(name) = network.name().await {
                                        if name == ssid {
                                            if network.connect().await.is_ok() {
                                                return true;
                                            }
                                            let fallback_ok = self.fallback_connect_with_iwctl(
                                                station_path.as_str(),
                                                &ssid,
                                                passphrase.as_deref(),
                                            );
                                            if fallback_ok {
                                                self.record_fallback_path("connect:iwctl");
                                            } else {
                                                self.record_error(format!(
                                                    "connect failed via iwd and iwctl for {}",
                                                    ssid
                                                ));
                                            }
                                            return fallback_ok;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    let fallback_ok = self.fallback_connect_with_iwctl(
                        station_path.as_str(),
                        &ssid,
                        passphrase.as_deref(),
                    );
                    if fallback_ok {
                        self.record_fallback_path("connect:iwctl");
                        return true;
                    }
                }
            }
        }
        self.record_error(format!("unable to locate/connect network {}", ssid));
        false
    }

    pub async fn toggle_wifi(&self, conn: &Connection, enable: bool) {
        self.clear_error();
        let candidate_paths = self.candidate_paths();
        if candidate_paths.is_empty() {
            self.set_unavailable_reason("no usable IWD adapter paths");
        } else {
            self.clear_unavailable_reason();
        }

        for (adapter_path, _) in candidate_paths {
            if let Ok(builder) = AdapterProxy::builder(conn).path(adapter_path.as_str()) {
                if let Ok(adapter) = builder.build().await {
                    if adapter.set_powered(enable).await.is_err() {
                        self.record_error(format!("failed to set Wi-Fi powered={enable}"));
                    }
                    return;
                }
            }
        }
        self.record_error(format!(
            "unable to find adapter to set Wi-Fi powered={enable}"
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::IwdBackend;
    use crate::services::network::types::WifiNetwork;

    #[test]
    fn object_path_validation_accepts_valid_paths() {
        assert!(IwdBackend::is_valid_object_path("/net/connman/iwd/0"));
        assert!(IwdBackend::is_valid_object_path("/net/connman/iwd/0/wlan0"));
    }

    #[test]
    fn object_path_validation_rejects_invalid_paths() {
        assert!(!IwdBackend::is_valid_object_path(""));
        assert!(!IwdBackend::is_valid_object_path("net/connman/iwd/0"));
        assert!(!IwdBackend::is_valid_object_path("/net//connman"));
    }

    #[test]
    fn parse_ssid_from_iw_dev_extracts_ssid() {
        let sample = "phy#0\n\tInterface wlan0\n\t\ttype managed\n\t\tssid MyNet\n";
        assert_eq!(
            IwdBackend::parse_ssid_from_iw_dev(sample),
            Some("MyNet".to_string())
        );
    }

    #[test]
    fn parse_networks_from_iwctl_extracts_entries() {
        let sample = "Available networks\n--------------------------------------------------------------------------------\nNetwork1           psk\nOpenHotspot        open\n";

        assert_eq!(
            IwdBackend::parse_networks_from_iwctl(sample),
            vec![
                WifiNetwork {
                    ssid: "Network1".to_string(),
                    security: "psk".to_string(),
                },
                WifiNetwork {
                    ssid: "OpenHotspot".to_string(),
                    security: "open".to_string(),
                },
            ]
        );
    }

    #[test]
    fn parse_networks_from_iwctl_strips_ansi_and_headers() {
        let sample = "\u{1b}[0m--------------------------------------------------------------\n\
\u{1b}[0m\u{1b}[1;90mNetwork name             Security        Signal\n\
\u{1b}[0m--------------------------------------------------------------\n\
\u{1b}[0m \u{1b}[1;90m>\u{1b}[0m v83Etz9b_Plus_5G      psk             ***\n";

        assert_eq!(
            IwdBackend::parse_networks_from_iwctl(sample),
            vec![WifiNetwork {
                ssid: "v83Etz9b_Plus_5G".to_string(),
                security: "psk".to_string(),
            }]
        );
    }

    #[test]
    fn parse_discovered_paths_prefers_station_level_path_shape() {
        let sample = "\
/net/connman/iwd/0
/net/connman/iwd/0/5
/net/connman/iwd/0/5/79383345747a39625f506c75735f3547_psk
";

        assert_eq!(
            IwdBackend::parse_discovered_paths(sample),
            Some((
                "/net/connman/iwd/0".to_string(),
                "/net/connman/iwd/0/5".to_string(),
            ))
        );
    }

    #[test]
    fn iwctl_connect_args_include_passphrase_when_provided() {
        let args = IwdBackend::build_iwctl_connect_args("wlan0", "MyWiFi", Some("secret"));
        assert_eq!(
            args,
            vec![
                "--passphrase",
                "secret",
                "station",
                "wlan0",
                "connect",
                "MyWiFi",
            ]
        );
    }

    #[test]
    fn iwctl_connect_args_without_passphrase_for_open_network() {
        let args = IwdBackend::build_iwctl_connect_args("wlan0", "OpenWiFi", None);
        assert_eq!(args, vec!["station", "wlan0", "connect", "OpenWiFi"]);
    }
}
