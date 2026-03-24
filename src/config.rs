use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct Config {
    pub font_path: String,
    pub font_name: String,
    pub network: NetworkConfig,
    pub appearance: AppearanceConfig,
    pub performance: PerformanceConfig,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct NetworkConfig {
    pub adapter_path: String,
    pub station_path: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct AppearanceConfig {
    pub bar_height: u32,
    pub opacity: f32,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            adapter_path: "/net/connman/iwd/0".to_string(),
            station_path: "/net/connman/iwd/0/wlan0".to_string(),
        }
    }
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            bar_height: 24,
            opacity: 0.85,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct PerformanceConfig {
    pub profile: String,
    pub tick_brightness_secs: u64,
    pub tick_thermal_secs: u64,
    pub tick_slow_secs: u64,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            profile: "normal".to_string(),
            tick_brightness_secs: 1,
            tick_thermal_secs: 2,
            tick_slow_secs: 10,
        }
    }
}

impl PerformanceConfig {
    fn normalized_profile(&self) -> &str {
        match self.profile.as_str() {
            "low_power" => "low_power",
            "high_responsiveness" => "high_responsiveness",
            _ => "normal",
        }
    }

    pub fn profile_badge(&self) -> &'static str {
        match self.normalized_profile() {
            "low_power" => "LP",
            "high_responsiveness" => "HR",
            _ => "NRM",
        }
    }

    pub fn cycle_profile_runtime(&mut self) {
        let next = match self.normalized_profile() {
            "normal" => "low_power",
            "low_power" => "high_responsiveness",
            _ => "normal",
        };
        self.profile = next.to_string();

        // Runtime cycle should apply profile defaults immediately.
        // Keep explicit overrides possible via config file by setting values > 0 there.
        self.tick_brightness_secs = 0;
        self.tick_thermal_secs = 0;
        self.tick_slow_secs = 0;
    }

    pub fn effective_intervals(&self) -> (u64, u64, u64) {
        let (mut b, mut t, mut s) = match self.normalized_profile() {
            "low_power" => (2, 4, 20),
            "high_responsiveness" => (1, 1, 6),
            _ => (1, 2, 10),
        };

        // Explicit numeric values in config override profile defaults.
        if self.tick_brightness_secs > 0 {
            b = self.tick_brightness_secs;
        }
        if self.tick_thermal_secs > 0 {
            t = self.tick_thermal_secs;
        }
        if self.tick_slow_secs > 0 {
            s = self.tick_slow_secs;
        }

        (b.max(1), t.max(1), s.max(1))
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            font_path: "/usr/share/fonts/TTF/JetBrainsMonoNerdFont-Regular.ttf".to_string(),
            font_name: "JetBrainsMonoNL NFP".to_string(),
            network: NetworkConfig::default(),
            appearance: AppearanceConfig::default(),
            performance: PerformanceConfig::default(),
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

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn config_defaults_include_performance_intervals() {
        let cfg = Config::default();
        assert_eq!(cfg.performance.profile, "normal");
        assert_eq!(cfg.performance.tick_brightness_secs, 1);
        assert_eq!(cfg.performance.tick_thermal_secs, 2);
        assert_eq!(cfg.performance.tick_slow_secs, 10);
    }

    #[test]
    fn config_parsing_fills_missing_performance_section() {
        let input = r#"
font_path = "/tmp/font.ttf"
font_name = "Test Font"

[network]
adapter_path = "/net/connman/iwd/0"
station_path = "/net/connman/iwd/0/5"

[appearance]
bar_height = 24
opacity = 0.9
"#;

        let cfg: Config = toml::from_str(input).expect("config parse should succeed");
        assert_eq!(cfg.performance.profile, "normal");
        assert_eq!(cfg.performance.tick_brightness_secs, 1);
        assert_eq!(cfg.performance.tick_thermal_secs, 2);
        assert_eq!(cfg.performance.tick_slow_secs, 10);
    }

    #[test]
    fn performance_profile_low_power_is_applied() {
        let input = r#"
font_path = "/tmp/font.ttf"
font_name = "Test Font"

[network]
adapter_path = "/net/connman/iwd/0"
station_path = "/net/connman/iwd/0/5"

[appearance]
bar_height = 24
opacity = 0.9

[performance]
profile = "low_power"
tick_brightness_secs = 0
tick_thermal_secs = 0
tick_slow_secs = 0
"#;
        let cfg: Config = toml::from_str(input).expect("config parse should succeed");
        assert_eq!(cfg.performance.effective_intervals(), (2, 4, 20));
    }

    #[test]
    fn performance_profile_allows_explicit_overrides() {
        let input = r#"
font_path = "/tmp/font.ttf"
font_name = "Test Font"

[network]
adapter_path = "/net/connman/iwd/0"
station_path = "/net/connman/iwd/0/5"

[appearance]
bar_height = 24
opacity = 0.9

[performance]
profile = "low_power"
tick_brightness_secs = 3
tick_thermal_secs = 5
tick_slow_secs = 30
"#;
        let cfg: Config = toml::from_str(input).expect("config parse should succeed");
        assert_eq!(cfg.performance.effective_intervals(), (3, 5, 30));
    }

    #[test]
    fn performance_cycle_runtime_rotates_profiles() {
        let mut cfg = Config::default();
        assert_eq!(cfg.performance.profile_badge(), "NRM");
        cfg.performance.cycle_profile_runtime();
        assert_eq!(cfg.performance.profile_badge(), "LP");
        cfg.performance.cycle_profile_runtime();
        assert_eq!(cfg.performance.profile_badge(), "HR");
        cfg.performance.cycle_profile_runtime();
        assert_eq!(cfg.performance.profile_badge(), "NRM");
    }
}
