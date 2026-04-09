use std::{
    cell::Cell,
    collections::HashMap,
    process::{Command, Stdio},
    rc::Rc,
    sync::{Arc, Mutex, Once},
    thread,
    time::Duration,
};

use iced::futures::SinkExt;
use pipewire as pw;
use tokio::sync::mpsc::UnboundedSender;

static PIPEWIRE_INIT: Once = Once::new();
const PIPEWIRE_RETRY_DELAY: Duration = Duration::from_secs(2);

type SharedDiagnostics = Arc<Mutex<AudioEventRuntimeDiagnostics>>;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AudioEventRuntimeDiagnostics {
    pub listener_running: bool,
    pub tracked_nodes: usize,
    pub tracked_metadata: usize,
    pub event_count: u64,
    pub reconnects: u64,
    pub last_event: Option<String>,
    pub last_error: Option<String>,
}

impl AudioEventRuntimeDiagnostics {
    pub fn summary(&self) -> String {
        let mut summary = format!(
            "{} nodes:{} meta:{} events:{} reconn:{}",
            if self.listener_running {
                "running"
            } else {
                "stopped"
            },
            self.tracked_nodes,
            self.tracked_metadata,
            self.event_count,
            self.reconnects
        );

        if let Some(last_event) = &self.last_event {
            summary.push_str(" last:");
            summary.push_str(last_event);
        }

        if let Some(last_error) = &self.last_error {
            summary.push_str(" err:");
            summary.push_str(last_error);
        }

        summary
    }
}

#[derive(Debug, Clone)]
pub struct WpctlAudioBackend {
    diagnostics: SharedDiagnostics,
}

impl Default for WpctlAudioBackend {
    fn default() -> Self {
        Self {
            diagnostics: Arc::new(Mutex::new(AudioEventRuntimeDiagnostics::default())),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum AudioListenerCommand {
    Terminate,
}

struct PipeWireThreadGuard {
    command_tx: pw::channel::Sender<AudioListenerCommand>,
    handle: Option<std::thread::JoinHandle<()>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrackedAudioObject {
    Node,
    Metadata,
}

#[derive(Default)]
struct AudioObjectStore {
    proxies: HashMap<u32, Box<dyn pw::proxy::ProxyT>>,
    listeners: HashMap<u32, Box<dyn pw::proxy::Listener>>,
    tracked: HashMap<u32, TrackedAudioObject>,
}

impl AudioObjectStore {
    fn insert(
        &mut self,
        id: u32,
        kind: TrackedAudioObject,
        proxy: Box<dyn pw::proxy::ProxyT>,
        listener: Box<dyn pw::proxy::Listener>,
    ) {
        self.proxies.insert(id, proxy);
        self.listeners.insert(id, listener);
        self.tracked.insert(id, kind);
    }

    fn remove(&mut self, id: u32) -> Option<TrackedAudioObject> {
        let removed = self.tracked.remove(&id);
        self.proxies.remove(&id);
        self.listeners.remove(&id);
        removed
    }

    fn tracked_nodes(&self) -> usize {
        self.tracked
            .values()
            .filter(|kind| matches!(kind, TrackedAudioObject::Node))
            .count()
    }

    fn tracked_metadata(&self) -> usize {
        self.tracked
            .values()
            .filter(|kind| matches!(kind, TrackedAudioObject::Metadata))
            .count()
    }
}

impl Drop for PipeWireThreadGuard {
    fn drop(&mut self) {
        let _ = self.command_tx.send(AudioListenerCommand::Terminate);
        self.handle.take();
    }
}

impl PipeWireThreadGuard {
    fn spawn(
        event_tx: UnboundedSender<crate::services::controls::ControlsEvent>,
        diagnostics: SharedDiagnostics,
    ) -> Self {
        let (command_tx, command_rx) = pw::channel::channel();
        let handle =
            thread::spawn(move || run_pipewire_audio_thread(event_tx, command_rx, diagnostics));
        Self {
            command_tx,
            handle: Some(handle),
        }
    }
}

impl WpctlAudioBackend {
    fn get_volume(target: &str) -> Option<(u32, bool)> {
        let output = Command::new("wpctl")
            .args(["get-volume", target])
            .output()
            .ok()?;
        let stdout = String::from_utf8(output.stdout).ok()?;
        parse_wpctl_volume(&stdout)
    }

    fn get_device_summary() -> crate::services::controls::AudioDeviceSummary {
        Command::new("wpctl")
            .arg("status")
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .as_deref()
            .map(parse_wpctl_route_summary)
            .unwrap_or_default()
    }
}

impl super::AudioBackend for WpctlAudioBackend {
    fn backend_name(&self) -> &'static str {
        "wpctl+pipewire"
    }

    fn capability_mode(&self) -> crate::services::capabilities::CapabilityMode {
        crate::services::capabilities::CapabilityMode::Hybrid
    }

    fn diagnostics_summary(&self) -> Option<String> {
        lock_diagnostics(&self.diagnostics)
            .map(|diagnostics| diagnostics.summary())
            .or_else(|| Some("stopped err:diagnostics-lock".to_string()))
    }

    fn audio_info(&self) -> crate::services::controls::AudioInfo {
        let (volume, muted) = Self::get_volume("@DEFAULT_AUDIO_SINK@").unwrap_or((0, false));
        crate::services::controls::AudioInfo { volume, muted }
    }

    fn mic_info(&self) -> crate::modules::mic::MicInfo {
        let (volume, muted) = Self::get_volume("@DEFAULT_AUDIO_SOURCE@").unwrap_or((0, false));
        crate::modules::mic::MicInfo { volume, muted }
    }

    fn device_summary(&self) -> crate::services::controls::AudioDeviceSummary {
        Self::get_device_summary()
    }

    fn set_volume(&self, percent: u32) -> super::BackendFuture<'_, ()> {
        Box::pin(async move {
            let vol_str = format!("{:.2}", percent as f32 / 100.0);
            let _ = tokio::process::Command::new("wpctl")
                .args(["set-volume", "@DEFAULT_AUDIO_SINK@", &vol_str])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await;
        })
    }

    fn toggle_audio_mute(&self) -> super::BackendFuture<'_, ()> {
        Box::pin(async move {
            let _ = tokio::process::Command::new("wpctl")
                .args(["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await;
        })
    }

    fn set_output_route(&self, id: String) -> super::BackendFuture<'_, bool> {
        Box::pin(async move { set_audio_route(id).await })
    }

    fn set_mic_volume(&self, percent: u32) -> super::BackendFuture<'_, ()> {
        Box::pin(async move {
            let vol_str = format!("{:.2}", percent as f32 / 100.0);
            let _ = tokio::process::Command::new("wpctl")
                .args(["set-volume", "@DEFAULT_AUDIO_SOURCE@", &vol_str])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await;
        })
    }

    fn toggle_mic_mute(&self) -> super::BackendFuture<'_, ()> {
        Box::pin(async move {
            let _ = tokio::process::Command::new("wpctl")
                .args(["set-mute", "@DEFAULT_AUDIO_SOURCE@", "toggle"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await;
        })
    }

    fn set_input_route(&self, id: String) -> super::BackendFuture<'_, bool> {
        Box::pin(async move { set_audio_route(id).await })
    }

    fn subscription(&self) -> iced::Subscription<crate::services::controls::ControlsEvent> {
        struct AudioListener;
        let diagnostics = self.diagnostics.clone();
        iced::Subscription::run_with_id(
            std::any::TypeId::of::<AudioListener>(),
            iced::stream::channel(1, move |mut output| async move {
                let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();
                let _thread_guard = PipeWireThreadGuard::spawn(event_tx, diagnostics);

                while let Some(event) = event_rx.recv().await {
                    let _ = output.send(event).await;
                }
            }),
        )
    }
}

fn run_pipewire_audio_thread(
    event_tx: UnboundedSender<crate::services::controls::ControlsEvent>,
    mut command_rx: pw::channel::Receiver<AudioListenerCommand>,
    diagnostics: SharedDiagnostics,
) {
    ensure_pipewire_initialized();

    loop {
        let (receiver, terminated) =
            match run_pipewire_audio_session(event_tx.clone(), command_rx, diagnostics.clone()) {
                Ok(result) => result,
                Err((receiver, error)) => {
                    command_rx = receiver;
                    note_error(&diagnostics, error);
                    note_reconnect(&diagnostics);
                    thread::sleep(PIPEWIRE_RETRY_DELAY);
                    continue;
                }
            };

        command_rx = receiver;
        set_listener_running(&diagnostics, false);
        if terminated {
            break;
        }

        note_reconnect(&diagnostics);
        thread::sleep(PIPEWIRE_RETRY_DELAY);
    }
}

fn run_pipewire_audio_session(
    event_tx: UnboundedSender<crate::services::controls::ControlsEvent>,
    command_rx: pw::channel::Receiver<AudioListenerCommand>,
    diagnostics: SharedDiagnostics,
) -> Result<
    (pw::channel::Receiver<AudioListenerCommand>, bool),
    (pw::channel::Receiver<AudioListenerCommand>, String),
> {
    let main_loop = match pw::main_loop::MainLoopRc::new(None) {
        Ok(main_loop) => main_loop,
        Err(error) => return Err((command_rx, error.to_string())),
    };
    let should_terminate = Rc::new(Cell::new(false));

    let attached_command_rx = command_rx.attach(main_loop.loop_(), {
        let main_loop = main_loop.clone();
        let should_terminate = should_terminate.clone();
        move |command| {
            if matches!(command, AudioListenerCommand::Terminate) {
                should_terminate.set(true);
                main_loop.quit();
            }
        }
    });

    let context = match pw::context::ContextRc::new(&main_loop, None) {
        Ok(context) => context,
        Err(error) => return Err((attached_command_rx.deattach(), error.to_string())),
    };
    let core = match context.connect_rc(None) {
        Ok(core) => core,
        Err(error) => return Err((attached_command_rx.deattach(), error.to_string())),
    };
    let registry = match core.get_registry_rc() {
        Ok(registry) => registry,
        Err(error) => return Err((attached_command_rx.deattach(), error.to_string())),
    };

    let object_store = Rc::new(std::cell::RefCell::new(AudioObjectStore::default()));

    let _core_listener = core
        .add_listener_local()
        .error({
            let diagnostics = diagnostics.clone();
            let main_loop = main_loop.clone();
            move |id, _seq, _res, message| {
                if id == 0 {
                    note_error(&diagnostics, format!("core:{message}"));
                    main_loop.quit();
                }
            }
        })
        .register();

    let _registry_listener = registry
        .add_listener_local()
        .global({
            let diagnostics = diagnostics.clone();
            let event_tx = event_tx.clone();
            let object_store = object_store.clone();
            let registry = registry.clone();
            move |obj| match obj.type_ {
                pw::types::ObjectType::Node => {
                    let media_class = dict_value(obj.props, *pw::keys::MEDIA_CLASS);
                    if !should_track_audio_node(media_class) {
                        return;
                    }

                    let Ok(node) = registry.bind::<pw::node::Node, _>(obj) else {
                        note_error(
                            &diagnostics,
                            format!(
                                "bind-node-failed:{}",
                                dict_value(obj.props, *pw::keys::NODE_NAME).unwrap_or("unknown")
                            ),
                        );
                        return;
                    };
                    node.subscribe_params(&[
                        pw::spa::param::ParamType::Props,
                        pw::spa::param::ParamType::Route,
                        pw::spa::param::ParamType::Profile,
                    ]);

                    let listener = node
                        .add_listener_local()
                        .info({
                            let diagnostics = diagnostics.clone();
                            let event_tx = event_tx.clone();
                            move |info| {
                                let media_class = dict_value(info.props(), *pw::keys::MEDIA_CLASS);
                                if should_track_audio_node(media_class)
                                    && should_emit_audio_node_info(info.change_mask())
                                {
                                    emit_audio_changed(&diagnostics, &event_tx, "node:info");
                                }
                            }
                        })
                        .param({
                            let diagnostics = diagnostics.clone();
                            let event_tx = event_tx.clone();
                            move |_seq, id, _index, _next, _param| {
                                if should_emit_audio_param(id) {
                                    emit_audio_changed(
                                        &diagnostics,
                                        &event_tx,
                                        audio_param_event_label(id),
                                    );
                                }
                            }
                        })
                        .register();

                    {
                        let mut store = object_store.borrow_mut();
                        store.insert(
                            obj.id,
                            TrackedAudioObject::Node,
                            Box::new(node),
                            Box::new(listener),
                        );
                        set_tracked_counts(
                            &diagnostics,
                            store.tracked_nodes(),
                            store.tracked_metadata(),
                        );
                    }
                    emit_audio_changed(&diagnostics, &event_tx, "node:add");
                }
                pw::types::ObjectType::Metadata => {
                    let metadata_name = dict_value(obj.props, "metadata.name");
                    if !should_track_audio_metadata(metadata_name) {
                        return;
                    }

                    let Ok(metadata) = registry.bind::<pw::metadata::Metadata, _>(obj) else {
                        note_error(&diagnostics, "bind-metadata-failed:default".to_string());
                        return;
                    };
                    let listener = metadata
                        .add_listener_local()
                        .property({
                            let diagnostics = diagnostics.clone();
                            let event_tx = event_tx.clone();
                            move |_subject, key, _type, _value| {
                                if should_emit_audio_metadata_property(key) {
                                    emit_audio_changed(
                                        &diagnostics,
                                        &event_tx,
                                        metadata_event_label(key),
                                    );
                                }
                                0
                            }
                        })
                        .register();

                    {
                        let mut store = object_store.borrow_mut();
                        store.insert(
                            obj.id,
                            TrackedAudioObject::Metadata,
                            Box::new(metadata),
                            Box::new(listener),
                        );
                        set_tracked_counts(
                            &diagnostics,
                            store.tracked_nodes(),
                            store.tracked_metadata(),
                        );
                    }
                    emit_audio_changed(&diagnostics, &event_tx, "metadata:add");
                }
                _ => {}
            }
        })
        .global_remove({
            let diagnostics = diagnostics.clone();
            let event_tx = event_tx.clone();
            let object_store = object_store.clone();
            move |id| {
                let removed_kind = {
                    let mut store = object_store.borrow_mut();
                    let removed_kind = store.remove(id);
                    set_tracked_counts(
                        &diagnostics,
                        store.tracked_nodes(),
                        store.tracked_metadata(),
                    );
                    removed_kind
                };
                if let Some(kind) = removed_kind {
                    emit_audio_changed(
                        &diagnostics,
                        &event_tx,
                        match kind {
                            TrackedAudioObject::Node => "node:remove",
                            TrackedAudioObject::Metadata => "metadata:remove",
                        },
                    );
                }
            }
        })
        .register();

    set_listener_running(&diagnostics, true);
    clear_last_error(&diagnostics);
    main_loop.run();
    Ok((attached_command_rx.deattach(), should_terminate.get()))
}

fn ensure_pipewire_initialized() {
    PIPEWIRE_INIT.call_once(pw::init);
}

fn lock_diagnostics(
    diagnostics: &SharedDiagnostics,
) -> Option<std::sync::MutexGuard<'_, AudioEventRuntimeDiagnostics>> {
    diagnostics.lock().ok()
}

fn set_listener_running(diagnostics: &SharedDiagnostics, running: bool) {
    if let Some(mut state) = lock_diagnostics(diagnostics) {
        state.listener_running = running;
    }
}

fn clear_last_error(diagnostics: &SharedDiagnostics) {
    if let Some(mut state) = lock_diagnostics(diagnostics) {
        state.last_error = None;
    }
}

fn note_error(diagnostics: &SharedDiagnostics, error: String) {
    if let Some(mut state) = lock_diagnostics(diagnostics) {
        state.listener_running = false;
        state.last_error = Some(error);
    }
}

fn note_reconnect(diagnostics: &SharedDiagnostics) {
    if let Some(mut state) = lock_diagnostics(diagnostics) {
        state.reconnects = state.reconnects.saturating_add(1);
    }
}

fn set_tracked_counts(
    diagnostics: &SharedDiagnostics,
    tracked_nodes: usize,
    tracked_metadata: usize,
) {
    if let Some(mut state) = lock_diagnostics(diagnostics) {
        state.tracked_nodes = tracked_nodes;
        state.tracked_metadata = tracked_metadata;
    }
}

fn emit_audio_changed(
    diagnostics: &SharedDiagnostics,
    event_tx: &UnboundedSender<crate::services::controls::ControlsEvent>,
    label: &str,
) {
    if let Some(mut state) = lock_diagnostics(diagnostics) {
        state.event_count = state.event_count.saturating_add(1);
        state.last_event = Some(label.to_string());
    }
    let _ = event_tx.send(crate::services::controls::ControlsEvent::AudioServer);
}

fn dict_value<'a>(props: Option<&'a pw::spa::utils::dict::DictRef>, key: &str) -> Option<&'a str> {
    props.and_then(|props| props.get(key))
}

fn should_track_audio_node(media_class: Option<&str>) -> bool {
    matches!(
        media_class,
        Some("Audio/Sink") | Some("Audio/Source") | Some("Audio/Duplex")
    )
}

fn should_track_audio_metadata(metadata_name: Option<&str>) -> bool {
    matches!(metadata_name, Some("default"))
}

fn should_emit_audio_metadata_property(key: Option<&str>) -> bool {
    matches!(
        key,
        None | Some("default.audio.sink") | Some("default.audio.source")
    )
}

fn metadata_event_label(key: Option<&str>) -> &'static str {
    match key {
        Some("default.audio.sink") => "metadata:default.audio.sink",
        Some("default.audio.source") => "metadata:default.audio.source",
        _ => "metadata:clear",
    }
}

fn should_emit_audio_param(id: pw::spa::param::ParamType) -> bool {
    matches!(
        id,
        pw::spa::param::ParamType::Props
            | pw::spa::param::ParamType::Route
            | pw::spa::param::ParamType::Profile
    )
}

fn audio_param_event_label(id: pw::spa::param::ParamType) -> &'static str {
    match id {
        pw::spa::param::ParamType::Props => "node:param-props",
        pw::spa::param::ParamType::Route => "node:param-route",
        pw::spa::param::ParamType::Profile => "node:param-profile",
        _ => "node:param-other",
    }
}

fn should_emit_audio_node_info(change_mask: pw::node::NodeChangeMask) -> bool {
    change_mask.intersects(
        pw::node::NodeChangeMask::PROPS
            | pw::node::NodeChangeMask::PARAMS
            | pw::node::NodeChangeMask::STATE,
    )
}

pub(crate) fn parse_wpctl_volume(output: &str) -> Option<(u32, bool)> {
    let line = output.trim();
    let muted = line.contains("[MUTED]");
    let volume = line
        .split_whitespace()
        .nth(1)?
        .parse::<f32>()
        .ok()
        .map(|value| (value * 100.0).round() as u32)?;
    Some((volume, muted))
}

#[cfg(test)]
pub(crate) fn parse_wpctl_default_routes(
    output: &str,
) -> crate::services::controls::AudioDeviceSummary {
    let summary = parse_wpctl_route_summary(output);
    crate::services::controls::AudioDeviceSummary {
        output_route: summary.output_route,
        input_route: summary.input_route,
        ..crate::services::controls::AudioDeviceSummary::default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AudioRouteSection {
    Sinks,
    Sources,
}

impl AudioRouteSection {
    fn heading(self) -> &'static str {
        match self {
            Self::Sinks => "Sinks:",
            Self::Sources => "Sources:",
        }
    }
}

fn parse_wpctl_route_summary(output: &str) -> crate::services::controls::AudioDeviceSummary {
    let output_routes = parse_wpctl_route_entries(output, AudioRouteSection::Sinks);
    let input_routes = parse_wpctl_route_entries(output, AudioRouteSection::Sources);

    crate::services::controls::AudioDeviceSummary {
        output_route: output_routes.first().map(|route| route.name.clone()),
        input_route: input_routes.first().map(|route| route.name.clone()),
        output_routes,
        input_routes,
    }
}

fn parse_wpctl_route_entries(
    output: &str,
    section: AudioRouteSection,
) -> Vec<crate::services::controls::AudioRouteInfo> {
    let mut in_section = false;
    let mut routes = Vec::new();

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed.ends_with(section.heading()) || trimmed == section.heading() {
            in_section = true;
            continue;
        }

        if in_section {
            if trimmed.ends_with(':') && !trimmed.contains('*') {
                break;
            }

            let Some(route) = parse_wpctl_route_entry_line(line) else {
                continue;
            };
            if route.is_default {
                routes.insert(0, route.info);
            } else {
                routes.push(route.info);
            }
        }
    }

    routes
}

struct ParsedWpctlRoute {
    info: crate::services::controls::AudioRouteInfo,
    is_default: bool,
}

fn parse_wpctl_route_entry_line(line: &str) -> Option<ParsedWpctlRoute> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.ends_with(':') {
        return None;
    }

    let is_default = line.contains('*');
    let candidate = trimmed
        .trim_start_matches(['│', '├', '└', '─', ' '])
        .trim_start_matches('*')
        .trim();
    let candidate = candidate.split(" [").next().unwrap_or(candidate).trim();
    let (id, name) = candidate.split_once(". ")?;
    let id = id.trim();
    let name = name.trim();
    if id.is_empty() || name.is_empty() {
        return None;
    }

    Some(ParsedWpctlRoute {
        info: crate::services::controls::AudioRouteInfo {
            id: id.to_string(),
            name: name.to_string(),
            origin: classify_audio_route_origin(name),
        },
        is_default,
    })
}

fn classify_audio_route_origin(name: &str) -> crate::services::controls::AudioRouteOrigin {
    let lower = name.to_ascii_lowercase();
    if lower.contains("bluez")
        || lower.contains("bluetooth")
        || lower.contains("a2dp")
        || lower.contains("hfp")
    {
        crate::services::controls::AudioRouteOrigin::Bluetooth
    } else if lower.contains("usb") || lower.contains("dac") {
        crate::services::controls::AudioRouteOrigin::Usb
    } else if lower.contains("hdmi")
        || lower.contains("displayport")
        || lower.contains("display port")
        || lower.contains("dp ")
    {
        crate::services::controls::AudioRouteOrigin::Hdmi
    } else if lower.contains("monitor")
        || lower.contains("virtual")
        || lower.contains("loopback")
        || lower.contains("null")
    {
        crate::services::controls::AudioRouteOrigin::Virtual
    } else if lower.contains("speaker")
        || lower.contains("headphone")
        || lower.contains("headset")
        || lower.contains("microphone")
        || lower.contains("mic")
        || lower.contains("built-in")
        || lower.contains("internal")
    {
        crate::services::controls::AudioRouteOrigin::Internal
    } else {
        crate::services::controls::AudioRouteOrigin::Unknown
    }
}

async fn set_audio_route(id: String) -> bool {
    tokio::process::Command::new("wpctl")
        .args(["set-default", &id])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::{
        audio_param_event_label, metadata_event_label, parse_wpctl_default_routes,
        parse_wpctl_route_summary, parse_wpctl_volume, should_emit_audio_metadata_property,
        should_emit_audio_node_info, should_emit_audio_param, should_track_audio_metadata,
        should_track_audio_node, AudioEventRuntimeDiagnostics, WpctlAudioBackend,
    };

    #[test]
    fn parse_wpctl_volume_extracts_percent_and_mute_state() {
        assert_eq!(
            parse_wpctl_volume("Volume: 0.42 [MUTED]\n"),
            Some((42, true))
        );
        assert_eq!(parse_wpctl_volume("Volume: 0.73\n"), Some((73, false)));
    }

    #[test]
    fn parse_wpctl_volume_rejects_malformed_output() {
        assert_eq!(parse_wpctl_volume("garbage"), None);
    }

    #[test]
    fn parse_wpctl_default_routes_extracts_default_sink_and_source() {
        let parsed = parse_wpctl_default_routes(
            "Audio\n\
             ├─ Sinks:\n\
             │  *   52. Built-in Audio Analog Stereo [vol: 0.50]\n\
             │      77. USB Audio DAC [vol: 0.70]\n\
             ├─ Sources:\n\
             │  *   54. Internal Microphone [vol: 1.00]\n",
        );

        assert_eq!(
            parsed.output_route.as_deref(),
            Some("Built-in Audio Analog Stereo")
        );
        assert_eq!(parsed.input_route.as_deref(), Some("Internal Microphone"));
    }

    #[test]
    fn audio_node_tracking_targets_only_device_classes() {
        assert!(should_track_audio_node(Some("Audio/Sink")));
        assert!(should_track_audio_node(Some("Audio/Source")));
        assert!(should_track_audio_node(Some("Audio/Duplex")));
        assert!(!should_track_audio_node(Some("Stream/Output/Audio")));
        assert!(!should_track_audio_node(Some("Video/Source")));
        assert!(!should_track_audio_node(None));
    }

    #[test]
    fn audio_metadata_tracking_targets_default_metadata_only() {
        assert!(should_track_audio_metadata(Some("default")));
        assert!(!should_track_audio_metadata(Some("settings")));
        assert!(!should_track_audio_metadata(Some("route-settings")));
        assert!(!should_track_audio_metadata(None));
    }

    #[test]
    fn audio_metadata_property_filter_only_emits_for_default_sink_source() {
        assert!(should_emit_audio_metadata_property(None));
        assert!(should_emit_audio_metadata_property(Some(
            "default.audio.sink"
        )));
        assert!(should_emit_audio_metadata_property(Some(
            "default.audio.source"
        )));
        assert!(!should_emit_audio_metadata_property(Some("clock.quantum")));
    }

    #[test]
    fn audio_param_filter_limits_refresh_triggers_to_audio_relevant_params() {
        assert!(should_emit_audio_param(
            pipewire::spa::param::ParamType::Props
        ));
        assert!(should_emit_audio_param(
            pipewire::spa::param::ParamType::Route
        ));
        assert!(should_emit_audio_param(
            pipewire::spa::param::ParamType::Profile
        ));
        assert!(!should_emit_audio_param(
            pipewire::spa::param::ParamType::Format
        ));
    }

    #[test]
    fn node_info_filter_only_emits_for_stateful_audio_changes() {
        assert!(should_emit_audio_node_info(
            pipewire::node::NodeChangeMask::PROPS
        ));
        assert!(should_emit_audio_node_info(
            pipewire::node::NodeChangeMask::PARAMS
        ));
        assert!(should_emit_audio_node_info(
            pipewire::node::NodeChangeMask::STATE
        ));
        assert!(!should_emit_audio_node_info(
            pipewire::node::NodeChangeMask::INPUT_PORTS
        ));
    }

    #[test]
    fn event_labels_are_deterministic() {
        assert_eq!(
            metadata_event_label(Some("default.audio.sink")),
            "metadata:default.audio.sink"
        );
        assert_eq!(
            metadata_event_label(Some("default.audio.source")),
            "metadata:default.audio.source"
        );
        assert_eq!(metadata_event_label(None), "metadata:clear");
        assert_eq!(
            audio_param_event_label(pipewire::spa::param::ParamType::Route),
            "node:param-route"
        );
    }

    #[test]
    fn runtime_diagnostics_summary_reports_listener_state_and_reason() {
        let diagnostics = AudioEventRuntimeDiagnostics {
            listener_running: true,
            tracked_nodes: 3,
            tracked_metadata: 1,
            event_count: 9,
            reconnects: 2,
            last_event: Some("node:param-route".to_string()),
            last_error: Some("core:broken pipe".to_string()),
        };

        let summary = diagnostics.summary();
        assert!(summary.contains("running"));
        assert!(summary.contains("nodes:3"));
        assert!(summary.contains("meta:1"));
        assert!(summary.contains("events:9"));
        assert!(summary.contains("reconn:2"));
        assert!(summary.contains("last:node:param-route"));
        assert!(summary.contains("err:core:broken pipe"));
    }

    #[test]
    fn backend_name_reports_pipewire_event_runtime() {
        let backend = WpctlAudioBackend::default();
        assert_eq!(
            <WpctlAudioBackend as crate::services::controls_backends::AudioBackend>::backend_name(
                &backend
            ),
            "wpctl+pipewire"
        );
        assert!(
            <WpctlAudioBackend as crate::services::controls_backends::AudioBackend>::diagnostics_summary(
                &backend
            )
            .unwrap()
            .contains("stopped")
        );
    }

    #[test]
    fn parse_wpctl_default_routes_classifies_route_origins() {
        let sample = r#"Audio
 ├─ Sinks:
 │  * 52. Built-in Audio Analog Stereo [vol: 0.60]
 │    77. WH-1000XM5 a2dp-sink [vol: 0.42]
 │    88. USB Audio DAC [vol: 0.55]
 ├─ Sources:
 │  * 63. Built-in Microphone [vol: 0.70]
 │    91. USB Audio CODEC Mono [vol: 0.33]
"#;

        let routes = parse_wpctl_route_summary(sample);

        assert_eq!(
            routes.output_routes,
            vec![
                crate::services::controls::AudioRouteInfo {
                    id: "52".to_string(),
                    name: "Built-in Audio Analog Stereo".to_string(),
                    origin: crate::services::controls::AudioRouteOrigin::Internal,
                },
                crate::services::controls::AudioRouteInfo {
                    id: "77".to_string(),
                    name: "WH-1000XM5 a2dp-sink".to_string(),
                    origin: crate::services::controls::AudioRouteOrigin::Bluetooth,
                },
                crate::services::controls::AudioRouteInfo {
                    id: "88".to_string(),
                    name: "USB Audio DAC".to_string(),
                    origin: crate::services::controls::AudioRouteOrigin::Usb,
                },
            ]
        );
        assert_eq!(
            routes.input_routes,
            vec![
                crate::services::controls::AudioRouteInfo {
                    id: "63".to_string(),
                    name: "Built-in Microphone".to_string(),
                    origin: crate::services::controls::AudioRouteOrigin::Internal,
                },
                crate::services::controls::AudioRouteInfo {
                    id: "91".to_string(),
                    name: "USB Audio CODEC Mono".to_string(),
                    origin: crate::services::controls::AudioRouteOrigin::Usb,
                },
            ]
        );
    }
}
