pub mod audio_routes;
pub mod bluetooth_devices;
pub mod displays;
pub mod stats;
pub mod system_info;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PopupMetricRow {
    pub icon: &'static str,
    pub label: &'static str,
    pub value: String,
}

impl PopupMetricRow {
    pub fn new(icon: &'static str, label: &'static str, value: impl Into<String>) -> Self {
        Self {
            icon,
            label,
            value: value.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupSectionTone {
    Accent,
    Success,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PopupSection {
    pub title: &'static str,
    pub tone: PopupSectionTone,
    pub rows: Vec<PopupMetricRow>,
}

impl PopupSection {
    pub fn new(title: &'static str, tone: PopupSectionTone, rows: Vec<PopupMetricRow>) -> Self {
        Self { title, tone, rows }
    }
}
