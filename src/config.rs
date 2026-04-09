use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct Config {
    pub font_path: String,
    pub font_name: String,
    pub compositor: CompositorConfig,
    pub network: NetworkConfig,
    pub appearance: AppearanceConfig,
    pub performance: PerformanceConfig,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct CompositorConfig {
    pub backend: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct NetworkConfig {
    pub backend: String,
    pub adapter_path: String,
    pub station_path: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct AppearanceConfig {
    pub bar_height: u32,
    pub opacity: f32,
    pub audio_visualizer: AudioVisualizerConfig,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct AudioVisualizerConfig {
    pub enabled: bool,
    pub mode: String,
    pub bars: u8,
    pub min_height: f32,
    pub max_height: f32,
    pub bar_width: u8,
    pub gap: u8,
    pub padding_x: u8,
    pub padding_y: u8,
    pub fps: u8,
    pub min_freq_hz: f32,
    pub max_freq_hz: f32,
    pub color_profile: String,
    pub decay_profile: String,
}

impl Default for CompositorConfig {
    fn default() -> Self {
        Self {
            backend: "hyprland".to_string(),
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            backend: "iwd".to_string(),
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
            audio_visualizer: AudioVisualizerConfig::default(),
        }
    }
}

impl Default for AudioVisualizerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: "compact".to_string(),
            bars: 16,
            min_height: 4.0,
            max_height: 18.0,
            bar_width: 3,
            gap: 2,
            padding_x: 0,
            padding_y: 0,
            fps: 24,
            min_freq_hz: 60.0,
            max_freq_hz: 12_000.0,
            color_profile: "heat".to_string(),
            decay_profile: "smooth".to_string(),
        }
    }
}

impl AudioVisualizerConfig {
    pub fn normalized_mode(&self) -> &str {
        match self.mode.as_str() {
            "expressive" => "expressive",
            _ => "compact",
        }
    }

    pub fn normalized_bars(&self) -> usize {
        self.bars.clamp(8, 24) as usize
    }

    pub fn normalized_fps(&self) -> u8 {
        self.fps.clamp(10, 30)
    }

    pub fn normalized_min_height(&self) -> f32 {
        self.min_height.clamp(2.0, 10.0)
    }

    pub fn normalized_max_height(&self) -> f32 {
        self.max_height
            .max(self.normalized_min_height() + 2.0)
            .clamp(8.0, 28.0)
    }

    pub fn normalized_bar_width(&self) -> f32 {
        f32::from(self.bar_width.clamp(2, 6))
    }

    pub fn normalized_gap(&self) -> u16 {
        u16::from(self.gap.clamp(1, 4))
    }

    pub fn normalized_padding_x(&self) -> u16 {
        if self.padding_x > 0 {
            return u16::from(self.padding_x.clamp(2, 16));
        }

        match self.normalized_mode() {
            "expressive" => 10,
            _ => 6,
        }
    }

    pub fn normalized_padding_y(&self) -> u16 {
        if self.padding_y > 0 {
            return u16::from(self.padding_y.clamp(1, 8));
        }

        match self.normalized_mode() {
            "expressive" => 4,
            _ => 2,
        }
    }

    pub fn normalized_min_freq_hz(&self) -> f32 {
        self.min_freq_hz.clamp(20.0, 2_000.0)
    }

    pub fn normalized_max_freq_hz(&self) -> f32 {
        self.max_freq_hz
            .max(self.normalized_min_freq_hz() * 2.0)
            .clamp(2_000.0, 20_000.0)
    }

    pub fn normalized_color_profile(&self) -> &str {
        match self.color_profile.as_str() {
            "accent" => "accent",
            "mono" => "mono",
            _ => "heat",
        }
    }

    pub fn normalized_decay_profile(&self) -> &str {
        match self.decay_profile.as_str() {
            "tight" => "tight",
            "expressive" => "expressive",
            _ => "smooth",
        }
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
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
            compositor: CompositorConfig::default(),
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
backend = "iwd"
adapter_path = "/net/connman/iwd/0"
station_path = "/net/connman/iwd/0/5"

[appearance]
bar_height = 24
opacity = 0.9
"#;

        let cfg: Config = toml::from_str(input).expect("config parse should succeed");
        assert_eq!(cfg.compositor.backend, "hyprland");
        assert_eq!(cfg.network.backend, "iwd");
        assert_eq!(cfg.performance.profile, "normal");
        assert!(cfg.appearance.audio_visualizer.enabled);
        assert_eq!(cfg.appearance.audio_visualizer.normalized_bars(), 16);
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
backend = "iwd"
adapter_path = "/net/connman/iwd/0"
station_path = "/net/connman/iwd/0/5"

[appearance]
bar_height = 24
opacity = 0.9

[appearance.audio_visualizer]
enabled = false
mode = "expressive"
bars = 20
min_height = 3.0
max_height = 16.0
bar_width = 4
gap = 3
padding_x = 12
padding_y = 5
fps = 20
min_freq_hz = 80.0
max_freq_hz = 10000.0
color_profile = "accent"
decay_profile = "expressive"

[performance]
profile = "low_power"
tick_brightness_secs = 0
tick_thermal_secs = 0
tick_slow_secs = 0
"#;
        let cfg: Config = toml::from_str(input).expect("config parse should succeed");
        assert!(!cfg.appearance.audio_visualizer.enabled);
        assert_eq!(
            cfg.appearance.audio_visualizer.normalized_mode(),
            "expressive"
        );
        assert_eq!(cfg.appearance.audio_visualizer.normalized_bars(), 20);
        assert_eq!(cfg.appearance.audio_visualizer.normalized_fps(), 20);
        assert_eq!(cfg.appearance.audio_visualizer.normalized_min_height(), 3.0);
        assert_eq!(
            cfg.appearance.audio_visualizer.normalized_max_height(),
            16.0
        );
        assert_eq!(cfg.appearance.audio_visualizer.normalized_bar_width(), 4.0);
        assert_eq!(cfg.appearance.audio_visualizer.normalized_gap(), 3);
        assert_eq!(cfg.appearance.audio_visualizer.normalized_padding_x(), 12);
        assert_eq!(cfg.appearance.audio_visualizer.normalized_padding_y(), 5);
        assert_eq!(
            cfg.appearance.audio_visualizer.normalized_color_profile(),
            "accent"
        );
        assert_eq!(
            cfg.appearance.audio_visualizer.normalized_decay_profile(),
            "expressive"
        );
        assert_eq!(cfg.performance.effective_intervals(), (2, 4, 20));
    }

    #[test]
    fn visualizer_normalization_clamps_style_and_profiles() {
        let input = r#"
font_path = "/tmp/font.ttf"
font_name = "Test Font"

[appearance.audio_visualizer]
mode = "unknown"
bar_width = 9
gap = 0
padding_x = 0
padding_y = 0
color_profile = "unknown"
decay_profile = "unknown"
"#;

        let cfg: Config = toml::from_str(input).expect("config parse should succeed");
        assert_eq!(cfg.appearance.audio_visualizer.normalized_mode(), "compact");
        assert_eq!(cfg.appearance.audio_visualizer.normalized_bar_width(), 6.0);
        assert_eq!(cfg.appearance.audio_visualizer.normalized_gap(), 1);
        assert_eq!(cfg.appearance.audio_visualizer.normalized_padding_x(), 6);
        assert_eq!(cfg.appearance.audio_visualizer.normalized_padding_y(), 2);
        assert_eq!(
            cfg.appearance.audio_visualizer.normalized_color_profile(),
            "heat"
        );
        assert_eq!(
            cfg.appearance.audio_visualizer.normalized_decay_profile(),
            "smooth"
        );
    }

    #[test]
    fn visualizer_expressive_mode_uses_roomier_padding_defaults() {
        let input = r#"
font_path = "/tmp/font.ttf"
font_name = "Test Font"

[appearance.audio_visualizer]
mode = "expressive"
"#;

        let cfg: Config = toml::from_str(input).expect("config parse should succeed");
        assert_eq!(cfg.appearance.audio_visualizer.normalized_padding_x(), 10);
        assert_eq!(cfg.appearance.audio_visualizer.normalized_padding_y(), 4);
    }

    #[test]
    fn performance_profile_allows_explicit_overrides() {
        let input = r#"
font_path = "/tmp/font.ttf"
font_name = "Test Font"

[network]
backend = "iwd"
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

    #[test]
    fn config_supports_backend_skeleton_keys() {
        let input = r#"
font_path = "/tmp/font.ttf"
font_name = "Test Font"

[compositor]
backend = "niri"

[network]
backend = "networkmanager"
adapter_path = "/net/connman/iwd/0"
station_path = "/net/connman/iwd/0/5"
"#;

        let cfg: Config = toml::from_str(input).expect("config parse should succeed");
        assert_eq!(cfg.compositor.backend, "niri");
        assert_eq!(cfg.network.backend, "networkmanager");
    }
}
