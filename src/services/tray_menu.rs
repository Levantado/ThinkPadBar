use std::collections::HashMap;
use system_tray::menu::{MenuItem, MenuType, TrayMenu};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnedTrayMenuNode {
    Action(OwnedTrayMenuAction),
    Separator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedTrayMenuAction {
    pub id: i32,
    pub label: String,
    pub enabled: bool,
    pub depth: usize,
    pub activatable: bool,
    pub prefetch_path: Vec<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OwnedTrayMenu {
    nodes: Vec<OwnedTrayMenuNode>,
    actions: HashMap<i32, OwnedTrayMenuAction>,
}

impl OwnedTrayMenu {
    pub fn from_layout(layout: &TrayMenu) -> Self {
        let mut menu = Self::default();
        Self::collect_nodes(&layout.submenus, 0, &[], &mut menu.nodes, &mut menu.actions);
        menu
    }

    pub fn nodes(&self) -> &[OwnedTrayMenuNode] {
        &self.nodes
    }

    pub fn has_visible_actions(&self) -> bool {
        self.actions.values().any(|action| action.activatable)
    }

    pub fn action(&self, id: i32) -> Option<&OwnedTrayMenuAction> {
        self.actions.get(&id)
    }

    pub fn popup_height(&self) -> u32 {
        let rows = self.nodes.len() as u32;
        let height = 56 + rows.saturating_mul(30);
        height.clamp(140, 420)
    }

    fn collect_nodes(
        items: &[MenuItem],
        depth: usize,
        ancestors: &[i32],
        nodes: &mut Vec<OwnedTrayMenuNode>,
        actions: &mut HashMap<i32, OwnedTrayMenuAction>,
    ) {
        for item in items {
            if !item.visible {
                continue;
            }
            if item.menu_type == MenuType::Separator {
                nodes.push(OwnedTrayMenuNode::Separator);
                continue;
            }

            let label = item
                .label
                .as_ref()
                .map(|v| v.replace('_', ""))
                .filter(|v| !v.trim().is_empty())
                .unwrap_or_else(|| "(item)".to_string());

            let mut prefetch_path = ancestors.to_vec();
            prefetch_path.push(item.id);
            let activatable = item.enabled
                && (item.submenu.is_empty()
                    || item.children_display.as_deref() != Some("submenu"));

            let action = OwnedTrayMenuAction {
                id: item.id,
                label,
                enabled: item.enabled,
                depth,
                activatable,
                prefetch_path: prefetch_path.clone(),
            };
            nodes.push(OwnedTrayMenuNode::Action(action.clone()));
            actions.insert(item.id, action);

            if !item.submenu.is_empty() {
                Self::collect_nodes(&item.submenu, depth + 1, &prefetch_path, nodes, actions);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{OwnedTrayMenu, OwnedTrayMenuNode};
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
    fn owned_menu_flattens_visible_entries_and_tracks_depth() {
        let mut parent = menu_item(10, "_Parent", true, true);
        parent.submenu = vec![menu_item(11, "Child", true, true)];
        let hidden = menu_item(12, "Hidden", true, false);
        let layout = TrayMenu {
            id: 0,
            submenus: vec![parent, hidden],
        };

        let model = OwnedTrayMenu::from_layout(&layout);
        let nodes = model.nodes();
        assert_eq!(nodes.len(), 2);
        match &nodes[0] {
            OwnedTrayMenuNode::Action(action) => {
                assert_eq!(action.id, 10);
                assert_eq!(action.label, "Parent");
                assert_eq!(action.depth, 0);
                assert_eq!(action.prefetch_path, vec![10]);
            }
            OwnedTrayMenuNode::Separator => panic!("expected action"),
        }
        match &nodes[1] {
            OwnedTrayMenuNode::Action(action) => {
                assert_eq!(action.id, 11);
                assert_eq!(action.depth, 1);
                assert_eq!(action.prefetch_path, vec![10, 11]);
            }
            OwnedTrayMenuNode::Separator => panic!("expected action"),
        }
        assert!(model.action(10).is_some());
        assert!(model.action(11).is_some());
        assert!(model.action(12).is_none());
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
        let parent = match &model.nodes()[0] {
            OwnedTrayMenuNode::Action(action) => action,
            OwnedTrayMenuNode::Separator => panic!("expected action"),
        };
        let child = match &model.nodes()[1] {
            OwnedTrayMenuNode::Action(action) => action,
            OwnedTrayMenuNode::Separator => panic!("expected action"),
        };

        assert!(!parent.activatable);
        assert!(child.activatable);
        assert!(model.has_visible_actions());
    }

    #[test]
    fn popup_height_scales_with_menu_length() {
        let layout = TrayMenu {
            id: 0,
            submenus: vec![
                menu_item(1, "One", true, true),
                menu_item(2, "Two", true, true),
            ],
        };
        let model = OwnedTrayMenu::from_layout(&layout);

        assert_eq!(model.popup_height(), 140);
    }
}
