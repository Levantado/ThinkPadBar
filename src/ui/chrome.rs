use iced::{
    widget::{button, text, Row, Space},
    Alignment, Background, Length, Padding,
};

use crate::app::{Message, Popup};
use crate::ui::theme::ThemeTokens;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetailHeaderAction {
    pub icon: &'static str,
    pub label: &'static str,
    pub target: Popup,
}

pub fn detail_popup_header_action(popup: &Popup) -> Option<DetailHeaderAction> {
    match popup {
        Popup::Displays => Some(DetailHeaderAction {
            icon: "󰈈",
            label: "System Info",
            target: Popup::SystemMonitor,
        }),
        _ => None,
    }
}

pub fn detail_popup_header_row(
    theme: ThemeTokens,
    title: &'static str,
    popup: &Popup,
) -> Row<'static, Message> {
    let mut row = Row::new()
        .spacing(theme.gap_medium)
        .align_y(Alignment::Center)
        .push(text(title).size(18))
        .push(Space::with_width(Length::Fill));

    if let Some(action) = detail_popup_header_action(popup) {
        row = row.push(chrome_button(
            theme,
            Row::new()
                .spacing(theme.gap_small)
                .align_y(Alignment::Center)
                .push(text(action.icon).size(13))
                .push(text(action.label).size(11)),
            Message::TogglePopup(action.target),
        ));
    }

    row.push(chrome_button(
        theme,
        text("Close").size(11),
        Message::TogglePopup(popup.clone()),
    ))
}

pub fn domain_nav_focus_popup(popup: &Popup) -> Popup {
    match popup {
        Popup::AudioRoutes | Popup::Displays => Popup::Controls,
        Popup::BluetoothDevices => Popup::Connectivity,
        Popup::Power => Popup::Power,
        Popup::Controls => Popup::Controls,
        Popup::Connectivity => Popup::Connectivity,
        _ => Popup::Stats,
    }
}

pub fn domain_popup_nav_row(theme: ThemeTokens, active_popup: &Popup) -> Row<'static, Message> {
    let mut row = Row::new().spacing(theme.gap_medium);

    for (icon, label, target, is_active) in domain_popup_nav_items(active_popup) {
        let mut btn = button(
            Row::new()
                .spacing(theme.gap_small)
                .align_y(Alignment::Center)
                .push(text(icon).size(14))
                .push(text(label).size(11)),
        )
        .width(Length::FillPortion(1))
        .padding(Padding::from([8, 10]))
        .style(move |_, _| {
            if is_active {
                iced::widget::button::Style {
                    background: Some(Background::Color(theme.accent)),
                    text_color: theme.text_on_accent,
                    border: iced::Border {
                        radius: theme.button_radius.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            } else {
                iced::widget::button::Style {
                    background: Some(Background::Color(theme.surface)),
                    text_color: theme.text,
                    border: iced::Border {
                        radius: theme.button_radius.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            }
        });
        if !is_active {
            btn = btn.on_press(Message::TogglePopup(target));
        }
        row = row.push(btn);
    }

    row
}

pub fn domain_popup_nav_items(
    active_popup: &Popup,
) -> [(&'static str, &'static str, Popup, bool); 4] {
    [
        ("", "Stats", Popup::Stats, active_popup == &Popup::Stats),
        ("", "Power", Popup::Power, active_popup == &Popup::Power),
        (
            "󰖀",
            "Controls",
            Popup::Controls,
            active_popup == &Popup::Controls,
        ),
        (
            "󰖩",
            "Connectivity",
            Popup::Connectivity,
            active_popup == &Popup::Connectivity,
        ),
    ]
}

fn chrome_button<'a>(
    theme: ThemeTokens,
    content: impl Into<iced::Element<'a, Message>>,
    message: Message,
) -> iced::widget::Button<'a, Message> {
    button(content)
        .padding(Padding::from([6, 10]))
        .on_press(message)
        .style(move |_, _| iced::widget::button::Style {
            background: Some(Background::Color(theme.surface)),
            text_color: theme.text,
            border: iced::Border {
                radius: 9.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
}

#[cfg(test)]
mod tests {
    use super::{detail_popup_header_action, domain_nav_focus_popup, domain_popup_nav_items};
    use crate::app::Popup;

    #[test]
    fn detail_popup_header_action_remains_contextual() {
        assert_eq!(
            detail_popup_header_action(&Popup::Displays).map(|action| (
                action.icon,
                action.label,
                action.target
            )),
            Some(("󰈈", "System Info", Popup::SystemMonitor))
        );
        assert_eq!(detail_popup_header_action(&Popup::AudioRoutes), None);
    }

    #[test]
    fn detail_popup_focus_maps_to_primary_domains() {
        assert_eq!(domain_nav_focus_popup(&Popup::AudioRoutes), Popup::Controls);
        assert_eq!(domain_nav_focus_popup(&Popup::Displays), Popup::Controls);
        assert_eq!(
            domain_nav_focus_popup(&Popup::BluetoothDevices),
            Popup::Connectivity
        );
        assert_eq!(domain_nav_focus_popup(&Popup::SystemMonitor), Popup::Stats);
    }

    #[test]
    fn domain_popup_nav_items_cover_variant_a_domains() {
        assert_eq!(
            domain_popup_nav_items(&Popup::Controls),
            [
                ("", "Stats", Popup::Stats, false),
                ("", "Power", Popup::Power, false),
                ("󰖀", "Controls", Popup::Controls, true),
                ("󰖩", "Connectivity", Popup::Connectivity, false),
            ]
        );
    }
}
