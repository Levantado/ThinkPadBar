use crate::{
    app::AppCoalescingDiagnostics,
    services::{
        compositor::CompositorDiagnostics, controls::ControlsDiagnostics,
        network::NetworkDiagnostics, system_info::SystemInfoDiagnostics,
        tray_model::TrayDiagnostics,
    },
    ui::popups::{system_info, PopupMetricRow},
};

pub struct SystemInfoPopupInputs<'a> {
    pub version: &'static str,
    pub debug_ui_enabled: bool,
    pub sys_data: &'a crate::modules::system::SysData,
    pub battery: &'a crate::services::controls::BatteryInfo,
    pub power_profile: &'a str,
    pub fan: &'a crate::services::controls::FanInfo,
    pub idle_snapshot: &'a crate::services::idle_inhibitor::IdleInhibitorSnapshot,
    pub wayland_snapshot: &'a crate::services::wayland_runtime::WaylandRuntimeSnapshot,
    pub system_diagnostics: &'a SystemInfoDiagnostics,
    pub compositor_diagnostics: &'a CompositorDiagnostics,
    pub controls_diagnostics: &'a ControlsDiagnostics,
    pub network_diagnostics: &'a NetworkDiagnostics,
    pub tray_diagnostics: &'a TrayDiagnostics,
    pub coalescing_diagnostics: &'a AppCoalescingDiagnostics,
    pub audio_visualizer_runtime: String,
    pub runtime_capabilities_summary: String,
    pub capability_providers_summary: String,
    pub capability_degradations_summary: String,
    pub service_backends_summary: String,
}

pub fn build_system_info_popup_model(
    inputs: SystemInfoPopupInputs<'_>,
) -> system_info::SystemInfoPopupModel {
    let hardware_rows = system_info::hardware_rows(
        inputs.battery,
        inputs.power_profile,
        inputs.fan,
        inputs.sys_data,
        inputs.idle_snapshot,
    );
    let mut overview_rows = vec![
        PopupMetricRow::new("", "CPU Usage", inputs.sys_data.cpu_str.clone()),
        PopupMetricRow::new("󰍛", "Memory Usage", inputs.sys_data.mem_str.clone()),
        PopupMetricRow::new("󰍛", "Swap Usage", inputs.sys_data.swap_str.clone()),
        PopupMetricRow::new("", "Temperature", inputs.sys_data.temp_str.clone()),
        PopupMetricRow::new("DSK", "Disk Usage /", inputs.sys_data.disk_root_str.clone()),
        PopupMetricRow::new(
            "BOT",
            "Disk Usage /boot",
            inputs.sys_data.disk_boot_str.clone(),
        ),
        PopupMetricRow::new("NET", "IP Address", inputs.sys_data.ip_address.clone()),
        PopupMetricRow::new("DL", "Download Speed", inputs.sys_data.net_down_str.clone()),
        PopupMetricRow::new("UL", "Upload Speed", inputs.sys_data.net_up_str.clone()),
    ];
    overview_rows.extend(crate::ui::popups::displays::summary_rows(
        inputs.wayland_snapshot,
    ));

    let mut observability_rows = vec![PopupMetricRow::new(
        "VIZ",
        "Visualizer Runtime",
        inputs.audio_visualizer_runtime,
    )];
    let mut warning_rows = Vec::new();

    if inputs.debug_ui_enabled {
        observability_rows.push(PopupMetricRow::new(
            "MOD",
            "Runtime Capabilities",
            inputs.runtime_capabilities_summary,
        ));
        observability_rows.push(PopupMetricRow::new(
            "PRV",
            "Capability Providers",
            inputs.capability_providers_summary,
        ));
        observability_rows.push(PopupMetricRow::new(
            "API",
            "Capability Degradations",
            inputs.capability_degradations_summary,
        ));
        observability_rows.push(PopupMetricRow::new(
            "SVC",
            "Service Backends",
            inputs.service_backends_summary,
        ));
        observability_rows.push(PopupMetricRow::new(
            "CMP",
            "Compositor Runtime",
            inputs.compositor_diagnostics.summary(),
        ));
        observability_rows.push(PopupMetricRow::new(
            "CTL",
            "Controls Backends",
            inputs.controls_diagnostics.summary(),
        ));
        observability_rows.push(PopupMetricRow::new(
            "COL",
            "Coalescing Runtime",
            inputs.coalescing_diagnostics.summary(),
        ));
        observability_rows.push(PopupMetricRow::new(
            "NET",
            "Network Runtime",
            inputs.network_diagnostics.summary(),
        ));
        observability_rows.push(PopupMetricRow::new(
            "AUD",
            "Audio Runtime",
            inputs
                .controls_diagnostics
                .audio_runtime
                .clone()
                .unwrap_or_else(|| "n/a".to_string()),
        ));
        observability_rows.push(PopupMetricRow::new(
            "󰾆",
            "Power Runtime",
            inputs
                .controls_diagnostics
                .power_runtime
                .clone()
                .unwrap_or_else(|| "n/a".to_string()),
        ));
        observability_rows.push(PopupMetricRow::new(
            "SYS",
            "System Runtime",
            inputs.system_diagnostics.summary(),
        ));
        observability_rows.push(PopupMetricRow::new(
            "WAY",
            "Wayland Runtime",
            inputs.wayland_snapshot.runtime_summary(),
        ));
        observability_rows.push(PopupMetricRow::new(
            "CAP",
            "Wayland Capabilities",
            inputs.wayland_snapshot.capability_summary(),
        ));
        observability_rows.push(PopupMetricRow::new(
            "OUT",
            "Wayland Outputs",
            inputs.wayland_snapshot.output_summary(),
        ));
        observability_rows.push(PopupMetricRow::new(
            "DET",
            "Wayland Outputs Detail",
            inputs.wayland_snapshot.output_detail_summary(),
        ));
        observability_rows.push(PopupMetricRow::new(
            "IDL",
            "Idle Inhibitor Runtime",
            inputs.idle_snapshot.debug_summary(),
        ));
        observability_rows.push(PopupMetricRow::new(
            "TRY",
            "Tray Icons",
            inputs.tray_diagnostics.summary(),
        ));
        observability_rows.push(PopupMetricRow::new(
            "TRT",
            "Tray Runtime",
            inputs.tray_diagnostics.runtime.summary(),
        ));

        if let Some(last_unresolved) = inputs.tray_diagnostics.last_unresolved_item.clone() {
            warning_rows.push(PopupMetricRow::new(
                "WRN",
                "Tray Icon Last Unresolved",
                last_unresolved,
            ));
        }
        if let Some(unavailable_reason) = inputs.compositor_diagnostics.unavailable_reason.clone() {
            warning_rows.push(PopupMetricRow::new(
                "WRN",
                "Compositor Unavailable",
                unavailable_reason,
            ));
        }
        if let Some(last_error) = inputs.network_diagnostics.last_error.clone() {
            warning_rows.push(PopupMetricRow::new("WRN", "Network Last Error", last_error));
        }
        if let Some(unavailable_reason) = inputs.network_diagnostics.unavailable_reason.clone() {
            warning_rows.push(PopupMetricRow::new(
                "WRN",
                "Network Unavailable",
                unavailable_reason,
            ));
        }
        if let Some(unavailable_reason) = inputs.wayland_snapshot.unavailable_reason.clone() {
            warning_rows.push(PopupMetricRow::new(
                "WRN",
                "Wayland Unavailable",
                unavailable_reason,
            ));
        }
        if let Some(missing_caps) = inputs.wayland_snapshot.missing_capabilities() {
            warning_rows.push(PopupMetricRow::new(
                "WRN",
                "Wayland Missing Caps",
                missing_caps,
            ));
        }
        if let Some(last_failure) = inputs
            .tray_diagnostics
            .runtime
            .last_dispatch_failure
            .clone()
        {
            warning_rows.push(PopupMetricRow::new(
                "WRN",
                "Tray Dispatch Failure",
                last_failure,
            ));
        }
        if let Some(menu_error) = inputs
            .tray_diagnostics
            .runtime
            .last_menu_activation_error
            .clone()
        {
            warning_rows.push(PopupMetricRow::new("WRN", "Tray Menu Error", menu_error));
        }
    }

    system_info::SystemInfoPopupModel::new(
        inputs.version,
        overview_rows,
        hardware_rows,
        observability_rows,
        warning_rows,
    )
}

#[cfg(test)]
mod tests {
    use super::{build_system_info_popup_model, SystemInfoPopupInputs};
    use crate::{
        app::AppCoalescingDiagnostics,
        modules::system::SysData,
        services::{
            compositor::CompositorDiagnostics,
            controls::{BatteryInfo, ControlsDiagnostics, FanInfo},
            icon_resolver::IconResolverDiagnostics,
            idle_inhibitor::IdleInhibitorSnapshot,
            network::{NetworkBackendKind, NetworkDiagnostics},
            system_info::SystemInfoDiagnostics,
            tray_model::{TrayDiagnostics, TrayRuntimeDiagnostics},
            wayland_runtime::WaylandRuntimeSnapshot,
        },
    };

    #[test]
    fn system_info_exposes_visualizer_runtime_even_without_debug_ui() {
        let model = build_system_info_popup_model(SystemInfoPopupInputs {
            version: "1.0.0",
            debug_ui_enabled: false,
            sys_data: &SysData::default(),
            battery: &BatteryInfo {
                capacity: 0,
                status: "Unknown".to_string(),
                time_remaining: None,
                ac_online: None,
                health_percent: None,
                power_rate_mw: None,
                pack_voltage_mv: None,
                cycle_count: None,
                full_charge_mwh: None,
                design_capacity_mwh: None,
                charge_start_threshold: None,
                charge_end_threshold: None,
            },
            power_profile: "balanced",
            fan: &FanInfo {
                speed: "---".to_string(),
                level: "auto".to_string(),
            },
            idle_snapshot: &IdleInhibitorSnapshot::default(),
            wayland_snapshot: &WaylandRuntimeSnapshot::default(),
            system_diagnostics: &SystemInfoDiagnostics::default(),
            compositor_diagnostics: &CompositorDiagnostics {
                configured_backend: crate::services::compositor::CompositorBackendKind::Hyprland,
                active_backend: crate::services::compositor::CompositorBackendKind::Hyprland,
                refresh_inflight: false,
                refresh_queued: false,
                last_refresh_ms: None,
                unavailable_reason: None,
            },
            controls_diagnostics: &ControlsDiagnostics {
                audio_backend: "wpctl",
                audio_runtime: None,
                brightness_backend: "brightnessctl",
                fan_backend: "thinkfan",
                bluetooth_backend: "bluez",
                power_backend: "ppd",
                power_runtime: None,
            },
            network_diagnostics: &NetworkDiagnostics {
                configured_backend: NetworkBackendKind::Iwd,
                active_backend: NetworkBackendKind::Iwd,
                fallback_path: None,
                unavailable_reason: None,
                last_error: None,
            },
            tray_diagnostics: &TrayDiagnostics {
                total_items: 0,
                resolved_icons: 0,
                unresolved_icons: 0,
                fallback_labels: 0,
                resolver: IconResolverDiagnostics::default(),
                last_unresolved_item: None,
                runtime: TrayRuntimeDiagnostics::default(),
            },
            coalescing_diagnostics: &AppCoalescingDiagnostics::default(),
            audio_visualizer_runtime:
                "running rate:48000 bars:16 fps:24 events:7 pub:3 lvl:27 reconn:0".to_string(),
            runtime_capabilities_summary: String::new(),
            capability_providers_summary: String::new(),
            capability_degradations_summary: String::new(),
            service_backends_summary: String::new(),
        });

        assert!(model
            .observability_rows
            .iter()
            .any(|row| row.label == "Visualizer Runtime" && row.value.contains("pub:3")));
    }
}
