#[derive(Debug, Clone)]
pub struct TrayRuntimeEvent(pub(crate) crate::services::tray_model::TrayMessage);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayUiPrimaryAction {
    ResolveCandidates { id: String, candidates: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayUiSecondaryAction {
    OpenMenu(String),
    ActivateSecondary(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayUiSelectionAction {
    ActivateMenuItem { id: String, menu_item_id: i32 },
    CloseMenu,
}

pub struct TrayUiService {
    tray: crate::services::tray_model::Tray,
    menu_cursor: Option<(i32, i32)>,
}

impl TrayUiService {
    pub fn new() -> Self {
        Self {
            tray: crate::services::tray_model::Tray::new(),
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

    pub fn clear_menu_cursor(&mut self) {
        self.menu_cursor = None;
    }

    pub fn owned_menu_for(&self, id: &str) -> Option<&crate::services::tray_menu::OwnedTrayMenu> {
        self.tray.owned_menu_for(id)
    }

    pub fn handle_runtime_message(
        &mut self,
        msg: TrayRuntimeEvent,
        open_tray_menu_id: Option<&str>,
    ) -> bool {
        self.tray.update(msg.0);
        if let Some(open_id) = open_tray_menu_id {
            return !self.tray.items.contains_key(open_id);
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
        open_tray_menu_id: Option<&str>,
        cursor: Option<(i32, i32)>,
    ) -> TrayUiSecondaryAction {
        if self.tray.has_menu_entries(&id) {
            self.menu_cursor = cursor;
            if open_tray_menu_id.is_some_and(|open_id| open_id == id) {
                self.menu_cursor = None;
            }
            return TrayUiSecondaryAction::OpenMenu(id);
        }
        self.tray
            .update(crate::services::tray_model::TrayMessage::ActivateItemSecondary(id.clone()));
        TrayUiSecondaryAction::ActivateSecondary(id)
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
        id: String,
        menu_item_id: i32,
    ) -> TrayUiSelectionAction {
        let is_valid = self
            .tray
            .owned_menu_for(&id)
            .is_some_and(|menu| menu.contains_action_id(menu_item_id));
        self.menu_cursor = None;
        if !is_valid {
            return TrayUiSelectionAction::CloseMenu;
        }
        self.tray
            .update(crate::services::tray_model::TrayMessage::ActivateMenuItem(
                id.clone(),
                menu_item_id,
            ));
        TrayUiSelectionAction::ActivateMenuItem { id, menu_item_id }
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
}

#[cfg(test)]
mod tests {
    use super::{TrayUiSelectionAction, TrayUiService};

    #[test]
    fn tray_candidate_generation_is_generic_and_normalized() {
        let item = crate::services::tray_model::TrayItem {
            _id: "irrelevant".to_string(),
            title: Some("My App".to_string()),
            icon_name: Some("org.example.myapp-panel-symbolic".to_string()),
            icon_handle: None,
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
        let action = service.handle_menu_selection("missing".to_string(), 42);
        assert_eq!(action, TrayUiSelectionAction::CloseMenu);
        assert!(service.menu_cursor().is_none());
    }
}
