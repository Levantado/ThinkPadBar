use super::LauncherBackend;

pub struct RofiLauncherBackend;

impl LauncherBackend for RofiLauncherBackend {
    fn backend_name(&self) -> &'static str {
        "rofi"
    }

    fn toggle_launcher(&self) -> bool {
        if let Ok(mut child) = std::process::Command::new("rofi")
            .args(["-replace", "-show", "drun"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            std::mem::drop(std::thread::spawn(move || {
                let _ = child.wait();
            }));
            true
        } else {
            false
        }
    }
}
