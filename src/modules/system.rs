use sysinfo::{System, Networks, Disks, Components};

#[derive(Clone, Default, Debug)]
pub struct SysData {
    pub cpu_usage: f32,
    pub mem_total: u64,
    pub mem_used: u64,
    pub swap_total: u64,
    pub swap_used: u64,
    pub temp: f32,
    pub disk_root_total: u64,
    pub disk_root_used: u64,
    pub disk_boot_total: u64,
    pub disk_boot_used: u64,
    pub net_down: u64,
    pub net_up: u64,
    pub ip_address: String,
}

pub struct SysMonitor {
    sys: System,
    networks: Networks,
    disks: Disks,
    components: Components,
}

impl SysMonitor {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_cpu_usage();
        Self {
            sys,
            networks: Networks::new_with_refreshed_list(),
            disks: Disks::new_with_refreshed_list(),
            components: Components::new_with_refreshed_list(),
        }
    }

    pub fn update(&mut self) -> SysData {
        self.sys.refresh_all();
        self.networks.refresh(true);
        self.disks.refresh(true);
        self.components.refresh(true);

        let cpu_usage = self.sys.global_cpu_usage();
        let mem_total = self.sys.total_memory();
        let mem_used = self.sys.used_memory();
        let swap_total = self.sys.total_swap();
        let swap_used = self.sys.used_swap();

        let mut temp = 0.0;
        for comp in &self.components {
            if comp.label().contains("coretemp") || comp.label().contains("Package") || comp.label().contains("acpitz") || comp.label().contains("thinkpad") {
                temp = comp.temperature().unwrap_or(0.0);
                if temp > 0.0 { break; }
            }
        }
        if temp == 0.0 {
            if let Some(c) = self.components.iter().next() {
                temp = c.temperature().unwrap_or(0.0);
            }
        }

        let mut disk_root_total = 0;
        let mut disk_root_used = 0;
        let mut disk_boot_total = 0;
        let mut disk_boot_used = 0;

        for disk in &self.disks {
            let path = disk.mount_point();
            if path == std::path::Path::new("/") {
                disk_root_total = disk.total_space();
                disk_root_used = disk_root_total.saturating_sub(disk.available_space());
            } else if path == std::path::Path::new("/boot") {
                disk_boot_total = disk.total_space();
                disk_boot_used = disk_boot_total.saturating_sub(disk.available_space());
            }
        }

        let mut net_down = 0;
        let mut net_up = 0;
        let mut ip_address = String::from("Disconnected");

        for (iface, data) in &self.networks {
            if iface.starts_with("wl") || iface.starts_with("en") || iface.starts_with("eth") {
                net_down += data.received();
                net_up += data.transmitted();
                
                if ip_address == "Disconnected" {
                    if let Ok(out) = std::process::Command::new("ip").arg("-4").arg("addr").arg("show").arg(iface).output() {
                        let out_str = String::from_utf8_lossy(&out.stdout);
                        for line in out_str.lines() {
                            if line.contains("inet ") {
                                let parts: Vec<&str> = line.split_whitespace().collect();
                                if parts.len() > 1 {
                                    ip_address = parts[1].split('/').next().unwrap_or("").to_string();
                                }
                            }
                        }
                    }
                }
            }
        }

        SysData {
            cpu_usage, mem_total, mem_used, swap_total, swap_used, temp,
            disk_root_total, disk_root_used, disk_boot_total, disk_boot_used,
            net_down, net_up, ip_address
        }
    }
}
