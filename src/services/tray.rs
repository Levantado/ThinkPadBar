use crate::services::tray_model::{
    activate_secondary_with_plan, choose_secondary_plan, current_cursor_pos_with_fallback,
    destination_from_item_address, ensure_context_connection, get_secondary_capabilities,
    resolve_item_address, update_secondary_preference, SecondaryAction, TrayCommand, TrayMessage,
    TrayRuntimeUpdate,
};
use std::collections::HashMap;
use system_tray::client::{Client, Event};
use tokio::time::Instant;
use tracing::{debug, warn};

fn tray_menu_destination(address: &str) -> Option<zbus::names::BusName<'_>> {
    zbus::names::BusName::try_from(destination_from_item_address(address)).ok()
}

fn reset_runtime_activation_state(
    context_connection: &mut Option<zbus::Connection>,
    resolved_item_addresses: &mut HashMap<String, String>,
    preferred_secondary_actions: &mut HashMap<String, SecondaryAction>,
) {
    *context_connection = None;
    resolved_item_addresses.clear();
    preferred_secondary_actions.clear();
}

pub fn subscription() -> iced::Subscription<TrayMessage> {
    struct TrayListener;

    iced::Subscription::run_with_id(
        std::any::TypeId::of::<TrayListener>(),
        iced::stream::channel(100, |mut output| async move {
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            let (atx, mut arx) = tokio::sync::mpsc::unbounded_channel::<TrayCommand>();
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
                        reset_runtime_activation_state(
                            &mut context_connection,
                            &mut resolved_item_addresses,
                            &mut preferred_secondary_actions,
                        );
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
                        Some(command) = arx.recv() => {
                            let (x, y) = current_cursor_pos_with_fallback(last_cursor_pos);
                            last_cursor_pos = (x, y);
                            match command {
                                TrayCommand::Default(id) => {
                                    let resolved_address = resolve_item_address(
                                        &mut context_connection,
                                        &mut resolved_item_addresses,
                                        &id,
                                    )
                                    .await;
                                    let _ = client.activate(system_tray::client::ActivateRequest::Default {
                                        address: resolved_address,
                                        x,
                                        y,
                                    }).await;
                                }
                                TrayCommand::Secondary(id) => {
                                    let resolved_address = resolve_item_address(
                                        &mut context_connection,
                                        &mut resolved_item_addresses,
                                        &id,
                                    )
                                    .await;
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
                                    let route = format!(
                                        "{}->{}",
                                        plan.primary.as_str(),
                                        plan.fallback
                                            .map(SecondaryAction::as_str)
                                            .unwrap_or("none")
                                    );
                                    let failure = (!result.succeeded()).then(|| {
                                        format!("{} resolved={} {}", id, resolved_address, result)
                                    });
                                    let _ = tx.send(TrayMessage::RuntimeUpdate(
                                        TrayRuntimeUpdate::SecondaryObserved {
                                            route,
                                            result: result.to_string(),
                                            failure,
                                        },
                                    ));
                                    if !result.succeeded() {
                                        warn!("tray secondary activation failed for {}", id);
                                    }
                                }
                                TrayCommand::MenuItem {
                                    id,
                                    menu_item_id,
                                } => {
                                    let resolved_address = resolve_item_address(
                                        &mut context_connection,
                                        &mut resolved_item_addresses,
                                        &id,
                                    )
                                    .await;
                                    let conn = match ensure_context_connection(&mut context_connection).await {
                                        Ok(conn) => conn,
                                        Err(err) => {
                                            let _ = tx.send(TrayMessage::RuntimeUpdate(
                                                TrayRuntimeUpdate::MenuActivationError(Some(
                                                    format!(
                                                        "{} item={} session-bus-connect {}",
                                                        id, menu_item_id, err
                                                    ),
                                                )),
                                            ));
                                            continue;
                                        }
                                    };
                                    let (_, _, menu_path) = get_secondary_capabilities(&client, &id);
                                    if let Some(menu_path) = menu_path {
                                        let dest = tray_menu_destination(&resolved_address);
                                        let path = zbus::zvariant::ObjectPath::try_from(menu_path);
                                        let interface = zbus::names::InterfaceName::from_static_str_unchecked(
                                            "com.canonical.dbusmenu",
                                        );

                                        if let (Some(dest), Ok(path)) = (dest, path) {
                                            match zbus::Proxy::new(conn, dest, path, interface).await {
                                                Ok(proxy) => {
                                                    let timestamp = chrono::offset::Local::now()
                                                        .timestamp_subsec_micros();

                                                    debug!(
                                                        "tray menu item direct event id={} item={} timestamp={}",
                                                        id, menu_item_id, timestamp
                                                    );

                                                    let value = zbus::zvariant::Value::I32(32)
                                                        .try_to_owned()
                                                        .unwrap_or_else(|_| {
                                                            zbus::zvariant::OwnedValue::from(32)
                                                        });

                                                    if let Err(err) = proxy
                                                        .call::<_, _, ()>(
                                                            "Event",
                                                            &(
                                                                menu_item_id,
                                                                "clicked",
                                                                &value,
                                                                timestamp,
                                                            ),
                                                        )
                                                        .await
                                                    {
                                                        let _ = tx.send(TrayMessage::RuntimeUpdate(
                                                            TrayRuntimeUpdate::MenuActivationError(
                                                                Some(format!(
                                                                    "{} item={} {}",
                                                                    id, menu_item_id, err
                                                                )),
                                                            ),
                                                        ));
                                                        warn!(
                                                            "tray menu item direct event failed for {} item={} err={}",
                                                            id, menu_item_id, err
                                                        );
                                                    } else {
                                                        let _ = proxy
                                                            .call::<_, _, zbus::zvariant::OwnedValue>(
                                                                "GetLayout",
                                                                &(0, -1, &[] as &[&str]),
                                                            )
                                                            .await;

                                                        let _ = tx.send(TrayMessage::RuntimeUpdate(
                                                            TrayRuntimeUpdate::MenuActivationError(
                                                                None,
                                                            ),
                                                        ));
                                                    }
                                                }
                                                Err(err) => {
                                                    let _ = tx.send(TrayMessage::RuntimeUpdate(
                                                        TrayRuntimeUpdate::MenuActivationError(
                                                            Some(format!(
                                                                "{} item={} proxy-create {}",
                                                                id, menu_item_id, err
                                                            )),
                                                        ),
                                                    ));
                                                }
                                            }
                                        } else {
                                            let _ = tx.send(TrayMessage::RuntimeUpdate(
                                                TrayRuntimeUpdate::MenuActivationError(
                                                    Some(format!(
                                                        "{} item={} invalid-destination-or-menu-path",
                                                        id, menu_item_id
                                                    )),
                                                ),
                                            ));
                                        }
                                    } else {
                                        let _ = tx.send(TrayMessage::RuntimeUpdate(
                                            TrayRuntimeUpdate::MenuActivationError(Some(
                                                format!(
                                                    "{} item={} missing-menu-path",
                                                    id, menu_item_id
                                                ),
                                            )),
                                        ));
                                    }
                                }
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

#[cfg(test)]
mod tests {
    use super::{reset_runtime_activation_state, tray_menu_destination, SecondaryAction};
    use std::collections::HashMap;

    #[test]
    fn reconnect_state_reset_clears_cached_routes_and_addresses() {
        let mut context_connection = None;
        let mut resolved_item_addresses = HashMap::from([(
            ":1.42".to_string(),
            ":1.42/org/ayatana/NotificationItem/X".to_string(),
        )]);
        let mut preferred_secondary_actions =
            HashMap::from([(":1.42".to_string(), SecondaryAction::ContextMenu)]);

        reset_runtime_activation_state(
            &mut context_connection,
            &mut resolved_item_addresses,
            &mut preferred_secondary_actions,
        );

        assert!(context_connection.is_none());
        assert!(resolved_item_addresses.is_empty());
        assert!(preferred_secondary_actions.is_empty());
    }

    #[test]
    fn tray_menu_destination_strips_item_object_path() {
        let dest = tray_menu_destination(":1.42/org/ayatana/NotificationItem/app")
            .expect("expected valid bus name");

        assert_eq!(dest.as_str(), ":1.42");
    }
}
