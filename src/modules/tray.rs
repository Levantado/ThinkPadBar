use iced::widget::image::Handle;
use std::collections::HashMap;
use system_tray::client::{Client, Event, UpdateEvent, ActivateRequest};
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
    ItemAdded(String, StatusNotifierItem),
    ItemUpdated(String, UpdateEvent),
    ItemRemoved(String),
    EventBatch(Vec<Event>),
    ActivateItem(String),
    Initialize(tokio::sync::mpsc::UnboundedSender<String>),
}

pub struct Tray {
    pub items: HashMap<String, TrayItem>,
    pub activate_tx: Option<tokio::sync::mpsc::UnboundedSender<String>>,
}

impl Tray {
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
            activate_tx: None,
        }
    }

    pub fn update(&mut self, message: TrayMessage) {
        match message {
            TrayMessage::ItemAdded(id, item) => {
                let mut icon_handle = get_icon_handle(&item);
                if icon_handle.is_none() {
                    if let Some(ref name) = item.icon_name {
                        icon_handle = find_icon(name, item.icon_theme_path.as_deref());
                    }
                }
                self.items.insert(id.clone(), TrayItem {
                    _id: id,
                    title: item.title,
                    icon_name: item.icon_name,
                    icon_handle,
                });
            }
            TrayMessage::ItemUpdated(id, event) => {
                if let Some(item) = self.items.get_mut(&id) {
                    match event {
                        UpdateEvent::Title(title) => item.title = title,
                        UpdateEvent::Icon { icon_name, icon_pixmap } => {
                            if let Some(name) = icon_name {
                                item.icon_name = Some(name.clone());
                                if item.icon_handle.is_none() {
                                    item.icon_handle = find_icon(&name, None);
                                }
                            }
                            if let Some(pixmap) = icon_pixmap {
                                item.icon_handle = pixmap_to_handle(&pixmap);
                            }
                        }
                        _ => {}
                    }
                }
            }
            TrayMessage::ItemRemoved(id) => {
                self.items.remove(&id);
            }
            TrayMessage::EventBatch(events) => {
                for event in events {
                    match event {
                        Event::Add(id, item) => self.update(TrayMessage::ItemAdded(id, *item)),
                        Event::Update(id, update) => self.update(TrayMessage::ItemUpdated(id, update)),
                        Event::Remove(id) => self.update(TrayMessage::ItemRemoved(id)),
                    }
                }
            }
            TrayMessage::ActivateItem(id) => {
                if let Some(tx) = &self.activate_tx {
                    let _ = tx.send(id);
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
            iced::stream::channel(
                100,
                |mut output| async move {
                    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
                    let (atx, mut arx) = tokio::sync::mpsc::unbounded_channel::<String>();
                    
                    let _ = tx.send(TrayMessage::Initialize(atx));
                    
                    std::thread::spawn(move || {
                        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
                        rt.block_on(async {
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
                                let initial_map = items_arc.lock().unwrap();
                                initial_map.iter().map(|(id, (item, _))| (id.clone(), item.clone())).collect()
                            };

                            for (id, item) in initial_items {
                                let _ = tx.send(TrayMessage::ItemAdded(id, item));
                            }

                            loop {
                                tokio::select! {
                                    Ok(event) = crx.recv() => {
                                        let _ = tx.send(TrayMessage::EventBatch(vec![event]));
                                    }
                                    Some(id) = arx.recv() => {
                                        // Activate item
                                        let _ = client.activate(ActivateRequest::Default {
                                            address: id,
                                            x: 0,
                                            y: 0,
                                        }).await;
                                    }
                                }
                            }
                        });
                    });

                    while let Some(msg) = rx.recv().await {
                        let _ = output.try_send(msg);
                    }
                }
            )
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
    if let Some(theme) = theme_path {
        let p = format!("{}/{}.png", theme, name);
        if std::path::Path::new(&p).exists() {
            return Some(Handle::from_path(p));
        }
        let p = format!("{}/{}.svg", theme, name);
        if std::path::Path::new(&p).exists() {
            return Some(Handle::from_path(p));
        }
    }
    
    let home = std::env::var("HOME").unwrap_or_default();
    let mut paths = vec![
        format!("{}/.local/share/icons/hicolor/scalable/apps/{}.svg", home, name),
        format!("{}/.local/share/icons/hicolor/48x48/apps/{}.png", home, name),
        format!("{}/.local/share/icons/hicolor/32x32/apps/{}.png", home, name),
        format!("{}/.icons/hicolor/scalable/apps/{}.svg", home, name),
        format!("{}/.icons/hicolor/48x48/apps/{}.png", home, name),
        format!("/usr/share/icons/hicolor/scalable/apps/{}.svg", name),
        format!("/usr/share/icons/hicolor/48x48/apps/{}.png", name),
        format!("/usr/share/icons/hicolor/32x32/apps/{}.png", name),
        format!("/usr/share/pixmaps/{}.png", name),
        format!("/usr/share/pixmaps/{}.svg", name),
        // Flatpak paths
        format!("{}/.local/share/flatpak/exports/share/icons/hicolor/scalable/apps/{}.svg", home, name),
        format!("{}/.local/share/flatpak/exports/share/icons/hicolor/48x48/apps/{}.png", home, name),
        format!("/var/lib/flatpak/exports/share/icons/hicolor/scalable/apps/{}.svg", name),
        format!("/var/lib/flatpak/exports/share/icons/hicolor/48x48/apps/{}.png", name),
    ];
    
    // Add common variations
    paths.push(format!("/usr/share/icons/hicolor/48x48/apps/org.localsend.localsend_app.png"));
    paths.push(format!("/usr/share/icons/hicolor/scalable/apps/org.localsend.localsend_app.svg"));
    
    for p in paths {
        if std::path::Path::new(&p).exists() {
            return Some(Handle::from_path(p));
        }
    }
    None
}
