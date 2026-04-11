pub mod audio_routes;
pub mod bluetooth_devices;
pub mod calendar;
pub mod connectivity;
pub mod controls;
pub mod displays;
pub mod media;
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
    pub card_spacing: u16,
    pub card_padding: u16,
}

pub const fn standard_domain_popup_layout() -> DomainPopupLayout {
    DomainPopupLayout {
        width: 432,
        outer_padding_x: 20,
        outer_padding_y: 20,
        section_spacing: 16,
        card_spacing: 10,
        card_padding: 14,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PopupTypeScale {
    pub title: u16,
    pub section: u16,
    pub body: u16,
    pub meta: u16,
    pub micro: u16,
}

pub const fn standard_popup_type_scale() -> PopupTypeScale {
    PopupTypeScale {
        title: 18,
        section: 14,
        body: 13,
        meta: 11,
        micro: 10,
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

#[cfg(test)]
mod tests {
    use super::{standard_domain_popup_layout, standard_popup_type_scale};

    #[test]
    fn standard_domain_popup_layout_stays_compact_and_stable() {
        let layout = standard_domain_popup_layout();

        assert_eq!(layout.width, 432);
        assert_eq!(layout.outer_padding_x, 20);
        assert_eq!(layout.outer_padding_y, 20);
        assert_eq!(layout.section_spacing, 16);
        assert_eq!(layout.card_spacing, 10);
        assert_eq!(layout.card_padding, 14);
    }

    #[test]
    fn standard_popup_type_scale_stays_readable_and_stable() {
        let scale = standard_popup_type_scale();
        assert_eq!(scale.title, 18);
        assert_eq!(scale.section, 14);
        assert_eq!(scale.body, 13);
        assert_eq!(scale.meta, 11);
        assert_eq!(scale.micro, 10);
    }
}
