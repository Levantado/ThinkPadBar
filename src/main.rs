mod app;
mod config;
mod modules;
mod services;
mod ui;

use app::ThinkPadBar;
use iced::Font;
use tracing::{info, warn};

fn main() -> iced::Result {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Starting ThinkPadBar v{}", env!("CARGO_PKG_VERSION"));

    let config = config::load_config();
    info!("Configuration loaded");

    let mut font = None;
    if let Ok(font_bytes) = std::fs::read(&config.font_path) {
        font = Some(font_bytes);
        info!("Font loaded: {}", config.font_path);
    } else {
        warn!("Failed to load font at: {}", config.font_path);
    }

    let font_name = Box::leak(config.font_name.clone().into_boxed_str());

    let mut daemon = iced::daemon(ThinkPadBar::title, ThinkPadBar::update, ThinkPadBar::view)
        .subscription(ThinkPadBar::subscription)
        .theme(ThinkPadBar::theme)
        .style(ThinkPadBar::style)
        .default_font(Font::with_name(font_name));

    if let Some(f) = font {
        daemon = daemon.font(std::borrow::Cow::Owned(f));
    }

    daemon.run_with(ThinkPadBar::new(config))
}
