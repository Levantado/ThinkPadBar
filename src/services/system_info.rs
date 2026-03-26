pub use crate::modules::system::SysData;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemInfoRefreshKind {
    Fast,
    Thermal,
    Slow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SystemInfoDiagnostics {
    pub last_refresh_kind: Option<SystemInfoRefreshKind>,
    pub thermal_sensor_available: bool,
}

impl SystemInfoDiagnostics {
    pub fn summary(&self) -> String {
        let last = self
            .last_refresh_kind
            .map_or_else(|| "-".to_string(), |kind| format!("{kind:?}"));
        format!(
            "last {} thermal_sensor:{}",
            last, self.thermal_sensor_available
        )
    }
}

pub struct SystemInfoService {
    monitor: crate::modules::system::SysMonitor,
    snapshot: SysData,
    diagnostics: SystemInfoDiagnostics,
}

impl SystemInfoService {
    pub fn new() -> Self {
        let mut monitor = crate::modules::system::SysMonitor::new();
        let snapshot = monitor.update(false);
        let diagnostics = SystemInfoDiagnostics {
            last_refresh_kind: Some(SystemInfoRefreshKind::Slow),
            thermal_sensor_available: !snapshot.temp_str.is_empty(),
        };
        Self {
            monitor,
            snapshot,
            diagnostics,
        }
    }

    pub fn snapshot(&self) -> &SysData {
        &self.snapshot
    }

    pub fn diagnostics(&self) -> SystemInfoDiagnostics {
        self.diagnostics
    }

    pub fn refresh(&mut self, kind: SystemInfoRefreshKind) -> &SysData {
        self.diagnostics.last_refresh_kind = Some(kind);
        match kind {
            SystemInfoRefreshKind::Fast => {
                self.snapshot = self.monitor.update(true);
                self.diagnostics.thermal_sensor_available = !self.snapshot.temp_str.is_empty();
            }
            SystemInfoRefreshKind::Thermal => {
                self.diagnostics.thermal_sensor_available = Self::apply_thermal_reading(
                    &mut self.snapshot,
                    crate::modules::system::read_temperature_celsius(),
                );
            }
            SystemInfoRefreshKind::Slow => {
                self.snapshot = self.monitor.update(false);
                self.diagnostics.thermal_sensor_available = !self.snapshot.temp_str.is_empty();
            }
        }
        &self.snapshot
    }

    fn apply_thermal_reading(snapshot: &mut SysData, temp: Option<f32>) -> bool {
        if let Some(temp) = temp {
            snapshot.temp = temp;
            snapshot.temp_str = format!("{}°C", temp.round() as u64);
            return true;
        }
        false
    }

    #[cfg(test)]
    pub fn with_snapshot(snapshot: SysData) -> Self {
        Self {
            monitor: crate::modules::system::SysMonitor::new(),
            snapshot,
            diagnostics: SystemInfoDiagnostics::default(),
        }
    }

    #[cfg(test)]
    pub fn refresh_thermal_for_tests(&mut self, temp: Option<f32>) -> &SysData {
        self.diagnostics.last_refresh_kind = Some(SystemInfoRefreshKind::Thermal);
        self.diagnostics.thermal_sensor_available =
            Self::apply_thermal_reading(&mut self.snapshot, temp);
        &self.snapshot
    }
}

#[cfg(test)]
mod tests {
    use super::{SystemInfoRefreshKind, SystemInfoService};

    #[test]
    fn service_exposes_initial_snapshot() {
        let service = SystemInfoService::with_snapshot(crate::modules::system::SysData {
            cpu_str: "15%".to_string(),
            temp_str: "48°C".to_string(),
            ..crate::modules::system::SysData::default()
        });
        let snapshot = service.snapshot();
        assert_eq!(snapshot.cpu_str, "15%");
        assert_eq!(snapshot.temp_str, "48°C");
    }

    #[test]
    fn thermal_refresh_preserves_existing_strings_when_sensor_missing() {
        let mut service = SystemInfoService::with_snapshot(crate::modules::system::SysData {
            temp: 48.0,
            temp_str: "48°C".to_string(),
            ..crate::modules::system::SysData::default()
        });
        let before = service.snapshot().temp_str.clone();
        let _ = service.refresh_thermal_for_tests(None);
        assert_eq!(service.snapshot().temp_str, before);
        assert_eq!(
            service.diagnostics().last_refresh_kind,
            Some(SystemInfoRefreshKind::Thermal)
        );
        assert!(!service.diagnostics().thermal_sensor_available);
    }

    #[test]
    fn thermal_refresh_updates_snapshot_from_provided_reading() {
        let mut service = SystemInfoService::with_snapshot(crate::modules::system::SysData {
            temp_str: "48°C".to_string(),
            ..crate::modules::system::SysData::default()
        });
        let _ = service.refresh_thermal_for_tests(Some(52.4));
        assert_eq!(service.snapshot().temp, 52.4);
        assert_eq!(service.snapshot().temp_str, "52°C");
        assert!(service.diagnostics().thermal_sensor_available);
    }

    #[test]
    fn diagnostics_summary_reports_last_refresh_kind() {
        let mut service =
            SystemInfoService::with_snapshot(crate::modules::system::SysData::default());
        let _ = service.refresh_thermal_for_tests(Some(43.0));

        assert_eq!(
            service.diagnostics().summary(),
            "last Thermal thermal_sensor:true"
        );
    }
}
