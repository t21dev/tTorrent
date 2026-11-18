//! One way to write sizes, speeds, times and dates, everywhere (VERIFY.md: no
//! inconsistent formats).

use chrono::{Local, TimeZone};

const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];

/// Binary sizes with decimal-looking units, like qBittorrent: `5.9 GB`, `870 MB`, `0 B`.
pub fn size(bytes: u64) -> String {
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut v = bytes as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if v >= 100.0 {
        format!("{v:.0} {}", UNITS[i])
    } else if v >= 10.0 {
        format!("{v:.1} {}", UNITS[i])
    } else {
        format!("{v:.2} {}", UNITS[i])
    }
}

pub fn speed(bps: u64) -> String {
    format!("{}/s", size(bps))
}

/// Empty for zero, so idle rows stay quiet.
pub fn speed_or_blank(bps: u64) -> String {
    if bps == 0 { String::new() } else { speed(bps) }
}

pub fn limit(bps: u64) -> String {
    if bps == 0 { "∞".into() } else { speed(bps) }
}

/// `62.0%`, `100%`, `0.0%`.
pub fn percent(p: f32) -> String {
    let p = (p * 100.0).clamp(0.0, 100.0);
    if p >= 100.0 {
        "100%".into()
    } else {
        // Never round 99.96 % up to "100.0%" while a piece is still missing.
        let t = (p * 10.0).floor() / 10.0;
        format!("{t:.1}%")
    }
}

pub fn ratio(r: f32) -> String {
    if !r.is_finite() || r > 9999.0 {
        "∞".into()
    } else {
        format!("{r:.2}")
    }
}

/// `4m`, `2h 10m`, `3d 4h`, `< 1m`.
pub fn eta(secs: u64) -> String {
    if secs >= 100 * 86_400 {
        return "∞".into();
    }
    let (d, h, m) = (secs / 86_400, (secs % 86_400) / 3600, (secs % 3600) / 60);
    if d > 0 {
        format!("{d}d {h}h")
    } else if h > 0 {
        format!("{h}h {m}m")
    } else if m > 0 {
        format!("{m}m")
    } else {
        "< 1m".into()
    }
}

/// `Nov 17, 21:33` in local time.
pub fn date(unix: i64) -> String {
    match Local.timestamp_opt(unix, 0).single() {
        Some(t) => t.format("%b %d, %H:%M").to_string(),
        None => String::new(),
    }
}

/// `Nov 17, 2026 21:33` in local time (dates that may be years old).
pub fn date_long(unix: i64) -> String {
    match Local.timestamp_opt(unix, 0).single() {
        Some(t) => t.format("%b %d, %Y %H:%M").to_string(),
        None => String::new(),
    }
}

/// `5m ago`, `just now`.
pub fn ago(unix: i64) -> String {
    let now = Local::now().timestamp();
    let d = (now - unix).max(0) as u64;
    if d < 60 {
        "just now".into()
    } else {
        format!("{} ago", eta(d))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes() {
        assert_eq!(size(0), "0 B");
        assert_eq!(size(1023), "1023 B");
        assert_eq!(size(1024), "1.00 KB");
        assert_eq!(size(870 * 1024 * 1024), "870 MB");
        assert_eq!(size(6_335_076_761), "5.90 GB");
        assert_eq!(size(15 * 1024 * 1024 * 1024), "15.0 GB");
    }

    #[test]
    fn percents_never_round_up_to_done() {
        assert_eq!(percent(0.9999), "99.9%");
        assert_eq!(percent(1.0), "100%");
        assert_eq!(percent(0.62), "62.0%");
    }

    #[test]
    fn etas() {
        assert_eq!(eta(30), "< 1m");
        assert_eq!(eta(240), "4m");
        assert_eq!(eta(7800), "2h 10m");
        assert_eq!(eta(90_000), "1d 1h");
    }
}
