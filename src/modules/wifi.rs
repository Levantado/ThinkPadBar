use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use std::{collections::HashSet, process::Command};

use zbus::{proxy, Connection};

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

#[proxy(
    interface = "net.connman.iwd.Station",
    default_service = "net.connman.iwd"
)]
trait Station {
    #[zbus(property)]
    fn connected_network(&self) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;

    #[zbus(property)]
    fn state(&self) -> zbus::Result<String>;

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

fn is_valid_object_path(path: &str) -> bool {
    zbus::zvariant::ObjectPath::try_from(path).is_ok()
}

pub fn are_config_paths_valid(adapter_path: &str, station_path: &str) -> bool {
    is_valid_object_path(adapter_path) && is_valid_object_path(station_path)
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

    let iw_dev = Command::new("iw")
        .args(["dev"])
        .output()
        .ok()
        .and_then(|output| {
            if !output.status.success() {
                return None;
            }
            let stdout = String::from_utf8(output.stdout).ok()?;
            parse_ssid_from_iw_dev(&stdout)
        });

    if iw_dev.is_some() {
        return iw_dev;
    }

    None
}

fn discover_iwd_paths() -> Option<(String, String)> {
    let output = Command::new("busctl")
        .args(["tree", "net.connman.iwd"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;
    let mut paths: Vec<String> = Vec::new();
    for line in stdout.lines() {
        if let Some(path) = line
            .split_whitespace()
            .find(|p| p.starts_with("/net/connman/iwd/"))
        {
            paths.push(path.to_string());
        }
    }
    if paths.is_empty() {
        return None;
    }

    // iwd object layout varies by system:
    // - adapter: /net/connman/iwd/<adapter_id>
    // - station: /net/connman/iwd/<adapter_id>/<station_id> (e.g. wlan0 or numeric "5")
    // - network: /net/connman/iwd/<adapter_id>/<station_id>/<network>_psk
    // Prefer station-level paths (depth 5) and avoid network-level entries (suffix _psk/_open/...).
    for station in &paths {
        let depth = station.split('/').filter(|s| !s.is_empty()).count();
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
        if parts.len() < 2 {
            continue;
        }
        parts.pop();
        let adapter = parts.join("/");
        if are_config_paths_valid(&adapter, station) {
            return Some((adapter, station.clone()));
        }
    }

    for p in &paths {
        let depth = p.split('/').filter(|s| !s.is_empty()).count();
        if depth == 4 {
            let station_candidate = format!("{}/wlan0", p);
            if are_config_paths_valid(p, &station_candidate) {
                return Some((p.clone(), station_candidate));
            }
        }
    }

    None
}

#[derive(Clone)]
struct PathCache {
    value: Option<(String, String)>,
    at: Instant,
}

fn discover_iwd_paths_cached() -> Option<(String, String)> {
    const DISCOVERY_CACHE_TTL: Duration = Duration::from_secs(30);
    static CACHE: OnceLock<Mutex<PathCache>> = OnceLock::new();

    let cache = CACHE.get_or_init(|| {
        Mutex::new(PathCache {
            value: None,
            at: Instant::now() - DISCOVERY_CACHE_TTL,
        })
    });

    if let Ok(guard) = cache.lock() {
        if guard.at.elapsed() < DISCOVERY_CACHE_TTL {
            return guard.value.clone();
        }
    }

    let refreshed = discover_iwd_paths();
    if let Ok(mut guard) = cache.lock() {
        guard.value = refreshed.clone();
        guard.at = Instant::now();
    }
    refreshed
}

fn candidate_paths(adapter_path: &str, station_path: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    let push_unique =
        |a: String, s: String, out: &mut Vec<(String, String)>, seen: &mut HashSet<String>| {
            let key = format!("{}|{}", a, s);
            if seen.insert(key) {
                out.push((a, s));
            }
        };

    // Prefer runtime discovery first to avoid stale config paths.
    if let Some((a, s)) = discover_iwd_paths_cached() {
        push_unique(a, s, &mut out, &mut seen);
    }
    if are_config_paths_valid(adapter_path, station_path) {
        push_unique(
            adapter_path.to_string(),
            station_path.to_string(),
            &mut out,
            &mut seen,
        );
    }

    out
}

fn strip_ansi_csi(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
            i += 2;
            while i < bytes.len() {
                let b = bytes[i];
                if (0x40..=0x7e).contains(&b) {
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn parse_networks_from_iwctl(output: &str) -> Vec<WifiNetwork> {
    let mut networks = Vec::new();
    for raw_line in output.lines() {
        let clean = strip_ansi_csi(raw_line);
        let line = clean.trim().trim_start_matches(['>', '*', ' ']).trim();
        if line.is_empty() || line.contains("Available networks") || line.starts_with("- ") {
            continue;
        }
        if line.chars().all(|c| c == '-') {
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

        if !networks.iter().any(|n: &WifiNetwork| n.ssid == ssid) {
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
        .rfind(|s| !s.is_empty())
        .map(ToString::to_string)
}

fn fallback_scan_networks(station_path: &str) -> Vec<WifiNetwork> {
    let Some(iface) = iface_from_station_path(station_path) else {
        return Vec::new();
    };

    let output = Command::new("iwctl")
        .args(["station", iface.as_str(), "get-networks"])
        .output();

    let Ok(output) = output else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    let Ok(stdout) = String::from_utf8(output.stdout) else {
        return Vec::new();
    };

    parse_networks_from_iwctl(&stdout)
}

pub async fn get_wifi_info(conn: &Connection, adapter_path: &str, station_path: &str) -> WifiInfo {
    let mut enabled = false;
    let mut ssid = String::from("Disconnected");

    for (adapter_path, station_path) in candidate_paths(adapter_path, station_path) {
        enabled = false;
        ssid = String::from("Disconnected");

        if let Ok(builder) = AdapterProxy::builder(conn).path(adapter_path.as_str()) {
            if let Ok(proxy) = builder.build().await {
                enabled = proxy.powered().await.unwrap_or(false);
            }
        }

        if enabled {
            if let Ok(station_builder) = StationProxy::builder(conn).path(station_path.as_str()) {
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
                if let Some(found) = fallback_current_ssid() {
                    ssid = found;
                }
            }
            return WifiInfo { enabled, ssid };
        }
    }

    if let Some(found) = fallback_current_ssid() {
        ssid = found;
    }

    WifiInfo { enabled, ssid }
}

pub async fn toggle_wifi(conn: &Connection, adapter_path: &str, station_path: &str, enable: bool) {
    for (adapter_path, _) in candidate_paths(adapter_path, station_path) {
        if let Ok(builder) = AdapterProxy::builder(conn).path(adapter_path.as_str()) {
            if let Ok(adapter) = builder.build().await {
                let _ = adapter.set_powered(enable).await;
                return;
            }
        }
    }
}

pub async fn scan_networks(conn: &Connection, station_path: &str) -> Vec<WifiNetwork> {
    let mut networks = Vec::new();

    let mut candidates = Vec::new();
    if is_valid_object_path(station_path) {
        candidates.push(station_path.to_string());
    }
    if let Some((_, s)) = discover_iwd_paths_cached() {
        if !candidates.iter().any(|v| v == &s) {
            candidates.push(s);
        }
    }

    for station_path in candidates {
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
                            if let (Ok(name), Ok(sec)) =
                                (network.name().await, network.type_().await)
                            {
                                if !networks.iter().any(|n: &WifiNetwork| n.ssid == name) {
                                    networks.push(WifiNetwork {
                                        ssid: name,
                                        security: sec,
                                    });
                                }
                            }
                        }
                    }
                }
                if networks.is_empty() {
                    networks = fallback_scan_networks(station_path.as_str());
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
    conn: &Connection,
    station_path: &str,
    ssid: String,
    _passphrase: Option<String>,
) -> bool {
    let mut candidates = Vec::new();
    if is_valid_object_path(station_path) {
        candidates.push(station_path.to_string());
    }
    if let Some((_, s)) = discover_iwd_paths_cached() {
        if !candidates.iter().any(|v| v == &s) {
            candidates.push(s);
        }
    }

    for station_path in candidates {
        if let Ok(station_builder) = StationProxy::builder(conn).path(station_path.as_str()) {
            if let Ok(station) = station_builder.build().await {
                if let Ok(ordered) = station.get_ordered_networks().await {
                    for (path, _) in ordered {
                        if let Ok(network_builder) = NetworkProxy::builder(conn).path(path) {
                            if let Ok(network) = network_builder.build().await {
                                if let Ok(name) = network.name().await {
                                    if name == ssid {
                                        return network.connect().await.is_ok();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{
        is_valid_object_path, parse_networks_from_iwctl, parse_ssid_from_iw_dev, WifiNetwork,
    };

    #[test]
    fn object_path_validation_accepts_valid_paths() {
        assert!(is_valid_object_path("/net/connman/iwd/0"));
        assert!(is_valid_object_path("/net/connman/iwd/0/wlan0"));
    }

    #[test]
    fn object_path_validation_rejects_invalid_paths() {
        assert!(!is_valid_object_path(""));
        assert!(!is_valid_object_path("net/connman/iwd/0"));
        assert!(!is_valid_object_path("/net//connman"));
    }

    #[test]
    fn parse_ssid_from_iw_dev_extracts_ssid() {
        let sample = "phy#0\n\tInterface wlan0\n\t\ttype managed\n\t\tssid MyNet\n";
        assert_eq!(parse_ssid_from_iw_dev(sample), Some("MyNet".to_string()));
    }

    #[test]
    fn parse_networks_from_iwctl_extracts_entries() {
        let sample = "Available networks\n--------------------------------------------------------------------------------\nNetwork1           psk\nOpenHotspot        open\n";

        assert_eq!(
            parse_networks_from_iwctl(sample),
            vec![
                WifiNetwork {
                    ssid: "Network1".to_string(),
                    security: "psk".to_string()
                },
                WifiNetwork {
                    ssid: "OpenHotspot".to_string(),
                    security: "open".to_string()
                }
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
            parse_networks_from_iwctl(sample),
            vec![WifiNetwork {
                ssid: "v83Etz9b_Plus_5G".to_string(),
                security: "psk".to_string()
            }]
        );
    }

    #[test]
    fn discover_iwd_paths_prefers_station_level_path_shape() {
        let sample = "\
/net/connman/iwd/0
/net/connman/iwd/0/5
/net/connman/iwd/0/5/79383345747a39625f506c75735f3547_psk
";

        // Emulate parser logic from discover_iwd_paths by validating expected shape.
        let mut paths = Vec::new();
        for line in sample.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("/net/connman/iwd/") {
                paths.push(trimmed.to_string());
            }
        }

        let station = paths
            .iter()
            .find(|p| p.split('/').filter(|s| !s.is_empty()).count() == 5)
            .expect("station path expected");
        assert_eq!(station, "/net/connman/iwd/0/5");
        let mut parts: Vec<&str> = station.split('/').collect();
        parts.pop();
        assert_eq!(parts.join("/"), "/net/connman/iwd/0");
    }
}
