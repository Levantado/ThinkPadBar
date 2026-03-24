use iced::widget::image::Handle;
use std::collections::HashMap;
use system_tray::client::{ActivateRequest, Client, Event, UpdateEvent};
use system_tray::item::StatusNotifierItem;

#[derive(Debug, Clone)]
pub struct TrayItem {
    pub _id: String,
    pub title: Option<String>,
    pub icon_name: Option<String>,
    pub icon_handle: Option<Handle>,
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

                let _ = tx.send(TrayMessage::Initialize(atx));

                let client = match Client::new().await {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Failed to init tray client: {:?}", e);
                        loop {
                            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
                        }
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

                loop {
                    tokio::select! {
                        Ok(event) = crx.recv() => {
                            let _ = tx.send(TrayMessage::EventBatch(vec![event]));
                        }
                        Some(raw_id) = arx.recv() => {
                            let (x, y) = current_cursor_pos();
                            let (is_secondary, id) = parse_activation_channel_id(raw_id);
                            if is_secondary {
                                let context_result = activate_context_menu(&id, x, y).await;
                                if context_result.is_err() {
                                    let _ = client.activate(ActivateRequest::Secondary {
                                        address: id,
                                        x,
                                        y,
                                    }).await;
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

fn current_cursor_pos() -> (i32, i32) {
    let out = std::process::Command::new("hyprctl")
        .args(["-j", "cursorpos"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output();

    if let Ok(output) = out {
        if output.status.success() {
            if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                let x = v.get("x").and_then(|n| n.as_f64()).unwrap_or(0.0).round() as i32;
                let y = v.get("y").and_then(|n| n.as_f64()).unwrap_or(0.0).round() as i32;
                return (x, y);
            }
        }
    }
    (0, 0)
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

async fn activate_context_menu(address: &str, x: i32, y: i32) -> Result<(), zbus::Error> {
    let (destination, path) = parse_status_notifier_address(address);
    let connection = zbus::Connection::session().await?;
    let proxy =
        zbus::Proxy::new(&connection, destination, path, "org.kde.StatusNotifierItem").await?;
    let _: () = proxy.call("ContextMenu", &(x, y)).await?;
    Ok(())
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
        icon_name_candidates, parse_activation_channel_id, parse_status_notifier_address,
        themed_icon_paths,
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
}
