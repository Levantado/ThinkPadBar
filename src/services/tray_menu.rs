use std::collections::HashSet;
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
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OwnedTrayMenu {
    nodes: Vec<OwnedTrayMenuNode>,
    action_ids: HashSet<i32>,
}

impl OwnedTrayMenu {
    pub fn from_layout(layout: &TrayMenu) -> Self {
        let mut menu = Self::default();
        Self::collect_nodes(&layout.submenus, 0, &mut menu.nodes, &mut menu.action_ids);
        menu
    }

    pub fn nodes(&self) -> &[OwnedTrayMenuNode] {
        &self.nodes
    }

    pub fn has_visible_actions(&self) -> bool {
        !self.action_ids.is_empty()
    }

    pub fn contains_action_id(&self, id: i32) -> bool {
        self.action_ids.contains(&id)
    }

    fn collect_nodes(
        items: &[MenuItem],
        depth: usize,
        nodes: &mut Vec<OwnedTrayMenuNode>,
        action_ids: &mut HashSet<i32>,
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

            nodes.push(OwnedTrayMenuNode::Action(OwnedTrayMenuAction {
                id: item.id,
                label,
                enabled: item.enabled,
                depth,
            }));
            action_ids.insert(item.id);

            if !item.submenu.is_empty() {
                Self::collect_nodes(&item.submenu, depth + 1, nodes, action_ids);
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
            }
            OwnedTrayMenuNode::Separator => panic!("expected action"),
        }
        match &nodes[1] {
            OwnedTrayMenuNode::Action(action) => {
                assert_eq!(action.id, 11);
                assert_eq!(action.depth, 1);
            }
            OwnedTrayMenuNode::Separator => panic!("expected action"),
        }
        assert!(model.contains_action_id(10));
        assert!(model.contains_action_id(11));
        assert!(!model.contains_action_id(12));
    }
}
