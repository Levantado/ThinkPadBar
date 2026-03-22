mod app;
mod modules;

use app::ThinkPadBar;
use iced::Font;

fn main() -> iced::Result {
    let mut font = None;
    if let Ok(font_bytes) = std::fs::read("/usr/share/fonts/TTF/JetBrainsMonoNerdFont-Regular.ttf") {
        font = Some(font_bytes);
    }

    let mut daemon = iced::daemon(ThinkPadBar::title, ThinkPadBar::update, ThinkPadBar::view)
        .subscription(ThinkPadBar::subscription)
        .theme(ThinkPadBar::theme)
        .style(ThinkPadBar::style)
        .default_font(Font::with_name("JetBrainsMonoNL NFP"));
        
    if let Some(f) = font {
        daemon = daemon.font(std::borrow::Cow::Owned(f));
    }
    
    daemon.run_with(ThinkPadBar::new())
}
