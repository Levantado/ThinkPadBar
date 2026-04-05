use std::path::Path;

#[derive(Debug, Default, Clone, Copy)]
pub struct ProcfsFanBackend;

const FAN_CONTROL_PATH: &str = "/proc/acpi/ibm/fan";

impl super::FanBackend for ProcfsFanBackend {
    fn backend_name(&self) -> &'static str {
        "procfs+pkexec"
    }

    fn capability_mode(&self) -> crate::services::capabilities::CapabilityMode {
        if Path::new(FAN_CONTROL_PATH).exists() {
            crate::services::capabilities::CapabilityMode::Fallback
        } else {
            crate::services::capabilities::CapabilityMode::Unavailable
        }
    }

    fn diagnostics_summary(&self) -> Option<String> {
        if Path::new(FAN_CONTROL_PATH).exists() {
            Some("direct procfs write + pkexec fallback".to_string())
        } else {
            Some("thinkpad_acpi fan interface missing".to_string())
        }
    }

    fn info(&self) -> crate::services::controls::FanInfo {
        crate::modules::fan::get_fan_info()
    }

    fn set_level(&self, level: &str) -> crate::services::controls_backends::BackendFuture<'_, ()> {
        let command = format!("level {}", level);
        Box::pin(async move {
            let _ =
                crate::services::controls_backends::privileged::write_file_with_pkexec_fallback(
                    Path::new(FAN_CONTROL_PATH),
                    &command,
                )
                .await;
        })
    }
}

#[cfg(test)]
mod tests {
    use super::ProcfsFanBackend;
    use crate::services::{capabilities::CapabilityMode, controls_backends::FanBackend};

    #[test]
    fn backend_reports_consistent_provider_identity() {
        let backend = ProcfsFanBackend;
        assert_eq!(backend.backend_name(), "procfs+pkexec");
        assert!(matches!(
            backend.capability_mode(),
            CapabilityMode::Fallback | CapabilityMode::Unavailable
        ));
    }
}
