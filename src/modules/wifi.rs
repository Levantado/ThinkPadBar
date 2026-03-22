use std::process::Command;

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

fn strip_ansi(s: &str) -> String {
    let mut result = String::new();
    let mut in_escape = false;
    for c in s.chars() {
        if c == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if c.is_ascii_alphabetic() {
                in_escape = false;
            }
        } else {
            result.push(c);
        }
    }
    result
}

pub fn get_wifi_info() -> WifiInfo {
    let mut enabled = false;
    let mut ssid = String::from("Disconnected");

    if let Ok(output) = Command::new("rfkill").arg("list").arg("wifi").output() {
        let out_str = strip_ansi(&String::from_utf8_lossy(&output.stdout));
        if !out_str.contains("Soft blocked: yes") && !out_str.contains("Hard blocked: yes") && !out_str.is_empty() {
            enabled = true;
        }
    }

    if enabled {
        if let Ok(st_out) = Command::new("iwctl").env("NO_COLOR", "1").arg("station").arg("list").output() {
            let st_str = strip_ansi(&String::from_utf8_lossy(&st_out.stdout));
            for line in st_str.lines() {
                if line.contains("connected") {
                    let words: Vec<&str> = line.split_whitespace().collect();
                    if !words.is_empty() {
                        let iface = words[0];
                        if let Ok(output) = Command::new("iwctl").env("NO_COLOR", "1").arg("station").arg(iface).arg("show").output() {
                            let out_str = strip_ansi(&String::from_utf8_lossy(&output.stdout));
                            for l in out_str.lines() {
                                if l.contains("Connected network") {
                                    let parts: Vec<&str> = l.split("Connected network").collect();
                                    if parts.len() > 1 {
                                        ssid = parts[1].trim().to_string();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    WifiInfo { enabled, ssid }
}

pub fn toggle_wifi(enable: bool) {
    if enable {
        let _ = Command::new("rfkill").arg("unblock").arg("wifi").spawn();
    } else {
        let _ = Command::new("rfkill").arg("block").arg("wifi").spawn();
    }
}

pub async fn scan_networks() -> Vec<WifiNetwork> {
    let _ = Command::new("iwctl").env("NO_COLOR", "1").arg("station").arg("wlan0").arg("scan").output();
    tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
    
    let mut networks = Vec::new();
    if let Ok(output) = Command::new("iwctl").env("NO_COLOR", "1").arg("station").arg("wlan0").arg("get-networks").output() {
        let out_str = strip_ansi(&String::from_utf8_lossy(&output.stdout));
        for line in out_str.lines().skip(4) {
            let content = if line.starts_with("  > ") || line.starts_with("    ") { &line[4..] } else { line };
            let trimmed = content.trim_end();
            if trimmed.is_empty() { continue; }
            let words: Vec<&str> = trimmed.split_whitespace().collect();
            if words.len() >= 2 {
                let security = words[words.len() - 2];
                if ["psk", "open", "wep", "8021x"].contains(&security) {
                    let ssid_words = &words[..words.len() - 2];
                    let ssid = ssid_words.join(" ");
                    if !networks.iter().any(|n: &WifiNetwork| n.ssid == ssid) && !ssid.is_empty() {
                        networks.push(WifiNetwork { ssid, security: security.to_string() });
                    }
                }
            }
        }
    }
    networks
}

pub async fn connect_network(ssid: String, passphrase: Option<String>) -> bool {
    if let Some(pass) = passphrase {
        let status = Command::new("iwctl").env("NO_COLOR", "1").arg("--passphrase").arg(pass).arg("station").arg("wlan0").arg("connect").arg(&ssid).output();
        status.map(|s| s.status.success()).unwrap_or(false)
    } else {
        let status = Command::new("iwctl").env("NO_COLOR", "1").arg("station").arg("wlan0").arg("connect").arg(&ssid).output();
        status.map(|s| s.status.success()).unwrap_or(false)
    }
}
