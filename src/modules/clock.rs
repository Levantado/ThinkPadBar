use std::time::Duration;

pub fn tick() -> iced::Subscription<crate::app::Message> {
    iced::time::every(Duration::from_secs(1))
        .map(|_| crate::app::Message::Tick(chrono::Local::now()))
}
