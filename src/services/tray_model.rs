use iced::widget::image::Handle;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use system_tray::client::{ActivateRequest, Client, Event, UpdateEvent};
use system_tray::item::{IconPixmap, StatusNotifierItem};
pub use system_tray::menu::TrayMenu;
use system_tray::menu::{MenuDiff, MenuItem, MenuItemUpdate};
use tokio::time::{timeout, Duration};
use tracing::debug;

#[derive(Debug, Clone)]
pub struct TrayItem {
    pub _id: String,
    pub title: Option<String>,
    pub icon_name: Option<String>,
    pub icon_handle: Option<Handle>,
    pub icon_signature: Option<u64>,
    pub icon_source: TrayIconSource,
    pub item_is_menu: bool,
    pub menu_path: Option<String>,
    pub menu_layout: Option<TrayMenu>,
    pub owned_menu: Option<crate::services::tray_menu::OwnedTrayMenu>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TrayIconSource {
    #[default]
    None,
    Theme,
    Pixmap,
}

impl TrayItem {
    pub fn fallback_label(&self) -> String {
        self.title
            .as_deref()
            .and_then(first_visible_alphanumeric)
            .or_else(|| {
                self.icon_name
                    .as_deref()
                    .and_then(icon_fallback_label_from_name)
            })
            .map(|ch| ch.to_string())
            .unwrap_or_else(|| {
                if self.item_is_menu {
                    "≡".to_string()
                } else {
                    "•".to_string()
                }
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayDiagnostics {
    pub total_items: usize,
    pub resolved_icons: usize,
    pub unresolved_icons: usize,
    pub fallback_labels: usize,
    pub resolver: crate::services::icon_resolver::IconResolverDiagnostics,
    pub last_unresolved_item: Option<String>,
    pub runtime: TrayRuntimeDiagnostics,
}

impl TrayDiagnostics {
    pub fn summary(&self) -> String {
        format!(
            "{}/{} resolved fallback {} {}",
            self.resolved_icons,
            self.total_items,
            self.fallback_labels,
            self.resolver.summary()
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TrayRuntimeDiagnostics {
    pub last_secondary_route: Option<String>,
    pub last_secondary_result: Option<String>,
    pub last_dispatch_failure: Option<String>,
    pub last_menu_activation_error: Option<String>,
}

impl TrayRuntimeDiagnostics {
    pub fn summary(&self) -> String {
        let route = self.last_secondary_route.as_deref().unwrap_or("-");
        let result = self.last_secondary_result.as_deref().unwrap_or("-");
        let failure = self.last_dispatch_failure.as_deref().unwrap_or("-");
        let menu = self.last_menu_activation_error.as_deref().unwrap_or("-");
        format!(
            "route {} result {} fail {} menu {}",
            route, result, failure, menu
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayRuntimeUpdate {
    SecondaryObserved {
        route: String,
        result: String,
        failure: Option<String>,
    },
    MenuActivationError(Option<String>),
}

#[derive(Debug, Clone)]
pub enum TrayMessage {
    ItemAdded(String, Box<StatusNotifierItem>),
    ItemUpdated(String, UpdateEvent),
    ItemRemoved(String),
    EventBatch(Vec<Event>),
    RuntimeUpdate(TrayRuntimeUpdate),
    ActivateItem(String),
    ActivateItemSecondary(String),
    ActivateMenuItem(String, i32),
    Initialize(tokio::sync::mpsc::UnboundedSender<TrayCommand>),
}

#[derive(Debug, Clone)]
pub enum TrayCommand {
    Default(String),
    Secondary(String),
    MenuItem { id: String, menu_item_id: i32 },
}

pub struct Tray {
    pub items: HashMap<String, TrayItem>,
    pub activate_tx: Option<tokio::sync::mpsc::UnboundedSender<TrayCommand>>,
    icon_resolver: crate::services::icon_resolver::IconResolver,
    runtime_diagnostics: TrayRuntimeDiagnostics,
}

impl Tray {
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
            activate_tx: None,
            icon_resolver: crate::services::icon_resolver::IconResolver::new(),
            runtime_diagnostics: TrayRuntimeDiagnostics::default(),
        }
    }

    #[cfg(test)]
    fn with_icon_resolver_for_tests(
        icon_resolver: crate::services::icon_resolver::IconResolver,
    ) -> Self {
        Self {
            items: HashMap::new(),
            activate_tx: None,
            icon_resolver,
            runtime_diagnostics: TrayRuntimeDiagnostics::default(),
        }
    }

    fn resolve_item_icon(
        &mut self,
        icon_name: Option<&str>,
        title: Option<&str>,
        theme_path: Option<&str>,
    ) -> Option<Handle> {
        icon_name
            .and_then(|name| self.icon_resolver.resolve(name, theme_path))
            .or_else(|| {
                title.and_then(|title| self.icon_resolver.resolve_title_hint(title, theme_path))
            })
    }

    fn rebuild_owned_menu(item: &mut TrayItem) {
        item.owned_menu = item
            .menu_layout
            .as_ref()
            .map(crate::services::tray_menu::OwnedTrayMenu::from_layout);
    }

    pub fn update(&mut self, message: TrayMessage) {
        match message {
            TrayMessage::ItemAdded(id, item) => {
                let (mut icon_handle, icon_signature) = get_icon_handle_and_signature(&item);
                let mut icon_source = if icon_handle.is_some() {
                    TrayIconSource::Pixmap
                } else {
                    TrayIconSource::None
                };
                if icon_handle.is_none() {
                    icon_handle = self.resolve_item_icon(
                        item.icon_name.as_deref(),
                        item.title.as_deref(),
                        item.icon_theme_path.as_deref(),
                    );
                    if icon_handle.is_some() {
                        icon_source = TrayIconSource::Theme;
                    }
                }
                self.items.insert(
                    id.clone(),
                    TrayItem {
                        _id: id,
                        title: item.title.clone(),
                        icon_name: item.icon_name.clone(),
                        icon_handle,
                        icon_signature,
                        icon_source,
                        item_is_menu: item.item_is_menu,
                        menu_path: item.menu.clone(),
                        menu_layout: None,
                        owned_menu: None,
                    },
                );
            }
            TrayMessage::ItemUpdated(id, event) => {
                let mut cache_lookup_name: Option<String> = None;
                let mut cache_lookup_title: Option<String> = None;
                if let Some(item) = self.items.get_mut(&id) {
                    match event {
                        UpdateEvent::Title(title) => {
                            item.title = title.clone();
                            if item.icon_handle.is_none() && item.icon_name.is_none() {
                                cache_lookup_title = title;
                            }
                        }
                        UpdateEvent::Icon {
                            icon_name,
                            icon_pixmap,
                        } => {
                            let mut updated_name = None;
                            if let Some(name) = icon_name {
                                item.icon_name = Some(name.clone());
                                updated_name = Some(name);
                            }
                            if let Some(pixmap) = icon_pixmap {
                                update_item_pixmap_icon(item, &pixmap);
                                if item.icon_handle.is_none() {
                                    cache_lookup_name =
                                        updated_name.or_else(|| item.icon_name.clone());
                                }
                            } else if item.icon_handle.is_none() {
                                cache_lookup_name = updated_name;
                            }
                        }
                        UpdateEvent::MenuConnect(path) => {
                            item.menu_path = Some(path);
                        }
                        UpdateEvent::Menu(layout) => {
                            item.menu_layout = Some(layout);
                            Self::rebuild_owned_menu(item);
                        }
                        UpdateEvent::MenuDiff(diffs) => {
                            if let Some(layout) = item.menu_layout.as_mut() {
                                apply_menu_diffs(layout, &diffs);
                                Self::rebuild_owned_menu(item);
                            }
                        }
                        _ => {}
                    }
                }
                if let Some(name) = cache_lookup_name {
                    let resolved = self.resolve_item_icon(Some(&name), None, None);
                    if let Some(item) = self.items.get_mut(&id) {
                        if item.icon_handle.is_none() {
                            item.icon_handle = resolved;
                            if item.icon_handle.is_some() {
                                item.icon_source = TrayIconSource::Theme;
                            }
                        }
                    }
                } else if let Some(title) = cache_lookup_title {
                    let resolved = self.resolve_item_icon(None, Some(&title), None);
                    if let Some(item) = self.items.get_mut(&id) {
                        if item.icon_handle.is_none() {
                            item.icon_handle = resolved;
                            if item.icon_handle.is_some() {
                                item.icon_source = TrayIconSource::Theme;
                            }
                        }
                    }
                }
            }
            TrayMessage::ItemRemoved(id) => {
                self.items.remove(&id);
            }
            TrayMessage::EventBatch(events) => {
                for event in events {
                    match event {
                        Event::Add(id, item) => self.update(TrayMessage::ItemAdded(id, item)),
                        Event::Update(id, update) => {
                            self.update(TrayMessage::ItemUpdated(id, update))
                        }
                        Event::Remove(id) => self.update(TrayMessage::ItemRemoved(id)),
                    }
                }
            }
            TrayMessage::RuntimeUpdate(update) => match update {
                TrayRuntimeUpdate::SecondaryObserved {
                    route,
                    result,
                    failure,
                } => {
                    self.runtime_diagnostics.last_secondary_route = Some(route);
                    self.runtime_diagnostics.last_secondary_result = Some(result);
                    self.runtime_diagnostics.last_dispatch_failure = failure;
                }
                TrayRuntimeUpdate::MenuActivationError(error) => {
                    self.runtime_diagnostics.last_menu_activation_error = error;
                }
            },
            TrayMessage::ActivateItem(id) => {
                if let Some(tx) = &self.activate_tx {
                    let _ = tx.send(TrayCommand::Default(id));
                }
            }
            TrayMessage::ActivateItemSecondary(id) => {
                if let Some(tx) = &self.activate_tx {
                    let _ = tx.send(TrayCommand::Secondary(id));
                }
            }
            TrayMessage::ActivateMenuItem(id, menu_item_id) => {
                if let Some(tx) = &self.activate_tx {
                    let _ = tx.send(TrayCommand::MenuItem { id, menu_item_id });
                }
            }
            TrayMessage::Initialize(tx) => {
                self.activate_tx = Some(tx);
            }
        }
    }

    pub fn owned_menu_for(&self, id: &str) -> Option<&crate::services::tray_menu::OwnedTrayMenu> {
        self.items.get(id).and_then(|item| item.owned_menu.as_ref())
    }

    pub fn has_menu_entries(&self, id: &str) -> bool {
        self.owned_menu_for(id)
            .is_some_and(crate::services::tray_menu::OwnedTrayMenu::has_visible_actions)
    }

    pub fn diagnostics(&self) -> TrayDiagnostics {
        let total_items = self.items.len();
        let resolved_icons = self
            .items
            .values()
            .filter(|item| item.icon_handle.is_some())
            .count();
        let fallback_labels = self
            .items
            .values()
            .filter(|item| item.icon_handle.is_none())
            .filter(|item| item.title.is_some() || item.icon_name.is_some())
            .count();
        let last_unresolved_item = self.items.values().find_map(|item| {
            if item.icon_handle.is_some() {
                return None;
            }
            item.title
                .clone()
                .or_else(|| item.icon_name.clone())
                .or_else(|| Some(item._id.clone()))
        });

        TrayDiagnostics {
            total_items,
            resolved_icons,
            unresolved_icons: total_items.saturating_sub(resolved_icons),
            fallback_labels,
            resolver: self.icon_resolver.diagnostics(),
            last_unresolved_item,
            runtime: self.runtime_diagnostics.clone(),
        }
    }
}

fn first_visible_alphanumeric(raw: &str) -> Option<char> {
    raw.chars()
        .find(|ch| ch.is_alphanumeric())
        .map(to_display_uppercase)
}

fn icon_fallback_label_from_name(raw: &str) -> Option<char> {
    let basename = raw
        .rsplit('/')
        .next()
        .unwrap_or(raw)
        .trim_end_matches(".svg")
        .trim_end_matches(".png")
        .trim_end_matches(".xpm")
        .trim_end_matches("-symbolic")
        .trim_end_matches("-panel");

    basename
        .rsplit(|ch: char| !ch.is_alphanumeric())
        .find(|segment| !segment.is_empty())
        .and_then(first_visible_alphanumeric)
}

fn to_display_uppercase(ch: char) -> char {
    ch.to_uppercase().next().unwrap_or(ch)
}

fn apply_menu_diffs(tray_menu: &mut TrayMenu, diffs: &[MenuDiff]) {
    for diff in diffs {
        if let Some(item) = find_menu_item_mut(&mut tray_menu.submenus, diff.id) {
            apply_menu_item_update(item, &diff.update);
            apply_menu_item_remove(item, &diff.remove);
        }
    }
}

fn find_menu_item_mut(items: &mut [MenuItem], id: i32) -> Option<&mut MenuItem> {
    for item in items {
        if item.id == id {
            return Some(item);
        }
        if let Some(found) = find_menu_item_mut(&mut item.submenu, id) {
            return Some(found);
        }
    }
    None
}

fn apply_menu_item_update(item: &mut MenuItem, update: &MenuItemUpdate) {
    if let Some(label) = &update.label {
        item.label.clone_from(label);
    }
    if let Some(enabled) = update.enabled {
        item.enabled = enabled;
    }
    if let Some(visible) = update.visible {
        item.visible = visible;
    }
    if let Some(icon_name) = &update.icon_name {
        item.icon_name.clone_from(icon_name);
    }
    if let Some(icon_data) = &update.icon_data {
        item.icon_data.clone_from(icon_data);
    }
    if let Some(toggle_state) = update.toggle_state {
        item.toggle_state = toggle_state;
    }
    if let Some(disposition) = update.disposition {
        item.disposition = disposition;
    }
}

fn apply_menu_item_remove(item: &mut MenuItem, remove: &[String]) {
    for field in remove {
        match field.as_str() {
            "label" => item.label = None,
            "enabled" => item.enabled = true,
            "visible" => item.visible = true,
            "icon-name" => item.icon_name = None,
            "icon-data" => item.icon_data = None,
            "toggle-state" => item.toggle_state = Default::default(),
            "disposition" => item.disposition = Default::default(),
            _ => {}
        }
    }
}

fn get_icon_handle_and_signature(item: &StatusNotifierItem) -> (Option<Handle>, Option<u64>) {
    if let Some(pixmaps) = &item.icon_pixmap {
        (
            pixmap_to_handle(pixmaps),
            pixmap_signature_for_best_pixmap(pixmaps),
        )
    } else {
        (None, None)
    }
}

fn best_pixmap(pixmaps: &[IconPixmap]) -> Option<&IconPixmap> {
    let target_size = 24;
    pixmaps.iter().min_by_key(|p| (p.width - target_size).abs())
}

fn pixmap_signature_for_best_pixmap(pixmaps: &[IconPixmap]) -> Option<u64> {
    let best = best_pixmap(pixmaps)?;
    let mut hasher = DefaultHasher::new();
    best.width.hash(&mut hasher);
    best.height.hash(&mut hasher);
    best.pixels.len().hash(&mut hasher);
    best.pixels.hash(&mut hasher);
    Some(hasher.finish())
}

fn update_item_pixmap_icon(item: &mut TrayItem, pixmaps: &[IconPixmap]) -> bool {
    let signature = pixmap_signature_for_best_pixmap(pixmaps);
    if signature == item.icon_signature {
        return false;
    }
    if item.icon_source == TrayIconSource::Pixmap && item.icon_signature.is_some() {
        return false;
    }
    item.icon_signature = signature;
    item.icon_handle = pixmap_to_handle(pixmaps);
    item.icon_source = if item.icon_handle.is_some() {
        TrayIconSource::Pixmap
    } else {
        TrayIconSource::None
    };
    true
}

fn pixmap_to_handle(pixmaps: &[IconPixmap]) -> Option<Handle> {
    if let Some(best) = best_pixmap(pixmaps) {
        let width = best.width as u32;
        let height = best.height as u32;
        let mut rgba_pixels = Vec::with_capacity((width * height * 4) as usize);

        // Network byte order ARGB32 implies bytes are: A, R, G, B
        for chunk in best.pixels.chunks_exact(4) {
            let a = chunk[0];
            let r = chunk[1];
            let g = chunk[2];
            let b = chunk[3];
            rgba_pixels.push(r);
            rgba_pixels.push(g);
            rgba_pixels.push(b);
            rgba_pixels.push(a);
        }

        Some(Handle::from_rgba(width, height, rgba_pixels))
    } else {
        None
    }
}

#[cfg(test)]
fn parse_cursor_pos(raw: &str) -> Option<(i32, i32)> {
    let value = serde_json::from_str::<serde_json::Value>(raw).ok()?;
    let x = value.get("x")?.as_f64()?.round() as i32;
    let y = value.get("y")?.as_f64()?.round() as i32;
    Some((x, y))
}

pub(crate) fn current_cursor_pos_with_fallback(last_known: (i32, i32)) -> (i32, i32) {
    crate::services::compositor::cursor_position().unwrap_or(last_known)
}

fn parse_status_notifier_address(address: &str) -> (&str, String) {
    address
        .split_once('/')
        .map_or((address, String::from("/StatusNotifierItem")), |(d, p)| {
            (d, format!("/{p}"))
        })
}

pub(crate) fn destination_from_item_address(address: &str) -> &str {
    address
        .split_once('/')
        .map_or(address, |(destination, _)| destination)
}

fn select_registered_item_address<'a>(
    destination: &str,
    registered: &'a [String],
) -> Option<&'a str> {
    registered
        .iter()
        .find(|address| destination_from_item_address(address) == destination)
        .map(|s| s.as_str())
}

async fn fetch_registered_item_addresses(
    connection: &zbus::Connection,
) -> Result<Vec<String>, zbus::Error> {
    let proxy = zbus::Proxy::new(
        connection,
        "org.kde.StatusNotifierWatcher",
        "/StatusNotifierWatcher",
        "org.kde.StatusNotifierWatcher",
    )
    .await?;
    proxy.get_property("RegisteredStatusNotifierItems").await
}

pub(crate) async fn resolve_item_address(
    connection: &mut Option<zbus::Connection>,
    cache: &mut HashMap<String, String>,
    destination: &str,
) -> String {
    if let Some(cached) = cache.get(destination) {
        return cached.clone();
    }

    let fetch = async {
        let conn = ensure_context_connection(connection).await?;
        fetch_registered_item_addresses(conn).await
    };
    let resolved = match timeout(Duration::from_millis(600), fetch).await {
        Ok(Ok(registered)) => {
            if let Some(address) = select_registered_item_address(destination, &registered) {
                address.to_string()
            } else {
                destination.to_string()
            }
        }
        Ok(Err(err)) => {
            debug!("tray watcher lookup failed for {}: {}", destination, err);
            destination.to_string()
        }
        Err(_) => {
            debug!("tray watcher lookup timed out for {}", destination);
            destination.to_string()
        }
    };
    cache.insert(destination.to_string(), resolved.clone());
    resolved
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SecondaryAction {
    ContextMenu,
    SecondaryActivate,
    DefaultActivate,
    MenuRootActivate,
}

impl SecondaryAction {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::ContextMenu => "context_menu",
            Self::SecondaryActivate => "secondary_activate",
            Self::DefaultActivate => "default_activate",
            Self::MenuRootActivate => "menu_root_activate",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SecondaryPlan {
    pub(crate) primary: SecondaryAction,
    pub(crate) fallback: Option<SecondaryAction>,
}

pub(crate) fn choose_secondary_plan(
    item_is_menu: bool,
    has_menu_path: bool,
    preferred: Option<SecondaryAction>,
) -> SecondaryPlan {
    if let Some(preferred_action) = preferred {
        return SecondaryPlan {
            primary: preferred_action,
            fallback: Some(opposite_secondary_action(preferred_action)),
        };
    }

    if item_is_menu || has_menu_path {
        SecondaryPlan {
            primary: SecondaryAction::ContextMenu,
            fallback: Some(SecondaryAction::SecondaryActivate),
        }
    } else {
        SecondaryPlan {
            primary: SecondaryAction::SecondaryActivate,
            fallback: Some(SecondaryAction::ContextMenu),
        }
    }
}

fn opposite_secondary_action(action: SecondaryAction) -> SecondaryAction {
    match action {
        SecondaryAction::ContextMenu => SecondaryAction::SecondaryActivate,
        SecondaryAction::SecondaryActivate => SecondaryAction::ContextMenu,
        SecondaryAction::DefaultActivate => SecondaryAction::ContextMenu,
        SecondaryAction::MenuRootActivate => SecondaryAction::ContextMenu,
    }
}

pub(crate) fn get_secondary_capabilities(
    client: &Client,
    id: &str,
) -> (bool, bool, Option<String>) {
    let items_arc = client.items();
    if let Ok(items) = items_arc.lock() {
        if let Some((item, _)) = items.get(id) {
            return (item.item_is_menu, item.menu.is_some(), item.menu.clone());
        }
    }
    (false, false, None)
}

#[derive(Debug, Clone)]
pub(crate) enum ActivationResult {
    PrimaryOk(SecondaryAction),
    FallbackOk {
        primary: SecondaryAction,
        fallback: SecondaryAction,
    },
    Failed {
        primary: SecondaryAction,
        fallback: Option<SecondaryAction>,
    },
}

impl ActivationResult {
    pub(crate) fn succeeded(&self) -> bool {
        matches!(
            self,
            ActivationResult::PrimaryOk(_) | ActivationResult::FallbackOk { .. }
        )
    }
}

impl std::fmt::Display for ActivationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActivationResult::PrimaryOk(method) => write!(f, "primary_ok:{}", method.as_str()),
            ActivationResult::FallbackOk { primary, fallback } => {
                write!(f, "fallback_ok:{}->{}", primary.as_str(), fallback.as_str())
            }
            ActivationResult::Failed { primary, fallback } => {
                if let Some(fallback) = fallback {
                    write!(f, "failed:{}->{}", primary.as_str(), fallback.as_str())
                } else {
                    write!(f, "failed:{}", primary.as_str())
                }
            }
        }
    }
}

pub(crate) async fn ensure_context_connection(
    cached: &mut Option<zbus::Connection>,
) -> Result<&zbus::Connection, zbus::Error> {
    if cached.is_none() {
        *cached = Some(zbus::Connection::session().await?);
    }
    match cached.as_ref() {
        Some(conn) => Ok(conn),
        None => unreachable!("context connection missing after successful initialization"),
    }
}

async fn activate_context_menu(
    connection: &zbus::Connection,
    address: &str,
    x: i32,
    y: i32,
) -> Result<(), zbus::Error> {
    let (destination, path) = parse_status_notifier_address(address);
    let proxy =
        zbus::Proxy::new(connection, destination, path, "org.kde.StatusNotifierItem").await?;
    let _: () = proxy.call("ContextMenu", &(x, y)).await?;
    Ok(())
}

async fn try_context_menu(
    connection: &mut Option<zbus::Connection>,
    id: &str,
    x: i32,
    y: i32,
) -> bool {
    let fut = async {
        let conn = ensure_context_connection(connection).await?;
        activate_context_menu(conn, id, x, y).await
    };
    match timeout(Duration::from_millis(1000), fut).await {
        Ok(Ok(())) => true,
        Ok(Err(err)) => {
            debug!("tray context menu call failed for {}: {}", id, err);
            false
        }
        Err(_) => {
            debug!("tray context menu call timed out for {}", id);
            false
        }
    }
}

async fn try_secondary_activate(client: &Client, id: &str, x: i32, y: i32) -> bool {
    client
        .activate(ActivateRequest::Secondary {
            address: id.to_string(),
            x,
            y,
        })
        .await
        .is_ok()
}

async fn try_default_activate(client: &Client, id: &str, x: i32, y: i32) -> bool {
    client
        .activate(ActivateRequest::Default {
            address: id.to_string(),
            x,
            y,
        })
        .await
        .is_ok()
}

async fn try_menu_root_activate(client: &Client, id: &str, menu_path: Option<&str>) -> bool {
    let Some(menu_path) = menu_path else {
        return false;
    };
    let address = id.to_string();
    let menu_path = menu_path.to_string();
    let _ = client
        .about_to_show_menuitem(address.clone(), menu_path.clone(), 0)
        .await;
    client
        .activate(ActivateRequest::MenuItem {
            address,
            menu_path,
            submenu_id: 0,
        })
        .await
        .is_ok()
}

async fn run_secondary_action(
    action: SecondaryAction,
    client: &Client,
    connection: &mut Option<zbus::Connection>,
    id: &str,
    menu_path: Option<&str>,
    x: i32,
    y: i32,
) -> bool {
    match action {
        SecondaryAction::ContextMenu => try_context_menu(connection, id, x, y).await,
        SecondaryAction::SecondaryActivate => try_secondary_activate(client, id, x, y).await,
        SecondaryAction::DefaultActivate => try_default_activate(client, id, x, y).await,
        SecondaryAction::MenuRootActivate => try_menu_root_activate(client, id, menu_path).await,
    }
}

pub(crate) async fn activate_secondary_with_plan(
    client: &Client,
    connection: &mut Option<zbus::Connection>,
    id: &str,
    menu_path: Option<&str>,
    x: i32,
    y: i32,
    plan: SecondaryPlan,
) -> ActivationResult {
    let primary_ok =
        run_secondary_action(plan.primary, client, connection, id, menu_path, x, y).await;

    if primary_ok {
        return ActivationResult::PrimaryOk(plan.primary);
    }

    if let Some(fallback) = plan.fallback {
        let fallback_ok =
            run_secondary_action(fallback, client, connection, id, menu_path, x, y).await;
        if fallback_ok {
            ActivationResult::FallbackOk {
                primary: plan.primary,
                fallback,
            }
        } else {
            let default_ok = run_secondary_action(
                SecondaryAction::DefaultActivate,
                client,
                connection,
                id,
                menu_path,
                x,
                y,
            )
            .await;
            if default_ok {
                ActivationResult::FallbackOk {
                    primary: plan.primary,
                    fallback: SecondaryAction::DefaultActivate,
                }
            } else {
                let menu_root_ok = run_secondary_action(
                    SecondaryAction::MenuRootActivate,
                    client,
                    connection,
                    id,
                    menu_path,
                    x,
                    y,
                )
                .await;
                if menu_root_ok {
                    ActivationResult::FallbackOk {
                        primary: plan.primary,
                        fallback: SecondaryAction::MenuRootActivate,
                    }
                } else {
                    ActivationResult::Failed {
                        primary: plan.primary,
                        fallback: Some(SecondaryAction::MenuRootActivate),
                    }
                }
            }
        }
    } else {
        let default_ok = run_secondary_action(
            SecondaryAction::DefaultActivate,
            client,
            connection,
            id,
            menu_path,
            x,
            y,
        )
        .await;
        if default_ok {
            ActivationResult::FallbackOk {
                primary: plan.primary,
                fallback: SecondaryAction::DefaultActivate,
            }
        } else {
            let menu_root_ok = run_secondary_action(
                SecondaryAction::MenuRootActivate,
                client,
                connection,
                id,
                menu_path,
                x,
                y,
            )
            .await;
            if menu_root_ok {
                ActivationResult::FallbackOk {
                    primary: plan.primary,
                    fallback: SecondaryAction::MenuRootActivate,
                }
            } else {
                ActivationResult::Failed {
                    primary: plan.primary,
                    fallback: Some(SecondaryAction::MenuRootActivate),
                }
            }
        }
    }
}

pub(crate) fn update_secondary_preference(
    preferred_secondary_actions: &mut HashMap<String, SecondaryAction>,
    id: &str,
    result: &ActivationResult,
) {
    match result {
        ActivationResult::PrimaryOk(success_action) => {
            preferred_secondary_actions.insert(id.to_string(), *success_action);
        }
        ActivationResult::FallbackOk { .. } | ActivationResult::Failed { .. } => {
            preferred_secondary_actions.remove(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        choose_secondary_plan, destination_from_item_address, parse_cursor_pos,
        parse_status_notifier_address, pixmap_signature_for_best_pixmap,
        select_registered_item_address, update_item_pixmap_icon, update_secondary_preference,
        ActivationResult, SecondaryAction, SecondaryPlan, Tray, TrayIconSource, TrayItem, TrayMenu,
        TrayMessage, TrayRuntimeUpdate, UpdateEvent,
    };
    use iced::widget::image::Handle;
    use std::collections::HashMap;
    use system_tray::item::{IconPixmap, StatusNotifierItem};
    use system_tray::menu::{MenuDiff, MenuItemUpdate};

    #[test]
    fn tray_has_menu_entries_uses_visible_menu_items() {
        let mut tray = Tray::new();
        tray.items.insert(
            "item".to_string(),
            TrayItem {
                _id: "item".to_string(),
                title: None,
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
        assert!(tray.has_menu_entries("item"));
    }

    #[test]
    fn tray_item_fallback_label_prefers_title_then_icon_name_then_generic() {
        let titled = TrayItem {
            _id: "item".to_string(),
            title: Some("My App".to_string()),
            icon_name: Some("org.example.raw-icon".to_string()),
            icon_handle: None,
            icon_signature: None,
            icon_source: TrayIconSource::None,
            item_is_menu: false,
            menu_path: None,
            menu_layout: None,
            owned_menu: None,
        };
        assert_eq!(titled.fallback_label(), "M");

        let icon_named = TrayItem {
            _id: "item".to_string(),
            title: None,
            icon_name: Some("org.example.myapp-panel-symbolic".to_string()),
            icon_handle: None,
            icon_signature: None,
            icon_source: TrayIconSource::None,
            item_is_menu: false,
            menu_path: None,
            menu_layout: None,
            owned_menu: None,
        };
        assert_eq!(icon_named.fallback_label(), "M");

        let menu = TrayItem {
            _id: "menu".to_string(),
            title: None,
            icon_name: None,
            icon_handle: None,
            icon_signature: None,
            icon_source: TrayIconSource::None,
            item_is_menu: true,
            menu_path: None,
            menu_layout: None,
            owned_menu: None,
        };
        assert_eq!(menu.fallback_label(), "≡");
    }

    #[test]
    fn tray_diagnostics_count_resolved_and_unresolved_icons() {
        let mut tray = Tray::new();
        tray.items.insert(
            "resolved".to_string(),
            TrayItem {
                _id: "resolved".to_string(),
                title: Some("Resolved".to_string()),
                icon_name: Some("resolved".to_string()),
                icon_handle: Some(Handle::from_path("/tmp/resolved.png")),
                icon_signature: None,
                icon_source: TrayIconSource::Theme,
                item_is_menu: false,
                menu_path: None,
                menu_layout: None,
                owned_menu: None,
            },
        );
        tray.items.insert(
            "fallback".to_string(),
            TrayItem {
                _id: "fallback".to_string(),
                title: Some("Fallback".to_string()),
                icon_name: None,
                icon_handle: None,
                icon_signature: None,
                icon_source: TrayIconSource::None,
                item_is_menu: false,
                menu_path: None,
                menu_layout: None,
                owned_menu: None,
            },
        );

        let diagnostics = tray.diagnostics();
        assert_eq!(diagnostics.total_items, 2);
        assert_eq!(diagnostics.resolved_icons, 1);
        assert_eq!(diagnostics.unresolved_icons, 1);
        assert_eq!(diagnostics.fallback_labels, 1);
        assert_eq!(
            diagnostics.last_unresolved_item.as_deref(),
            Some("Fallback")
        );
        assert!(diagnostics.summary().contains("1/2 resolved"));
    }

    #[test]
    fn tray_runtime_diagnostics_capture_secondary_route_and_failure() {
        let mut tray = Tray::new();
        tray.update(TrayMessage::RuntimeUpdate(
            TrayRuntimeUpdate::SecondaryObserved {
                route: "context_menu->secondary_activate".to_string(),
                result: "failed:context_menu->menu_root_activate".to_string(),
                failure: Some(":1.55 resolved=:1.55 failed".to_string()),
            },
        ));

        let diagnostics = tray.diagnostics();
        assert_eq!(
            diagnostics.runtime.last_secondary_route.as_deref(),
            Some("context_menu->secondary_activate")
        );
        assert_eq!(
            diagnostics.runtime.last_secondary_result.as_deref(),
            Some("failed:context_menu->menu_root_activate")
        );
        assert_eq!(
            diagnostics.runtime.last_dispatch_failure.as_deref(),
            Some(":1.55 resolved=:1.55 failed")
        );
    }

    #[test]
    fn tray_runtime_diagnostics_clear_menu_activation_error_on_success() {
        let mut tray = Tray::new();
        tray.update(TrayMessage::RuntimeUpdate(
            TrayRuntimeUpdate::MenuActivationError(Some(
                ":1.55 item=42 org.freedesktop.DBus.Error.UnknownMethod".to_string(),
            )),
        ));
        assert!(tray
            .diagnostics()
            .runtime
            .last_menu_activation_error
            .is_some());

        tray.update(TrayMessage::RuntimeUpdate(
            TrayRuntimeUpdate::MenuActivationError(None),
        ));
        assert!(tray
            .diagnostics()
            .runtime
            .last_menu_activation_error
            .is_none());
    }

    #[test]
    fn tray_title_only_icon_resolution_does_not_grow_negative_cache() {
        let mut tray = Tray::new();
        tray.update(TrayMessage::ItemAdded(
            "item".to_string(),
            Box::new(StatusNotifierItem {
                id: "item".to_string(),
                category: system_tray::item::Category::ApplicationStatus,
                title: Some("Vesktop - unread 1".to_string()),
                status: system_tray::item::Status::Active,
                window_id: 0,
                icon_name: None,
                overlay_icon_name: None,
                overlay_icon_pixmap: None,
                attention_icon_name: None,
                attention_icon_pixmap: None,
                attention_movie_name: None,
                icon_theme_path: None,
                icon_pixmap: None,
                tool_tip: None,
                item_is_menu: false,
                menu: None,
            }),
        ));
        tray.update(TrayMessage::ItemUpdated(
            "item".to_string(),
            UpdateEvent::Title(Some("Vesktop - unread 2".to_string())),
        ));

        let diagnostics = tray.diagnostics();
        assert_eq!(diagnostics.resolver.cache_entries, 0);
        assert_eq!(diagnostics.resolver.negative_entries, 0);
    }

    #[test]
    fn tray_item_update_resolves_icon_from_title_when_icon_name_missing() {
        let temp = std::env::temp_dir().join(format!(
            "thinkpadbar-tray-title-icon-{}",
            std::process::id()
        ));
        let data_home = temp.join("share");
        let applications = data_home.join("applications");
        let icons = data_home.join("icons/hicolor/48x48/apps");
        std::fs::create_dir_all(&applications).expect("applications dir");
        std::fs::create_dir_all(&icons).expect("icons dir");
        std::fs::write(
            applications.join("vesktop.desktop"),
            "[Desktop Entry]\nName=Vesktop\nIcon=vesktop\nExec=/usr/bin/vesktop\n",
        )
        .expect("desktop entry");
        std::fs::write(icons.join("vesktop.png"), b"png").expect("icon");

        let resolver =
            crate::services::icon_resolver::IconResolver::with_data_home_for_tests(data_home);
        let mut tray = Tray::with_icon_resolver_for_tests(resolver);
        tray.items.insert(
            "item".to_string(),
            TrayItem {
                _id: "item".to_string(),
                title: None,
                icon_name: None,
                icon_handle: None,
                icon_signature: None,
                icon_source: TrayIconSource::None,
                item_is_menu: false,
                menu_path: None,
                menu_layout: None,
                owned_menu: None,
            },
        );

        tray.update(TrayMessage::ItemUpdated(
            "item".to_string(),
            UpdateEvent::Title(Some("Vesktop".to_string())),
        ));

        assert!(tray
            .items
            .get("item")
            .and_then(|item| item.icon_handle.as_ref())
            .is_some());

        let _ = std::fs::remove_dir_all(temp);
    }

    #[test]
    fn pixmap_signature_is_stable_for_identical_pixmaps() {
        let pixmap = vec![IconPixmap {
            width: 24,
            height: 24,
            pixels: vec![255; 24 * 24 * 4],
        }];

        assert_eq!(
            pixmap_signature_for_best_pixmap(&pixmap),
            pixmap_signature_for_best_pixmap(&pixmap)
        );
    }

    #[test]
    fn identical_pixmap_update_is_deduped() {
        let pixmap = vec![IconPixmap {
            width: 24,
            height: 24,
            pixels: vec![255; 24 * 24 * 4],
        }];
        let mut item = TrayItem {
            _id: "item".to_string(),
            title: None,
            icon_name: None,
            icon_handle: None,
            icon_signature: None,
            icon_source: TrayIconSource::None,
            item_is_menu: false,
            menu_path: None,
            menu_layout: None,
            owned_menu: None,
        };

        assert!(update_item_pixmap_icon(&mut item, &pixmap));
        let first_signature = item.icon_signature;
        assert!(item.icon_handle.is_some());
        assert!(!update_item_pixmap_icon(&mut item, &pixmap));
        assert_eq!(item.icon_signature, first_signature);
    }

    #[test]
    fn changed_pixmap_update_replaces_signature() {
        let first = vec![IconPixmap {
            width: 24,
            height: 24,
            pixels: vec![255; 24 * 24 * 4],
        }];
        let second = vec![IconPixmap {
            width: 24,
            height: 24,
            pixels: vec![127; 24 * 24 * 4],
        }];
        let mut item = TrayItem {
            _id: "item".to_string(),
            title: None,
            icon_name: None,
            icon_handle: None,
            icon_signature: None,
            icon_source: TrayIconSource::None,
            item_is_menu: false,
            menu_path: None,
            menu_layout: None,
            owned_menu: None,
        };

        assert!(update_item_pixmap_icon(&mut item, &first));
        let first_signature = item.icon_signature;
        assert!(!update_item_pixmap_icon(&mut item, &second));
        assert_eq!(item.icon_signature, first_signature);
    }

    #[test]
    fn pixmap_updates_are_frozen_after_first_dynamic_handle() {
        let first = vec![IconPixmap {
            width: 24,
            height: 24,
            pixels: vec![255; 24 * 24 * 4],
        }];
        let second = vec![IconPixmap {
            width: 24,
            height: 24,
            pixels: vec![127; 24 * 24 * 4],
        }];
        let mut item = TrayItem {
            _id: "item".to_string(),
            title: None,
            icon_name: None,
            icon_handle: None,
            icon_signature: None,
            icon_source: TrayIconSource::None,
            item_is_menu: false,
            menu_path: None,
            menu_layout: None,
            owned_menu: None,
        };

        assert!(update_item_pixmap_icon(&mut item, &first));
        let first_handle = item.icon_handle.clone();
        let first_signature = item.icon_signature;

        assert!(!update_item_pixmap_icon(&mut item, &second));
        assert_eq!(item.icon_signature, first_signature);
        assert_eq!(item.icon_handle, first_handle);
    }

    #[test]
    fn menu_diff_updates_nested_item_and_owned_menu_state() {
        let mut tray = Tray::new();
        let mut parent = system_tray::menu::MenuItem {
            id: 1,
            label: Some("Parent".to_string()),
            visible: true,
            enabled: true,
            ..Default::default()
        };
        parent.submenu = vec![system_tray::menu::MenuItem {
            id: 2,
            label: Some("Child".to_string()),
            visible: true,
            enabled: true,
            ..Default::default()
        }];
        let layout = TrayMenu {
            id: 0,
            submenus: vec![parent],
        };

        tray.items.insert(
            "item".to_string(),
            TrayItem {
                _id: "item".to_string(),
                title: None,
                icon_name: None,
                icon_handle: None,
                icon_signature: None,
                icon_source: TrayIconSource::None,
                item_is_menu: true,
                menu_path: Some("/menu".to_string()),
                owned_menu: Some(crate::services::tray_menu::OwnedTrayMenu::from_layout(
                    &layout,
                )),
                menu_layout: Some(layout),
            },
        );
        tray.update(TrayMessage::ItemUpdated(
            "item".to_string(),
            UpdateEvent::MenuDiff(vec![MenuDiff {
                id: 2,
                update: MenuItemUpdate {
                    enabled: Some(false),
                    label: Some(Some("Child Disabled".to_string())),
                    ..Default::default()
                },
                remove: Vec::new(),
            }]),
        ));

        let menu = tray.owned_menu_for("item").expect("menu should exist");
        let child = menu
            .nodes()
            .iter()
            .find_map(|n| match n {
                crate::services::tray_menu::OwnedTrayMenuNode::Action(a) if a.id == 2 => Some(a),
                _ => None,
            })
            .expect("child action should exist");
        assert_eq!(child.label, "Child Disabled");
        assert!(!child.enabled);
    }

    #[test]
    fn parse_status_notifier_address_supports_explicit_path() {
        let (dest, path) = parse_status_notifier_address(":1.58/org/ayatana/NotificationItem/x");
        assert_eq!(dest, ":1.58");
        assert_eq!(path, "/org/ayatana/NotificationItem/x");
    }

    #[test]
    fn parse_status_notifier_address_uses_default_path() {
        let (dest, path) = parse_status_notifier_address("org.kde.StatusNotifierItem-1-1");
        assert_eq!(dest, "org.kde.StatusNotifierItem-1-1");
        assert_eq!(path, "/StatusNotifierItem");
    }

    #[test]
    fn destination_from_item_address_extracts_destination() {
        assert_eq!(destination_from_item_address(":1.533"), ":1.533");
        assert_eq!(
            destination_from_item_address(":1.533/org/ayatana/NotificationItem/foo"),
            ":1.533"
        );
    }

    #[test]
    fn select_registered_item_address_matches_destination_with_custom_path() {
        let registered = vec![
            ":1.528/StatusNotifierItem".to_string(),
            ":1.533/org/ayatana/NotificationItem/foo".to_string(),
        ];
        let found = select_registered_item_address(":1.533", &registered);
        assert_eq!(found, Some(":1.533/org/ayatana/NotificationItem/foo"));
    }

    #[test]
    fn parse_cursor_pos_extracts_coordinates() {
        let parsed = parse_cursor_pos(r#"{"x": 101.2, "y": 202.8}"#);
        assert_eq!(parsed, Some((101, 203)));
    }

    #[test]
    fn choose_secondary_plan_prefers_context_for_menu_items() {
        let plan_item_menu = choose_secondary_plan(true, false, None);
        assert_eq!(plan_item_menu.primary, SecondaryAction::ContextMenu);
        assert_eq!(
            plan_item_menu.fallback,
            Some(SecondaryAction::SecondaryActivate)
        );

        let plan_has_menu = choose_secondary_plan(false, true, None);
        assert_eq!(plan_has_menu.primary, SecondaryAction::ContextMenu);
        assert_eq!(
            plan_has_menu.fallback,
            Some(SecondaryAction::SecondaryActivate)
        );
    }

    #[test]
    fn choose_secondary_plan_prefers_secondary_for_non_menu_items() {
        let plan = choose_secondary_plan(false, false, None);
        assert_eq!(plan.primary, SecondaryAction::SecondaryActivate);
        assert_eq!(plan.fallback, Some(SecondaryAction::ContextMenu));
    }

    #[test]
    fn choose_secondary_plan_respects_preferred_action() {
        let plan = choose_secondary_plan(true, true, Some(SecondaryAction::SecondaryActivate));
        assert_eq!(plan.primary, SecondaryAction::SecondaryActivate);
        assert_eq!(plan.fallback, Some(SecondaryAction::ContextMenu));
    }

    #[tokio::test]
    async fn execute_secondary_plan_runs_single_fallback_and_reports_success() {
        let plan = choose_secondary_plan(true, false, None);
        let mut stub = StubSecondaryExecutor::new(vec![false], vec![true], vec![true], vec![true]);
        let result = stub.execute(plan).await;

        assert_eq!(
            stub.attempts,
            vec![
                SecondaryAction::ContextMenu,
                SecondaryAction::SecondaryActivate
            ]
        );
        assert_eq!(
            result.to_string(),
            "fallback_ok:context_menu->secondary_activate"
        );
    }

    #[tokio::test]
    async fn execute_secondary_plan_skips_fallback_when_primary_succeeds() {
        let plan = choose_secondary_plan(false, false, None);
        let mut stub = StubSecondaryExecutor::new(vec![true], vec![true], vec![true], vec![true]);
        let result = stub.execute(plan).await;

        assert_eq!(stub.attempts, vec![SecondaryAction::SecondaryActivate]);
        assert_eq!(result.to_string(), "primary_ok:secondary_activate");
    }

    #[tokio::test]
    async fn execute_secondary_plan_uses_default_activate_after_two_failures() {
        let plan = choose_secondary_plan(true, false, None);
        let mut stub = StubSecondaryExecutor::new(vec![false], vec![false], vec![true], vec![true]);
        let result = stub.execute(plan).await;

        assert_eq!(
            stub.attempts,
            vec![
                SecondaryAction::ContextMenu,
                SecondaryAction::SecondaryActivate,
                SecondaryAction::DefaultActivate
            ]
        );
        assert_eq!(
            result.to_string(),
            "fallback_ok:context_menu->default_activate"
        );
    }

    #[tokio::test]
    async fn execute_secondary_plan_uses_menu_root_when_default_fails() {
        let plan = choose_secondary_plan(true, false, None);
        let mut stub =
            StubSecondaryExecutor::new(vec![false], vec![false], vec![false], vec![true]);
        let result = stub.execute(plan).await;

        assert_eq!(
            stub.attempts,
            vec![
                SecondaryAction::ContextMenu,
                SecondaryAction::SecondaryActivate,
                SecondaryAction::DefaultActivate,
                SecondaryAction::MenuRootActivate
            ]
        );
        assert_eq!(
            result.to_string(),
            "fallback_ok:context_menu->menu_root_activate"
        );
    }

    #[test]
    fn update_secondary_preference_pins_successful_action_and_clears_on_failure() {
        let mut prefs = HashMap::new();
        update_secondary_preference(
            &mut prefs,
            "item-a",
            &ActivationResult::FallbackOk {
                primary: SecondaryAction::ContextMenu,
                fallback: SecondaryAction::SecondaryActivate,
            },
        );
        assert!(!prefs.contains_key("item-a"));

        update_secondary_preference(
            &mut prefs,
            "item-a",
            &ActivationResult::PrimaryOk(SecondaryAction::SecondaryActivate),
        );
        assert_eq!(
            prefs.get("item-a"),
            Some(&SecondaryAction::SecondaryActivate)
        );

        update_secondary_preference(
            &mut prefs,
            "item-a",
            &ActivationResult::Failed {
                primary: SecondaryAction::SecondaryActivate,
                fallback: Some(SecondaryAction::ContextMenu),
            },
        );
        assert!(!prefs.contains_key("item-a"));
    }

    struct StubSecondaryExecutor {
        context_results: std::collections::VecDeque<bool>,
        secondary_results: std::collections::VecDeque<bool>,
        default_results: std::collections::VecDeque<bool>,
        menu_root_results: std::collections::VecDeque<bool>,
        attempts: Vec<SecondaryAction>,
    }

    impl StubSecondaryExecutor {
        fn new(
            context_results: Vec<bool>,
            secondary_results: Vec<bool>,
            default_results: Vec<bool>,
            menu_root_results: Vec<bool>,
        ) -> Self {
            Self {
                context_results: context_results.into(),
                secondary_results: secondary_results.into(),
                default_results: default_results.into(),
                menu_root_results: menu_root_results.into(),
                attempts: Vec::new(),
            }
        }

        async fn execute(&mut self, plan: SecondaryPlan) -> ActivationResult {
            let primary_ok = self.run_action(plan.primary).await;
            if primary_ok {
                return ActivationResult::PrimaryOk(plan.primary);
            }

            if let Some(fallback) = plan.fallback {
                if self.run_action(fallback).await {
                    ActivationResult::FallbackOk {
                        primary: plan.primary,
                        fallback,
                    }
                } else if self.run_action(SecondaryAction::DefaultActivate).await {
                    ActivationResult::FallbackOk {
                        primary: plan.primary,
                        fallback: SecondaryAction::DefaultActivate,
                    }
                } else if self.run_action(SecondaryAction::MenuRootActivate).await {
                    ActivationResult::FallbackOk {
                        primary: plan.primary,
                        fallback: SecondaryAction::MenuRootActivate,
                    }
                } else {
                    ActivationResult::Failed {
                        primary: plan.primary,
                        fallback: Some(SecondaryAction::MenuRootActivate),
                    }
                }
            } else if self.run_action(SecondaryAction::DefaultActivate).await {
                ActivationResult::FallbackOk {
                    primary: plan.primary,
                    fallback: SecondaryAction::DefaultActivate,
                }
            } else if self.run_action(SecondaryAction::MenuRootActivate).await {
                ActivationResult::FallbackOk {
                    primary: plan.primary,
                    fallback: SecondaryAction::MenuRootActivate,
                }
            } else {
                ActivationResult::Failed {
                    primary: plan.primary,
                    fallback: Some(SecondaryAction::MenuRootActivate),
                }
            }
        }

        async fn run_action(&mut self, action: SecondaryAction) -> bool {
            self.attempts.push(action);
            match action {
                SecondaryAction::ContextMenu => self.context_results.pop_front().unwrap_or(false),
                SecondaryAction::SecondaryActivate => {
                    self.secondary_results.pop_front().unwrap_or(false)
                }
                SecondaryAction::DefaultActivate => {
                    self.default_results.pop_front().unwrap_or(false)
                }
                SecondaryAction::MenuRootActivate => {
                    self.menu_root_results.pop_front().unwrap_or(false)
                }
            }
        }
    }
}
