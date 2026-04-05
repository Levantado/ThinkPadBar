use iced::futures::SinkExt;
use std::{
    ffi::CString,
    fs, io,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};
use tracing::{info, warn};
use zbus::{proxy, zvariant::OwnedObjectPath, Connection};

const LOGIN1_SERVICE: &str = "org.freedesktop.login1";
const LOGIN1_MANAGER_PATH: &str = "/org/freedesktop/login1";
const BACKLIGHT_SUBSYSTEM: &str = "backlight";
const BACKLIGHT_SYSFS_ROOT: &str = "/sys/class/backlight";
const BRIGHTNESS_EVENT_RETRY_DELAY: Duration = Duration::from_secs(2);

#[proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
trait LoginManager {
    fn get_session_by_pid(&self, pid: u32) -> zbus::Result<OwnedObjectPath>;
}

#[proxy(
    interface = "org.freedesktop.login1.Session",
    default_service = "org.freedesktop.login1"
)]
trait LoginSession {
    fn set_brightness(&self, subsystem: &str, name: &str, brightness: u32) -> zbus::Result<()>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SysfsBrightnessBackend;

impl super::BrightnessBackend for SysfsBrightnessBackend {
    fn backend_name(&self) -> &'static str {
        "logind+sysfs+brightnessctl+light"
    }

    fn capability_mode(&self) -> crate::services::capabilities::CapabilityMode {
        crate::services::capabilities::CapabilityMode::Hybrid
    }

    fn snapshot(&self) -> crate::services::controls::BrightnessSnapshot {
        crate::services::controls::BrightnessSnapshot::from_percent(
            read_backlight_percent().unwrap_or(0),
        )
    }

    fn set_brightness(&self, percent: u32) -> super::BackendFuture<'_, ()> {
        let percent = percent.clamp(0, 100);
        Box::pin(async move {
            info!("Attempting to set brightness to {}%", percent);
            let logind_target =
                match tokio::task::spawn_blocking(move || first_backlight_target(percent)).await {
                    Ok(target) => target,
                    Err(error) => {
                        warn!("Brightness target discovery join failed: {}", error);
                        None
                    }
                };

            if let Some(target) = logind_target {
                match set_brightness_via_logind(&target).await {
                    Ok(()) => {
                        info!(
                            "Brightness set to {} ({:?}) via logind session API",
                            target.raw, target.device_name
                        );
                        return;
                    }
                    Err(error) => {
                        warn!("Brightness update via logind failed: {}", error);
                    }
                }
            }

            let fallback = match tokio::task::spawn_blocking(move || {
                try_set_brightness_without_pkexec(percent)
            })
            .await
            {
                Ok(BrightnessWriteResult::Done) => None,
                Ok(BrightnessWriteResult::NeedsPkexec(target)) => Some(target),
                Ok(BrightnessWriteResult::Unavailable) => None,
                Err(error) => {
                    warn!("Brightness worker join failed: {}", error);
                    None
                }
            };

            if let Some(target) = fallback {
                if crate::services::controls_backends::privileged::write_file_via_pkexec(
                    &target.path,
                    &target.raw.to_string(),
                )
                .await
                .is_ok()
                {
                    info!("Brightness set to {} via pkexec", target.raw);
                } else {
                    warn!("Brightness update failed: logind, direct write, brightnessctl, light, and pkexec paths unavailable or denied");
                }
            }
        })
    }

    fn subscription(&self) -> iced::Subscription<crate::services::controls::ControlsEvent> {
        struct BrightnessListener;

        iced::Subscription::run_with_id(
            std::any::TypeId::of::<BrightnessListener>(),
            iced::stream::channel(1, move |mut output| async move {
                loop {
                    match tokio::task::spawn_blocking(wait_for_backlight_event).await {
                        Ok(Ok(())) => {
                            if output
                                .send(crate::services::controls::ControlsEvent::Brightness)
                                .await
                                .is_err()
                            {
                                return;
                            }
                        }
                        Ok(Err(error)) => {
                            warn!("Brightness event listener failed: {}", error);
                            tokio::time::sleep(BRIGHTNESS_EVENT_RETRY_DELAY).await;
                        }
                        Err(error) => {
                            warn!("Brightness event listener join failed: {}", error);
                            tokio::time::sleep(BRIGHTNESS_EVENT_RETRY_DELAY).await;
                        }
                    }
                }
            }),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BrightnessTarget {
    path: PathBuf,
    device_name: String,
    raw: u32,
}

fn read_backlight_percent() -> Option<u32> {
    let entries = fs::read_dir(BACKLIGHT_SYSFS_ROOT).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        let current = fs::read_to_string(path.join("brightness"))
            .ok()?
            .trim()
            .parse::<u32>()
            .ok()?;
        let max = fs::read_to_string(path.join("max_brightness"))
            .ok()?
            .trim()
            .parse::<u32>()
            .ok()?;
        if max > 0 {
            return Some(percent_from_raw(current, max));
        }
    }
    None
}

fn write_backlight_percent(percent: u32) -> bool {
    let Ok(entries) = fs::read_dir(BACKLIGHT_SYSFS_ROOT) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let max = fs::read_to_string(path.join("max_brightness"))
            .ok()
            .and_then(|value| value.trim().parse::<u32>().ok())
            .unwrap_or(0);
        if max == 0 {
            continue;
        }
        let target = raw_from_percent(percent, max);
        let brightness_path = path.join("brightness");
        match fs::write(&brightness_path, target.to_string()) {
            Ok(_) => {
                info!(
                    "Brightness set to {} ({}%) via direct write",
                    target, percent
                );
                return true;
            }
            Err(error) => {
                warn!(
                    "Direct brightness write failed for {:?}: {}",
                    brightness_path, error
                );
            }
        }
    }
    false
}

fn first_backlight_target(percent: u32) -> Option<BrightnessTarget> {
    let entries = fs::read_dir(BACKLIGHT_SYSFS_ROOT).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        let max = fs::read_to_string(path.join("max_brightness"))
            .ok()
            .and_then(|value| value.trim().parse::<u32>().ok())
            .unwrap_or(0);
        if max == 0 {
            continue;
        }
        if let Some(target) = brightness_target_for_entry(&path, max, percent) {
            return Some(target);
        }
    }
    None
}

fn brightness_target_for_entry(
    entry_path: &Path,
    max: u32,
    percent: u32,
) -> Option<BrightnessTarget> {
    Some(BrightnessTarget {
        path: entry_path.join("brightness"),
        device_name: backlight_device_name(entry_path)?,
        raw: raw_from_percent(percent, max),
    })
}

fn backlight_device_name(entry_path: &Path) -> Option<String> {
    entry_path.file_name()?.to_str().map(ToOwned::to_owned)
}

enum BrightnessWriteResult {
    Done,
    NeedsPkexec(BrightnessTarget),
    Unavailable,
}

async fn set_brightness_via_logind(target: &BrightnessTarget) -> Result<(), String> {
    let connection = Connection::system()
        .await
        .map_err(|error| error.to_string())?;
    let manager = LoginManagerProxy::builder(&connection)
        .destination(LOGIN1_SERVICE)
        .map_err(|error| error.to_string())?
        .path(LOGIN1_MANAGER_PATH)
        .map_err(|error| error.to_string())?
        .build()
        .await
        .map_err(|error| error.to_string())?;
    let session_path = manager
        .get_session_by_pid(std::process::id())
        .await
        .map_err(|error| error.to_string())?;
    let session = LoginSessionProxy::builder(&connection)
        .path(session_path.as_str())
        .map_err(|error| error.to_string())?
        .build()
        .await
        .map_err(|error| error.to_string())?;
    session
        .set_brightness(BACKLIGHT_SUBSYSTEM, &target.device_name, target.raw)
        .await
        .map_err(|error| error.to_string())
}

fn try_set_brightness_without_pkexec(percent: u32) -> BrightnessWriteResult {
    if write_backlight_percent(percent) {
        return BrightnessWriteResult::Done;
    }

    let percent_arg = format!("{}%", percent.clamp(1, 100));
    let brightnessctl_ok = Command::new("brightnessctl")
        .args(["-q", "s", &percent_arg])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    if brightnessctl_ok {
        info!("Brightness set to {} via brightnessctl", percent_arg);
        return BrightnessWriteResult::Done;
    }

    let light_ok = Command::new("light")
        .args(["-S", &percent.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    if light_ok {
        info!("Brightness set to {} via light", percent);
        return BrightnessWriteResult::Done;
    }

    first_backlight_target(percent)
        .map(BrightnessWriteResult::NeedsPkexec)
        .unwrap_or(BrightnessWriteResult::Unavailable)
}

fn wait_for_backlight_event() -> Result<(), String> {
    let watch_paths = brightness_watch_paths(Path::new(BACKLIGHT_SYSFS_ROOT))?;
    let fd = unsafe { libc::inotify_init1(libc::IN_CLOEXEC) };
    if fd < 0 {
        return Err(io::Error::last_os_error().to_string());
    }
    let _fd_guard = InotifyFd(fd);

    let mut watched_any = false;
    for path in watch_paths {
        if add_watch(fd, &path).is_ok() {
            watched_any = true;
        }
    }
    if !watched_any {
        return Err("no watchable backlight paths".to_string());
    }

    let mut buffer = [0_u8; 4096];
    let bytes_read = unsafe { libc::read(fd, buffer.as_mut_ptr().cast(), buffer.len()) };
    if bytes_read < 0 {
        return Err(io::Error::last_os_error().to_string());
    }
    if bytes_read == 0 {
        return Err("brightness watcher reached EOF".to_string());
    }
    Ok(())
}

fn brightness_watch_paths(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut watch_paths = vec![root.to_path_buf()];
    let entries = fs::read_dir(root).map_err(|error| error.to_string())?;
    let mut device_paths = Vec::new();

    for entry in entries.flatten() {
        let entry_path = entry.path();
        if !entry_path.is_dir() {
            continue;
        }
        for candidate in ["brightness", "actual_brightness"] {
            let candidate_path = entry_path.join(candidate);
            if candidate_path.exists() {
                device_paths.push(candidate_path);
            }
        }
    }

    if device_paths.is_empty() {
        return Err("no backlight devices found".to_string());
    }

    watch_paths.extend(device_paths);
    Ok(watch_paths)
}

fn add_watch(fd: i32, path: &Path) -> Result<(), String> {
    let mask = libc::IN_MODIFY
        | libc::IN_CLOSE_WRITE
        | libc::IN_ATTRIB
        | libc::IN_CREATE
        | libc::IN_DELETE
        | libc::IN_MOVED_TO
        | libc::IN_MOVED_FROM;
    let raw_path = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| format!("brightness watch path contains interior NUL: {:?}", path))?;
    let watch_descriptor = unsafe { libc::inotify_add_watch(fd, raw_path.as_ptr(), mask) };
    if watch_descriptor < 0 {
        return Err(io::Error::last_os_error().to_string());
    }
    Ok(())
}

struct InotifyFd(i32);

impl Drop for InotifyFd {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.0);
        }
    }
}

pub(crate) fn percent_from_raw(current: u32, max: u32) -> u32 {
    if max == 0 {
        return 0;
    }
    (current.saturating_mul(100)) / max
}

pub(crate) fn raw_from_percent(percent: u32, max: u32) -> u32 {
    percent.clamp(0, 100).saturating_mul(max) / 100
}

#[cfg(test)]
mod tests {
    use super::{
        backlight_device_name, brightness_target_for_entry, brightness_watch_paths,
        percent_from_raw, raw_from_percent,
    };
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn percent_from_raw_uses_integer_backlight_scale() {
        assert_eq!(percent_from_raw(50, 200), 25);
        assert_eq!(percent_from_raw(0, 0), 0);
    }

    #[test]
    fn raw_from_percent_scales_to_max_brightness() {
        assert_eq!(raw_from_percent(25, 200), 50);
        assert_eq!(raw_from_percent(130, 200), 200);
    }

    #[test]
    fn backlight_device_name_uses_backlight_entry_leaf() {
        assert_eq!(
            backlight_device_name(Path::new("/sys/class/backlight/amdgpu_bl1")),
            Some("amdgpu_bl1".to_string())
        );
        assert_eq!(backlight_device_name(Path::new("/")), None);
    }

    #[test]
    fn brightness_target_for_entry_builds_logind_request_shape() {
        let target =
            brightness_target_for_entry(Path::new("/sys/class/backlight/amdgpu_bl1"), 255, 40)
                .expect("target");
        assert_eq!(target.device_name, "amdgpu_bl1");
        assert_eq!(
            target.path,
            PathBuf::from("/sys/class/backlight/amdgpu_bl1/brightness")
        );
        assert_eq!(target.raw, 102);
    }

    #[test]
    fn brightness_watch_paths_include_root_and_runtime_brightness_files() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("thinkpadbar-brightness-watch-{unique}"));
        let device = root.join("amdgpu_bl1");
        fs::create_dir_all(&device).expect("create backlight root");
        fs::write(device.join("brightness"), "120").expect("write brightness");
        fs::write(device.join("actual_brightness"), "118").expect("write actual_brightness");

        let watch_paths = brightness_watch_paths(&root).expect("watch paths");

        assert!(watch_paths.contains(&root));
        assert!(watch_paths.contains(&device.join("brightness")));
        assert!(watch_paths.contains(&device.join("actual_brightness")));

        let _ = fs::remove_dir_all(&root);
    }
}
