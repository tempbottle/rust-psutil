//! Read information about the operating system from `/proc`.

use std::str::FromStr;
use std::path::Path;
use std::collections::HashMap;

use std::io::{Result, ErrorKind, Error};

use utils::read_file;

#[derive(Debug)]
pub struct VirtualMemory {
    /// Amount of total memory
    pub total: u64,

    /// Amount of memory available for new processes
    pub available: u64,

    /// Percent of memory used
    pub percent: f32,

    /// Memory currently in use
    pub used: u64,

    /// Memory not being used
    pub free: u64,

    /// Memory currently in use
    pub active: u64,

    /// Memory that is not in use
    pub inactive: u64,

    /// Temporary storage for raw disk blocks
    pub buffers: u64,

    /// Memory used by the page cache
    pub cached: u64,

    /// Amount of memory consumed by tmpfs filesystems
    pub shared: u64,
}

impl VirtualMemory {
    pub fn new(
        total: u64,
        available: u64,
        shared: u64,
        free: u64,
        buffers: u64,
        cached: u64,
        active: u64,
        inactive: u64,
    ) -> VirtualMemory {
        let used = total - free - cached - buffers;

        VirtualMemory {
            total: total,
            available: available,
            shared: shared,
            free: free,
            buffers: buffers,
            cached: cached,
            active: active,
            inactive: inactive,
            used: used,
            percent: (used as f32 / total as f32) * 100.0,
        }
    }
}
#[derive(Debug)]
pub struct SwapMemory {
    /// Amount of total swap memory
    pub total: u64,

    /// Amount of used swap memory
    pub used: u64,

    /// Amount of free swap memory
    pub free: u64,

    /// Percent of sway memory used
    pub percent: f32,

    /// Amount of memory swapped in from disk
    pub sin: u64,

    /// Amount of memory swapped to disk
    pub sout: u64,
}

impl SwapMemory {
    pub fn new(total: u64, free: u64, sin: u64, sout: u64) -> SwapMemory {
        let used = total - free;
        let percent = (used as f32 / total as f32) * 100.0;

        SwapMemory {
            total: total,
            used: used,
            free: free,
            percent: percent,
            sin: sin,
            sout: sout,
        }
    }
}

/// Returns the system uptime in seconds.
///
/// `/proc/uptime` contains the system uptime and idle time.
pub fn uptime() -> isize {
    let data = read_file(Path::new("/proc/uptime")).unwrap();
    uptime_internal(&data)
}

/// Returns the system uptime in seconds.
///
/// Input should be in the format '12489513.08 22906637.29\n'
fn uptime_internal(data: &str) -> isize {
    let numbers: Vec<&str> = data.split(' ').collect();
    let uptime: Vec<&str> = numbers[0].split('.').collect();
    FromStr::from_str(uptime[0]).unwrap()
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn uptime_parses() {
        assert_eq!(uptime_internal("12489513.08 22906637.29\n"), 12489513);
    }

    #[test]
    fn make_map_spaces() {
        let input = "field1: 23\nfield2: 45\nfield3: 100\n";
        let out = make_map(&input);
        assert_eq!(out.get("field1:"), Some(&23));
        assert_eq!(out.get("field2:"), Some(&45));
    }

    #[test]
    fn make_map_tabs() {
        let input = "field1:\t\t\t45\nfield2:\t\t100\nfield4:\t\t\t\t4\n";
        let out = make_map(&input);
        assert_eq!(out.get("field1:"), Some(&45));
        assert_eq!(out.get("field2:"), Some(&100));
    }

    #[test]
    fn make_map_with_ext() {
        let input = "field1: 100 kB\n field2: 200";
        let out = make_map(&input);
        assert_eq!(out.get("field1:"), Some(&102400));
        assert_eq!(out.get("field2:"), Some(&200));
    }

    #[test]
    fn multipler_kb() {
        assert_eq!(get_multiplier(&mut vec!["100", "kB"]), Some(1024));
    }

    #[test]
    fn multiplier_none() {
        assert_eq!(get_multiplier(&mut vec!["100", "200"]), None);
    }

    #[test]
    fn multiplier_last() {
        assert_eq!(
            get_multiplier(&mut vec!["100", "200", "400", "700", "kB"]),
            Some(1024)
        );
    }
}

fn not_found(key: &str) -> Error {
    Error::new(ErrorKind::NotFound, format!("{} not found", key))
}

/// Returns information about virtual memory usage
///
/// `/proc/meminfo` contains the virtual memory statistics
pub fn virtual_memory() -> Result<VirtualMemory> {
    let data = read_file(Path::new("/proc/meminfo"))?;
    let mem_info = make_map(&data);

    let total = *mem_info.get("MemTotal:").ok_or(not_found("MemTotal"))?;
    let free = *mem_info.get("MemFree:").ok_or(not_found("MemFree"))?;
    let buffers = *mem_info.get("Buffers:").ok_or(not_found("Buffers"))?;
    let cached = *mem_info.get("Cached:").ok_or(not_found("Cached"))?;
    let active = *mem_info.get("Active:").ok_or(not_found("Active"))?;
    let inactive = *mem_info.get("Inactive:").ok_or(not_found("Inactive"))?;

    // MemAvailable was introduced in kernel 3.14. The original psutil computes it if it's not
    // found, but since 3.14 has already reached EOL, let's assume that it's there.
    let available = *mem_info.get("MemAvailable:").ok_or(
        not_found("MemAvailable"),
    )?;

    // Shmem was introduced in 2.6.19
    let shared = *mem_info.get("Shmem:").ok_or(not_found("Shmem"))?;

    Ok(VirtualMemory::new(
        total,
        available,
        shared,
        free,
        buffers,
        cached,
        active,
        inactive,
    ))
}

/// Returns information about swap memory usage
///
/// `/proc/meminfo` and `/proc/vmstat` contains the information
pub fn swap_memory() -> Result<SwapMemory> {
    let data = read_file(Path::new("/proc/meminfo"))?;
    let swap_info = make_map(&data);

    let vmstat = read_file(Path::new("/proc/vmstat"))?;
    let vmstat_info = make_map(&vmstat);

    let total = *swap_info.get("SwapTotal:").ok_or(not_found("SwapTotal"))?;
    let free = *swap_info.get("SwapFree:").ok_or(not_found("SwapFree"))?;
    let sin = *vmstat_info.get("pswpin").ok_or(not_found("pswpin"))?;
    let sout = *vmstat_info.get("pswpout").ok_or(not_found("pswpout"))?;

    Ok(SwapMemory::new(total, free, sin, sout))
}

fn get_multiplier(fields: &mut Vec<&str>) -> Option<u64> {
    if let Some(ext) = fields.pop() {
        let multiplier = match ext {
            "kB" => Some(1024),
            _ => None,
        };
        fields.push(ext);

        multiplier
    } else {
        None
    }
}

fn make_map(data: &str) -> HashMap<&str, u64> {
    let lines: Vec<&str> = data.lines().collect();
    let mut map = HashMap::new();

    for line in lines {
        let mut fields: Vec<&str> = line.split_whitespace().collect();
        let key = fields[0];
        let mut value = fields[1].parse::<u64>().unwrap();

        if let Some(multiplier) = get_multiplier(&mut fields) {
            value = value * multiplier;
        }

        map.insert(key, value);
    }

    map
}
