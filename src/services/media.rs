use futures_util::SinkExt;
use futures_util::StreamExt;
use std::collections::{HashMap, HashSet};
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
    fn can_go_next(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn can_go_previous(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn can_play(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn can_pause(&self) -> zbus::Result<bool>;

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
    pub cover_handle: Option<iced::widget::image::Handle>,
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
    cover_cache: Option<(String, iced::widget::image::Handle)>,
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
        let mut last_position_tick = std::time::Instant::now();

        let mut signal_rx: Option<mpsc::UnboundedReceiver<PlayerSignal>> = None;
        let mut _signal_tasks: Vec<JoinHandle<()>> = Vec::new();

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
                &mut _signal_tasks,
                &self.event_tx,
            )
            .await;
        }

        loop {
            let sleep_ms = if active_player.is_some() && snapshot.playback_status == "Playing" {
                100
            } else {
                500
            };

            tokio::select! {
                Some(change) = owner_changes.next() => {
                    if let Ok(args) = change.args() {
                        let name = args.name().as_str();
                        if name.starts_with("org.mpris.MediaPlayer2.") {
                            if args.new_owner().is_none() {
                                if let Some((ref active_name, _)) = active_player {
                                    if active_name == name {
                                        active_player = None;
                                        signal_rx = None;
                                        _signal_tasks.clear();
                                        snapshot = MediaSnapshot::default();
                                        let _ = self.event_tx.send(MediaEvent::SnapshotUpdated(snapshot.clone()));
                                    }
                                }
                            } else if active_player.is_none() {
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
                                        &mut _signal_tasks,
                                        &self.event_tx,
                                    )
                                    .await;
                                }
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
                                if let Ok(metadata) = proxy.metadata().await {
                                    if let Some(track_id) = metadata.get("mpris:trackid") {
                                        if let Ok(path) = track_id.downcast_ref::<zbus::zvariant::ObjectPath<'_>>() {
                                            let _ = proxy.set_position(path.clone(), denormalize_time(pos, time_scale)).await;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Some(signal) = async {
                    if let Some(ref mut rx) = signal_rx {
                        rx.recv().await
                    } else {
                        None
                    }
                } => {
                    if let Some((ref name, ref proxy)) = active_player {
                        if let Some(new_snapshot) = self.update_snapshot(name, proxy, &mut read_volume_scale, &mut time_scale).await {
                            if should_emit_snapshot(&new_snapshot, &snapshot, &mut pending_volume, signal) {
                                snapshot = new_snapshot;
                                let _ = self.event_tx.send(MediaEvent::SnapshotUpdated(snapshot.clone()));
                            }
                        }
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_millis(sleep_ms)) => {
                    if let Some((ref name, ref proxy)) = active_player {
                        if let Some(mut new_snapshot) = self.update_snapshot(name, proxy, &mut read_volume_scale, &mut time_scale).await {
                            new_snapshot.position = synthesize_playing_position(&new_snapshot, &snapshot, last_position_tick);
                            last_position_tick = std::time::Instant::now();

                            if should_emit_snapshot(&new_snapshot, &snapshot, &mut pending_volume, PlayerSignal::PlaybackStatus) {
                                snapshot = new_snapshot;
                                let _ = self.event_tx.send(MediaEvent::SnapshotUpdated(snapshot.clone()));
                            }
                        }
                    }
                }
            }
        }
    }

    async fn choose_best_player(
        &mut self,
        conn: &Connection,
        dbus: &fdo::DBusProxy<'static>,
        read_volume_scale: &mut VolumeScale,
        time_scale: &mut TimeScale,
    ) -> Option<(String, PlayerProxy<'static>, MediaSnapshot)> {
        let names = dbus.list_names().await.ok()?;
        let mut candidates = Vec::new();

        for name in names {
            let name_str = name.as_str();
            if name_str.starts_with("org.mpris.MediaPlayer2.") {
                if let Ok(proxy) = PlayerProxy::builder(conn)
                    .destination(name_str.to_string())
                    .ok()?
                    .build()
                    .await
                {
                    if let Some(snap) = self
                        .update_snapshot(name_str, &proxy, read_volume_scale, time_scale)
                        .await
                    {
                        candidates.push((name_str.to_string(), proxy, snap));
                    }
                }
            }
        }

        candidates.sort_by_key(|(_, _, snap)| std::cmp::Reverse(player_hint_score(snap)));
        candidates.into_iter().next()
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
                    Some(val.clone())
                } else if let Ok(val) = v.downcast_ref::<u64>() {
                    Some(val.clone() as i64)
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

        let cover_handle = self.resolve_cover_handle(cover_url.as_deref()).await;

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
            cover_handle,
        })
    }

    async fn resolve_cover_handle(
        &mut self,
        cover_url: Option<&str>,
    ) -> Option<iced::widget::image::Handle> {
        let Some(url) = cover_url else {
            self.cover_cache = None;
            return None;
        };

        if let Some((cached_url, cached_handle)) = &self.cover_cache {
            if cached_url == url {
                return Some(cached_handle.clone());
            }
        }

        let bytes = if url.starts_with("file://") {
            std::fs::read(url.trim_start_matches("file://")).ok()
        } else if url.starts_with("http://") || url.starts_with("https://") {
            if let Ok(response) = self.http_client.get(url).send().await {
                response.bytes().await.ok().map(|b| b.to_vec())
            } else {
                None
            }
        } else {
            None
        };

        let handle = bytes.map(iced::widget::image::Handle::from_bytes);
        if let Some(ref h) = handle {
            self.cover_cache = Some((url.to_string(), h.clone()));
        }
        handle
    }
}

async fn attach_player(
    name: String,
    proxy: PlayerProxy<'static>,
    snapshot: MediaSnapshot,
    read_volume_scale: VolumeScale,
    write_volume_scale: &mut VolumeScale,
    pending_volume: &mut Option<PendingVolumeWrite>,
    last_position_tick: &mut std::time::Instant,
    active_player: &mut Option<(String, PlayerProxy<'static>)>,
    signal_rx: &mut Option<mpsc::UnboundedReceiver<PlayerSignal>>,
    signal_tasks: &mut Vec<JoinHandle<()>>,
    event_tx: &broadcast::Sender<MediaEvent>,
) {
    *write_volume_scale = read_volume_scale;
    *pending_volume = None;
    *last_position_tick = std::time::Instant::now();
    *active_player = Some((name, proxy.clone()));

    let (rx, tasks) = setup_player_signals(proxy);
    *signal_rx = Some(rx);
    *signal_tasks = tasks;

    let _ = event_tx.send(MediaEvent::SnapshotUpdated(snapshot));
}

fn setup_player_signals(
    proxy: PlayerProxy<'static>,
) -> (mpsc::UnboundedReceiver<PlayerSignal>, Vec<JoinHandle<()>>) {
    let (tx, rx) = mpsc::unbounded_channel();
    let mut tasks = Vec::new();

    let proxy_meta = proxy.clone();
    let tx_meta = tx.clone();
    tasks.push(tokio::spawn(async move {
        let mut stream = proxy_meta.receive_metadata_changed().await;
        while stream.next().await.is_some() {
            let _ = tx_meta.send(PlayerSignal::Metadata);
        }
    }));

    let proxy_vol = proxy.clone();
    let tx_vol = tx.clone();
    tasks.push(tokio::spawn(async move {
        let mut stream = proxy_vol.receive_volume_changed().await;
        while stream.next().await.is_some() {
            let _ = tx_vol.send(PlayerSignal::Volume);
        }
    }));

    let proxy_status = proxy.clone();
    let tx_status = tx.clone();
    tasks.push(tokio::spawn(async move {
        let mut stream = proxy_status.receive_playback_status_changed().await;
        while stream.next().await.is_some() {
            let _ = tx_status.send(PlayerSignal::PlaybackStatus);
        }
    }));

    (rx, tasks)
}

fn should_emit_snapshot(
    new: &MediaSnapshot,
    old: &MediaSnapshot,
    pending_volume: &mut Option<PendingVolumeWrite>,
    signal: PlayerSignal,
) -> bool {
    if new.title != old.title
        || new.playback_status != old.playback_status
        || new.track_id != old.track_id
    {
        return true;
    }

    match signal {
        PlayerSignal::Volume => {
            if let Some(pending) = pending_volume {
                if (new.volume - pending.target).abs() < 0.01 {
                    *pending_volume = None;
                    return true;
                }
                false
            } else {
                (new.volume - old.volume).abs() > 0.01
            }
        }
        PlayerSignal::Metadata | PlayerSignal::PlaybackStatus => {
            (new.position - old.position).abs() > 500_000
        }
    }
}

fn synthesize_playing_position(
    new: &MediaSnapshot,
    old: &MediaSnapshot,
    last_tick: std::time::Instant,
) -> i64 {
    if new.playback_status != "Playing" {
        return new.position;
    }

    if new.track_id != old.track_id {
        return new.position;
    }

    let elapsed = last_tick.elapsed().as_micros() as i64;
    let backend_moved = (new.position - old.position).abs() > 100_000;

    let base_position = if backend_moved {
        new.position
    } else {
        old.position + elapsed
    };

    if new.duration > 0 {
        base_position.min(new.duration)
    } else {
        base_position
    }
}

fn player_hint_score(snap: &MediaSnapshot) -> i32 {
    let mut score = 0;
    if snap.playback_status == "Playing" {
        score += 100;
    }
    if !snap.title.is_empty() && snap.title != "Unknown" {
        score += 50;
    }
    if snap.player_name.to_lowercase().contains("playerctld") {
        score -= 200;
    }
    score
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VolumeScale {
    Unit,    // 0.0 - 1.0
    Percent, // 0.0 - 100.0
}

fn detect_volume_scale(val: f64, current: VolumeScale) -> VolumeScale {
    if val > 1.1 {
        VolumeScale::Percent
    } else {
        current
    }
}

fn normalize_volume(val: f64, scale: VolumeScale) -> f64 {
    match scale {
        VolumeScale::Unit => val.clamp(0.0, 1.0),
        VolumeScale::Percent => (val / 100.0).clamp(0.0, 1.0),
    }
}

fn denormalize_volume(val: f64, scale: VolumeScale) -> f64 {
    match scale {
        VolumeScale::Unit => val,
        VolumeScale::Percent => val * 100.0,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TimeScale {
    Micros,
    Millis,
}

fn detect_time_scale(val: i64, current: TimeScale) -> TimeScale {
    if val > 10_000_000_000 {
        TimeScale::Micros
    } else if val > 10_000_000 {
        TimeScale::Millis
    } else {
        current
    }
}

fn normalize_time(val: i64, scale: TimeScale) -> i64 {
    match scale {
        TimeScale::Micros => val,
        TimeScale::Millis => val * 1000,
    }
}

fn denormalize_time(val: i64, scale: TimeScale) -> i64 {
    match scale {
        TimeScale::Micros => val,
        TimeScale::Millis => val / 1000,
    }
}

fn normalize_cover_url(player: &str, url: &str) -> String {
    if player.contains("yandex") && url.contains("%%") {
        url.replace("%%", "400x400")
    } else {
        url.to_string()
    }
}

struct PendingVolumeWrite {
    target: f64,
    _timestamp: std::time::Instant,
}

impl PendingVolumeWrite {
    fn new(target: f64) -> Self {
        Self {
            target,
            _timestamp: std::time::Instant::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_default_is_empty_but_valid() {
        let snap = MediaSnapshot::default();
        assert_eq!(snap.title, "");
        assert_eq!(snap.has_player, false);
    }

    #[test]
    fn player_hint_score_prefers_playing_and_known_title() {
        let mut s1 = MediaSnapshot::default();
        s1.playback_status = "Playing".to_string();
        s1.title = "Song".to_string();

        let mut s2 = MediaSnapshot::default();
        s2.playback_status = "Paused".to_string();
        s2.title = "Song".to_string();

        assert!(player_hint_score(&s1) > player_hint_score(&s2));
    }

    #[test]
    fn player_hint_score_penalizes_non_controllable_and_playerctld() {
        let mut s = MediaSnapshot::default();
        s.player_name = "playerctld".to_string();
        assert!(player_hint_score(&s) < 0);
    }

    #[test]
    fn volume_scale_detects_non_standard_percent_mode() {
        assert_eq!(
            detect_volume_scale(50.0, VolumeScale::Unit),
            VolumeScale::Percent
        );
        assert_eq!(
            detect_volume_scale(0.5, VolumeScale::Percent),
            VolumeScale::Percent
        );
    }

    #[test]
    fn volume_scale_keeps_standard_unit_mode() {
        assert_eq!(
            detect_volume_scale(0.8, VolumeScale::Unit),
            VolumeScale::Unit
        );
    }

    #[test]
    fn time_scale_detects_millis_and_normalizes_to_micros() {
        let millis = 300_000; // 5 min
        assert_eq!(
            detect_time_scale(millis, TimeScale::Millis),
            TimeScale::Millis
        );
        assert_eq!(normalize_time(millis, TimeScale::Millis), 300_000_000);
    }

    #[test]
    fn time_scale_keeps_micros_for_mpris_values() {
        let micros = 300_000_000_000i64; // 300k seconds in micros
        assert_eq!(
            detect_time_scale(micros, TimeScale::Micros),
            TimeScale::Micros
        );
    }

    #[test]
    fn denormalize_time_respects_detected_scale() {
        assert_eq!(
            denormalize_time(300_000_000, TimeScale::Micros),
            300_000_000
        );
    }

    #[test]
    fn normalize_cover_url_upscales_yandex_size_tokens() {
        let url = "https://avatars.yandex.net/get-music-content/123/%%";
        assert_eq!(
            normalize_cover_url("yandex", url),
            "https://avatars.yandex.net/get-music-content/123/400x400"
        );
    }

    #[test]
    fn normalize_cover_url_keeps_non_yandex_sources() {
        let url = "https://i.scdn.co/image/abc";
        assert_eq!(normalize_cover_url("spotify", url), url);
    }

    #[test]
    fn should_emit_snapshot_detects_track_changes() {
        let mut s1 = MediaSnapshot::default();
        s1.title = "T1".to_string();
        let mut s2 = MediaSnapshot::default();
        s2.title = "T2".to_string();
        assert!(should_emit_snapshot(
            &s2,
            &s1,
            &mut None,
            PlayerSignal::Metadata
        ));
    }

    #[test]
    fn should_emit_snapshot_ignores_identical_snapshots() {
        let s = MediaSnapshot::default();
        assert!(!should_emit_snapshot(
            &s,
            &s,
            &mut None,
            PlayerSignal::PlaybackStatus
        ));
    }

    #[test]
    fn synthesize_playing_position_advances_when_backend_position_is_stale() {
        let mut old = MediaSnapshot::default();
        old.playback_status = "Playing".to_string();
        old.position = 1000;

        let old_clone = old.clone();
        let mut new = old.clone();
        // backend reports same 1000 (stale)

        let now = std::time::Instant::now();
        // Wait 10ms
        let tick = now - std::time::Duration::from_millis(10);

        let synth = synthesize_playing_position(&new, &old_clone, tick);
        assert!(synth > 1000);
    }

    #[test]
    fn synthesize_playing_position_keeps_reported_progress_when_backend_moves() {
        let mut old = MediaSnapshot::default();
        old.playback_status = "Playing".to_string();
        old.position = 1000;

        let old_clone = old.clone();
        let mut new = old.clone();
        new.position = 200_000; // backend moved significantly (> 100_000 threshold)

        let synth = synthesize_playing_position(&new, &old_clone, std::time::Instant::now());
        assert_eq!(synth, 200_000);
    }
    #[test]
    fn synthesize_playing_position_does_not_exceed_duration() {
        let mut old = MediaSnapshot::default();
        old.playback_status = "Playing".to_string();
        old.position = 950;
        old.duration = 1000;

        let old_clone = old.clone();
        let mut new = old.clone();

        let tick = std::time::Instant::now() - std::time::Duration::from_millis(100);
        let synth = synthesize_playing_position(&new, &old_clone, tick);
        assert_eq!(synth, 1000);
    }

    #[test]
    fn synthesize_playing_position_does_not_carry_over_on_track_change() {
        let mut old = MediaSnapshot::default();
        old.track_id = "T1".to_string();
        old.position = 5000;

        let old_clone = old.clone();
        let mut new = MediaSnapshot::default();
        new.track_id = "T2".to_string();
        new.position = 0;
        new.playback_status = "Playing".to_string();

        let synth = synthesize_playing_position(&new, &old_clone, std::time::Instant::now());
        assert_eq!(synth, 0);
    }
}
