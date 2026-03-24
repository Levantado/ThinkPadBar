use libc::{getifaddrs, ifaddrs, sockaddr_in, AF_INET};
use std::ffi::{CStr, CString};
use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::Read;
use std::mem::MaybeUninit;
use std::net::Ipv4Addr;

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
    pub cpu_str: String,
    pub mem_str: String,
    pub swap_str: String,
    pub temp_str: String,
    pub net_down_str: String,
    pub net_up_str: String,
    pub disk_root_str: String,
    pub disk_boot_str: String,
}

pub struct SysMonitor {
    pub last_data: SysData,
    last_cpu_total: u64,
    last_cpu_idle: u64,
    last_net_down: u64,
    last_net_up: u64,
    stat_buf: String,
    mem_buf: String,
}

pub fn read_temperature_celsius() -> Option<f32> {
    for i in 0..5 {
        let path = format!("/sys/class/thermal/thermal_zone{}/temp", i);
        if let Ok(temp_str) = fs::read_to_string(path) {
            if let Ok(temp_raw) = temp_str.trim().parse::<f32>() {
                let temp = temp_raw / 1000.0;
                if temp > 20.0 {
                    return Some(temp);
                }
            }
        }
    }
    None
}

impl SysMonitor {
    pub fn new() -> Self {
        Self {
            last_data: SysData::default(),
            last_cpu_total: 0,
            last_cpu_idle: 0,
            last_net_down: 0,
            last_net_up: 0,
            stat_buf: String::with_capacity(1024),
            mem_buf: String::with_capacity(2048),
        }
    }

    pub fn update(&mut self, fast: bool) -> SysData {
        let mut data = std::mem::take(&mut self.last_data);

        // 1. CPU Usage
        if let Ok(mut file) = File::open("/proc/stat") {
            self.stat_buf.clear();
            if file.read_to_string(&mut self.stat_buf).is_ok() {
                if let Some(line) = self.stat_buf.lines().next() {
                    let mut total = 0_u64;
                    let mut idle = 0_u64;
                    let mut parsed = 0_usize;
                    for (idx, value) in line.split_whitespace().skip(1).enumerate() {
                        if let Ok(v) = value.parse::<u64>() {
                            if idx == 3 {
                                idle = v;
                            }
                            total = total.saturating_add(v);
                            parsed += 1;
                        }
                    }

                    if parsed >= 4 {
                        let total_diff = total.saturating_sub(self.last_cpu_total);
                        let idle_diff = idle.saturating_sub(self.last_cpu_idle);
                        if total_diff > 0 {
                            data.cpu_usage = (1.0 - (idle_diff as f32 / total_diff as f32)) * 100.0;
                        }
                        self.last_cpu_total = total;
                        self.last_cpu_idle = idle;
                    }
                }
            }
        }
        write_percent_string(&mut data.cpu_str, data.cpu_usage.round() as u64);

        // 2. Memory Usage
        if let Ok(mut file) = File::open("/proc/meminfo") {
            self.mem_buf.clear();
            if file.read_to_string(&mut self.mem_buf).is_ok() {
                let mut total = 0;
                let mut free = 0;
                let mut avail = 0;
                let mut s_total = 0;
                let mut s_free = 0;
                for line in self.mem_buf.lines() {
                    if line.starts_with("MemTotal:") {
                        total = parse_mem_kb(line);
                    } else if line.starts_with("MemFree:") {
                        free = parse_mem_kb(line);
                    } else if line.starts_with("MemAvailable:") {
                        avail = parse_mem_kb(line);
                    } else if line.starts_with("SwapTotal:") {
                        s_total = parse_mem_kb(line);
                    } else if line.starts_with("SwapFree:") {
                        s_free = parse_mem_kb(line);
                    }
                }
                data.mem_total = total * 1024;
                let used_kb = if avail > 0 {
                    total.saturating_sub(avail)
                } else {
                    total.saturating_sub(free)
                };
                data.mem_used = used_kb * 1024;
                data.swap_total = s_total * 1024;
                data.swap_used = s_total.saturating_sub(s_free) * 1024;
            }
        }
        let mem_percent = if data.mem_total > 0 {
            (data.mem_used as f64 / data.mem_total as f64 * 100.0).round() as u64
        } else {
            0
        };
        write_percent_string(&mut data.mem_str, mem_percent);

        let swap_percent = if data.swap_total > 0 {
            (data.swap_used as f64 / data.swap_total as f64 * 100.0).round() as u64
        } else {
            0
        };
        write_percent_string(&mut data.swap_str, swap_percent);

        // 3. Temperature
        if !fast {
            data.temp = read_temperature_celsius().unwrap_or(0.0);
            write_temp_string(&mut data.temp_str, data.temp);
        }

        // 4. Network
        let mut current_net_down = 0;
        let mut current_net_up = 0;
        if let Ok(netdev) = fs::read_to_string("/proc/net/dev") {
            for line in netdev.lines().skip(2) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() > 9 {
                    let iface = parts[0].trim_end_matches(':');
                    if iface.starts_with("wl")
                        || iface.starts_with("en")
                        || iface.starts_with("eth")
                    {
                        current_net_down += parts[1].parse::<u64>().unwrap_or(0);
                        current_net_up += parts[9].parse::<u64>().unwrap_or(0);
                        if !fast
                            || data.ip_address == "Disconnected"
                            || data.ip_address == "Loading..."
                        {
                            if let Some(ip) = get_iface_ip_native(iface) {
                                data.ip_address = ip;
                            }
                        }
                    }
                }
            }
        }
        data.net_down = current_net_down.saturating_sub(self.last_net_down);
        data.net_up = current_net_up.saturating_sub(self.last_net_up);
        self.last_net_down = current_net_down;
        self.last_net_up = current_net_up;

        write_net_rate_string(&mut data.net_down_str, data.net_down);
        write_net_rate_string(&mut data.net_up_str, data.net_up);

        // 5. Disks
        if !fast {
            if let Some((total, used)) = get_disk_usage("/") {
                data.disk_root_total = total;
                data.disk_root_used = used;
                let root_percent = if total > 0 {
                    (used as f64 / total as f64 * 100.0).round() as u64
                } else {
                    0
                };
                write_percent_string(&mut data.disk_root_str, root_percent);
            }
            if let Some((total, used)) = get_disk_usage("/boot") {
                data.disk_boot_total = total;
                data.disk_boot_used = used;
                let boot_percent = if total > 0 {
                    (used as f64 / total as f64 * 100.0).round() as u64
                } else {
                    0
                };
                write_percent_string(&mut data.disk_boot_str, boot_percent);
            }
        }

        self.last_data = data;
        self.last_data.clone()
    }
}

fn parse_mem_kb(line: &str) -> u64 {
    line.split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

fn write_percent_string(out: &mut String, value: u64) {
    out.clear();
    let _ = write!(out, "{}%", value);
}

fn write_temp_string(out: &mut String, temp: f32) {
    out.clear();
    let _ = write!(out, "{}°C", temp.round() as u64);
}

fn write_net_rate_string(out: &mut String, bytes: u64) {
    out.clear();
    if bytes > 1024 * 1024 {
        let _ = write!(out, "{:.1} MB/s", bytes as f64 / (1024.0 * 1024.0));
    } else {
        let _ = write!(out, "{} KB/s", bytes / 1024);
    }
}

fn get_iface_ip_native(target_iface: &str) -> Option<String> {
    let mut ifap: *mut ifaddrs = std::ptr::null_mut();
    unsafe {
        if getifaddrs(&mut ifap) == 0 {
            let mut curr = ifap;
            while !curr.is_null() {
                let iface_name = CStr::from_ptr((*curr).ifa_name).to_string_lossy();
                if iface_name == target_iface {
                    let addr = (*curr).ifa_addr;
                    if !addr.is_null() && (*addr).sa_family == AF_INET as u16 {
                        let sock_in = addr as *const sockaddr_in;
                        let s_addr = (*sock_in).sin_addr.s_addr;
                        // sin_addr is in network byte order
                        let ip = Ipv4Addr::from(u32::from_be(s_addr));
                        libc::freeifaddrs(ifap);
                        return Some(ip.to_string());
                    }
                }
                curr = (*curr).ifa_next;
            }
            libc::freeifaddrs(ifap);
        }
    }
    None
}

fn get_disk_usage(path: &str) -> Option<(u64, u64)> {
    let c_path = CString::new(path).ok()?;
    let mut stats = MaybeUninit::<libc::statvfs>::uninit();
    unsafe {
        if libc::statvfs(c_path.as_ptr(), stats.as_mut_ptr()) == 0 {
            let stats = stats.assume_init();
            let total = stats.f_blocks * stats.f_frsize;
            let free = stats.f_bfree * stats.f_frsize;
            Some((total, total.saturating_sub(free)))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_mem_kb;

    #[test]
    fn parse_mem_kb_extracts_numeric_value() {
        assert_eq!(parse_mem_kb("MemTotal:       16384256 kB"), 16_384_256);
    }

    #[test]
    fn parse_mem_kb_returns_zero_for_missing_number() {
        assert_eq!(parse_mem_kb("MemTotal:       kB"), 0);
        assert_eq!(parse_mem_kb("Invalid line"), 0);
    }
}
