use crate::modules::tray::{
    activate_secondary_with_plan, choose_secondary_plan, current_cursor_pos_with_fallback,
    get_secondary_capabilities, parse_activation_channel_id, resolve_item_address,
    update_secondary_preference, SecondaryAction, TrayMessage,
};
use std::collections::HashMap;
use system_tray::client::{ActivateRequest, Client, Event};
use tokio::time::Instant;
use tracing::{debug, warn};

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
            let mut preferred_secondary_actions: HashMap<String, SecondaryAction> = HashMap::new();
            let mut resolved_item_addresses: HashMap<String, String> = HashMap::new();

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
                                    if let Event::Remove(id) = &event {
                                        resolved_item_addresses.remove(id);
                                        preferred_secondary_actions.remove(id);
                                    }
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
                            let resolved_address = resolve_item_address(
                                &mut context_connection,
                                &mut resolved_item_addresses,
                                &id,
                            )
                            .await;
                            if is_secondary {
                                let (item_is_menu, has_menu_path, menu_path) =
                                    get_secondary_capabilities(&client, &id);
                                let preferred_action =
                                    preferred_secondary_actions.get(&id).copied();
                                let plan = choose_secondary_plan(
                                    item_is_menu,
                                    has_menu_path,
                                    preferred_action,
                                );
                                let started_at = Instant::now();
                                let result = activate_secondary_with_plan(
                                    &client,
                                    &mut context_connection,
                                    &resolved_address,
                                    menu_path.as_deref(),
                                    x,
                                    y,
                                    plan,
                                )
                                .await;
                                update_secondary_preference(
                                    &mut preferred_secondary_actions,
                                    &id,
                                    &result,
                                );
                                debug!(
                                    "tray secondary click id={} resolved={} route_primary={} route_fallback={} preferred={} menu_only={} has_menu={} cursor=({}, {}) result={} elapsed_ms={}",
                                    id,
                                    resolved_address,
                                    plan.primary.as_str(),
                                    plan.fallback.map(SecondaryAction::as_str).unwrap_or("none"),
                                    preferred_action.map(SecondaryAction::as_str).unwrap_or("none"),
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
                                    address: resolved_address,
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
