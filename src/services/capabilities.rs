use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityMode {
    Native,
    Hybrid,
    Fallback,
    ReadOnly,
    Unavailable,
}

impl CapabilityMode {
    pub fn short_label(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Hybrid => "hybrid",
            Self::Fallback => "fallback",
            Self::ReadOnly => "read-only",
            Self::Unavailable => "unavailable",
        }
    }
}

impl fmt::Display for CapabilityMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.short_label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityStatus {
    pub key: &'static str,
    pub label: &'static str,
    pub mode: CapabilityMode,
    pub provider: String,
    pub detail: Option<String>,
}

impl CapabilityStatus {
    pub fn summary(&self) -> String {
        match &self.detail {
            Some(detail) if !detail.is_empty() => {
                format!("{} via {} ({detail})", self.mode, self.provider)
            }
            _ => format!("{} via {}", self.mode, self.provider),
        }
    }

    pub fn is_degraded(&self) -> bool {
        !matches!(self.mode, CapabilityMode::Native)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuntimeCapabilities {
    pub items: Vec<CapabilityStatus>,
}

impl RuntimeCapabilities {
    pub fn new(items: Vec<CapabilityStatus>) -> Self {
        Self { items }
    }

    pub fn summary(&self) -> String {
        if self.items.is_empty() {
            return "none".to_string();
        }

        self.items
            .iter()
            .map(|item| format!("{}={}", item.key, item.mode.short_label()))
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn degraded_summary(&self) -> Option<String> {
        let degraded = self
            .items
            .iter()
            .filter(|item| item.is_degraded())
            .map(|item| format!("{}={}", item.key, item.mode.short_label()))
            .collect::<Vec<_>>();

        (!degraded.is_empty()).then(|| degraded.join(", "))
    }

    pub fn provider_summary(&self) -> String {
        if self.items.is_empty() {
            return "none".to_string();
        }

        self.items
            .iter()
            .map(|item| format!("{} {}", item.key, item.summary()))
            .collect::<Vec<_>>()
            .join("; ")
    }
}

#[cfg(test)]
mod tests {
    use super::{CapabilityMode, CapabilityStatus, RuntimeCapabilities};

    #[test]
    fn runtime_capabilities_summary_is_stable() {
        let capabilities = RuntimeCapabilities::new(vec![
            CapabilityStatus {
                key: "cmp",
                label: "Compositor",
                mode: CapabilityMode::Native,
                provider: "hyprland".to_string(),
                detail: None,
            },
            CapabilityStatus {
                key: "bt",
                label: "Bluetooth",
                mode: CapabilityMode::Fallback,
                provider: "bluetoothctl".to_string(),
                detail: Some("cli path".to_string()),
            },
        ]);

        assert_eq!(capabilities.summary(), "cmp=native bt=fallback");
        assert_eq!(
            capabilities.degraded_summary().as_deref(),
            Some("bt=fallback")
        );
        assert_eq!(
            capabilities.provider_summary(),
            "cmp native via hyprland; bt fallback via bluetoothctl (cli path)"
        );
    }

    #[test]
    fn capability_status_summary_includes_detail_when_present() {
        let status = CapabilityStatus {
            key: "pwr",
            label: "Power",
            mode: CapabilityMode::Hybrid,
            provider: "ppd+platform_profile".to_string(),
            detail: Some("dbus write + sysfs fallback".to_string()),
        };

        assert_eq!(
            status.summary(),
            "hybrid via ppd+platform_profile (dbus write + sysfs fallback)"
        );
    }
}
