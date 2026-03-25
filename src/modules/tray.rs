use iced::widget::image::Handle;
use std::collections::HashMap;
use system_tray::client::{ActivateRequest, Client, Event, UpdateEvent};
use system_tray::item::StatusNotifierItem;
use tokio::time::{timeout, Duration, Instant};
use tracing::{debug, warn};

#[derive(Debug, Clone)]
pub struct TrayItem {
    pub _id: String,
    pub title: Option<String>,
    pub icon_name: Option<String>,
    pub icon_handle: Option<Handle>,
    pub item_is_menu: bool,
    pub menu_path: Option<String>,
}

#[derive(Debug, Clone)]
pub enum TrayMessage {
    ItemAdded(String, Box<StatusNotifierItem>),
    ItemUpdated(String, UpdateEvent),
    ItemRemoved(String),
    EventBatch(Vec<Event>),
    ActivateItem(String),
    ActivateItemSecondary(String),
    Initialize(tokio::sync::mpsc::UnboundedSender<String>),
}

pub struct Tray {
    pub items: HashMap<String, TrayItem>,
    pub activate_tx: Option<tokio::sync::mpsc::UnboundedSender<String>>,
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
                    let _ = tx.send(id);
                }
            }
            TrayMessage::ActivateItemSecondary(id) => {
                if let Some(tx) = &self.activate_tx {
                    let _ = tx.send(format!("secondary:{id}"));
                }
            }
            TrayMessage::Initialize(tx) => {
                self.activate_tx = Some(tx);
            }
        }
    }

    pub fn subscription() -> iced::Subscription<TrayMessage> {
        struct TrayListener;

        iced::Subscription::run_with_id(
            std::any::TypeId::of::<TrayListener>(),
            iced::stream::channel(100, |mut output| async move {
                let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
                let (atx, mut arx) = tokio::sync::mpsc::unbounded_channel::<String>();
                let mut retry_delay = std::time::Duration::from_secs(1);
                let mut context_connection: Option<zbus::Connection> = None;
                let mut last_cursor_pos = (0, 0);

                let _ = tx.send(TrayMessage::Initialize(atx));

                loop {
                    let client = match Client::new().await {
                        Ok(c) => {
                            retry_delay = std::time::Duration::from_secs(1);
                            c
                        }
                        Err(e) => {
                            eprintln!("Failed to init tray client: {:?}", e);
                            tokio::time::sleep(retry_delay).await;
                            retry_delay = (retry_delay * 2).min(std::time::Duration::from_secs(30));
                            continue;
                        }
                    };

                    let mut crx = client.subscribe();

                    let initial_items: Vec<_> = {
                        let items_arc = client.items();
                        let lock_result = items_arc.lock();
                        if let Ok(initial_map) = lock_result {
                            initial_map
                                .iter()
                                .map(|(id, (item, _))| (id.clone(), item.clone()))
                                .collect()
                        } else {
                            Vec::new()
                        }
                    };

                    for (id, item) in initial_items {
                        let _ = tx.send(TrayMessage::ItemAdded(id, Box::new(item)));
                    }

                    let mut reconnect = false;
                    while !reconnect {
                        tokio::select! {
                            event = crx.recv() => {
                                match event {
                                    Ok(event) => {
                                        let _ = tx.send(TrayMessage::EventBatch(vec![event]));
                                    }
                                    Err(_) => {
                                        reconnect = true;
                                    }
                                }
                            }
                            Some(raw_id) = arx.recv() => {
                                let (x, y) = current_cursor_pos_with_fallback(last_cursor_pos);
                                last_cursor_pos = (x, y);
                                let (is_secondary, id) = parse_activation_channel_id(raw_id);
                                if is_secondary {
                                    let (item_is_menu, has_menu_path) =
                                        get_secondary_capabilities(&client, &id);
                                    let route = choose_secondary_route(item_is_menu, has_menu_path);
                                    let started_at = Instant::now();
                                    let result = activate_secondary_with_strategy(
                                        &client,
                                        &mut context_connection,
                                        &id,
                                        x,
                                        y,
                                        route,
                                    )
                                    .await;
                                    debug!(
                                        "tray secondary click id={} route={:?} menu_only={} has_menu={} cursor=({}, {}) result={} elapsed_ms={}",
                                        id,
                                        route,
                                        item_is_menu,
                                        has_menu_path,
                                        x,
                                        y,
                                        result,
                                        started_at.elapsed().as_millis()
                                    );
                                    if !result.succeeded() {
                                        warn!("tray secondary activation failed for {}", id);
                                    }
                                } else {
                                    let _ = client.activate(ActivateRequest::Default {
                                        address: id,
                                        x,
                                        y,
                                    }).await;
                                }
                            }
                            Some(msg) = rx.recv() => {
                                let _ = output.try_send(msg);
                            }
                        }
                    }
                }
            }),
        )
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

fn current_cursor_pos_with_fallback(last_known: (i32, i32)) -> (i32, i32) {
    crate::modules::workspaces::hyprland_command("j/cursorpos")
        .and_then(|raw| parse_cursor_pos(&raw))
        .unwrap_or(last_known)
}

fn parse_activation_channel_id(raw: String) -> (bool, String) {
    if let Some(address) = raw.strip_prefix("secondary:") {
        (true, address.to_string())
    } else {
        (false, raw)
    }
}

fn parse_status_notifier_address(address: &str) -> (&str, String) {
    address
        .split_once('/')
        .map_or((address, String::from("/StatusNotifierItem")), |(d, p)| {
            (d, format!("/{p}"))
        })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SecondaryRoute {
    ContextMenuFirst,
    SecondaryFirst,
}

fn choose_secondary_route(item_is_menu: bool, has_menu_path: bool) -> SecondaryRoute {
    if item_is_menu || has_menu_path {
        SecondaryRoute::ContextMenuFirst
    } else {
        SecondaryRoute::SecondaryFirst
    }
}

fn get_secondary_capabilities(client: &Client, id: &str) -> (bool, bool) {
    let items_arc = client.items();
    if let Ok(items) = items_arc.lock() {
        if let Some((item, _)) = items.get(id) {
            return (item.item_is_menu, item.menu.is_some());
        }
    }
    (false, false)
}

#[derive(Debug, Clone)]
enum ActivationResult {
    PrimaryOk(&'static str),
    FallbackOk {
        primary: &'static str,
        fallback: &'static str,
    },
    Failed {
        primary: &'static str,
        fallback: Option<&'static str>,
    },
}

impl ActivationResult {
    fn succeeded(&self) -> bool {
        matches!(
            self,
            ActivationResult::PrimaryOk(_) | ActivationResult::FallbackOk { .. }
        )
    }
}

impl std::fmt::Display for ActivationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActivationResult::PrimaryOk(method) => write!(f, "primary_ok:{}", method),
            ActivationResult::FallbackOk { primary, fallback } => {
                write!(f, "fallback_ok:{}->{}", primary, fallback)
            }
            ActivationResult::Failed { primary, fallback } => {
                if let Some(fallback) = fallback {
                    write!(f, "failed:{}->{}", primary, fallback)
                } else {
                    write!(f, "failed:{}", primary)
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

async fn activate_secondary_with_strategy(
    client: &Client,
    connection: &mut Option<zbus::Connection>,
    id: &str,
    x: i32,
    y: i32,
    route: SecondaryRoute,
) -> ActivationResult {
    match route {
        SecondaryRoute::ContextMenuFirst => {
            if try_context_menu(connection, id, x, y).await {
                ActivationResult::PrimaryOk("context_menu")
            } else if try_secondary_activate(client, id, x, y).await {
                ActivationResult::FallbackOk {
                    primary: "context_menu",
                    fallback: "secondary_activate",
                }
            } else {
                ActivationResult::Failed {
                    primary: "context_menu",
                    fallback: Some("secondary_activate"),
                }
            }
        }
        SecondaryRoute::SecondaryFirst => {
            if try_secondary_activate(client, id, x, y).await {
                ActivationResult::PrimaryOk("secondary_activate")
            } else if try_context_menu(connection, id, x, y).await {
                ActivationResult::FallbackOk {
                    primary: "secondary_activate",
                    fallback: "context_menu",
                }
            } else {
                ActivationResult::Failed {
                    primary: "secondary_activate",
                    fallback: Some("context_menu"),
                }
            }
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
        choose_secondary_route, icon_name_candidates, parse_activation_channel_id,
        parse_cursor_pos, parse_status_notifier_address, themed_icon_paths, SecondaryRoute,
    };

    #[test]
    fn icon_name_candidates_include_base_name_without_extension() {
        let c = icon_name_candidates("sample-icon.svg");
        assert!(c.iter().any(|v| v == "sample-icon.svg"));
        assert!(c.iter().any(|v| v == "sample-icon"));
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
    fn parse_activation_channel_id_detects_secondary_prefix() {
        let (secondary, id) = parse_activation_channel_id("secondary:org.test.Item".to_string());
        assert!(secondary);
        assert_eq!(id, "org.test.Item");
    }

    #[test]
    fn parse_activation_channel_id_keeps_default_id() {
        let (secondary, id) = parse_activation_channel_id("org.test.Item".to_string());
        assert!(!secondary);
        assert_eq!(id, "org.test.Item");
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
    fn parse_cursor_pos_extracts_coordinates() {
        let parsed = parse_cursor_pos(r#"{"x": 101.2, "y": 202.8}"#);
        assert_eq!(parsed, Some((101, 203)));
    }

    #[test]
    fn choose_secondary_route_prefers_context_for_menu_items() {
        assert_eq!(
            choose_secondary_route(true, false),
            SecondaryRoute::ContextMenuFirst
        );
        assert_eq!(
            choose_secondary_route(false, true),
            SecondaryRoute::ContextMenuFirst
        );
    }

    #[test]
    fn choose_secondary_route_prefers_secondary_for_non_menu_items() {
        assert_eq!(
            choose_secondary_route(false, false),
            SecondaryRoute::SecondaryFirst
        );
    }
}
