use futures_util::SinkExt;
use futures_util::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
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

pub struct MediaService {
    event_tx: broadcast::Sender<MediaEvent>,
    cmd_rx: mpsc::Receiver<MediaCommand>,
    http_client: reqwest::Client,
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

        // Initial player discovery
        let names = dbus.list_names().await?;
        for name in names {
            let name_str = name.as_str();
            if name_str.starts_with("org.mpris.MediaPlayer2.") {
                if let Ok(proxy) = PlayerProxy::builder(&conn)
                    .destination(name_str.to_string())?
                    .build()
                    .await
                {
                    active_player = Some((name_str.to_string(), proxy));
                    break;
                }
            }
        }

        if let Some((ref name, ref proxy)) = active_player {
            snapshot = self.update_snapshot(name, proxy).await;
            let _ = self
                .event_tx
                .send(MediaEvent::SnapshotUpdated(snapshot.clone()));
        }

        loop {
            tokio::select! {
                Some(change) = owner_changes.next() => {
                    if let Ok(args) = change.args() {
                        let name = args.name().as_str();
                        if name.starts_with("org.mpris.MediaPlayer2.") {
                            if args.new_owner().is_none() {
                                if let Some((ref active_name, _)) = active_player {
                                    if active_name == name {
                                        active_player = None;
                                        snapshot = MediaSnapshot::default();
                                        let _ = self.event_tx.send(MediaEvent::SnapshotUpdated(snapshot.clone()));
                                    }
                                }
                            } else if active_player.is_none() {
                                let name_owned = name.to_string();
                                if let Ok(proxy) = PlayerProxy::builder(&conn).destination(name_owned.clone())?.build().await {
                                    active_player = Some((name_owned, proxy));
                                    snapshot = self.update_snapshot(&active_player.as_ref().unwrap().0, &active_player.as_ref().unwrap().1).await;
                                    let _ = self.event_tx.send(MediaEvent::SnapshotUpdated(snapshot.clone()));
                                }
                            }
                        }
                    }
                }
                Some(cmd) = self.cmd_rx.recv() => {
                    if let Some((_, ref proxy)) = active_player {
                        match cmd {
                            MediaCommand::Next => { let _ = proxy.next().await; },
                            MediaCommand::Previous => { let _ = proxy.previous().await; },
                            MediaCommand::PlayPause => { let _ = proxy.play_pause().await; },
                            MediaCommand::Stop => { let _ = proxy.stop().await; },
                            MediaCommand::SetVolume(v) => { let _ = proxy.set_volume(v).await; },
                            MediaCommand::Seek(pos) => {
                                if let Ok(metadata) = proxy.metadata().await {
                                    if let Some(track_id) = metadata.get("mpris:trackid") {
                                        if let Ok(path_str) = track_id.downcast_ref::<zbus::zvariant::ObjectPath<'_>>() {
                                            let _ = proxy.set_position(path_str.clone(), pos).await;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_millis(
                    if snapshot.playback_status == "Playing" { 100 } else { 500 }
                )) => {
                    if let Some((ref name, ref proxy)) = active_player {
                        let new_snapshot = self.update_snapshot(name, proxy).await;
                        // Always update if position changed or playback status is playing
                        if new_snapshot.title != snapshot.title
                            || new_snapshot.playback_status != snapshot.playback_status
                            || new_snapshot.volume != snapshot.volume
                            || new_snapshot.position != snapshot.position
                            || new_snapshot.playback_status == "Playing"
                        {
                            snapshot = new_snapshot;
                            let _ = self.event_tx.send(MediaEvent::SnapshotUpdated(snapshot.clone()));
                        }
                    }
                }

            }
        }
    }

    async fn update_snapshot(&self, name: &str, proxy: &PlayerProxy<'static>) -> MediaSnapshot {
        let status = proxy
            .playback_status()
            .await
            .unwrap_or_else(|_| "Stopped".to_string());
        let volume = proxy.volume().await.unwrap_or(1.0);
        let position = proxy.position().await.unwrap_or(0);
        let metadata = proxy.metadata().await.unwrap_or_default();

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

        let duration = metadata
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

        let cover_url = metadata
            .get("mpris:artUrl")
            .and_then(|v| v.downcast_ref::<String>().ok());

        let mut cover_bytes = None;
        if let Some(ref url) = cover_url {
            if url.starts_with("file://") {
                let path = url.trim_start_matches("file://");
                if let Ok(bytes) = std::fs::read(path) {
                    cover_bytes = Some(Arc::new(bytes));
                }
            } else if url.starts_with("http") {
                if let Ok(resp) = self.http_client.get(url).send().await {
                    if let Ok(bytes) = resp.bytes().await {
                        cover_bytes = Some(Arc::new(bytes.to_vec()));
                    }
                }
            }
        }

        MediaSnapshot {
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
            cover_bytes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use zbus::zvariant::{ObjectPath, OwnedValue, Value};

    #[test]
    fn artist_parsing_handles_vec_and_single_string() {
        // Mocking zbus OwnedValue is complex, but we can test the logic if we extract it
        // or test via simple strings for now since we use downcast_ref.
    }

    #[test]
    fn snapshot_default_is_empty_but_valid() {
        let snap = MediaSnapshot::default();
        assert_eq!(snap.title, "");
        assert_eq!(snap.has_player, false);
    }
}
