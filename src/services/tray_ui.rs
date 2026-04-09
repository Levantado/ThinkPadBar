#[derive(Debug, Clone)]
pub struct TrayRuntimeEvent(pub(crate) crate::services::tray_model::TrayMessage);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayUiPrimaryAction {
    ResolveCandidates { id: String, candidates: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayUiSecondaryAction {
    OpenMenu,
    CloseMenu,
    ActivateSecondary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayUiSelectionAction {
    NavigateMenu,
    ActivateMenuItem(i32),
    CloseMenu,
}

pub struct TrayUiService {
    tray: crate::services::tray_model::Tray,
    open_menu_id: Option<String>,
    menu_navigation_path: Vec<i32>,
    menu_cursor: Option<(i32, i32)>,
}

impl TrayUiService {
    pub fn new() -> Self {
        Self {
            tray: crate::services::tray_model::Tray::new(),
            open_menu_id: None,
            menu_navigation_path: Vec::new(),
            menu_cursor: None,
        }
    }

    pub fn items(
        &self,
    ) -> &std::collections::HashMap<String, crate::services::tray_model::TrayItem> {
        &self.tray.items
    }

    pub fn subscription() -> iced::Subscription<TrayRuntimeEvent> {
        crate::services::tray::subscription().map(TrayRuntimeEvent)
    }

    pub fn menu_cursor(&self) -> Option<(i32, i32)> {
        self.menu_cursor
    }

    pub fn open_menu(&self) -> Option<&crate::services::tray_menu::OwnedTrayMenu> {
        self.open_menu_id
            .as_deref()
            .and_then(|id| self.tray.owned_menu_for(id))
    }

    pub fn open_menu_path(&self) -> &[i32] {
        &self.menu_navigation_path
    }

    pub fn diagnostics(&self) -> crate::services::tray_model::TrayDiagnostics {
        self.tray.diagnostics()
    }

    pub fn close_transient_ui(&mut self) {
        self.open_menu_id = None;
        self.menu_navigation_path.clear();
        self.menu_cursor = None;
    }

    pub fn handle_runtime_message(&mut self, msg: TrayRuntimeEvent) -> bool {
        self.tray.update(msg.0);
        if let Some(open_id) = self.open_menu_id.as_deref() {
            if !self.tray.items.contains_key(open_id) {
                self.close_transient_ui();
                return true;
            }
        }
        false
    }

    pub fn handle_primary_click(&self, id: &str) -> Option<TrayUiPrimaryAction> {
        let item = self.tray.items.get(id)?;
        Some(TrayUiPrimaryAction::ResolveCandidates {
            id: id.to_string(),
            candidates: Self::search_candidates(item, id),
        })
    }

    pub fn handle_secondary_click(
        &mut self,
        id: String,
        cursor: Option<(i32, i32)>,
    ) -> TrayUiSecondaryAction {
        if self.tray.has_menu_entries(&id) {
            if self
                .open_menu_id
                .as_deref()
                .is_some_and(|open_id| open_id == id)
            {
                self.close_transient_ui();
                return TrayUiSecondaryAction::CloseMenu;
            }
            self.open_menu_id = Some(id);
            self.menu_navigation_path.clear();
            self.menu_cursor = cursor;
            return TrayUiSecondaryAction::OpenMenu;
        }
        self.tray
            .update(crate::services::tray_model::TrayMessage::ActivateItemSecondary(id.clone()));
        TrayUiSecondaryAction::ActivateSecondary
    }

    pub fn handle_click_resolved(&mut self, id: String, found: bool) -> bool {
        if !found {
            self.tray
                .update(crate::services::tray_model::TrayMessage::ActivateItem(id));
        }
        true
    }

    pub fn handle_menu_selection(
        &mut self,
        menu_item_id: i32,
        _cursor: Option<(i32, i32)>,
    ) -> TrayUiSelectionAction {
        let Some(id) = self.open_menu_id.clone() else {
            self.close_transient_ui();
            return TrayUiSelectionAction::CloseMenu;
        };

        let action = self.tray.owned_menu_for(&id).and_then(|menu| {
            menu.item_in_level(&self.menu_navigation_path, menu_item_id)
                .cloned()
        });

        let Some(action) = action else {
            self.close_transient_ui();
            return TrayUiSelectionAction::CloseMenu;
        };

        if !action.children.is_empty() {
            self.menu_navigation_path.push(action.id);
            return TrayUiSelectionAction::NavigateMenu;
        }

        self.close_transient_ui();
        if !action.enabled || !action.activatable {
            return TrayUiSelectionAction::CloseMenu;
        }

        self.tray
            .update(crate::services::tray_model::TrayMessage::ActivateMenuItem(
                id,
                menu_item_id,
            ));
        TrayUiSelectionAction::ActivateMenuItem(menu_item_id)
    }

    pub fn handle_menu_back(&mut self) -> TrayUiSelectionAction {
        if self.open_menu_id.is_none() {
            self.close_transient_ui();
            return TrayUiSelectionAction::CloseMenu;
        }

        if self.menu_navigation_path.pop().is_some() {
            TrayUiSelectionAction::NavigateMenu
        } else {
            self.close_transient_ui();
            TrayUiSelectionAction::CloseMenu
        }
    }

    fn search_candidates(item: &crate::services::tray_model::TrayItem, id: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut push_candidate = |raw: &str| {
            let s = raw.trim().to_ascii_lowercase();
            if s.len() >= 2 && !out.iter().any(|e| e == &s) {
                out.push(s);
            }
        };

        if let Some(title) = &item.title {
            push_candidate(title);
        }
        if item.item_is_menu {
            push_candidate("menu");
        }
        if let Some(menu_path) = &item.menu_path {
            push_candidate(menu_path);
        }
        if let Some(icon_name) = &item.icon_name {
            push_candidate(icon_name);
            let icon_base = icon_name
                .rsplit('/')
                .next()
                .unwrap_or(icon_name)
                .trim_end_matches(".svg")
                .trim_end_matches(".png")
                .trim_end_matches(".xpm")
                .trim_end_matches("-symbolic")
                .trim_end_matches("-panel");
            if !icon_base.is_empty() {
                push_candidate(icon_base);
            }
        }

        push_candidate(id);
        for token in id
            .split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|t| t.len() >= 3)
        {
            if !matches!(
                token,
                "statusnotifieritem"
                    | "status"
                    | "notifier"
                    | "item"
                    | "org"
                    | "kde"
                    | "github"
                    | "com"
            ) {
                push_candidate(token);
            }
        }
        out
    }

    #[cfg(test)]
    pub fn set_open_menu_for_tests(
        &mut self,
        menu: Option<crate::services::tray_menu::OwnedTrayMenu>,
    ) {
        self.close_transient_ui();
        if let Some(menu) = menu {
            let id = "test-menu".to_string();
            self.tray.items.insert(
                id.clone(),
                crate::services::tray_model::TrayItem {
                    _id: id.clone(),
                    title: Some("Test Menu".to_string()),
                    icon_name: None,
                    icon_handle: None,
                    icon_signature: None,
                    icon_source: crate::services::tray_model::TrayIconSource::None,
                    item_is_menu: true,
                    menu_path: None,
                    menu_layout: None,
                    owned_menu: Some(menu),
                },
            );
            self.open_menu_id = Some(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{TrayRuntimeEvent, TrayUiSecondaryAction, TrayUiSelectionAction, TrayUiService};
    use crate::services::tray_model::{TrayIconSource, TrayItem, TrayMenu, TrayMessage};

    #[test]
    fn tray_candidate_generation_is_generic_and_normalized() {
        let item = TrayItem {
            _id: "irrelevant".to_string(),
            title: Some("My App".to_string()),
            icon_name: Some("org.example.myapp-panel-symbolic".to_string()),
            icon_handle: None,
            icon_signature: None,
            icon_source: TrayIconSource::None,
            item_is_menu: false,
            menu_path: None,
            menu_layout: None,
            owned_menu: None,
        };
        let candidates = TrayUiService::search_candidates(&item, "org.kde.StatusNotifierItem-1234");
        assert!(candidates.iter().any(|v| v == "my app"));
        assert!(candidates
            .iter()
            .any(|v| v == "org.example.myapp-panel-symbolic"));
        assert!(candidates.iter().any(|v| v == "org.example.myapp"));
        assert!(candidates.iter().any(|v| v == "1234"));
    }

    #[test]
    fn invalid_menu_selection_closes_menu() {
        let mut service = TrayUiService::new();
        let action = service.handle_menu_selection(42, None);
        assert_eq!(action, TrayUiSelectionAction::CloseMenu);
        assert!(service.menu_cursor().is_none());
    }

    #[test]
    fn secondary_click_toggles_open_menu_state_for_same_item() {
        let mut service = TrayUiService::new();
        service.tray.items.insert(
            "item".to_string(),
            TrayItem {
                _id: "item".to_string(),
                title: Some("Item".to_string()),
                icon_name: None,
                icon_handle: None,
                icon_signature: None,
                icon_source: TrayIconSource::None,
                item_is_menu: false,
                menu_path: Some("/menu".to_string()),
                menu_layout: Some(TrayMenu {
                    id: 1,
                    submenus: vec![system_tray::menu::MenuItem {
                        id: 42,
                        label: Some("Open".to_string()),
                        visible: true,
                        enabled: true,
                        ..Default::default()
                    }],
                }),
                owned_menu: Some(crate::services::tray_menu::OwnedTrayMenu::from_layout(
                    &TrayMenu {
                        id: 1,
                        submenus: vec![system_tray::menu::MenuItem {
                            id: 42,
                            label: Some("Open".to_string()),
                            visible: true,
                            enabled: true,
                            ..Default::default()
                        }],
                    },
                )),
            },
        );

        let first = service.handle_secondary_click("item".to_string(), Some((10, 20)));
        assert_eq!(first, TrayUiSecondaryAction::OpenMenu);
        assert_eq!(service.open_menu_id.as_deref(), Some("item"));
        assert_eq!(service.menu_cursor(), Some((10, 20)));

        let second = service.handle_secondary_click("item".to_string(), Some((30, 40)));
        assert_eq!(second, TrayUiSecondaryAction::CloseMenu);
        assert_eq!(service.open_menu_id, None);
        assert_eq!(service.menu_cursor(), None);
    }

    #[test]
    fn runtime_remove_closes_open_menu_state() {
        let mut service = TrayUiService::new();
        service.tray.items.insert(
            "item".to_string(),
            TrayItem {
                _id: "item".to_string(),
                title: Some("Item".to_string()),
                icon_name: None,
                icon_handle: None,
                icon_signature: None,
                icon_source: TrayIconSource::None,
                item_is_menu: true,
                menu_path: Some("/menu".to_string()),
                menu_layout: Some(TrayMenu {
                    id: 1,
                    submenus: vec![system_tray::menu::MenuItem {
                        id: 1,
                        label: Some("Open".to_string()),
                        visible: true,
                        enabled: true,
                        ..Default::default()
                    }],
                }),
                owned_menu: Some(crate::services::tray_menu::OwnedTrayMenu::from_layout(
                    &TrayMenu {
                        id: 1,
                        submenus: vec![system_tray::menu::MenuItem {
                            id: 1,
                            label: Some("Open".to_string()),
                            visible: true,
                            enabled: true,
                            ..Default::default()
                        }],
                    },
                )),
            },
        );
        let _ = service.handle_secondary_click("item".to_string(), Some((1, 2)));

        let closed = service.handle_runtime_message(TrayRuntimeEvent(TrayMessage::ItemRemoved(
            "item".to_string(),
        )));
        assert!(closed);
        assert_eq!(service.open_menu_id, None);
        assert_eq!(service.menu_cursor(), None);
    }

    #[test]
    fn menu_selection_uses_current_open_menu_id() {
        let mut service = TrayUiService::new();
        let mut parent = system_tray::menu::MenuItem {
            id: 5,
            label: Some("Parent".to_string()),
            visible: true,
            enabled: true,
            children_display: Some("submenu".to_string()),
            ..Default::default()
        };
        parent.submenu = vec![system_tray::menu::MenuItem {
            id: 7,
            label: Some("Open".to_string()),
            visible: true,
            enabled: true,
            ..Default::default()
        }];
        service.tray.items.insert(
            "item".to_string(),
            TrayItem {
                _id: "item".to_string(),
                title: Some("Item".to_string()),
                icon_name: None,
                icon_handle: None,
                icon_signature: None,
                icon_source: TrayIconSource::None,
                item_is_menu: false,
                menu_path: Some("/menu".to_string()),
                menu_layout: Some(TrayMenu {
                    id: 1,
                    submenus: vec![parent.clone()],
                }),
                owned_menu: Some(crate::services::tray_menu::OwnedTrayMenu::from_layout(
                    &TrayMenu {
                        id: 1,
                        submenus: vec![parent],
                    },
                )),
            },
        );
        let _ = service.handle_secondary_click("item".to_string(), Some((1, 2)));
        assert_eq!(
            service.handle_menu_selection(5, None),
            TrayUiSelectionAction::NavigateMenu
        );

        let action = service.handle_menu_selection(7, None);
        assert_eq!(action, TrayUiSelectionAction::ActivateMenuItem(7));
        assert_eq!(service.open_menu_id, None);
        assert_eq!(service.menu_cursor(), None);
    }

    #[test]
    fn submenu_header_selection_navigates_and_back_stays_open() {
        let mut service = TrayUiService::new();
        let mut parent = system_tray::menu::MenuItem {
            id: 41,
            label: Some("Parent".to_string()),
            visible: true,
            enabled: true,
            children_display: Some("submenu".to_string()),
            ..Default::default()
        };
        parent.submenu = vec![system_tray::menu::MenuItem {
            id: 42,
            label: Some("Child".to_string()),
            visible: true,
            enabled: true,
            ..Default::default()
        }];

        service.tray.items.insert(
            "item".to_string(),
            TrayItem {
                _id: "item".to_string(),
                title: Some("Item".to_string()),
                icon_name: None,
                icon_handle: None,
                icon_signature: None,
                icon_source: TrayIconSource::None,
                item_is_menu: false,
                menu_path: Some("/menu".to_string()),
                menu_layout: Some(TrayMenu {
                    id: 1,
                    submenus: vec![parent.clone()],
                }),
                owned_menu: Some(crate::services::tray_menu::OwnedTrayMenu::from_layout(
                    &TrayMenu {
                        id: 1,
                        submenus: vec![parent],
                    },
                )),
            },
        );
        let _ = service.handle_secondary_click("item".to_string(), Some((1, 2)));

        assert_eq!(
            service.handle_menu_selection(41, None),
            TrayUiSelectionAction::NavigateMenu
        );
        assert_eq!(service.open_menu_path(), &[41]);
        assert_eq!(
            service.handle_menu_back(),
            TrayUiSelectionAction::NavigateMenu
        );
        assert!(service.open_menu_path().is_empty());
        assert!(service.open_menu().is_some());
    }
}
