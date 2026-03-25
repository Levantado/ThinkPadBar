use iced::widget::image::Handle;
use std::collections::HashMap;
use system_tray::client::{ActivateRequest, Client, Event, UpdateEvent};
use system_tray::item::StatusNotifierItem;
pub use system_tray::menu::{MenuItem, MenuType, TrayMenu};
use tokio::time::{timeout, Duration};
use tracing::debug;

#[derive(Debug, Clone)]
pub struct TrayItem {
    pub _id: String,
    pub title: Option<String>,
    pub icon_name: Option<String>,
    pub icon_handle: Option<Handle>,
    pub item_is_menu: bool,
    pub menu_path: Option<String>,
    pub menu_layout: Option<TrayMenu>,
}

#[derive(Debug, Clone)]
pub enum TrayMessage {
    ItemAdded(String, Box<StatusNotifierItem>),
    ItemUpdated(String, UpdateEvent),
    ItemRemoved(String),
    EventBatch(Vec<Event>),
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
    icon_cache: HashMap<String, Handle>,
}

impl Tray {
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
            activate_tx: None,
            icon_cache: HashMap::new(),
        }
    }

    fn resolve_icon_cached(&mut self, name: &str, theme_path: Option<&str>) -> Option<Handle> {
        for candidate in icon_name_candidates(name) {
            if let Some(icon) = self.icon_cache.get(&candidate) {
                return Some(icon.clone());
            }
            if let Some(found) = find_icon(&candidate, theme_path) {
                self.icon_cache.insert(candidate, found.clone());
                return Some(found);
            }
        }
        None
    }

    pub fn update(&mut self, message: TrayMessage) {
        match message {
            TrayMessage::ItemAdded(id, item) => {
                let mut icon_handle = get_icon_handle(&item);
                if icon_handle.is_none() {
                    if let Some(ref name) = item.icon_name {
                        icon_handle =
                            self.resolve_icon_cached(name, item.icon_theme_path.as_deref());
                    }
                }
                self.items.insert(
                    id.clone(),
                    TrayItem {
                        _id: id,
                        title: item.title.clone(),
                        icon_name: item.icon_name.clone(),
                        icon_handle,
                        item_is_menu: item.item_is_menu,
                        menu_path: item.menu.clone(),
                        menu_layout: None,
                    },
                );
            }
            TrayMessage::ItemUpdated(id, event) => {
                let mut cache_lookup_name: Option<String> = None;
                if let Some(item) = self.items.get_mut(&id) {
                    match event {
                        UpdateEvent::Title(title) => item.title = title,
                        UpdateEvent::Icon {
                            icon_name,
                            icon_pixmap,
                        } => {
                            if let Some(name) = icon_name {
                                item.icon_name = Some(name.clone());
                                if item.icon_handle.is_none() {
                                    cache_lookup_name = Some(name);
                                }
                            }
                            if let Some(pixmap) = icon_pixmap {
                                item.icon_handle = pixmap_to_handle(&pixmap);
                            }
                        }
                        UpdateEvent::MenuConnect(path) => {
                            item.menu_path = Some(path);
                        }
                        UpdateEvent::Menu(layout) => {
                            item.menu_layout = Some(layout);
                        }
                        _ => {}
                    }
                }
                if let Some(name) = cache_lookup_name {
                    let resolved = self.resolve_icon_cached(&name, None);
                    if let Some(item) = self.items.get_mut(&id) {
                        if item.icon_handle.is_none() {
                            item.icon_handle = resolved;
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

    pub fn menu_for(&self, id: &str) -> Option<&TrayMenu> {
        self.items
            .get(id)
            .and_then(|item| item.menu_layout.as_ref())
    }

    pub fn has_menu_entries(&self, id: &str) -> bool {
        self.menu_for(id)
            .is_some_and(|menu| menu.submenus.iter().any(|item| item.visible))
    }
}

fn get_icon_handle(item: &StatusNotifierItem) -> Option<Handle> {
    if let Some(pixmaps) = &item.icon_pixmap {
        pixmap_to_handle(pixmaps)
    } else {
        None
    }
}

fn pixmap_to_handle(pixmaps: &[system_tray::item::IconPixmap]) -> Option<Handle> {
    // Find the best pixmap size (closest to 24x24) or just the first one
    let target_size = 24;
    let best = pixmaps.iter().min_by_key(|p| (p.width - target_size).abs());

    if let Some(best) = best {
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

fn find_icon(name: &str, theme_path: Option<&str>) -> Option<Handle> {
    if std::path::Path::new(name).exists() {
        return Some(Handle::from_path(name.to_string()));
    }

    if let Some(theme) = theme_path {
        for p in themed_icon_paths(theme, name) {
            if std::path::Path::new(&p).exists() {
                return Some(Handle::from_path(p));
            }
        }
    }

    let home = std::env::var("HOME").unwrap_or_default();
    let paths = vec![
        format!(
            "{}/.local/share/icons/hicolor/scalable/apps/{}.svg",
            home, name
        ),
        format!(
            "{}/.local/share/icons/hicolor/48x48/apps/{}.png",
            home, name
        ),
        format!(
            "{}/.local/share/icons/hicolor/32x32/apps/{}.png",
            home, name
        ),
        format!("{}/.icons/hicolor/scalable/apps/{}.svg", home, name),
        format!("{}/.icons/hicolor/48x48/apps/{}.png", home, name),
        format!("/usr/share/icons/hicolor/scalable/apps/{}.svg", name),
        format!("/usr/share/icons/hicolor/48x48/apps/{}.png", name),
        format!("/usr/share/icons/hicolor/32x32/apps/{}.png", name),
        format!("/usr/share/pixmaps/{}.png", name),
        format!("/usr/share/pixmaps/{}.svg", name),
        // Flatpak paths
        format!(
            "{}/.local/share/flatpak/exports/share/icons/hicolor/scalable/apps/{}.svg",
            home, name
        ),
        format!(
            "{}/.local/share/flatpak/exports/share/icons/hicolor/48x48/apps/{}.png",
            home, name
        ),
        format!(
            "/var/lib/flatpak/exports/share/icons/hicolor/scalable/apps/{}.svg",
            name
        ),
        format!(
            "/var/lib/flatpak/exports/share/icons/hicolor/48x48/apps/{}.png",
            name
        ),
    ];

    for p in paths {
        if std::path::Path::new(&p).exists() {
            return Some(Handle::from_path(p));
        }
    }
    None
}

fn parse_cursor_pos(raw: &str) -> Option<(i32, i32)> {
    let value = serde_json::from_str::<serde_json::Value>(raw).ok()?;
    let x = value.get("x")?.as_f64()?.round() as i32;
    let y = value.get("y")?.as_f64()?.round() as i32;
    Some((x, y))
}

pub(crate) fn current_cursor_pos_with_fallback(last_known: (i32, i32)) -> (i32, i32) {
    crate::modules::workspaces::hyprland_command("j/cursorpos")
        .and_then(|raw| parse_cursor_pos(&raw))
        .unwrap_or(last_known)
}

fn parse_status_notifier_address(address: &str) -> (&str, String) {
    address
        .split_once('/')
        .map_or((address, String::from("/StatusNotifierItem")), |(d, p)| {
            (d, format!("/{p}"))
        })
}

fn destination_from_item_address(address: &str) -> &str {
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

async fn ensure_context_connection(
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

fn icon_name_candidates(raw: &str) -> Vec<String> {
    let mut out = Vec::new();
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return out;
    }

    out.push(trimmed.to_string());

    let no_prefix = trimmed
        .strip_prefix("file://")
        .unwrap_or(trimmed)
        .trim_matches('"');
    if no_prefix != trimmed {
        out.push(no_prefix.to_string());
    }

    let file_name = std::path::Path::new(no_prefix)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(no_prefix);
    if !file_name.is_empty() && file_name != no_prefix {
        out.push(file_name.to_string());
    }

    let base = file_name
        .strip_suffix(".svg")
        .or_else(|| file_name.strip_suffix(".png"))
        .or_else(|| file_name.strip_suffix(".xpm"))
        .unwrap_or(file_name);
    if !base.is_empty() && base != file_name {
        out.push(base.to_string());
    }
    let mut no_symbolic = base;
    if let Some(stripped) = base.strip_suffix("-symbolic") {
        if !stripped.is_empty() {
            out.push(stripped.to_string());
        }
        no_symbolic = stripped;
    }
    if let Some(stripped) = base.strip_suffix("-panel") {
        if !stripped.is_empty() {
            out.push(stripped.to_string());
        }
    }
    if let Some(stripped) = no_symbolic.strip_suffix("-panel") {
        if !stripped.is_empty() {
            out.push(stripped.to_string());
        }
    }

    out.sort();
    out.dedup();
    out
}

fn themed_icon_paths(theme_root: &str, name: &str) -> Vec<String> {
    let mut paths = Vec::new();
    let sizes = [
        "16x16", "22x22", "24x24", "32x32", "48x48", "64x64", "128x128", "256x256",
    ];
    let contexts = ["apps", "panel", "status"];
    let exts = ["png", "svg", "xpm"];

    for ext in exts {
        paths.push(format!("{}/{}.{}", theme_root, name, ext));
        for size in sizes {
            for ctx in contexts {
                paths.push(format!("{}/{}/{}/{}.{}", theme_root, size, ctx, name, ext));
            }
        }
    }
    paths
}

#[cfg(test)]
mod tests {
    use super::{
        choose_secondary_plan, destination_from_item_address, icon_name_candidates,
        parse_cursor_pos, parse_status_notifier_address, select_registered_item_address,
        themed_icon_paths, update_secondary_preference, ActivationResult, SecondaryAction,
        SecondaryPlan, Tray, TrayItem, TrayMenu,
    };
    use std::collections::HashMap;

    #[test]
    fn icon_name_candidates_include_base_name_without_extension() {
        let c = icon_name_candidates("sample-icon.svg");
        assert!(c.iter().any(|v| v == "sample-icon.svg"));
        assert!(c.iter().any(|v| v == "sample-icon"));
    }

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
                item_is_menu: false,
                menu_path: Some("/menu".to_string()),
                menu_layout: Some(TrayMenu {
                    id: 1,
                    submenus: vec![crate::modules::tray::MenuItem {
                        id: 42,
                        label: Some("Open".to_string()),
                        visible: true,
                        enabled: true,
                        ..Default::default()
                    }],
                }),
            },
        );
        assert!(tray.has_menu_entries("item"));
    }

    #[test]
    fn icon_name_candidates_handle_file_url_and_path() {
        let c = icon_name_candidates("file:///usr/share/icons/hicolor/scalable/apps/foo-bar.svg");
        assert!(c
            .iter()
            .any(|v| v == "/usr/share/icons/hicolor/scalable/apps/foo-bar.svg"));
        assert!(c.iter().any(|v| v == "foo-bar.svg"));
        assert!(c.iter().any(|v| v == "foo-bar"));
    }

    #[test]
    fn icon_name_candidates_strip_common_tray_suffixes() {
        let c = icon_name_candidates("sample-panel-symbolic");
        assert!(c.iter().any(|v| v == "sample-panel-symbolic"));
        assert!(c.iter().any(|v| v == "sample-panel"));
        assert!(c.iter().any(|v| v == "sample"));
    }

    #[test]
    fn themed_icon_paths_cover_panel_locations() {
        let p = themed_icon_paths("/usr/share/icons/Papirus-Dark", "sample-panel");
        assert!(p
            .iter()
            .any(|v| v == "/usr/share/icons/Papirus-Dark/22x22/panel/sample-panel.svg"));
        assert!(p
            .iter()
            .any(|v| v == "/usr/share/icons/Papirus-Dark/24x24/status/sample-panel.png"));
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
