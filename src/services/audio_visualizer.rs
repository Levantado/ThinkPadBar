use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{Arc, Mutex, Once},
    thread,
    time::{Duration, Instant},
};

use iced::futures::SinkExt;
use pipewire as pw;
use pw::{
    properties::properties,
    spa::{self, pod::Pod},
};
use tokio::sync::mpsc::UnboundedSender;

use serde_json::Value;

static PIPEWIRE_INIT: Once = Once::new();
const PIPEWIRE_RETRY_DELAY: Duration = Duration::from_secs(2);
const VISUALIZER_MAX_BARS: usize = 24;
const VISUALIZER_WINDOW_SIZE: usize = 512;
const VISUALIZER_SILENCE_THRESHOLD: f32 = 0.002;

type SharedDiagnostics = Arc<Mutex<AudioVisualizerDiagnostics>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioVisualizerSnapshot {
    pub bars: [u8; VISUALIZER_MAX_BARS],
    pub visible_bars: u8,
    pub active: bool,
}

impl Default for AudioVisualizerSnapshot {
    fn default() -> Self {
        Self {
            bars: [0; VISUALIZER_MAX_BARS],
            visible_bars: 16,
            active: false,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AudioVisualizerDiagnostics {
    pub listener_running: bool,
    pub sample_rate: u32,
    pub visible_bars: u8,
    pub fps: u8,
    pub event_count: u64,
    pub published_frames: u64,
    pub reconnects: u64,
    pub last_rms_percent: u8,
    pub target_device: Option<String>,
    pub last_error: Option<String>,
}

impl AudioVisualizerDiagnostics {
    pub fn summary(&self) -> String {
        format!(
            "{} rate:{} bars:{} fps:{} events:{} pub:{} lvl:{} reconn:{} target:{}{}",
            if self.listener_running {
                "running"
            } else {
                "stopped"
            },
            self.sample_rate,
            self.visible_bars,
            self.fps,
            self.event_count,
            self.published_frames,
            self.last_rms_percent,
            self.reconnects,
            self.target_device.as_deref().unwrap_or("default"),
            self.last_error
                .as_ref()
                .map(|error| format!(" err:{error}"))
                .unwrap_or_default()
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioVisualizerConfig {
    pub enabled: bool,
    pub bars: usize,
    pub min_height: f32,
    pub max_height: f32,
    pub bar_width: f32,
    pub gap: u16,
    pub publish_interval: Duration,
    pub min_freq_hz: f32,
    pub max_freq_hz: f32,
    pub color_profile: VisualizerColorProfile,
    pub smoothing: VisualizerSmoothing,
}

impl AudioVisualizerConfig {
    pub fn from_appearance(config: &crate::config::AudioVisualizerConfig) -> Self {
        let fps = config.normalized_fps();
        Self {
            enabled: config.enabled,
            bars: config.normalized_bars(),
            min_height: config.normalized_min_height(),
            max_height: config.normalized_max_height(),
            bar_width: config.normalized_bar_width(),
            gap: config.normalized_gap(),
            publish_interval: Duration::from_millis(1_000 / u64::from(fps)),
            min_freq_hz: config.normalized_min_freq_hz(),
            max_freq_hz: config.normalized_max_freq_hz(),
            color_profile: VisualizerColorProfile::from_config(config.normalized_color_profile()),
            smoothing: VisualizerSmoothing::from_profile(config.normalized_decay_profile()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualizerColorProfile {
    Heat,
    Accent,
    Mono,
}

impl VisualizerColorProfile {
    pub fn from_config(profile: &str) -> Self {
        match profile {
            "accent" => Self::Accent,
            "mono" => Self::Mono,
            _ => Self::Heat,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VisualizerSmoothing {
    pub rise_mix: f32,
    pub decay_mix: f32,
}

impl VisualizerSmoothing {
    pub fn from_profile(profile: &str) -> Self {
        match profile {
            "tight" => Self {
                rise_mix: 0.78,
                decay_mix: 0.18,
            },
            "expressive" => Self {
                rise_mix: 0.58,
                decay_mix: 0.04,
            },
            _ => Self {
                rise_mix: 0.62,
                decay_mix: 0.10,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct AudioVisualizerService {
    config: AudioVisualizerConfig,
    diagnostics: SharedDiagnostics,
}

impl AudioVisualizerService {
    pub fn new(config: AudioVisualizerConfig) -> Self {
        let fps = (1_000 / config.publish_interval.as_millis().max(1)) as u8;
        let visible_bars = config.bars as u8;
        Self {
            config,
            diagnostics: Arc::new(Mutex::new(AudioVisualizerDiagnostics {
                visible_bars,
                fps,
                ..AudioVisualizerDiagnostics::default()
            })),
        }
    }

    pub fn diagnostics_summary(&self) -> Option<String> {
        self.diagnostics.lock().ok().map(|state| state.summary())
    }

    pub fn subscription(&self) -> iced::Subscription<AudioVisualizerSnapshot> {
        if !self.config.enabled {
            return iced::Subscription::none();
        }

        struct AudioVisualizerListener;
        let diagnostics = self.diagnostics.clone();
        let config = self.config.clone();
        iced::Subscription::run_with_id(
            std::any::TypeId::of::<AudioVisualizerListener>(),
            iced::stream::channel(1, move |mut output| async move {
                let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();
                let _thread_guard = PipeWireThreadGuard::spawn(event_tx, diagnostics, config);

                while let Some(snapshot) = event_rx.recv().await {
                    let _ = output.send(snapshot).await;
                }
            }),
        )
    }
}

#[derive(Debug, Clone, Copy)]
enum VisualizerListenerCommand {
    Terminate,
}

struct PipeWireThreadGuard {
    command_tx: pw::channel::Sender<VisualizerListenerCommand>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Drop for PipeWireThreadGuard {
    fn drop(&mut self) {
        let _ = self.command_tx.send(VisualizerListenerCommand::Terminate);
        self.handle.take();
    }
}

impl PipeWireThreadGuard {
    fn spawn(
        snapshot_tx: UnboundedSender<AudioVisualizerSnapshot>,
        diagnostics: SharedDiagnostics,
        config: AudioVisualizerConfig,
    ) -> Self {
        let (command_tx, command_rx) = pw::channel::channel();
        let handle = thread::spawn(move || {
            run_pipewire_visualizer_thread(snapshot_tx, command_rx, diagnostics, config)
        });
        Self {
            command_tx,
            handle: Some(handle),
        }
    }
}

#[derive(Debug)]
struct AnalyzerState {
    config: AudioVisualizerConfig,
    sample_rate: u32,
    ring: [f32; VISUALIZER_WINDOW_SIZE],
    write_index: usize,
    filled: usize,
    smoothed_bars: [f32; VISUALIZER_MAX_BARS],
    last_publish: Instant,
}

impl AnalyzerState {
    fn new(config: AudioVisualizerConfig) -> Self {
        let publish_interval = config.publish_interval;
        Self {
            config,
            sample_rate: 48_000,
            ring: [0.0; VISUALIZER_WINDOW_SIZE],
            write_index: 0,
            filled: 0,
            smoothed_bars: [0.0; VISUALIZER_MAX_BARS],
            last_publish: Instant::now()
                .checked_sub(publish_interval)
                .unwrap_or_else(Instant::now),
        }
    }
    fn set_sample_rate(&mut self, sample_rate: u32) {
        self.sample_rate = sample_rate.max(1);
    }

    fn push_interleaved_f32(
        &mut self,
        bytes: &[u8],
        channels: usize,
    ) -> Option<(AudioVisualizerSnapshot, u8)> {
        if channels == 0 {
            return None;
        }

        let samples = bytes.chunks_exact(std::mem::size_of::<f32>());
        if samples.len() < channels {
            return None;
        }

        let mut mono_acc = 0.0_f32;
        let mut channel_index = 0_usize;

        for sample in samples {
            let value = f32::from_le_bytes(sample.try_into().ok()?);
            mono_acc += value;
            channel_index += 1;
            if channel_index == channels {
                let mono = mono_acc / channels as f32;
                self.ring[self.write_index] = mono;
                self.write_index = (self.write_index + 1) % VISUALIZER_WINDOW_SIZE;
                self.filled = self.filled.saturating_add(1).min(VISUALIZER_WINDOW_SIZE);
                mono_acc = 0.0;
                channel_index = 0;
            }
        }

        if self.filled < VISUALIZER_WINDOW_SIZE
            || self.last_publish.elapsed() < self.config.publish_interval
        {
            return None;
        }

        self.last_publish = Instant::now();
        let mut ordered = [0.0_f32; VISUALIZER_WINDOW_SIZE];
        let split = VISUALIZER_WINDOW_SIZE - self.write_index;
        ordered[..split].copy_from_slice(&self.ring[self.write_index..]);
        ordered[split..].copy_from_slice(&self.ring[..self.write_index]);

        let rms = root_mean_square(&ordered);
        let bars = analyze_bands(&ordered, self.sample_rate, &self.config);
        let active = bars[..self.config.bars].iter().any(|bar| *bar > 0);
        let snapshot = smooth_bars(
            &mut self.smoothed_bars,
            &bars,
            active,
            self.config.bars,
            self.config.smoothing,
        );
        Some((snapshot, (rms * 100.0).clamp(0.0, 100.0).round() as u8))
    }
}

struct VisualizerUserData {
    format: spa::param::audio::AudioInfoRaw,
    analyzer: AnalyzerState,
    diagnostics: SharedDiagnostics,
    snapshot_tx: UnboundedSender<AudioVisualizerSnapshot>,
}

fn run_pipewire_visualizer_thread(
    snapshot_tx: UnboundedSender<AudioVisualizerSnapshot>,
    mut command_rx: pw::channel::Receiver<VisualizerListenerCommand>,
    diagnostics: SharedDiagnostics,
    config: AudioVisualizerConfig,
) {
    ensure_pipewire_initialized();

    loop {
        let (receiver, terminated) = match run_pipewire_visualizer_session(
            snapshot_tx.clone(),
            command_rx,
            diagnostics.clone(),
            config.clone(),
        ) {
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
        thread::sleep(Duration::from_millis(50));
    }
}

fn set_target_device(diagnostics: &SharedDiagnostics, target: Option<String>) {
    if let Ok(mut state) = diagnostics.lock() {
        state.target_device = target;
    }
}

fn get_target_device(diagnostics: &SharedDiagnostics) -> Option<String> {
    diagnostics
        .lock()
        .ok()
        .and_then(|state| state.target_device.clone())
}

fn run_pipewire_visualizer_session(
    snapshot_tx: UnboundedSender<AudioVisualizerSnapshot>,
    command_rx: pw::channel::Receiver<VisualizerListenerCommand>,
    diagnostics: SharedDiagnostics,
    config: AudioVisualizerConfig,
) -> Result<
    (pw::channel::Receiver<VisualizerListenerCommand>, bool),
    (pw::channel::Receiver<VisualizerListenerCommand>, String),
> {
    let main_loop = match pw::main_loop::MainLoopRc::new(None) {
        Ok(main_loop) => main_loop,
        Err(error) => return Err((command_rx, error.to_string())),
    };
    let should_terminate = Rc::new(Cell::new(false));
    let should_restart = Rc::new(Cell::new(false));

    let attached_command_rx = command_rx.attach(main_loop.loop_(), {
        let main_loop = main_loop.clone();
        let should_terminate = should_terminate.clone();
        move |command| {
            if matches!(command, VisualizerListenerCommand::Terminate) {
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

    let target_device = get_target_device(&diagnostics);

    let mut props = properties! {
        *pw::keys::MEDIA_TYPE => "Audio",
        *pw::keys::MEDIA_CATEGORY => "Capture",
        *pw::keys::MEDIA_ROLE => "Music",
        *pw::keys::STREAM_CAPTURE_SINK => "true",
        *pw::keys::NODE_NAME => "thinkpadbar-audio-visualizer",
    };

    if let Some(ref target) = target_device {
        props.insert("target.object", target.clone());
    }

    let stream = match pw::stream::StreamBox::new(&core, "thinkpadbar-visualizer", props) {
        Ok(stream) => stream,
        Err(error) => return Err((attached_command_rx.deattach(), error.to_string())),
    };

    let user_data = VisualizerUserData {
        format: Default::default(),
        analyzer: AnalyzerState::new(config),
        diagnostics: diagnostics.clone(),
        snapshot_tx,
    };

    let _listener = match stream
        .add_local_listener_with_user_data(user_data)
        .param_changed(|_, user_data, id, param| {
            let Some(param) = param else {
                return;
            };
            if id != pw::spa::param::ParamType::Format.as_raw() {
                return;
            }

            let (media_type, media_subtype) = match spa::param::format_utils::parse_format(param) {
                Ok(value) => value,
                Err(_) => return,
            };
            if media_type != spa::param::format::MediaType::Audio
                || media_subtype != spa::param::format::MediaSubtype::Raw
            {
                return;
            }

            if user_data.format.parse(param).is_ok() {
                let sample_rate = user_data.format.rate().max(1);
                user_data.analyzer.set_sample_rate(sample_rate);
                set_sample_rate(&user_data.diagnostics, sample_rate);
            }
        })
        .process(|stream, user_data| {
            let Some(mut buffer) = stream.dequeue_buffer() else {
                return;
            };
            let datas = buffer.datas_mut();
            if datas.is_empty() {
                return;
            }

            let channels = user_data.format.channels().max(1) as usize;
            let data = &mut datas[0];
            let chunk = data.chunk();
            let offset = chunk.offset() as usize;
            let size = chunk.size() as usize;
            let Some(bytes) = data.data() else {
                return;
            };
            if offset + size > bytes.len() {
                return;
            }
            let valid_bytes = &bytes[offset..offset + size];
            let Some((snapshot, rms_percent)) = user_data
                .analyzer
                .push_interleaved_f32(valid_bytes, channels)
            else {
                return;
            };
            note_event(&user_data.diagnostics);
            note_publish(&user_data.diagnostics, rms_percent);
            let _ = user_data.snapshot_tx.send(snapshot);
        })
        .register()
    {
        Ok(listener) => listener,
        Err(error) => return Err((attached_command_rx.deattach(), error.to_string())),
    };

    let registry = match core.get_registry_rc() {
        Ok(registry) => registry,
        Err(error) => return Err((attached_command_rx.deattach(), error.to_string())),
    };
    let metadata_handle = Rc::new(RefCell::new(None::<pw::metadata::Metadata>));
    let metadata_listener_handle = Rc::new(RefCell::new(None::<pw::metadata::MetadataListener>));

    let _registry_handle = registry
        .add_listener_local()
        .global({
            let metadata_handle = metadata_handle.clone();
            let metadata_listener_handle = metadata_listener_handle.clone();
            let registry = registry.clone();
            let diag = diagnostics.clone();
            let ml = main_loop.clone();
            let sr = should_restart.clone();
            let current_target = target_device.clone();

            move |obj| {
                if obj.type_ == pw::types::ObjectType::Metadata {
                    let name: Option<&str> = obj.props.and_then(|props| props.get("metadata.name"));
                    if name == Some("default") {
                        let Ok(metadata) = registry.bind::<pw::metadata::Metadata, _>(obj) else {
                            return;
                        };
                        let diag = diag.clone();
                        let ml = ml.clone();
                        let sr = sr.clone();
                        let current_target = current_target.clone();

                        let listener = metadata
                            .add_listener_local()
                            .property(move |id, key, _, value| {
                                if id == 0 && key == Some("default.audio.sink") {
                                    if let Some(v) = value {
                                        let name = serde_json::from_str::<Value>(v)
                                            .ok()
                                            .and_then(|v| v["name"].as_str().map(|s| s.to_string()))
                                            .unwrap_or_else(|| v.to_string());

                                        if !name.is_empty()
                                            && Some(&name) != current_target.as_ref()
                                        {
                                            set_target_device(&diag, Some(name));
                                            sr.set(true);
                                            ml.quit();
                                        }
                                    }
                                }
                                0
                            })
                            .register();

                        *metadata_listener_handle.borrow_mut() = Some(listener);
                        *metadata_handle.borrow_mut() = Some(metadata);
                    }
                }
            }
        })
        .register();

    let mut audio_info = spa::param::audio::AudioInfoRaw::new();
    audio_info.set_format(spa::param::audio::AudioFormat::F32LE);
    let obj = spa::pod::Object {
        type_: spa::utils::SpaTypes::ObjectParamFormat.as_raw(),
        id: spa::param::ParamType::EnumFormat.as_raw(),
        properties: audio_info.into(),
    };
    let values: Vec<u8> = match spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &spa::pod::Value::Object(obj),
    ) {
        Ok(value) => value.0.into_inner(),
        Err(error) => return Err((attached_command_rx.deattach(), error.to_string())),
    };

    let pod = match Pod::from_bytes(&values) {
        Some(pod) => pod,
        None => {
            return Err((
                attached_command_rx.deattach(),
                "pod-from-bytes-failed".to_string(),
            ))
        }
    };
    let mut params = [pod];

    if let Err(error) = stream.connect(
        spa::utils::Direction::Input,
        None,
        pw::stream::StreamFlags::AUTOCONNECT
            | pw::stream::StreamFlags::MAP_BUFFERS
            | pw::stream::StreamFlags::RT_PROCESS,
        &mut params,
    ) {
        return Err((attached_command_rx.deattach(), error.to_string()));
    }

    set_listener_running(&diagnostics, true);
    clear_last_error(&diagnostics);
    main_loop.run();

    let terminated = should_terminate.get();
    let restarting = should_restart.get();

    Ok((attached_command_rx.deattach(), terminated && !restarting))
}

fn analyze_bands(
    samples: &[f32; VISUALIZER_WINDOW_SIZE],
    sample_rate: u32,
    config: &AudioVisualizerConfig,
) -> [u8; VISUALIZER_MAX_BARS] {
    let rms = root_mean_square(samples);
    if rms <= VISUALIZER_SILENCE_THRESHOLD || sample_rate == 0 {
        return [0; VISUALIZER_MAX_BARS];
    }

    let loudness = ((rms - VISUALIZER_SILENCE_THRESHOLD) / 0.18)
        .clamp(0.0, 1.0)
        .sqrt();
    let mut energies = [0.0_f32; VISUALIZER_MAX_BARS];
    let mut max_energy = 0.0_f32;
    let nyquist = sample_rate as f32 / 2.0;

    for (index, energy) in energies.iter_mut().take(config.bars).enumerate() {
        let normalized = index as f32 / (config.bars.saturating_sub(1).max(1) as f32);
        let freq = config.min_freq_hz * (config.max_freq_hz / config.min_freq_hz).powf(normalized);
        let clamped_freq = freq.min(nyquist * 0.92).max(config.min_freq_hz);
        *energy = goertzel_energy(samples, sample_rate as f32, clamped_freq).sqrt();
        max_energy = max_energy.max(*energy);
    }

    if max_energy <= f32::EPSILON {
        return [0; VISUALIZER_MAX_BARS];
    }

    let mut bars = [0_u8; VISUALIZER_MAX_BARS];
    for (index, energy) in energies.iter().take(config.bars).enumerate() {
        let emphasis = 0.85 + normalized_band_position(index, config.bars) * 0.35;
        let normalized = ((*energy / max_energy) * loudness * emphasis)
            .clamp(0.0, 1.0)
            .powf(0.75);
        bars[index] = (normalized * 100.0).round() as u8;
    }
    bars
}

fn root_mean_square(samples: &[f32; VISUALIZER_WINDOW_SIZE]) -> f32 {
    (samples.iter().map(|sample| sample * sample).sum::<f32>() / samples.len() as f32).sqrt()
}

fn goertzel_energy(
    samples: &[f32; VISUALIZER_WINDOW_SIZE],
    sample_rate: f32,
    target_freq: f32,
) -> f32 {
    let sample_count = samples.len() as f32;
    let k = ((sample_count * target_freq) / sample_rate).round();
    let omega = (2.0 * std::f32::consts::PI * k) / sample_count;
    let coeff = 2.0 * omega.cos();
    let mut q1 = 0.0_f32;
    let mut q2 = 0.0_f32;

    for sample in samples {
        let q0 = coeff * q1 - q2 + *sample;
        q2 = q1;
        q1 = q0;
    }

    (q1 * q1 + q2 * q2 - q1 * q2 * coeff).max(0.0)
}

fn smooth_bars(
    smoothed_bars: &mut [f32; VISUALIZER_MAX_BARS],
    target_bars: &[u8; VISUALIZER_MAX_BARS],
    active: bool,
    visible_bars: usize,
    smoothing: VisualizerSmoothing,
) -> AudioVisualizerSnapshot {
    let mut bars = [0_u8; VISUALIZER_MAX_BARS];
    let mut any_visible = false;

    for (index, smoothed) in smoothed_bars.iter_mut().enumerate().take(visible_bars) {
        let target = if active {
            f32::from(target_bars[index]) / 100.0
        } else {
            0.0
        };
        let next = if target >= *smoothed {
            *smoothed * (1.0 - smoothing.rise_mix) + target * smoothing.rise_mix
        } else {
            *smoothed * (1.0 - smoothing.decay_mix) + target * smoothing.decay_mix
        };
        *smoothed = next.clamp(0.0, 1.0);
        bars[index] = (*smoothed * 100.0).round() as u8;
        any_visible |= bars[index] > 0;
    }

    AudioVisualizerSnapshot {
        bars,
        visible_bars: visible_bars as u8,
        active: active && any_visible,
    }
}

fn normalized_band_position(index: usize, visible_bars: usize) -> f32 {
    if visible_bars <= 1 {
        return 0.0;
    }
    index as f32 / (visible_bars - 1) as f32
}

fn ensure_pipewire_initialized() {
    PIPEWIRE_INIT.call_once(pw::init);
}

fn set_listener_running(diagnostics: &SharedDiagnostics, running: bool) {
    if let Ok(mut state) = diagnostics.lock() {
        state.listener_running = running;
    }
}

fn clear_last_error(diagnostics: &SharedDiagnostics) {
    if let Ok(mut state) = diagnostics.lock() {
        state.last_error = None;
    }
}

fn note_error(diagnostics: &SharedDiagnostics, error: String) {
    if let Ok(mut state) = diagnostics.lock() {
        state.listener_running = false;
        state.last_error = Some(error);
    }
}

fn note_reconnect(diagnostics: &SharedDiagnostics) {
    if let Ok(mut state) = diagnostics.lock() {
        state.reconnects = state.reconnects.saturating_add(1);
    }
}

fn note_event(diagnostics: &SharedDiagnostics) {
    if let Ok(mut state) = diagnostics.lock() {
        state.event_count = state.event_count.saturating_add(1);
    }
}

fn note_publish(diagnostics: &SharedDiagnostics, rms_percent: u8) {
    if let Ok(mut state) = diagnostics.lock() {
        state.published_frames = state.published_frames.saturating_add(1);
        state.last_rms_percent = rms_percent;
    }
}

fn set_sample_rate(diagnostics: &SharedDiagnostics, sample_rate: u32) {
    if let Ok(mut state) = diagnostics.lock() {
        state.sample_rate = sample_rate;
    }
}

#[cfg(test)]
mod tests {
    use super::{
        analyze_bands, smooth_bars, AudioVisualizerConfig, VisualizerColorProfile,
        VisualizerSmoothing, VISUALIZER_MAX_BARS, VISUALIZER_WINDOW_SIZE,
    };
    use std::time::Duration;

    fn test_config() -> AudioVisualizerConfig {
        AudioVisualizerConfig {
            enabled: true,
            bars: 16,
            min_height: 4.0,
            max_height: 18.0,
            bar_width: 3.0,
            gap: 2,
            publish_interval: Duration::from_millis(42),
            min_freq_hz: 60.0,
            max_freq_hz: 12_000.0,
            color_profile: VisualizerColorProfile::Heat,
            smoothing: VisualizerSmoothing::from_profile("smooth"),
        }
    }

    fn sine_wave(freq_hz: f32, sample_rate: u32) -> [f32; VISUALIZER_WINDOW_SIZE] {
        let mut samples = [0.0_f32; VISUALIZER_WINDOW_SIZE];
        for (index, sample) in samples.iter_mut().enumerate() {
            let t = index as f32 / sample_rate as f32;
            *sample = (2.0 * std::f32::consts::PI * freq_hz * t).sin() * 0.3;
        }
        samples
    }

    #[test]
    fn analyzer_returns_zero_for_silence() {
        let samples = [0.0_f32; VISUALIZER_WINDOW_SIZE];
        assert_eq!(
            analyze_bands(&samples, 48_000, &test_config()),
            [0; VISUALIZER_MAX_BARS]
        );
    }

    #[test]
    fn analyzer_emphasizes_lower_bands_for_low_tone() {
        let bars = analyze_bands(&sine_wave(120.0, 48_000), 48_000, &test_config());
        let low_peak = bars[..4].iter().copied().max().unwrap_or(0);
        let high_peak = bars[12..].iter().copied().max().unwrap_or(0);
        assert!(
            low_peak > high_peak,
            "expected low bands to dominate: {bars:?}"
        );
    }

    #[test]
    fn analyzer_emphasizes_upper_bands_for_high_tone() {
        let bars = analyze_bands(&sine_wave(5_000.0, 48_000), 48_000, &test_config());
        let low_peak = bars[..4].iter().copied().max().unwrap_or(0);
        let high_peak = bars[10..].iter().copied().max().unwrap_or(0);
        assert!(
            high_peak > low_peak,
            "expected high bands to dominate: {bars:?}"
        );
    }

    #[test]
    fn smoothing_preserves_decay_visibility() {
        let mut smoothed = [0.0_f32; VISUALIZER_MAX_BARS];
        let _loud = smooth_bars(
            &mut smoothed,
            &[100; VISUALIZER_MAX_BARS],
            true,
            16,
            VisualizerSmoothing::from_profile("smooth"),
        );
        let decay = smooth_bars(
            &mut smoothed,
            &[0; VISUALIZER_MAX_BARS],
            false,
            16,
            VisualizerSmoothing::from_profile("smooth"),
        );
        assert!(decay.bars.iter().any(|bar| *bar > 0));
    }

    #[test]
    fn expressive_decay_holds_signal_longer_than_tight_decay() {
        let mut expressive = [0.8_f32; VISUALIZER_MAX_BARS];
        let mut tight = [0.8_f32; VISUALIZER_MAX_BARS];

        let expressive_snapshot = smooth_bars(
            &mut expressive,
            &[0; VISUALIZER_MAX_BARS],
            false,
            16,
            VisualizerSmoothing::from_profile("expressive"),
        );
        let tight_snapshot = smooth_bars(
            &mut tight,
            &[0; VISUALIZER_MAX_BARS],
            false,
            16,
            VisualizerSmoothing::from_profile("tight"),
        );

        // Expressive has decay_mix 0.04, tight has 0.18
        // expressive_new = 0.8 * (1.0 - 0.04) = 0.768
        // tight_new = 0.8 * (1.0 - 0.18) = 0.656
        assert!(expressive_snapshot.bars[0] > tight_snapshot.bars[0]);
    }

    #[test]
    fn diagnostics_summary_reports_runtime_shape() {
        let diagnostics = super::AudioVisualizerDiagnostics {
            listener_running: true,
            sample_rate: 48_000,
            visible_bars: 20,
            fps: 24,
            event_count: 5,
            published_frames: 3,
            last_rms_percent: 27,
            reconnects: 1,
            ..super::AudioVisualizerDiagnostics::default()
        };
        assert!(diagnostics.summary().contains("bars:20"));
        assert!(diagnostics.summary().contains("pub:3"));
        assert!(diagnostics.summary().contains("lvl:27"));
    }
}
