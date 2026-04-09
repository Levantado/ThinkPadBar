use iced::{
    widget::{button, container, horizontal_rule, row, scrollable, text, Column},
    Background, Border, Color, Element, Length, Padding, Theme,
};

use crate::{app::Message, ui::theme::ThemeTokens};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayMenuPopupModel {
    pub rows: Vec<TrayMenuRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayMenuRow {
    Back { label: String },
    Separator,
    Action(TrayMenuAction),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayMenuAction {
    pub id: i32,
    pub label: String,
    pub enabled: bool,
    pub has_children: bool,
}

impl TrayMenuPopupModel {
    pub fn from_owned_menu(
        menu: Option<&crate::services::tray_menu::OwnedTrayMenu>,
        path: &[i32],
    ) -> Self {
        let mut rows = Vec::new();

        let Some(menu) = menu else {
            return Self { rows };
        };

        let level = menu.level(path);
        if level.has_back {
            rows.push(TrayMenuRow::Back {
                label: level.title.unwrap_or("Back").to_string(),
            });
            if !level.nodes.is_empty() {
                rows.push(TrayMenuRow::Separator);
            }
        }

        for node in level.nodes {
            match node {
                crate::services::tray_menu::OwnedTrayMenuNode::Separator => {
                    if !matches!(rows.last(), Some(TrayMenuRow::Separator)) {
                        rows.push(TrayMenuRow::Separator);
                    }
                }
                crate::services::tray_menu::OwnedTrayMenuNode::Item(item) => {
                    rows.push(TrayMenuRow::Action(TrayMenuAction {
                        id: item.id,
                        label: item.label.clone(),
                        enabled: item.enabled,
                        has_children: !item.children.is_empty(),
                    }));
                }
            }
        }

        while matches!(rows.last(), Some(TrayMenuRow::Separator)) {
            rows.pop();
        }

        Self { rows }
    }
}

#[derive(Debug, Clone, Copy)]
enum MenuRowKind {
    Normal,
    Back,
    Disabled,
}

pub fn view(opacity: f32, model: TrayMenuPopupModel) -> Element<'static, Message> {
    let theme = ThemeTokens::from_config(&crate::config::Config::default());
    let mut content = Column::new().spacing(4);

    for row_model in model.rows {
        match row_model {
            TrayMenuRow::Separator => {
                content = content.push(
                    container(horizontal_rule(1))
                        .padding(Padding::from([4, 8]))
                        .width(Length::Fill),
                );
            }
            TrayMenuRow::Back { label } => {
                let row = menu_row_content(format!("‹ {label}"), false);
                content = content.push(
                    button(row)
                        .width(Length::Fill)
                        .padding(Padding::from([8, 10]))
                        .style(move |iced_theme, status| {
                            menu_button_style(iced_theme, status, MenuRowKind::Back, theme)
                        })
                        .on_press(Message::TrayMenuBack),
                );
            }
            TrayMenuRow::Action(action) => {
                let row = menu_row_content(action.label, action.has_children);
                let kind = if action.enabled {
                    MenuRowKind::Normal
                } else {
                    MenuRowKind::Disabled
                };
                let mut button = button(row)
                    .width(Length::Fill)
                    .padding(Padding::from([8, 10]))
                    .style(move |iced_theme, status| {
                        menu_button_style(iced_theme, status, kind, theme)
                    });
                if action.enabled {
                    button = button.on_press(Message::TrayMenuItemSelected(action.id));
                }
                content = content.push(button);
            }
        }
    }

    let scroll = scrollable(content)
        .width(Length::Fill)
        .height(Length::Shrink);
    container(
        container(scroll)
            .width(Length::Fill)
            .padding(Padding::from([8, 8]))
            .style(move |_| {
                let mut style = crate::ui::chrome::popup_panel_style(theme);
                style.background = Some(Background::Color(Color {
                    a: opacity.max(0.94),
                    ..theme.panel
                }));
                style
            }),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn menu_row_content(
    label: String,
    has_children: bool,
) -> iced::widget::Row<'static, Message, Theme, iced::Renderer> {
    let mut content =
        row![text(label).size(13).width(Length::Fill)].align_y(iced::Alignment::Center);
    if has_children {
        content = content.push(text("›").size(13).style(|_| iced::widget::text::Style {
            color: Some(Color::from_rgb8(0x98, 0xa2, 0xc6)),
        }));
    }
    content
}

fn menu_button_style(
    _theme: &Theme,
    status: button::Status,
    kind: MenuRowKind,
    tokens: ThemeTokens,
) -> button::Style {
    let text_color = match kind {
        MenuRowKind::Disabled => Color::from_rgba8(0xe7, 0xec, 0xff, 0.38),
        MenuRowKind::Back => tokens.text,
        MenuRowKind::Normal => tokens.text,
    };

    let background = match (kind, status) {
        (MenuRowKind::Disabled, _) => None,
        (_, button::Status::Hovered) => {
            Some(Background::Color(Color::from_rgba8(0x6e, 0x8e, 0xff, 0.14)))
        }
        (_, button::Status::Pressed) => {
            Some(Background::Color(Color::from_rgba8(0x6e, 0x8e, 0xff, 0.22)))
        }
        (MenuRowKind::Back, _) => Some(Background::Color(tokens.surface)),
        _ => None,
    };

    button::Style {
        background,
        text_color,
        border: Border {
            radius: 10.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::{TrayMenuPopupModel, TrayMenuRow};

    #[test]
    fn empty_menu_builds_empty_model() {
        let model = TrayMenuPopupModel::from_owned_menu(None, &[]);
        assert!(model.rows.is_empty());
    }

    #[test]
    fn root_menu_model_surfaces_submenu_affordance_without_flattening_children() {
        let menu = crate::services::tray_menu::OwnedTrayMenu::new_for_tests(vec![
            crate::services::tray_menu::OwnedTrayMenuNode::Item(
                crate::services::tray_menu::OwnedTrayMenuItem {
                    id: 1,
                    label: "Open".to_string(),
                    enabled: true,
                    activatable: true,
                    children: Vec::new(),
                },
            ),
            crate::services::tray_menu::OwnedTrayMenuNode::Separator,
            crate::services::tray_menu::OwnedTrayMenuNode::Item(
                crate::services::tray_menu::OwnedTrayMenuItem {
                    id: 2,
                    label: "Audio".to_string(),
                    enabled: true,
                    activatable: false,
                    children: vec![crate::services::tray_menu::OwnedTrayMenuNode::Item(
                        crate::services::tray_menu::OwnedTrayMenuItem {
                            id: 3,
                            label: "Headphones".to_string(),
                            enabled: true,
                            activatable: true,
                            children: Vec::new(),
                        },
                    )],
                },
            ),
        ]);

        let model = TrayMenuPopupModel::from_owned_menu(Some(&menu), &[]);
        assert_eq!(model.rows.len(), 3);
        assert!(matches!(model.rows[1], TrayMenuRow::Separator));
        assert!(matches!(
            &model.rows[2],
            TrayMenuRow::Action(action) if action.label == "Audio" && action.has_children
        ));
    }

    #[test]
    fn submenu_level_includes_back_row_and_child_actions() {
        let menu = crate::services::tray_menu::OwnedTrayMenu::new_for_tests(vec![
            crate::services::tray_menu::OwnedTrayMenuNode::Item(
                crate::services::tray_menu::OwnedTrayMenuItem {
                    id: 10,
                    label: "Audio".to_string(),
                    enabled: true,
                    activatable: false,
                    children: vec![crate::services::tray_menu::OwnedTrayMenuNode::Item(
                        crate::services::tray_menu::OwnedTrayMenuItem {
                            id: 11,
                            label: "Headphones".to_string(),
                            enabled: true,
                            activatable: true,
                            children: Vec::new(),
                        },
                    )],
                },
            ),
        ]);

        let model = TrayMenuPopupModel::from_owned_menu(Some(&menu), &[10]);
        assert!(matches!(
            &model.rows[0],
            TrayMenuRow::Back { label } if label == "Audio"
        ));
        assert!(matches!(model.rows[1], TrayMenuRow::Separator));
        assert!(matches!(
            &model.rows[2],
            TrayMenuRow::Action(action) if action.label == "Headphones" && !action.has_children
        ));
    }
}
