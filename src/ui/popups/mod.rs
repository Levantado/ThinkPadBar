pub mod audio_routes;
pub mod bluetooth_devices;
pub mod calendar;
pub mod connectivity;
pub mod controls;
pub mod displays;
pub mod power;
pub mod stats;
pub mod system_info;
pub mod tray_menu;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DomainPopupLayout {
    pub width: u16,
    pub outer_padding_x: u16,
    pub outer_padding_y: u16,
    pub section_spacing: u16,
}

pub const fn standard_domain_popup_layout() -> DomainPopupLayout {
    DomainPopupLayout {
        width: 440,
        outer_padding_x: 24,
        outer_padding_y: 24,
        section_spacing: 20,
    }
}

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

#[cfg(test)]
mod tests {
    use super::standard_domain_popup_layout;

    #[test]
    fn standard_domain_popup_layout_stays_compact_and_stable() {
        let layout = standard_domain_popup_layout();

        assert_eq!(layout.width, 440);
        assert_eq!(layout.outer_padding_x, 24);
        assert_eq!(layout.outer_padding_y, 24);
        assert_eq!(layout.section_spacing, 20);
    }
}
