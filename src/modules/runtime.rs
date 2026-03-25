use crate::modules::capabilities::ModuleDescriptor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleEvent {
    TickFast,
    TickSlow,
    UserAction(&'static str),
    ExternalSignal(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleCommand {
    Refresh,
    EmitStatus(&'static str),
    Noop,
}

pub trait ModuleRuntime {
    fn descriptor(&self) -> ModuleDescriptor;

    fn on_start(&mut self) -> Vec<ModuleCommand> {
        vec![ModuleCommand::Noop]
    }

    fn on_stop(&mut self) -> Vec<ModuleCommand> {
        vec![ModuleCommand::Noop]
    }

    fn on_event(&mut self, _event: ModuleEvent) -> Vec<ModuleCommand> {
        vec![ModuleCommand::Noop]
    }
}

const CANONICAL_EVENTS: &[ModuleEvent] = &[
    ModuleEvent::TickFast,
    ModuleEvent::TickSlow,
    ModuleEvent::UserAction("generic"),
    ModuleEvent::ExternalSignal("generic"),
];

const CANONICAL_COMMANDS: &[ModuleCommand] = &[
    ModuleCommand::Refresh,
    ModuleCommand::EmitStatus("status"),
    ModuleCommand::Noop,
];

pub fn contract_version() -> &'static str {
    "v0-draft"
}

pub fn canonical_events() -> &'static [ModuleEvent] {
    CANONICAL_EVENTS
}

pub fn canonical_commands() -> &'static [ModuleCommand] {
    CANONICAL_COMMANDS
}

struct NoopRuntime;

impl ModuleRuntime for NoopRuntime {
    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor {
            name: "noop-runtime",
            capabilities: &[],
        }
    }
}

pub fn noop_runtime_descriptor_name() -> &'static str {
    let mut runtime = NoopRuntime;
    let _ = runtime.on_start();
    let _ = runtime.on_event(ModuleEvent::TickFast);
    let _ = runtime.on_stop();
    runtime.descriptor().name
}

#[cfg(test)]
mod tests {
    use super::{ModuleCommand, ModuleEvent, ModuleRuntime};
    use crate::modules::capabilities::{ModuleCapability, ModuleDescriptor};

    struct DummyModule;

    impl ModuleRuntime for DummyModule {
        fn descriptor(&self) -> ModuleDescriptor {
            ModuleDescriptor {
                name: "dummy",
                capabilities: &[ModuleCapability::EmitsEvents],
            }
        }

        fn on_event(&mut self, event: ModuleEvent) -> Vec<ModuleCommand> {
            match event {
                ModuleEvent::TickSlow => vec![ModuleCommand::Refresh],
                _ => vec![ModuleCommand::Noop],
            }
        }
    }

    #[test]
    fn module_runtime_descriptor_is_exposed() {
        let module = DummyModule;
        let desc = module.descriptor();
        assert_eq!(desc.name, "dummy");
    }

    #[test]
    fn module_runtime_can_emit_commands_from_events() {
        let mut module = DummyModule;
        let commands = module.on_event(ModuleEvent::TickSlow);
        assert_eq!(commands, vec![ModuleCommand::Refresh]);
    }
}
