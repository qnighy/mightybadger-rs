use std::fs::File;
use std::io::{BufRead, BufReader};

use crate::payload::{LoadInfo, MemoryInfo, Stats};

pub(crate) fn get_stats() -> Stats {
    Stats {
        mem: get_mem(),
        load: get_load(),
    }
}

fn get_mem() -> Option<MemoryInfo> {
    let file = File::open("/proc/meminfo").ok()?;
    let mut file = BufReader::new(file);
    let mut line = String::new();
    let mut meminfo = MemoryInfo::default();
    loop {
        line.clear();
        let numread = file.read_line(&mut line).unwrap_or(0);
        if numread == 0 {
            break;
        }
        let colon = if let Some(colon) = line.find(':') {
            colon
        } else {
            break;
        };
        let key = line[..colon].trim();
        let value = line[colon + 1..].trim();
        let kbvalue = if value.ends_with(" kB") {
            value[..value.len() - 3].parse::<i64>().ok()
        } else {
            None
        };
        let mbvalue = kbvalue.map(|kbvalue| kbvalue as f64 / 1024.0);
        match key {
            "MemTotal" => meminfo.total = mbvalue,
            "MemFree" => meminfo.free = mbvalue,
            "Buffers" => meminfo.buffers = mbvalue,
            "Cached" => meminfo.cached = mbvalue,
            _ => {}
        };
    }
    if let MemoryInfo {
        free: Some(free),
        buffers: Some(buffers),
        cached: Some(cached),
        ..
    } = meminfo
    {
        meminfo.free_total = Some(free + buffers + cached);
    }
    Some(meminfo)
}

fn get_load() -> Option<LoadInfo> {
    let file = File::open("/proc/loadavg").ok()?;
    let mut file = BufReader::new(file);
    let mut line = String::new();
    file.read_line(&mut line).ok()?;
    let mut loadinfo = LoadInfo::default();
    let mut tokens = line.split(' ').fuse();
    loadinfo.one = tokens.next().and_then(|token| token.parse::<f64>().ok());
    loadinfo.five = tokens.next().and_then(|token| token.parse::<f64>().ok());
    loadinfo.fifteen = tokens.next().and_then(|token| token.parse::<f64>().ok());
    Some(loadinfo)
}
