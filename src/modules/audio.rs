use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioInfo {
    pub volume: u32,
    pub muted: bool,
}

pub fn get_info() -> AudioInfo {
    if let Ok(output) = Command::new("wpctl")
        .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
        .output()
    {
        if let Ok(s) = String::from_utf8(output.stdout) {
            let s = s.trim();
            let muted = s.contains("[MUTED]");

            if let Some(vol_part) = s.split_whitespace().nth(1) {
                if let Ok(vol) = vol_part.parse::<f32>() {
                    return AudioInfo {
                        volume: (vol * 100.0).round() as u32,
                        muted,
                    };
                }
            }
        }
    }
    AudioInfo {
        volume: 0,
        muted: false,
    }
}

pub async fn set_volume(percent: u32) {
    let vol_str = format!("{:.2}", percent as f32 / 100.0);
    let _ = tokio::process::Command::new("wpctl")
        .args(["set-volume", "@DEFAULT_AUDIO_SINK@", &vol_str])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await;
}

pub async fn toggle_mute() {
    let _ = tokio::process::Command::new("wpctl")
        .args(["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await;
}

pub fn subscription() -> iced::Subscription<crate::app::Message> {
    struct AudioListener;
    iced::Subscription::run_with_id(
        std::any::TypeId::of::<AudioListener>(),
        iced::stream::channel(1, |mut output| async move {
            use std::process::Stdio;
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
                    if line.contains("sink") || line.contains("source") || line.contains("server") {
                        let _ = output.try_send(crate::app::Message::RefreshAudioMic);
                    }
                }
                // If pactl fails, wait before retry
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }),
    )
}
