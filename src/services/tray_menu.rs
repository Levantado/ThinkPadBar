use system_tray::menu::{MenuItem, MenuType, TrayMenu};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnedTrayMenuNode {
    Item(OwnedTrayMenuItem),
    Separator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedTrayMenuItem {
    pub id: i32,
    pub label: String,
    pub enabled: bool,
    pub activatable: bool,
    pub children: Vec<OwnedTrayMenuNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OwnedTrayMenu {
    root: Vec<OwnedTrayMenuNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedTrayMenuLevel<'a> {
    pub title: Option<&'a str>,
    pub nodes: &'a [OwnedTrayMenuNode],
    pub has_back: bool,
}

impl OwnedTrayMenu {
    pub fn from_layout(layout: &TrayMenu) -> Self {
        Self {
            root: Self::collect_nodes(&layout.submenus),
        }
    }

    pub fn has_visible_actions(&self) -> bool {
        Self::contains_activatable_items(&self.root)
    }

    pub fn level<'a>(&'a self, path: &[i32]) -> OwnedTrayMenuLevel<'a> {
        let mut title = None;
        let mut nodes = self.root.as_slice();

        for item_id in path {
            let Some(item) = Self::find_item(nodes, *item_id) else {
                break;
            };
            if item.children.is_empty() {
                break;
            }
            title = Some(item.label.as_str());
            nodes = item.children.as_slice();
        }

        OwnedTrayMenuLevel {
            title,
            nodes,
            has_back: !path.is_empty(),
        }
    }

    pub fn item_in_level<'a>(
        &'a self,
        path: &[i32],
        item_id: i32,
    ) -> Option<&'a OwnedTrayMenuItem> {
        let level = self.level(path);
        Self::find_item(level.nodes, item_id)
    }

    pub fn popup_height(&self, path: &[i32]) -> u32 {
        let level = self.level(path);
        let rows = level.nodes.len() as u32 + u32::from(level.has_back);
        let height = 20 + rows.saturating_mul(32);
        height.clamp(120, 320)
    }

    fn collect_nodes(items: &[MenuItem]) -> Vec<OwnedTrayMenuNode> {
        let mut nodes = Vec::new();

        for item in items {
            if !item.visible {
                continue;
            }

            if item.menu_type == MenuType::Separator {
                if !matches!(nodes.last(), Some(OwnedTrayMenuNode::Separator)) {
                    nodes.push(OwnedTrayMenuNode::Separator);
                }
                continue;
            }

            let children = Self::collect_nodes(&item.submenu);
            let label = item
                .label
                .as_ref()
                .map(|value| value.replace('_', ""))
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "(item)".to_string());
            let is_submenu =
                !children.is_empty() && item.children_display.as_deref() == Some("submenu");
            let activatable = item.enabled && !is_submenu;

            nodes.push(OwnedTrayMenuNode::Item(OwnedTrayMenuItem {
                id: item.id,
                label,
                enabled: item.enabled,
                activatable,
                children,
            }));
        }

        while matches!(nodes.last(), Some(OwnedTrayMenuNode::Separator)) {
            nodes.pop();
        }

        nodes
    }

    fn contains_activatable_items(nodes: &[OwnedTrayMenuNode]) -> bool {
        nodes.iter().any(|node| match node {
            OwnedTrayMenuNode::Separator => false,
            OwnedTrayMenuNode::Item(item) => {
                item.activatable || Self::contains_activatable_items(&item.children)
            }
        })
    }

    fn find_item(nodes: &[OwnedTrayMenuNode], item_id: i32) -> Option<&OwnedTrayMenuItem> {
        nodes.iter().find_map(|node| match node {
            OwnedTrayMenuNode::Separator => None,
            OwnedTrayMenuNode::Item(item) if item.id == item_id => Some(item),
            OwnedTrayMenuNode::Item(_) => None,
        })
    }

    #[cfg(test)]
    pub fn new_for_tests(nodes: Vec<OwnedTrayMenuNode>) -> Self {
        Self { root: nodes }
    }
}

#[cfg(test)]
mod tests {
    use super::{OwnedTrayMenu, OwnedTrayMenuItem, OwnedTrayMenuNode};
    use system_tray::menu::{MenuItem, MenuType, TrayMenu};

    fn menu_item(id: i32, label: &str, enabled: bool, visible: bool) -> MenuItem {
        MenuItem {
            id,
            menu_type: MenuType::Standard,
            label: Some(label.to_string()),
            enabled,
            visible,
            submenu: Vec::new(),
            ..Default::default()
        }
    }

    #[test]
    fn owned_menu_preserves_hierarchical_submenus() {
        let mut parent = menu_item(10, "_Audio", true, true);
        parent.children_display = Some("submenu".to_string());
        parent.submenu = vec![menu_item(11, "Headphones", true, true)];
        let layout = TrayMenu {
            id: 0,
            submenus: vec![parent],
        };

        let menu = OwnedTrayMenu::from_layout(&layout);
        let root_level = menu.level(&[]);
        let child_level = menu.level(&[10]);

        assert_eq!(root_level.nodes.len(), 1);
        assert!(!root_level.has_back);
        assert_eq!(child_level.title, Some("Audio"));
        assert!(child_level.has_back);
        assert_eq!(child_level.nodes.len(), 1);
    }

    #[test]
    fn submenu_parent_is_not_activatable_but_child_actions_remain_visible() {
        let mut parent = menu_item(10, "Parent", true, true);
        parent.children_display = Some("submenu".to_string());
        parent.submenu = vec![menu_item(11, "Child", true, true)];
        let layout = TrayMenu {
            id: 0,
            submenus: vec![parent],
        };

        let model = OwnedTrayMenu::from_layout(&layout);
        let parent = model
            .item_in_level(&[], 10)
            .expect("expected parent in root level");
        let child = model
            .item_in_level(&[10], 11)
            .expect("expected child in submenu level");

        assert!(!parent.activatable);
        assert!(child.activatable);
        assert!(model.has_visible_actions());
    }

    #[test]
    fn popup_height_scales_with_current_level_length() {
        let menu = OwnedTrayMenu::new_for_tests(vec![
            OwnedTrayMenuNode::Item(OwnedTrayMenuItem {
                id: 1,
                label: "Open".to_string(),
                enabled: true,
                activatable: true,
                children: Vec::new(),
            }),
            OwnedTrayMenuNode::Item(OwnedTrayMenuItem {
                id: 2,
                label: "Audio".to_string(),
                enabled: true,
                activatable: false,
                children: vec![OwnedTrayMenuNode::Item(OwnedTrayMenuItem {
                    id: 3,
                    label: "Headphones".to_string(),
                    enabled: true,
                    activatable: true,
                    children: Vec::new(),
                })],
            }),
        ]);

        assert_eq!(menu.popup_height(&[]), 120);
        assert_eq!(menu.popup_height(&[2]), 120);
    }

    #[test]
    fn separators_are_deduped_and_trimmed() {
        let layout = TrayMenu {
            id: 0,
            submenus: vec![
                MenuItem {
                    id: 1,
                    menu_type: MenuType::Separator,
                    ..Default::default()
                },
                menu_item(2, "Open", true, true),
                MenuItem {
                    id: 3,
                    menu_type: MenuType::Separator,
                    ..Default::default()
                },
                MenuItem {
                    id: 4,
                    menu_type: MenuType::Separator,
                    ..Default::default()
                },
            ],
        };

        let menu = OwnedTrayMenu::from_layout(&layout);
        let level = menu.level(&[]);

        assert_eq!(
            level.nodes,
            &[OwnedTrayMenuNode::Item(OwnedTrayMenuItem {
                id: 2,
                label: "Open".to_string(),
                enabled: true,
                activatable: true,
                children: Vec::new(),
            })]
        );
    }
}
