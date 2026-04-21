use anyhow::Result;
use regex::Regex;

use crate::cast::{CastEvent, CastHeader, read_cast_file};
use crate::metrics::{BtopFrame, MemMetrics, NetMetrics};
use crate::vterm::VirtualTerminal;

/// Extract btop metrics from a cast file.
pub fn extract_from_file(path: &std::path::Path) -> Result<Vec<BtopFrame>> {
    let (header, events) = read_cast_file(path)?;
    Ok(extract_from_events(&header, &events))
}

/// Extract btop metrics from pre-loaded events.
pub fn extract_from_events(header: &CastHeader, events: &[CastEvent]) -> Vec<BtopFrame> {
    let mut vt = VirtualTerminal::new(header.width as usize, header.height as usize);
    let mut frames = Vec::new();

    for event in events {
        if event.event_type != "o" {
            continue;
        }
        vt.sync_end_seen = false;
        vt.feed(event.data.as_bytes());

        if vt.sync_end_seen {
            if let Some(frame) = extract_frame(&vt, event.time) {
                frames.push(frame);
            }
        }
    }

    frames
}

/// Extract metrics from the current vterm grid state.
fn extract_frame(vt: &VirtualTerminal, timestamp: f64) -> Option<BtopFrame> {
    let text = vt.all_text();

    let cpu_total = extract_cpu_total(&text);
    let cpu_cores = extract_cpu_cores(&text);
    let load_avg = extract_load_avg(&text);
    let mem = extract_mem(&text);
    let net = extract_net(&text);

    // Only return a frame if we got at least one useful metric.
    if cpu_total.is_some() || !cpu_cores.is_empty() || mem.is_some() || net.is_some() {
        Some(BtopFrame {
            timestamp,
            cpu_total,
            cpu_cores,
            load_avg,
            mem,
            net,
        })
    } else {
        None
    }
}

fn extract_cpu_total(text: &[String]) -> Option<f64> {
    let re = Regex::new(r"CPU\s+.*?(\d+)\s*%").unwrap();
    for line in text {
        if let Some(caps) = re.captures(line) {
            if let Ok(v) = caps[1].parse::<f64>() {
                if (0.0..=100.0).contains(&v) {
                    return Some(v);
                }
            }
        }
    }
    None
}

fn extract_cpu_cores(text: &[String]) -> Vec<f64> {
    let re = Regex::new(r"[Cc](?:ore|pu)?(\d+)\s+.*?(\d+)\s*%").unwrap();
    let mut cores: Vec<(u32, f64)> = Vec::new();
    for line in text {
        for caps in re.captures_iter(line) {
            if let (Ok(idx), Ok(val)) = (caps[1].parse::<u32>(), caps[2].parse::<f64>()) {
                if (0.0..=100.0).contains(&val) {
                    cores.push((idx, val));
                }
            }
        }
    }
    cores.sort_by_key(|(idx, _)| *idx);
    cores.into_iter().map(|(_, v)| v).collect()
}

fn extract_load_avg(text: &[String]) -> Option<(f64, f64, f64)> {
    let re = Regex::new(r"Load avg:\s+([\d.]+)\s+([\d.]+)\s+([\d.]+)").unwrap();
    for line in text {
        if let Some(caps) = re.captures(line) {
            if let (Ok(a), Ok(b), Ok(c)) = (
                caps[1].parse::<f64>(),
                caps[2].parse::<f64>(),
                caps[3].parse::<f64>(),
            ) {
                return Some((a, b, c));
            }
        }
    }
    None
}

fn extract_mem(text: &[String]) -> Option<MemMetrics> {
    // Find "Total: <size>" in memory box area.
    let total_re = Regex::new(r"Total:\s+(\S+(?:\s+\S+)?)").unwrap();
    let pct_re = Regex::new(r"(Used|Available|Cached|Free):\s+.*?(\d+)\s*%").unwrap();

    let mut total = None;
    let mut used_pct = None;
    let mut available_pct = None;
    let mut cached_pct = None;
    let mut free_pct = None;

    for line in text {
        if total.is_none() {
            if let Some(caps) = total_re.captures(line) {
                total = Some(caps[1].to_string());
            }
        }
        for caps in pct_re.captures_iter(line) {
            let label = &caps[1];
            let pct: f64 = match caps[2].parse() {
                Ok(v) => v,
                Err(_) => continue,
            };
            match label {
                "Used" => used_pct = Some(pct),
                "Available" => available_pct = Some(pct),
                "Cached" => cached_pct = Some(pct),
                "Free" => free_pct = Some(pct),
                _ => {}
            }
        }
    }

    total.map(|total| MemMetrics {
        total,
        used_pct,
        available_pct,
        cached_pct,
        free_pct,
    })
}

fn extract_net(text: &[String]) -> Option<NetMetrics> {
    let down_re = Regex::new(r"▼\s+([\d.]+\s*\S+/s)").unwrap();
    let up_re = Regex::new(r"▲\s+([\d.]+\s*\S+/s)").unwrap();

    let mut download = None;
    let mut upload = None;

    for line in text {
        if download.is_none() {
            if let Some(caps) = down_re.captures(line) {
                download = Some(caps[1].to_string());
            }
        }
        if upload.is_none() {
            if let Some(caps) = up_re.captures(line) {
                upload = Some(caps[1].to_string());
            }
        }
    }

    if download.is_some() || upload.is_some() {
        Some(NetMetrics { download, upload })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_cpu_total() {
        let text = vec![
            "╭─ cpu ─────────────────────────────────╮".to_string(),
            "│  CPU  ████████████░░░░░░░░░░░░  42 %  │".to_string(),
        ];
        assert_eq!(extract_cpu_total(&text), Some(42.0));
    }

    #[test]
    fn test_extract_load_avg() {
        let text = vec!["  Load avg: 1.23 0.45 0.67  ".to_string()];
        assert_eq!(extract_load_avg(&text), Some((1.23, 0.45, 0.67)));
    }

    #[test]
    fn test_extract_mem() {
        let text = vec![
            "│  Total:  15.6 GiB                     │".to_string(),
            "│  Used: █████████░░░ 8.2 GiB  53 %     │".to_string(),
            "│  Free: ░░░░░░░░░░░  7.4 GiB  47 %     │".to_string(),
        ];
        let mem = extract_mem(&text).unwrap();
        assert_eq!(mem.total, "15.6 GiB");
        assert_eq!(mem.used_pct, Some(53.0));
        assert_eq!(mem.free_pct, Some(47.0));
    }

    #[test]
    fn test_extract_net() {
        let text = vec!["  ▼ 1.2 MiB/s  ▲ 256 KiB/s  ".to_string()];
        let net = extract_net(&text).unwrap();
        assert_eq!(net.download, Some("1.2 MiB/s".to_string()));
        assert_eq!(net.upload, Some("256 KiB/s".to_string()));
    }
}
