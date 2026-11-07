//! Presentation-only local facts. Never sent to the Kernel, Actor or memory.
#[derive(Default)]
pub(super) struct LocalInfo {
    pub system: String,
    pub logical_cpus: Option<usize>,
    pub directory: Option<String>,
    pub memory: Option<Memory>,
}
#[derive(Clone, Copy)]
pub(super) struct Memory {
    pub total: u64,
    pub available: Option<u64>,
}
impl LocalInfo {
    pub fn collect() -> Self {
        Self {
            system: format!(
                "{} · {}",
                if cfg!(windows) {
                    "Windows"
                } else if cfg!(target_os = "macos") {
                    "macOS"
                } else {
                    std::env::consts::OS
                },
                std::env::consts::ARCH
            ),
            logical_cpus: std::thread::available_parallelism().ok().map(usize::from),
            directory: std::env::current_dir()
                .ok()
                .map(|p| p.to_string_lossy().into_owned()),
            memory: memory(),
        }
    }
    pub fn refresh(&mut self) {
        self.memory = memory();
    }
}
impl Memory {
    pub fn label(self) -> String {
        let gib = 1024.0 * 1024.0 * 1024.0;
        match self.available.filter(|a| *a <= self.total) {
            Some(a) => format!(
                "{:.1} / {:.1} GiB",
                (self.total - a) as f64 / gib,
                self.total as f64 / gib
            ),
            None => format!("总计 {:.1} GiB · 已用未知", self.total as f64 / gib),
        }
    }
}
#[cfg(windows)]
fn memory() -> Option<Memory> {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    // All fields are integer counters; Win32 requires the structure size first.
    let mut status: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
    status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
    if unsafe { GlobalMemoryStatusEx(&mut status) } == 0 || status.ullTotalPhys == 0 {
        return None;
    }
    Some(Memory {
        total: status.ullTotalPhys,
        available: Some(status.ullAvailPhys),
    })
}
#[cfg(target_os = "linux")]
fn memory() -> Option<Memory> {
    use std::io::Read;
    let mut text = String::new();
    std::fs::File::open("/proc/meminfo")
        .ok()?
        .take(8192)
        .read_to_string(&mut text)
        .ok()?;
    linux_memory(&text)
}
#[cfg(any(target_os = "linux", test))]
fn linux_memory(text: &str) -> Option<Memory> {
    let value = |name: &str| {
        text.lines().find_map(|line| {
            let mut parts = line.split_whitespace();
            if parts.next()? != name {
                return None;
            }
            let kb = parts.next()?.parse::<u64>().ok()?;
            (parts.next() == Some("kB"))
                .then(|| kb.checked_mul(1024))
                .flatten()
        })
    };
    let total = value("MemTotal:")?;
    (total > 0).then(|| Memory {
        total,
        available: value("MemAvailable:").filter(|v| *v <= total),
    })
}
#[cfg(target_os = "macos")]
fn memory() -> Option<Memory> {
    let mut total = 0_u64;
    let mut size = std::mem::size_of::<u64>();
    // hw.memsize is the physical byte count. Do not invent a cross-platform
    // "used" value from free pages; macOS reclaimable/compressed accounting differs.
    let result = unsafe {
        libc::sysctlbyname(
            c"hw.memsize".as_ptr(),
            std::ptr::from_mut(&mut total).cast(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    (result == 0 && size == std::mem::size_of::<u64>() && total > 0).then_some(Memory {
        total,
        available: None,
    })
}
#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn memory() -> Option<Memory> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_or_invalid_available_memory_is_never_presented_as_zero_usage() {
        assert!(
            linux_memory("MemTotal: 4096 kB\nMemAvailable: 9000 kB")
                .unwrap()
                .label()
                .contains("已用未知")
        );
        assert!(linux_memory("MemTotal: 0 kB").is_none());
        assert!(linux_memory("MemTotal: 4096 MB").is_none());
        assert_eq!(
            linux_memory("MemTotal: 16777216 kB\nMemAvailable: 8388608 kB")
                .unwrap()
                .label(),
            "8.0 / 16.0 GiB"
        );
    }
    #[test]
    fn native_local_memory_probe_is_bounded_and_returns_physical_total() {
        let info = LocalInfo::collect();
        assert!(!info.system.is_empty());
        #[cfg(any(windows, target_os = "linux", target_os = "macos"))]
        assert!(
            info.memory
                .is_some_and(|m| m.total > 0 && m.available.is_none_or(|a| a <= m.total))
        );
    }
}
