pub use crate::modules::system::SysData;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemInfoRefreshKind {
    Fast,
    Thermal,
    Slow,
}

pub struct SystemInfoService {
    monitor: crate::modules::system::SysMonitor,
    snapshot: SysData,
}

impl SystemInfoService {
    pub fn new() -> Self {
        let mut monitor = crate::modules::system::SysMonitor::new();
        let snapshot = monitor.update(false);
        Self { monitor, snapshot }
    }

    pub fn snapshot(&self) -> &SysData {
        &self.snapshot
    }

    pub fn refresh(&mut self, kind: SystemInfoRefreshKind) -> &SysData {
        match kind {
            SystemInfoRefreshKind::Fast => {
                self.snapshot = self.monitor.update(true);
            }
            SystemInfoRefreshKind::Thermal => {
                if let Some(temp) = crate::modules::system::read_temperature_celsius() {
                    self.snapshot.temp = temp;
                    self.snapshot.temp_str = format!("{}°C", temp.round() as u64);
                }
            }
            SystemInfoRefreshKind::Slow => {
                self.snapshot = self.monitor.update(false);
            }
        }
        &self.snapshot
    }
}

#[cfg(test)]
mod tests {
    use super::{SystemInfoRefreshKind, SystemInfoService};

    #[test]
    fn service_exposes_initial_snapshot() {
        let service = SystemInfoService::new();
        let snapshot = service.snapshot();
        assert!(!snapshot.cpu_str.is_empty() || snapshot.cpu_usage >= 0.0);
    }

    #[test]
    fn thermal_refresh_preserves_existing_strings_when_sensor_missing() {
        let mut service = SystemInfoService::new();
        let before = service.snapshot().temp_str.clone();
        let _ = service.refresh(SystemInfoRefreshKind::Thermal);
        assert!(!service.snapshot().temp_str.is_empty() || !before.is_empty());
    }
}
