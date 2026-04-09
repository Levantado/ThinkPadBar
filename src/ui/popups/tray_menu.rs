use iced::{
    widget::{button, container, horizontal_rule, scrollable, text, Column},
    Color, Element, Length, Padding,
};

use crate::app::Message;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayMenuPopupModel {
    pub nodes: Vec<TrayMenuNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayMenuNode {
    Separator,
    Action(TrayMenuAction),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayMenuAction {
    pub id: i32,
    pub label: String,
    pub enabled: bool,
    pub activatable: bool,
}

impl TrayMenuPopupModel {
    pub fn from_owned_menu(menu: Option<&crate::services::tray_menu::OwnedTrayMenu>) -> Self {
        let mut nodes = Vec::new();
        if let Some(menu) = menu {
            for node in menu.nodes() {
                match node {
                    crate::services::tray_menu::OwnedTrayMenuNode::Separator => {
                        nodes.push(TrayMenuNode::Separator);
                    }
                    crate::services::tray_menu::OwnedTrayMenuNode::Action(action) => {
                        let mut label = String::new();
                        for _ in 0..action.depth {
                            label.push_str("  ");
                        }
                        label.push_str(&action.label);
                        if !action.activatable {
                            label.push_str("  ›");
                        }
                        nodes.push(TrayMenuNode::Action(TrayMenuAction {
                            id: action.id,
                            label,
                            enabled: action.enabled,
                            activatable: action.activatable,
                        }));
                    }
                }
            }
        }
        Self { nodes }
    }
}

pub fn view(opacity: f32, model: TrayMenuPopupModel) -> Element<'static, Message> {
    let mut content = Column::new()
        .spacing(6)
        .push(
            text("Tray Menu")
                .size(16)
                .style(|_| iced::widget::text::Style {
                    color: Some(Color::from_rgb8(0xc0, 0xca, 0xf5)),
                }),
        );

    for node in model.nodes {
        match node {
            TrayMenuNode::Separator => {
                content = content.push(horizontal_rule(1));
            }
            TrayMenuNode::Action(action) => {
                let mut btn = button(text(action.label).size(13))
                    .width(Length::Fill)
                    .padding(Padding::from([4, 8]));
                if action.enabled && action.activatable {
                    btn = btn.on_press(Message::TrayMenuItemSelected(action.id));
                }
                content = content.push(btn);
            }
        }
    }

    container(
        container(scrollable(content))
            .width(Length::Fill)
            .padding(Padding::from([12, 12]))
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(Color {
                    a: opacity,
                    ..Color::from_rgb8(0x11, 0x12, 0x1d)
                })),
                text_color: Some(Color::from_rgb8(0xc0, 0xca, 0xf5)),
                border: iced::Border {
                    radius: 12.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

#[cfg(test)]
mod tests {
    use super::{TrayMenuNode, TrayMenuPopupModel};

    #[test]
    fn empty_menu_builds_empty_model() {
        let model = TrayMenuPopupModel::from_owned_menu(None);
        assert!(model.nodes.is_empty());
    }

    #[test]
    fn menu_model_preserves_separator_and_nested_indicator() {
        let menu = crate::services::tray_menu::OwnedTrayMenu::new_for_tests(vec![
            crate::services::tray_menu::OwnedTrayMenuNode::Action(
                crate::services::tray_menu::OwnedTrayMenuAction {
                    id: 1,
                    label: "Open".to_string(),
                    enabled: true,
                    activatable: true,
                    depth: 0,
                    prefetch_path: vec![1],
                },
            ),
            crate::services::tray_menu::OwnedTrayMenuNode::Separator,
            crate::services::tray_menu::OwnedTrayMenuNode::Action(
                crate::services::tray_menu::OwnedTrayMenuAction {
                    id: 2,
                    label: "Audio".to_string(),
                    enabled: true,
                    activatable: false,
                    depth: 1,
                    prefetch_path: vec![1, 2],
                },
            ),
        ]);

        let model = TrayMenuPopupModel::from_owned_menu(Some(&menu));
        assert_eq!(model.nodes.len(), 3);
        assert!(matches!(model.nodes[1], TrayMenuNode::Separator));
        assert!(matches!(
            &model.nodes[2],
            TrayMenuNode::Action(action) if action.label == "  Audio  ›"
        ));
    }
}
