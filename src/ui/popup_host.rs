use iced::{
    widget::{container, Space},
    Element, Length, Theme,
};

use crate::{
    app::{Message, Popup},
    ui::{popups, theme::ThemeTokens},
};

#[derive(Debug, Clone)]
pub enum PopupHostModel {
    Hidden,
    TrayMenu(popups::tray_menu::TrayMenuPopupModel),
    Stats(popups::stats::StatsPopupModel),
    Displays(popups::displays::DisplaysPopupModel),
    AudioRoutes(popups::audio_routes::AudioRoutesPopupModel),
    BluetoothDevices(popups::bluetooth_devices::BluetoothDevicesPopupModel),
    Calendar(popups::calendar::CalendarPopupModel),
    SystemMonitor(popups::system_info::SystemInfoPopupModel),
    Power(popups::power::PowerPopupModel),
    Connectivity(popups::connectivity::ConnectivityPopupModel),
    Controls(popups::controls::ControlsPopupModel),
    Media(popups::media::MediaPopupModel),
}

pub fn view(
    theme: ThemeTokens,
    opacity: f32,
    model: PopupHostModel,
) -> Element<'static, Message, Theme, iced::Renderer> {
    match model {
        PopupHostModel::Hidden => empty_popup(),
        PopupHostModel::TrayMenu(model) => popups::tray_menu::view(opacity, model),
        PopupHostModel::Stats(model) => popups::stats::view(theme, model),
        PopupHostModel::Displays(model) => popups::displays::view(theme, opacity, model),
        PopupHostModel::AudioRoutes(model) => {
            popups::audio_routes::view(theme, opacity, Popup::AudioRoutes, model)
        }
        PopupHostModel::BluetoothDevices(model) => {
            popups::bluetooth_devices::view(theme, opacity, model)
        }
        PopupHostModel::Calendar(model) => popups::calendar::view(opacity, model),
        PopupHostModel::SystemMonitor(model) => popups::system_info::view(theme, opacity, model),
        PopupHostModel::Power(model) => popups::power::view(theme, model),
        PopupHostModel::Connectivity(model) => popups::connectivity::view(theme, model),
        PopupHostModel::Controls(model) => popups::controls::view(theme, model),
        PopupHostModel::Media(model) => popups::media::view(theme, &model),
    }
}

fn empty_popup() -> Element<'static, Message, Theme, iced::Renderer> {
    container(Space::with_width(Length::Shrink))
        .width(Length::Shrink)
        .height(Length::Shrink)
        .into()
}

#[cfg(test)]
mod tests {
    use super::PopupHostModel;
    use crate::ui::popups;

    #[test]
    fn popup_host_model_keeps_domain_variants_explicit() {
        assert!(matches!(PopupHostModel::Hidden, PopupHostModel::Hidden));
        assert!(matches!(
            PopupHostModel::Stats(popups::stats::StatsPopupModel::new(
                1.0, "1%", "2%", "40°C", "2700 RPM"
            )),
            PopupHostModel::Stats(_)
        ));
    }
}
