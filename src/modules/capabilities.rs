#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModuleCapability {
    ProvidesBarWidget,
    ProvidesPopupSection,
    EmitsEvents,
    ConsumesSystemDbus,
    ConsumesSessionDbus,
    ReadsProcfsSysfs,
    ControlsHardware,
    ControlsWorkspace,
    ControlsNetwork,
    ControlsAudio,
}

#[derive(Debug, Clone, Copy)]
pub struct ModuleDescriptor {
    pub name: &'static str,
    pub capabilities: &'static [ModuleCapability],
}

const AUDIO_CAPS: &[ModuleCapability] = &[
    ModuleCapability::ProvidesBarWidget,
    ModuleCapability::ProvidesPopupSection,
    ModuleCapability::EmitsEvents,
    ModuleCapability::ControlsAudio,
];
const BATTERY_CAPS: &[ModuleCapability] = &[
    ModuleCapability::ProvidesBarWidget,
    ModuleCapability::ReadsProcfsSysfs,
];
const BLUETOOTH_CAPS: &[ModuleCapability] = &[
    ModuleCapability::ProvidesBarWidget,
    ModuleCapability::ProvidesPopupSection,
    ModuleCapability::ControlsHardware,
];
const BRIGHTNESS_CAPS: &[ModuleCapability] = &[
    ModuleCapability::ProvidesBarWidget,
    ModuleCapability::ProvidesPopupSection,
    ModuleCapability::ControlsHardware,
    ModuleCapability::ReadsProcfsSysfs,
];
const FAN_CAPS: &[ModuleCapability] = &[
    ModuleCapability::ProvidesBarWidget,
    ModuleCapability::ProvidesPopupSection,
    ModuleCapability::ControlsHardware,
    ModuleCapability::ReadsProcfsSysfs,
];
const KEYBOARD_CAPS: &[ModuleCapability] = &[
    ModuleCapability::ProvidesBarWidget,
    ModuleCapability::ControlsWorkspace,
    ModuleCapability::ConsumesSessionDbus,
];
const MIC_CAPS: &[ModuleCapability] = &[
    ModuleCapability::ProvidesPopupSection,
    ModuleCapability::ControlsAudio,
    ModuleCapability::ControlsHardware,
];
const POWER_CAPS: &[ModuleCapability] = &[
    ModuleCapability::ProvidesPopupSection,
    ModuleCapability::ControlsHardware,
    ModuleCapability::ReadsProcfsSysfs,
];
const SYSTEM_CAPS: &[ModuleCapability] = &[
    ModuleCapability::ProvidesBarWidget,
    ModuleCapability::ProvidesPopupSection,
    ModuleCapability::ReadsProcfsSysfs,
];
const TRAY_CAPS: &[ModuleCapability] = &[
    ModuleCapability::ProvidesBarWidget,
    ModuleCapability::EmitsEvents,
    ModuleCapability::ConsumesSessionDbus,
];
const WIFI_CAPS: &[ModuleCapability] = &[
    ModuleCapability::ProvidesBarWidget,
    ModuleCapability::ProvidesPopupSection,
    ModuleCapability::ControlsNetwork,
    ModuleCapability::ConsumesSystemDbus,
];
const WORKSPACES_CAPS: &[ModuleCapability] = &[
    ModuleCapability::ProvidesBarWidget,
    ModuleCapability::EmitsEvents,
    ModuleCapability::ControlsWorkspace,
];

const BUILTIN_MODULES: &[ModuleDescriptor] = &[
    ModuleDescriptor {
        name: "audio",
        capabilities: AUDIO_CAPS,
    },
    ModuleDescriptor {
        name: "battery",
        capabilities: BATTERY_CAPS,
    },
    ModuleDescriptor {
        name: "bluetooth",
        capabilities: BLUETOOTH_CAPS,
    },
    ModuleDescriptor {
        name: "brightness",
        capabilities: BRIGHTNESS_CAPS,
    },
    ModuleDescriptor {
        name: "fan",
        capabilities: FAN_CAPS,
    },
    ModuleDescriptor {
        name: "keyboard",
        capabilities: KEYBOARD_CAPS,
    },
    ModuleDescriptor {
        name: "mic",
        capabilities: MIC_CAPS,
    },
    ModuleDescriptor {
        name: "power",
        capabilities: POWER_CAPS,
    },
    ModuleDescriptor {
        name: "system",
        capabilities: SYSTEM_CAPS,
    },
    ModuleDescriptor {
        name: "tray",
        capabilities: TRAY_CAPS,
    },
    ModuleDescriptor {
        name: "wifi",
        capabilities: WIFI_CAPS,
    },
    ModuleDescriptor {
        name: "workspaces",
        capabilities: WORKSPACES_CAPS,
    },
];

pub fn built_in_modules() -> &'static [ModuleDescriptor] {
    BUILTIN_MODULES
}
