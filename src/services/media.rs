use futures_util::SinkExt;
use futures_util::StreamExt;
use std::collections::{HashMap, HashSet};
use std::future::pending;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;
use zbus::{fdo, proxy, Connection};

#[proxy(
    interface = "org.mpris.MediaPlayer2.Player",
    default_path = "/org/mpris/MediaPlayer2"
)]
trait Player {
    fn next(&self) -> zbus::Result<()>;
    fn previous(&self) -> zbus::Result<()>;
    fn play_pause(&self) -> zbus::Result<()>;
    fn stop(&self) -> zbus::Result<()>;
    fn set_position(
        &self,
        track_id: zbus::zvariant::ObjectPath<'_>,
        position: i64,
    ) -> zbus::Result<()>;

    #[zbus(property)]
    fn playback_status(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn metadata(&self) -> zbus::Result<HashMap<String, zbus::zvariant::OwnedValue>>;

    #[zbus(property)]
    fn volume(&self) -> zbus::Result<f64>;

    #[zbus(property, name = "Volume")]
    fn set_volume(&self, volume: f64) -> zbus::Result<()>;

    #[zbus(property)]
    fn position(&self) -> zbus::Result<i64>;

    #[zbus(property)]
    fn can_control(&self) -> zbus::Result<bool>;
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MediaSnapshot {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub track_id: String,
    pub cover_url: Option<String>,
    pub playback_status: String,
    pub volume: f64,
    pub position: i64, // microseconds
    pub duration: i64, // microseconds
    pub player_name: String,
    pub has_player: bool,
    pub volume_supported: bool,
    pub cover_bytes: Option<Arc<Vec<u8>>>,
}

#[derive(Debug, Clone)]
pub enum MediaEvent {
    SnapshotUpdated(MediaSnapshot),
}

#[derive(Debug, Clone)]
pub enum MediaCommand {
    Next,
    Previous,
    PlayPause,
    Stop,
    SetVolume(f64),
    Seek(i64),
}

#[derive(Debug, Clone, Copy)]
enum PlayerSignal {
    Metadata,
    Volume,
    PlaybackStatus,
}

pub struct MediaService {
    event_tx: broadcast::Sender<MediaEvent>,
    cmd_rx: mpsc::Receiver<MediaCommand>,
    http_client: reqwest::Client,
    cover_cache: Option<(String, Arc<Vec<u8>>)>,
    unsupported_player_volume: HashSet<String>,
}

impl MediaService {
    pub fn new() -> (
        Self,
        broadcast::Receiver<MediaEvent>,
        mpsc::Sender<MediaCommand>,
    ) {
        let (event_tx, event_rx) = broadcast::channel(16);
        let (cmd_tx, cmd_rx) = mpsc::channel(16);
        (
            Self {
                event_tx,
                cmd_rx,
                http_client: reqwest::Client::new(),
                cover_cache: None,
                unsupported_player_volume: HashSet::new(),
            },
            event_rx,
            cmd_tx,
        )
    }

    pub fn subscription(
        event_rx: broadcast::Receiver<MediaEvent>,
    ) -> iced::Subscription<MediaEvent> {
        struct MediaListener;
        iced::Subscription::run_with_id(
            std::any::TypeId::of::<MediaListener>(),
            iced::stream::channel(1, move |mut output| async move {
                let mut rx = event_rx;
                while let Ok(event) = rx.recv().await {
                    let _ = output.send(event).await;
                }
            }),
        )
    }

    pub async fn run(mut self) -> zbus::Result<()> {
        let conn = Connection::session().await?;
        let dbus = fdo::DBusProxy::new(&conn).await?;
        let mut owner_changes = dbus.receive_name_owner_changed().await?;

        let mut active_player: Option<(String, PlayerProxy<'static>)> = None;
        let mut snapshot = MediaSnapshot::default();
        let mut read_volume_scale = VolumeScale::Unit;
        let mut write_volume_scale = VolumeScale::Unit;
        let mut time_scale = TimeScale::Micros;
        let mut pending_volume: Option<PendingVolumeWrite> = None;
        let mut last_position_tick = Instant::now();
        let mut signal_rx: Option<mpsc::Receiver<PlayerSignal>> = None;
        let mut signal_tasks: Vec<JoinHandle<()>> = Vec::new();

        let cleanup_signals =
            |tasks: &mut Vec<JoinHandle<()>>, rx: &mut Option<mpsc::Receiver<PlayerSignal>>| {
                for task in tasks.drain(..) {
                    task.abort();
                }
                *rx = None;
            };

        let attach_player =
            async |name: String,
                   proxy: PlayerProxy<'static>,
                   snapshot_in: MediaSnapshot,
                   read_scale: VolumeScale,
                   write_scale: &mut VolumeScale,
                   pending: &mut Option<PendingVolumeWrite>,
                   last_tick: &mut Instant,
                   active: &mut Option<(String, PlayerProxy<'static>)>,
                   rx: &mut Option<mpsc::Receiver<PlayerSignal>>,
                   tasks: &mut Vec<JoinHandle<()>>,
                   event_tx: &broadcast::Sender<MediaEvent>| {
                cleanup_signals(tasks, rx);
                let (new_rx, new_tasks) = Self::spawn_player_signal_tasks(proxy.clone()).await;
                *rx = Some(new_rx);
                *tasks = new_tasks;
                *active = Some((name, proxy));
                *write_scale = read_scale;
                *pending = None;
                *last_tick = Instant::now();
                let _ = event_tx.send(MediaEvent::SnapshotUpdated(snapshot_in));
            };

        // Initial player discovery: choose the most relevant active candidate.
        if let Some((name, proxy, new_snapshot)) = self
            .choose_best_player(&conn, &dbus, &mut read_volume_scale, &mut time_scale)
            .await
        {
            snapshot = new_snapshot.clone();
            attach_player(
                name,
                proxy,
                new_snapshot,
                read_volume_scale,
                &mut write_volume_scale,
                &mut pending_volume,
                &mut last_position_tick,
                &mut active_player,
                &mut signal_rx,
                &mut signal_tasks,
                &self.event_tx,
            )
            .await;
        }

        loop {
            tokio::select! {
                Some(change) = owner_changes.next() => {
                    if let Ok(args) = change.args() {
                        let name = args.name().as_str();
                        if args.new_owner().is_none() {
                            if let Some((ref active_name, _)) = active_player {
                                if active_name == name {
                                    active_player = None;
                                    snapshot = MediaSnapshot::default();
                                    read_volume_scale = VolumeScale::Unit;
                                    write_volume_scale = VolumeScale::Unit;
                                    time_scale = TimeScale::Micros;
                                    self.cover_cache = None;
                                    pending_volume = None;
                                    last_position_tick = Instant::now();
                                    cleanup_signals(&mut signal_tasks, &mut signal_rx);
                                    let _ = self.event_tx.send(MediaEvent::SnapshotUpdated(snapshot.clone()));
                                }
                            }
                        }

                        // Re-discover when no active player is attached.
                        if active_player.is_none() {
                            if let Some((new_name, new_proxy, new_snapshot)) = self
                                .choose_best_player(&conn, &dbus, &mut read_volume_scale, &mut time_scale)
                                .await
                            {
                                snapshot = new_snapshot.clone();
                                attach_player(
                                    new_name,
                                    new_proxy,
                                    new_snapshot,
                                    read_volume_scale,
                                    &mut write_volume_scale,
                                    &mut pending_volume,
                                    &mut last_position_tick,
                                    &mut active_player,
                                    &mut signal_rx,
                                    &mut signal_tasks,
                                    &self.event_tx,
                                )
                                .await;
                            }
                        }
                    }
                }
                Some(cmd) = self.cmd_rx.recv() => {
                    if let Some((ref name, ref proxy)) = active_player {
                        let proxy = proxy.clone();
                        match cmd {
                            MediaCommand::Next => { let _ = proxy.next().await; },
                            MediaCommand::Previous => { let _ = proxy.previous().await; },
                            MediaCommand::PlayPause => { let _ = proxy.play_pause().await; },
                            MediaCommand::Stop => { let _ = proxy.stop().await; },
                            MediaCommand::SetVolume(v) => {
                                if self.unsupported_player_volume.contains(name) {
                                    continue;
                                }
                                pending_volume = Some(PendingVolumeWrite::new(v));
                                let _ = proxy
                                    .set_volume(denormalize_volume(v, write_volume_scale))
                                    .await;
                            },
                            MediaCommand::Seek(pos) => {
                                snapshot.position = pos;
                                if let Ok(metadata) = proxy.metadata().await {
                                    if let Some(track_id) = metadata.get("mpris:trackid") {
                                        if let Ok(path_str) = track_id.downcast_ref::<zbus::zvariant::ObjectPath<'_>>() {
                                            let _ = proxy
                                                .set_position(
                                                    path_str.clone(),
                                                    denormalize_time(pos, time_scale),
                                                )
                                                .await;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                signal = async {
                    if let Some(rx) = signal_rx.as_mut() {
                        rx.recv().await
                    } else {
                        pending().await
                    }
                } => {
                    let Some(_signal) = signal else {
                        cleanup_signals(&mut signal_tasks, &mut signal_rx);
                        continue;
                    };

                    if let Some((ref name, ref proxy)) = active_player {
                        let elapsed_us = last_position_tick.elapsed().as_micros() as i64;
                        last_position_tick = Instant::now();
                        let mut new_snapshot = match self
                            .update_snapshot(name, proxy, &mut read_volume_scale, &mut time_scale)
                            .await
                        {
                            Some(snapshot) => snapshot,
                            None => {
                                active_player = None;
                                snapshot = MediaSnapshot::default();
                                read_volume_scale = VolumeScale::Unit;
                                write_volume_scale = VolumeScale::Unit;
                                time_scale = TimeScale::Micros;
                                pending_volume = None;
                                self.cover_cache = None;
                                cleanup_signals(&mut signal_tasks, &mut signal_rx);
                                let _ = self.event_tx.send(MediaEvent::SnapshotUpdated(snapshot.clone()));
                                continue;
                            }
                        };
                        synthesize_playing_position(&snapshot, &mut new_snapshot, elapsed_us);
                        maybe_apply_volume_fallback(
                            &mut pending_volume,
                            &new_snapshot,
                            &mut write_volume_scale,
                            proxy,
                            name,
                            &mut self.unsupported_player_volume,
                        )
                        .await;
                        if should_emit_snapshot(&snapshot, &new_snapshot) {
                            snapshot = new_snapshot;
                            let _ = self.event_tx.send(MediaEvent::SnapshotUpdated(snapshot.clone()));
                        }

                        if should_attempt_player_rediscovery(&snapshot) {
                            if let Some((new_name, new_proxy, new_snapshot)) = self
                                .choose_best_player(&conn, &dbus, &mut read_volume_scale, &mut time_scale)
                                .await
                            {
                                let switched = name != &new_name;
                                if switched {
                                    snapshot = new_snapshot.clone();
                                    attach_player(
                                        new_name,
                                        new_proxy,
                                        new_snapshot,
                                        read_volume_scale,
                                        &mut write_volume_scale,
                                        &mut pending_volume,
                                        &mut last_position_tick,
                                        &mut active_player,
                                        &mut signal_rx,
                                        &mut signal_tasks,
                                        &self.event_tx,
                                    )
                                    .await;
                                }
                            }
                        }
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_millis(
                    if snapshot.playback_status == "Playing" { 350 } else { 1800 }
                )) => {
                    if let Some((ref name, ref proxy)) = active_player {
                        let elapsed_us = last_position_tick.elapsed().as_micros() as i64;
                        last_position_tick = Instant::now();
                        let mut new_snapshot = match self
                            .update_snapshot(name, proxy, &mut read_volume_scale, &mut time_scale)
                            .await
                        {
                            Some(snapshot) => snapshot,
                            None => {
                                active_player = None;
                                snapshot = MediaSnapshot::default();
                                read_volume_scale = VolumeScale::Unit;
                                write_volume_scale = VolumeScale::Unit;
                                time_scale = TimeScale::Micros;
                                pending_volume = None;
                                self.cover_cache = None;
                                cleanup_signals(&mut signal_tasks, &mut signal_rx);
                                let _ = self.event_tx.send(MediaEvent::SnapshotUpdated(snapshot.clone()));
                                continue;
                            }
                        };
                        synthesize_playing_position(&snapshot, &mut new_snapshot, elapsed_us);
                        maybe_apply_volume_fallback(
                            &mut pending_volume,
                            &new_snapshot,
                            &mut write_volume_scale,
                            proxy,
                            name,
                            &mut self.unsupported_player_volume,
                        )
                        .await;
                        if should_emit_snapshot(&snapshot, &new_snapshot) {
                            snapshot = new_snapshot;
                            let _ = self.event_tx.send(MediaEvent::SnapshotUpdated(snapshot.clone()));
                        }

                        if should_attempt_player_rediscovery(&snapshot) {
                            if let Some((new_name, new_proxy, new_snapshot)) = self
                                .choose_best_player(&conn, &dbus, &mut read_volume_scale, &mut time_scale)
                                .await
                            {
                                let switched = name != &new_name;
                                if switched {
                                    snapshot = new_snapshot.clone();
                                    attach_player(
                                        new_name,
                                        new_proxy,
                                        new_snapshot,
                                        read_volume_scale,
                                        &mut write_volume_scale,
                                        &mut pending_volume,
                                        &mut last_position_tick,
                                        &mut active_player,
                                        &mut signal_rx,
                                        &mut signal_tasks,
                                        &self.event_tx,
                                    )
                                    .await;
                                }
                            }
                        }
                    }
                }

            }
        }
    }

    async fn spawn_player_signal_tasks(
        proxy: PlayerProxy<'static>,
    ) -> (mpsc::Receiver<PlayerSignal>, Vec<JoinHandle<()>>) {
        let (base_tx, rx) = mpsc::channel(32);
        let mut tasks = Vec::new();

        let metadata_proxy = proxy.clone();
        let mut stream = metadata_proxy.receive_metadata_changed().await;
        let metadata_tx = base_tx.clone();
        tasks.push(tokio::spawn(async move {
            while stream.next().await.is_some() {
                if metadata_tx.send(PlayerSignal::Metadata).await.is_err() {
                    break;
                }
            }
        }));

        let volume_proxy = proxy.clone();
        let mut stream = volume_proxy.receive_volume_changed().await;
        let volume_tx = base_tx.clone();
        tasks.push(tokio::spawn(async move {
            while stream.next().await.is_some() {
                if volume_tx.send(PlayerSignal::Volume).await.is_err() {
                    break;
                }
            }
        }));

        let mut stream = proxy.receive_playback_status_changed().await;
        let playback_tx = base_tx;
        tasks.push(tokio::spawn(async move {
            while stream.next().await.is_some() {
                if playback_tx
                    .send(PlayerSignal::PlaybackStatus)
                    .await
                    .is_err()
                {
                    break;
                }
            }
        }));

        (rx, tasks)
    }

    async fn update_snapshot(
        &mut self,
        name: &str,
        proxy: &PlayerProxy<'static>,
        read_volume_scale: &mut VolumeScale,
        time_scale: &mut TimeScale,
    ) -> Option<MediaSnapshot> {
        let status = proxy.playback_status().await.ok()?;
        let raw_volume = proxy.volume().await.unwrap_or(1.0);
        let metadata = proxy.metadata().await.unwrap_or_default();
        *read_volume_scale = detect_volume_scale(raw_volume, *read_volume_scale);
        let volume = normalize_volume(raw_volume, *read_volume_scale);

        let title = metadata
            .get("xesam:title")
            .and_then(|v| v.downcast_ref::<String>().ok())
            .unwrap_or_else(|| "Unknown".to_string());

        let artist = metadata
            .get("xesam:artist")
            .and_then(|v| {
                // Manually parse the xesam:artist which is often an Array of Strings
                let val: zbus::zvariant::Value = v.clone().into();
                if let Ok(vec) = <Vec<String>>::try_from(val) {
                    vec.first().cloned()
                } else if let Ok(s) = v.downcast_ref::<String>() {
                    Some(s.clone())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "Unknown Artist".to_string());

        let album = metadata
            .get("xesam:album")
            .and_then(|v| v.downcast_ref::<String>().ok())
            .unwrap_or_else(|| "Unknown Album".to_string());

        let track_id = metadata
            .get("mpris:trackid")
            .and_then(|v| v.downcast_ref::<zbus::zvariant::ObjectPath<'_>>().ok())
            .map(|p| p.to_string())
            .unwrap_or_default();

        let raw_duration = metadata
            .get("mpris:length")
            .and_then(|v| {
                if let Ok(val) = v.downcast_ref::<i64>() {
                    Some(val)
                } else if let Ok(val) = v.downcast_ref::<u64>() {
                    Some(val as i64)
                } else {
                    None
                }
            })
            .unwrap_or(0);
        *time_scale = detect_time_scale(raw_duration, *time_scale);
        let duration = normalize_time(raw_duration, *time_scale);
        let raw_position = proxy.position().await.unwrap_or(0);
        let position = normalize_time(raw_position, *time_scale);

        let cover_url = metadata
            .get("mpris:artUrl")
            .and_then(|v| v.downcast_ref::<String>().ok())
            .map(|url| normalize_cover_url(name, &url));

        let cover_bytes = self.resolve_cover_bytes(cover_url.as_deref()).await;

        Some(MediaSnapshot {
            title,
            artist,
            album,
            track_id,
            cover_url: cover_url.clone(),
            playback_status: status,
            volume,
            position,
            duration,
            player_name: name.replace("org.mpris.MediaPlayer2.", ""),
            has_player: true,
            volume_supported: !self.unsupported_player_volume.contains(name),
            cover_bytes,
        })
    }

    async fn resolve_cover_bytes(&mut self, cover_url: Option<&str>) -> Option<Arc<Vec<u8>>> {
        let Some(url) = cover_url else {
            self.cover_cache = None;
            return None;
        };

        if let Some((cached_url, cached_bytes)) = &self.cover_cache {
            if cached_url == url {
                return Some(cached_bytes.clone());
            }
        }

        let bytes = if url.starts_with("file://") {
            std::fs::read(url.trim_start_matches("file://"))
                .ok()
                .map(Arc::new)
        } else if url.starts_with("http://") || url.starts_with("https://") {
            if let Ok(response) = self.http_client.get(url).send().await {
                response
                    .bytes()
                    .await
                    .ok()
                    .map(|bytes| Arc::new(bytes.to_vec()))
            } else {
                None
            }
        } else {
            None
        };

        if let Some(bytes) = bytes {
            self.cover_cache = Some((url.to_string(), bytes.clone()));
            Some(bytes)
        } else {
            self.cover_cache = None;
            None
        }
    }

    async fn choose_best_player(
        &mut self,
        conn: &Connection,
        dbus: &fdo::DBusProxy<'_>,
        read_volume_scale: &mut VolumeScale,
        time_scale: &mut TimeScale,
    ) -> Option<(String, PlayerProxy<'static>, MediaSnapshot)> {
        let names = dbus.list_names().await.ok()?;
        let mut best: Option<(i32, String, PlayerProxy<'static>)> = None;

        for name in names {
            let name_str = name.as_str();
            if !name_str.starts_with("org.mpris.MediaPlayer2.") {
                continue;
            }

            let builder = match PlayerProxy::builder(conn).destination(name_str.to_string()) {
                Ok(builder) => builder,
                Err(_) => continue,
            };
            let proxy = match builder.build().await {
                Ok(proxy) => proxy,
                Err(_) => continue,
            };

            let status = match proxy.playback_status().await {
                Ok(status) => status,
                Err(_) => continue,
            };
            let can_control = proxy.can_control().await.unwrap_or(true);

            let title = proxy
                .metadata()
                .await
                .ok()
                .and_then(|metadata| {
                    metadata
                        .get("xesam:title")
                        .and_then(|v| v.downcast_ref::<String>().ok())
                })
                .unwrap_or_default();

            let score = player_hint_score(name_str, &status, &title, can_control);
            let take = best
                .as_ref()
                .map(|(best_score, _, _)| score > *best_score)
                .unwrap_or(true);
            if take {
                best = Some((score, name_str.to_string(), proxy));
            }
        }

        let (_, best_name, best_proxy) = best?;
        let snapshot = self
            .update_snapshot(&best_name, &best_proxy, read_volume_scale, time_scale)
            .await?;
        Some((best_name, best_proxy, snapshot))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VolumeScale {
    Unit,
    Percent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TimeScale {
    Micros,
    Millis,
}

#[derive(Debug, Clone, Copy)]
struct PendingVolumeWrite {
    target_normalized: f64,
    tried_fallback_scale: bool,
    created_at: Instant,
    checks: u8,
}

impl PendingVolumeWrite {
    fn new(target_normalized: f64) -> Self {
        Self {
            target_normalized: target_normalized.clamp(0.0, 1.0),
            tried_fallback_scale: false,
            created_at: Instant::now(),
            checks: 0,
        }
    }
}

fn detect_volume_scale(raw_volume: f64, previous: VolumeScale) -> VolumeScale {
    if raw_volume.is_finite() && (1.5..=100.0).contains(&raw_volume) {
        VolumeScale::Percent
    } else if raw_volume.is_finite() && (0.0..=1.5).contains(&raw_volume) {
        VolumeScale::Unit
    } else {
        previous
    }
}

fn normalize_volume(raw_volume: f64, scale: VolumeScale) -> f64 {
    let normalized = match scale {
        VolumeScale::Unit => raw_volume,
        VolumeScale::Percent => raw_volume / 100.0,
    };
    normalized.clamp(0.0, 1.0)
}

fn denormalize_volume(normalized: f64, scale: VolumeScale) -> f64 {
    let clamped = normalized.clamp(0.0, 1.0);
    match scale {
        VolumeScale::Unit => clamped,
        VolumeScale::Percent => clamped * 100.0,
    }
}

fn detect_time_scale(raw_duration: i64, previous: TimeScale) -> TimeScale {
    if raw_duration >= 10_000_000 {
        TimeScale::Micros
    } else if raw_duration >= 1_000 {
        TimeScale::Millis
    } else {
        previous
    }
}

fn normalize_time(raw_value: i64, scale: TimeScale) -> i64 {
    match scale {
        TimeScale::Micros => raw_value,
        TimeScale::Millis => raw_value.saturating_mul(1_000),
    }
}

fn denormalize_time(normalized_value: i64, scale: TimeScale) -> i64 {
    match scale {
        TimeScale::Micros => normalized_value,
        TimeScale::Millis => normalized_value / 1_000,
    }
}

fn alternate_volume_scale(scale: VolumeScale) -> VolumeScale {
    match scale {
        VolumeScale::Unit => VolumeScale::Percent,
        VolumeScale::Percent => VolumeScale::Unit,
    }
}

fn synthesize_playing_position(
    previous: &MediaSnapshot,
    current: &mut MediaSnapshot,
    elapsed_us: i64,
) {
    if current.playback_status != "Playing" {
        return;
    }

    if !same_track_identity(previous, current) {
        return;
    }

    if current.position > previous.position {
        return;
    }

    if elapsed_us <= 0 {
        return;
    }

    let advanced = previous.position.saturating_add(elapsed_us);
    current.position = if current.duration > 0 {
        advanced.min(current.duration)
    } else {
        advanced
    };
}

fn same_track_identity(previous: &MediaSnapshot, current: &MediaSnapshot) -> bool {
    if !previous.track_id.is_empty() && !current.track_id.is_empty() {
        return previous.track_id == current.track_id;
    }

    if !previous.title.is_empty() && !current.title.is_empty() {
        return previous.title == current.title && previous.artist == current.artist;
    }

    false
}

fn title_known(title: &str) -> bool {
    let trimmed = title.trim();
    !trimmed.is_empty() && !trimmed.eq_ignore_ascii_case("unknown")
}

fn should_attempt_player_rediscovery(snapshot: &MediaSnapshot) -> bool {
    snapshot.playback_status != "Playing" && !title_known(&snapshot.title)
}

fn should_emit_snapshot(previous: &MediaSnapshot, current: &MediaSnapshot) -> bool {
    current.title != previous.title
        || current.artist != previous.artist
        || current.album != previous.album
        || current.track_id != previous.track_id
        || current.cover_url != previous.cover_url
        || current.playback_status != previous.playback_status
        || (current.volume - previous.volume).abs() > f64::EPSILON
        || current.position != previous.position
        || current.duration != previous.duration
        || current.player_name != previous.player_name
        || current.has_player != previous.has_player
}

fn player_hint_score(player_name: &str, status: &str, title: &str, can_control: bool) -> i32 {
    let status_score = match status {
        "Playing" => 100,
        "Paused" => 60,
        "Stopped" => 20,
        _ => 0,
    };
    let title_score = if title_known(title) { 25 } else { 0 };
    let control_score = if can_control { 20 } else { -60 };
    let playerctld_penalty = if player_name.ends_with(".playerctld") {
        -30
    } else {
        0
    };
    status_score + title_score + control_score + playerctld_penalty
}

async fn maybe_apply_volume_fallback(
    pending: &mut Option<PendingVolumeWrite>,
    snapshot: &MediaSnapshot,
    scale: &mut VolumeScale,
    proxy: &PlayerProxy<'static>,
    player_name: &str,
    unsupported_player_volume: &mut HashSet<String>,
) {
    let Some(write) = pending.as_mut() else {
        return;
    };

    write.checks = write.checks.saturating_add(1);

    if write.created_at.elapsed() > Duration::from_secs(2) {
        *pending = None;
        return;
    }

    // Wait for at least one post-write observation before deciding success/fallback.
    if write.checks < 2 {
        return;
    }

    if (snapshot.volume - write.target_normalized).abs() <= 0.08 {
        *pending = None;
        return;
    }

    if write.tried_fallback_scale {
        if (snapshot.volume - write.target_normalized).abs() > 0.2 && write.checks >= 5 {
            unsupported_player_volume.insert(player_name.to_string());
            *pending = None;
        }
        return;
    }

    let fallback = alternate_volume_scale(*scale);
    let _ = proxy
        .set_volume(denormalize_volume(write.target_normalized, fallback))
        .await;
    *scale = fallback;
    write.tried_fallback_scale = true;
}

fn normalize_cover_url(player_name: &str, url: &str) -> String {
    let lower_url = url.to_ascii_lowercase();
    let lower_name = player_name.to_ascii_lowercase();
    let is_yandex = lower_url.contains("yandex") || lower_name.contains("yandex");
    if !is_yandex {
        return url.to_string();
    }

    let mut normalized = url.replace("%%x%%", "400x400");
    for size in [
        "30x30", "50x50", "80x80", "100x100", "150x150", "200x200", "300x300",
    ] {
        normalized = normalized.replace(size, "400x400");
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_default_is_empty_but_valid() {
        let snap = MediaSnapshot::default();
        assert_eq!(snap.title, "");
        assert!(!snap.has_player);
    }

    #[test]
    fn normalize_cover_url_upscales_yandex_size_tokens() {
        let url = "https://avatars.yandex.net/get-music-content/123/abc/80x80";
        let normalized = normalize_cover_url("org.mpris.MediaPlayer2.yandexmusic", url);
        assert!(normalized.contains("400x400"));
    }

    #[test]
    fn normalize_cover_url_keeps_non_yandex_sources() {
        let url = "file:///home/user/cover.jpg";
        assert_eq!(
            normalize_cover_url("org.mpris.MediaPlayer2.strawberry", url),
            url
        );
    }

    #[test]
    fn volume_scale_detects_non_standard_percent_mode() {
        assert_eq!(
            detect_volume_scale(64.0, VolumeScale::Unit),
            VolumeScale::Percent
        );
        assert_eq!(normalize_volume(64.0, VolumeScale::Percent), 0.64);
        assert_eq!(denormalize_volume(0.25, VolumeScale::Percent), 25.0);
    }

    #[test]
    fn volume_scale_keeps_standard_unit_mode() {
        assert_eq!(
            detect_volume_scale(0.42, VolumeScale::Percent),
            VolumeScale::Unit
        );
        assert_eq!(normalize_volume(0.42, VolumeScale::Unit), 0.42);
    }

    #[test]
    fn time_scale_detects_millis_and_normalizes_to_micros() {
        assert_eq!(
            detect_time_scale(180_000, TimeScale::Micros),
            TimeScale::Millis
        );
        assert_eq!(normalize_time(180_000, TimeScale::Millis), 180_000_000);
    }

    #[test]
    fn time_scale_keeps_micros_for_mpris_values() {
        assert_eq!(
            detect_time_scale(180_000_000, TimeScale::Millis),
            TimeScale::Micros
        );
        assert_eq!(normalize_time(180_000_000, TimeScale::Micros), 180_000_000);
    }

    #[test]
    fn denormalize_time_respects_detected_scale() {
        let seek_us = 90_000_000;
        assert_eq!(denormalize_time(seek_us, TimeScale::Micros), seek_us);
        assert_eq!(denormalize_time(seek_us, TimeScale::Millis), 90_000);
    }

    #[test]
    fn synthesize_playing_position_advances_when_backend_position_is_stale() {
        let previous = MediaSnapshot {
            playback_status: "Playing".to_string(),
            position: 50_000_000,
            duration: 180_000_000,
            track_id: "track:same".to_string(),
            ..MediaSnapshot::default()
        };
        let mut current = MediaSnapshot {
            playback_status: "Playing".to_string(),
            position: 50_000_000,
            duration: 180_000_000,
            track_id: "track:same".to_string(),
            ..MediaSnapshot::default()
        };

        synthesize_playing_position(&previous, &mut current, 500_000);
        assert_eq!(current.position, 50_500_000);
    }

    #[test]
    fn synthesize_playing_position_does_not_exceed_duration() {
        let previous = MediaSnapshot {
            playback_status: "Playing".to_string(),
            position: 179_900_000,
            duration: 180_000_000,
            track_id: "track:same".to_string(),
            ..MediaSnapshot::default()
        };
        let mut current = MediaSnapshot {
            playback_status: "Playing".to_string(),
            position: 179_900_000,
            duration: 180_000_000,
            track_id: "track:same".to_string(),
            ..MediaSnapshot::default()
        };

        synthesize_playing_position(&previous, &mut current, 500_000);
        assert_eq!(current.position, 180_000_000);
    }

    #[test]
    fn synthesize_playing_position_keeps_reported_progress_when_backend_moves() {
        let previous = MediaSnapshot {
            playback_status: "Playing".to_string(),
            position: 20_000_000,
            ..MediaSnapshot::default()
        };
        let mut current = MediaSnapshot {
            playback_status: "Playing".to_string(),
            position: 21_000_000,
            ..MediaSnapshot::default()
        };

        synthesize_playing_position(&previous, &mut current, 500_000);
        assert_eq!(current.position, 21_000_000);
    }

    #[test]
    fn synthesize_playing_position_does_not_carry_over_on_track_change() {
        let previous = MediaSnapshot {
            playback_status: "Playing".to_string(),
            position: 143_000_000,
            duration: 216_000_000,
            track_id: "/org/mpris/MediaPlayer2/track/old".to_string(),
            title: "Old".to_string(),
            artist: "Artist".to_string(),
            ..MediaSnapshot::default()
        };
        let mut current = MediaSnapshot {
            playback_status: "Playing".to_string(),
            position: 500_000,
            duration: 180_000_000,
            track_id: "/org/mpris/MediaPlayer2/track/new".to_string(),
            title: "New".to_string(),
            artist: "Artist".to_string(),
            ..MediaSnapshot::default()
        };

        synthesize_playing_position(&previous, &mut current, 800_000);
        assert_eq!(current.position, 500_000);
    }

    #[test]
    fn player_hint_score_prefers_playing_and_known_title() {
        assert!(
            player_hint_score(
                "org.mpris.MediaPlayer2.strawberry",
                "Playing",
                "Track A",
                true
            ) > player_hint_score(
                "org.mpris.MediaPlayer2.strawberry",
                "Paused",
                "Track A",
                true
            )
        );
        assert!(
            player_hint_score(
                "org.mpris.MediaPlayer2.strawberry",
                "Paused",
                "Track A",
                true
            ) > player_hint_score(
                "org.mpris.MediaPlayer2.strawberry",
                "Paused",
                "Unknown",
                true
            )
        );
    }

    #[test]
    fn player_hint_score_penalizes_non_controllable_and_playerctld() {
        let direct = player_hint_score(
            "org.mpris.MediaPlayer2.strawberry",
            "Playing",
            "Track A",
            true,
        );
        let playerctld = player_hint_score(
            "org.mpris.MediaPlayer2.playerctld",
            "Playing",
            "Track A",
            true,
        );
        let non_controllable = player_hint_score(
            "org.mpris.MediaPlayer2.someplayer",
            "Playing",
            "Track A",
            false,
        );
        assert!(direct > playerctld);
        assert!(direct > non_controllable);
    }

    #[test]
    fn should_emit_snapshot_detects_track_changes() {
        let previous = MediaSnapshot {
            title: "Track A".to_string(),
            position: 1_000_000,
            ..MediaSnapshot::default()
        };
        let current = MediaSnapshot {
            title: "Track B".to_string(),
            position: 10_000,
            ..MediaSnapshot::default()
        };
        assert!(should_emit_snapshot(&previous, &current));
    }

    #[test]
    fn should_emit_snapshot_ignores_identical_snapshots() {
        let snapshot = MediaSnapshot {
            title: "Track".to_string(),
            artist: "Artist".to_string(),
            album: "Album".to_string(),
            track_id: "/track/1".to_string(),
            cover_url: Some("file:///cover.jpg".to_string()),
            playback_status: "Playing".to_string(),
            volume: 0.6,
            position: 2_000_000,
            duration: 10_000_000,
            player_name: "player".to_string(),
            has_player: true,
            ..MediaSnapshot::default()
        };
        assert!(!should_emit_snapshot(&snapshot, &snapshot));
    }
}
