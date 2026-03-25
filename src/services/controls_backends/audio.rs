use std::{
    cell::Cell,
    collections::{HashMap, HashSet},
    process::{Command, Stdio},
    rc::Rc,
    sync::Once,
    thread,
    time::Duration,
};

use iced::futures::SinkExt;
use pipewire as pw;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug, Default, Clone, Copy)]
pub struct WpctlAudioBackend;

static PIPEWIRE_INIT: Once = Once::new();
const PIPEWIRE_RETRY_DELAY: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy)]
enum AudioListenerCommand {
    Terminate,
}

struct PipeWireThreadGuard {
    command_tx: pw::channel::Sender<AudioListenerCommand>,
    handle: Option<std::thread::JoinHandle<()>>,
}

#[derive(Default)]
struct AudioObjectStore {
    proxies: HashMap<u32, Box<dyn pw::proxy::ProxyT>>,
    listeners: HashMap<u32, Box<dyn pw::proxy::Listener>>,
    tracked_ids: HashSet<u32>,
}

impl AudioObjectStore {
    fn insert(
        &mut self,
        id: u32,
        proxy: Box<dyn pw::proxy::ProxyT>,
        listener: Box<dyn pw::proxy::Listener>,
    ) {
        self.proxies.insert(id, proxy);
        self.listeners.insert(id, listener);
        self.tracked_ids.insert(id);
    }

    fn remove(&mut self, id: u32) -> bool {
        let removed = self.tracked_ids.remove(&id);
        self.proxies.remove(&id);
        self.listeners.remove(&id);
        removed
    }
}

impl Drop for PipeWireThreadGuard {
    fn drop(&mut self) {
        let _ = self.command_tx.send(AudioListenerCommand::Terminate);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl PipeWireThreadGuard {
    fn spawn(event_tx: UnboundedSender<crate::services::controls::ControlsEvent>) -> Self {
        let (command_tx, command_rx) = pw::channel::channel();
        let handle = thread::spawn(move || run_pipewire_audio_thread(event_tx, command_rx));
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
}

impl super::AudioBackend for WpctlAudioBackend {
    fn backend_name(&self) -> &'static str {
        "wpctl+pipewire"
    }

    fn audio_info(&self) -> crate::services::controls::AudioInfo {
        let (volume, muted) = Self::get_volume("@DEFAULT_AUDIO_SINK@").unwrap_or((0, false));
        crate::services::controls::AudioInfo { volume, muted }
    }

    fn mic_info(&self) -> crate::modules::mic::MicInfo {
        let (volume, muted) = Self::get_volume("@DEFAULT_AUDIO_SOURCE@").unwrap_or((0, false));
        crate::modules::mic::MicInfo { volume, muted }
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

    fn subscription(&self) -> iced::Subscription<crate::services::controls::ControlsEvent> {
        struct AudioListener;
        iced::Subscription::run_with_id(
            std::any::TypeId::of::<AudioListener>(),
            iced::stream::channel(1, |mut output| async move {
                let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();
                let _thread_guard = PipeWireThreadGuard::spawn(event_tx);

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
) {
    ensure_pipewire_initialized();

    loop {
        let (receiver, terminated) = match run_pipewire_audio_session(event_tx.clone(), command_rx)
        {
            Ok(result) => result,
            Err((receiver, error)) => {
                command_rx = receiver;
                tracing::debug!("pipewire audio listener setup failed: {error}");
                thread::sleep(PIPEWIRE_RETRY_DELAY);
                continue;
            }
        };

        command_rx = receiver;
        if terminated {
            break;
        }

        tracing::debug!("pipewire audio listener reconnecting");
        thread::sleep(PIPEWIRE_RETRY_DELAY);
    }
}

fn run_pipewire_audio_session(
    event_tx: UnboundedSender<crate::services::controls::ControlsEvent>,
    command_rx: pw::channel::Receiver<AudioListenerCommand>,
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
            let main_loop = main_loop.clone();
            move |id, _seq, _res, message| {
                tracing::debug!("pipewire audio listener core error on id {id}: {message}");
                if id == 0 {
                    main_loop.quit();
                }
            }
        })
        .register();

    let _registry_listener = registry
        .add_listener_local()
        .global({
            let event_tx = event_tx.clone();
            let object_store = object_store.clone();
            let registry = registry.clone();
            move |obj| match obj.type_ {
                pw::types::ObjectType::Node => {
                    if !should_track_audio_node(dict_value(obj.props, *pw::keys::MEDIA_CLASS)) {
                        return;
                    }

                    let Ok(node) = registry.bind::<pw::node::Node, _>(obj) else {
                        return;
                    };
                    node.subscribe_params(&[
                        pw::spa::param::ParamType::Props,
                        pw::spa::param::ParamType::Route,
                        pw::spa::param::ParamType::Profile,
                    ]);

                    let listener = node
                        .add_listener_local()
                        .param({
                            let event_tx = event_tx.clone();
                            move |_seq, id, _index, _next, _param| {
                                if should_emit_audio_param(id) {
                                    let _ = event_tx.send(
                                        crate::services::controls::ControlsEvent::AudioServerChanged,
                                    );
                                }
                            }
                        })
                        .register();

                    object_store
                        .borrow_mut()
                        .insert(obj.id, Box::new(node), Box::new(listener));
                }
                pw::types::ObjectType::Metadata => {
                    if !should_track_audio_metadata(dict_value(obj.props, "metadata.name")) {
                        return;
                    }

                    let Ok(metadata) = registry.bind::<pw::metadata::Metadata, _>(obj) else {
                        return;
                    };
                    let listener = metadata
                        .add_listener_local()
                        .property({
                            let event_tx = event_tx.clone();
                            move |_subject, key, _type, _value| {
                                if should_emit_audio_metadata_property(key) {
                                    let _ = event_tx.send(
                                        crate::services::controls::ControlsEvent::AudioServerChanged,
                                    );
                                }
                                0
                            }
                        })
                        .register();

                    object_store
                        .borrow_mut()
                        .insert(obj.id, Box::new(metadata), Box::new(listener));
                }
                _ => {}
            }
        })
        .global_remove({
            let event_tx = event_tx.clone();
            let object_store = object_store.clone();
            move |id| {
                if object_store.borrow_mut().remove(id) {
                    let _ = event_tx.send(crate::services::controls::ControlsEvent::AudioServerChanged);
                }
            }
        })
        .register();

    main_loop.run();
    Ok((attached_command_rx.deattach(), should_terminate.get()))
}

fn ensure_pipewire_initialized() {
    PIPEWIRE_INIT.call_once(pw::init);
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

fn should_emit_audio_param(id: pw::spa::param::ParamType) -> bool {
    matches!(
        id,
        pw::spa::param::ParamType::Props
            | pw::spa::param::ParamType::Route
            | pw::spa::param::ParamType::Profile
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
mod tests {
    use super::{
        parse_wpctl_volume, should_emit_audio_metadata_property, should_emit_audio_param,
        should_track_audio_metadata, should_track_audio_node, WpctlAudioBackend,
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
    fn backend_name_reports_pipewire_event_runtime() {
        let backend = WpctlAudioBackend;
        assert_eq!(
            <WpctlAudioBackend as crate::services::controls_backends::AudioBackend>::backend_name(
                &backend
            ),
            "wpctl+pipewire"
        );
    }
}
