use iced::Task;

use crate::{
    app::{Message, PowerAction},
    services::{
        compositor::CompositorService,
        network::{NetworkEvent, NetworkFollowUp, NetworkService},
        session::{SessionCommand, SessionFollowUp, SessionService},
        tray_ui::{TrayUiSecondaryAction, TrayUiSelectionAction},
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkRuntimePlan {
    None,
    Scan,
    Connect {
        ssid: String,
        passphrase: Option<String>,
    },
    ToggleWifi(bool),
}

impl From<NetworkFollowUp> for NetworkRuntimePlan {
    fn from(value: NetworkFollowUp) -> Self {
        match value {
            NetworkFollowUp::None => Self::None,
            NetworkFollowUp::Scan => Self::Scan,
            NetworkFollowUp::Connect { ssid, passphrase } => Self::Connect { ssid, passphrase },
            NetworkFollowUp::TogglePower(enable) => Self::ToggleWifi(enable),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayPopupPlan {
    None,
    OpenMenu,
    CloseMenu,
}

pub fn tray_popup_plan_from_secondary(action: TrayUiSecondaryAction) -> TrayPopupPlan {
    match action {
        TrayUiSecondaryAction::OpenMenu => TrayPopupPlan::OpenMenu,
        TrayUiSecondaryAction::CloseMenu => TrayPopupPlan::CloseMenu,
        TrayUiSecondaryAction::ActivateSecondary => TrayPopupPlan::None,
    }
}

pub fn tray_popup_plan_from_selection(action: TrayUiSelectionAction) -> TrayPopupPlan {
    match action {
        TrayUiSelectionAction::CloseMenu | TrayUiSelectionAction::ActivateMenuItem(_) => {
            TrayPopupPlan::CloseMenu
        }
        TrayUiSelectionAction::NavigateMenu => TrayPopupPlan::None,
    }
}

pub fn session_command_from_power_action(action: PowerAction) -> SessionCommand {
    match action {
        PowerAction::Lock => SessionCommand::Lock,
        PowerAction::Sleep => SessionCommand::Sleep,
        PowerAction::Hibernate => SessionCommand::Hibernate,
        PowerAction::Restart => SessionCommand::Restart,
        PowerAction::Shutdown => SessionCommand::Shutdown,
        PowerAction::Logout => SessionCommand::Logout,
    }
}

pub fn session_follow_up_requests_refresh(follow_up: SessionFollowUp) -> bool {
    matches!(follow_up, SessionFollowUp::RefreshCompositor)
}

pub fn perform_session_command(
    session_service: SessionService,
    command: SessionCommand,
) -> Task<Message> {
    Task::perform(
        async move { session_service.execute(command).await },
        Message::SessionCommandCompleted,
    )
}

pub fn perform_tray_candidate_resolution(
    compositor_service: CompositorService,
    candidates: Vec<String>,
    id: String,
) -> Task<Message> {
    Task::perform(
        async move {
            for candidate in candidates {
                if compositor_service.find_and_switch_to_app(candidate).await {
                    return true;
                }
            }
            false
        },
        move |found| Message::TrayItemClickResolved(id.clone(), found),
    )
}

pub fn perform_network_plan(
    network_service: NetworkService,
    conn: Option<zbus::Connection>,
    plan: NetworkRuntimePlan,
) -> Task<Message> {
    let Some(conn) = conn else {
        return Task::none();
    };

    match plan {
        NetworkRuntimePlan::None => Task::none(),
        NetworkRuntimePlan::Scan => Task::perform(
            async move { network_service.scan_networks(&conn).await },
            |networks| Message::NetworkEvent(NetworkEvent::ScanCompleted(networks)),
        ),
        NetworkRuntimePlan::Connect { ssid, passphrase } => Task::perform(
            async move {
                let success = network_service
                    .connect_network(&conn, ssid.clone(), passphrase)
                    .await;
                NetworkEvent::ConnectCompleted { ssid, success }
            },
            Message::NetworkEvent,
        ),
        NetworkRuntimePlan::ToggleWifi(enable) => Task::perform(
            async move {
                network_service.toggle_wifi(&conn, enable).await;
                network_service.get_wifi_info(&conn).await
            },
            |info| Message::NetworkEvent(NetworkEvent::WifiInfoSynced(info)),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        session_command_from_power_action, session_follow_up_requests_refresh,
        tray_popup_plan_from_secondary, tray_popup_plan_from_selection, NetworkRuntimePlan,
        TrayPopupPlan,
    };
    use crate::{
        app::PowerAction,
        services::{
            network::NetworkFollowUp,
            session::{SessionCommand, SessionFollowUp},
            tray_ui::{TrayUiSecondaryAction, TrayUiSelectionAction},
        },
    };

    #[test]
    fn network_runtime_plan_maps_follow_ups_stably() {
        assert_eq!(
            NetworkRuntimePlan::from(NetworkFollowUp::None),
            NetworkRuntimePlan::None
        );
        assert_eq!(
            NetworkRuntimePlan::from(NetworkFollowUp::Scan),
            NetworkRuntimePlan::Scan
        );
        assert_eq!(
            NetworkRuntimePlan::from(NetworkFollowUp::Connect {
                ssid: "Home".to_string(),
                passphrase: Some("secret".to_string()),
            }),
            NetworkRuntimePlan::Connect {
                ssid: "Home".to_string(),
                passphrase: Some("secret".to_string()),
            }
        );
        assert_eq!(
            NetworkRuntimePlan::from(NetworkFollowUp::TogglePower(true)),
            NetworkRuntimePlan::ToggleWifi(true)
        );
    }

    #[test]
    fn tray_popup_plans_keep_secondary_and_selection_policy_centralized() {
        assert_eq!(
            tray_popup_plan_from_secondary(TrayUiSecondaryAction::OpenMenu),
            TrayPopupPlan::OpenMenu
        );
        assert_eq!(
            tray_popup_plan_from_secondary(TrayUiSecondaryAction::CloseMenu),
            TrayPopupPlan::CloseMenu
        );
        assert_eq!(
            tray_popup_plan_from_secondary(TrayUiSecondaryAction::ActivateSecondary),
            TrayPopupPlan::None
        );
        assert_eq!(
            tray_popup_plan_from_selection(TrayUiSelectionAction::CloseMenu),
            TrayPopupPlan::CloseMenu
        );
    }

    #[test]
    fn session_helpers_keep_power_action_mapping_explicit() {
        assert_eq!(
            session_command_from_power_action(PowerAction::Shutdown),
            SessionCommand::Shutdown
        );
        assert!(session_follow_up_requests_refresh(
            SessionFollowUp::RefreshCompositor
        ));
        assert!(!session_follow_up_requests_refresh(SessionFollowUp::None));
    }
}
