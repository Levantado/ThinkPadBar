use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub font_path: String,
    pub font_name: String,
    pub network: NetworkConfig,
    pub appearance: AppearanceConfig,
}

#[derive(Deserialize, Debug, Clone)]
pub struct NetworkConfig {
    pub adapter_path: String,
    pub station_path: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct AppearanceConfig {
    pub bar_height: u32,
    pub opacity: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            font_path: "/usr/share/fonts/TTF/JetBrainsMonoNerdFont-Regular.ttf".to_string(),
            font_name: "JetBrainsMonoNL NFP".to_string(),
            network: NetworkConfig {
                adapter_path: "/net/connman/iwd/0".to_string(),
                station_path: "/net/connman/iwd/0/wlan0".to_string(),
            },
            appearance: AppearanceConfig {
                bar_height: 24,
                opacity: 0.85,
            },
        }
    }
}

pub fn load_config() -> Config {
    let config_path = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("thinkpadbar/config.toml");

    if let Ok(content) = fs::read_to_string(config_path) {
        toml::from_str(&content).unwrap_or_default()
    } else {
        Config::default()
    }
}
