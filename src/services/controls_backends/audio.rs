use std::process::{Command, Stdio};

#[derive(Debug, Default, Clone, Copy)]
pub struct WpctlAudioBackend;

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
                use tokio::io::{AsyncBufReadExt, BufReader};
                use tokio::process::Command as AsyncCommand;

                loop {
                    let mut child = match AsyncCommand::new("pactl")
                        .arg("subscribe")
                        .stdout(Stdio::piped())
                        .spawn()
                    {
                        Ok(child) => child,
                        Err(_) => {
                            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                            continue;
                        }
                    };

                    let Some(stdout) = child.stdout.take() else {
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        continue;
                    };
                    let mut reader = BufReader::new(stdout).lines();

                    while let Ok(Some(line)) = reader.next_line().await {
                        if line.contains("sink")
                            || line.contains("source")
                            || line.contains("server")
                        {
                            let _ = output.try_send(
                                crate::services::controls::ControlsEvent::AudioServerChanged,
                            );
                        }
                    }

                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }),
        )
    }
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
    use super::parse_wpctl_volume;

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
}
