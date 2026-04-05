use iced::Color;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThemeTokens {
    pub text: Color,
    pub text_muted: Color,
    pub text_on_accent: Color,
    pub accent: Color,
    pub success: Color,
    pub warning: Color,
    pub danger: Color,
    pub surface: Color,
    pub surface_alt: Color,
    pub panel: Color,
    pub pill_radius: f32,
    pub button_radius: f32,
    pub panel_radius: f32,
    pub gap_small: u16,
    pub gap_medium: u16,
}

impl ThemeTokens {
    pub fn from_config(config: &crate::config::Config) -> Self {
        let opacity = config.appearance.opacity.clamp(0.0, 1.0);

        Self {
            text: Color::from_rgb8(0xc0, 0xca, 0xf5),
            text_muted: Color::from_rgb8(0x56, 0x5f, 0x89),
            text_on_accent: Color::from_rgb8(0x1a, 0x1b, 0x26),
            accent: Color::from_rgb8(0x7a, 0xa2, 0xf7),
            success: Color::from_rgb8(0x9e, 0xce, 0x6a),
            warning: Color::from_rgb8(0xe0, 0xaf, 0x68),
            danger: Color::from_rgb8(0xf7, 0x76, 0x8e),
            surface: Color {
                a: opacity,
                ..Color::from_rgb8(0x29, 0x2e, 0x42)
            },
            surface_alt: Color {
                a: opacity,
                ..Color::from_rgb8(0x41, 0x48, 0x68)
            },
            panel: Color {
                a: opacity.max(0.92),
                ..Color::from_rgb8(0x11, 0x12, 0x1d)
            },
            pill_radius: 12.0,
            button_radius: 10.0,
            panel_radius: 12.0,
            gap_small: 6,
            gap_medium: 8,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ThemeTokens;

    #[test]
    fn tokens_respect_configured_opacity_and_keep_popup_floor() {
        let mut config = crate::config::Config::default();
        config.appearance.opacity = 0.45;

        let tokens = ThemeTokens::from_config(&config);

        assert_eq!(tokens.surface.a, 0.45);
        assert_eq!(tokens.surface_alt.a, 0.45);
        assert_eq!(tokens.panel.a, 0.92);
    }
}
