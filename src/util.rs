use chrono::{Local, TimeZone};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::theme::{Quality, Rgb};

/// Format a count with thousands separators.
pub fn comma(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// Compact human duration, e.g. `now`, `5m`, `3h`, `2d`, `4w`, `3mo`, `2y`.
pub fn humanize(ts: i64, now: i64) -> String {
    let d = now.saturating_sub(ts);
    if d < 60 {
        return "now".into();
    }
    if d < 3_600 {
        return format!("{}m", d / 60);
    }
    if d < 86_400 {
        return format!("{}h", d / 3_600);
    }
    let days = d / 86_400;
    if days < 7 {
        return format!("{days}d");
    }
    let weeks = days / 7;
    if weeks < 5 {
        return format!("{weeks}w");
    }
    let months = days / 30;
    if months < 12 {
        return format!("{months}mo");
    }
    format!("{}y", days / 365)
}

/// Human duration for gap separators, e.g. `3 days`, `2 weeks`.
pub fn humanize_long(secs: i64) -> String {
    let secs = secs.max(0);
    let days = secs / 86_400;
    if secs < 3_600 {
        format!("{} min", (secs / 60).max(1))
    } else if secs < 86_400 {
        plural(secs / 3_600, "hour", "hours")
    } else if days < 7 {
        plural(days, "day", "days")
    } else if days < 60 {
        plural(days / 7, "week", "weeks")
    } else if days < 365 {
        plural(days / 30, "month", "months")
    } else {
        plural(days / 365, "year", "years")
    }
}

/// `"1 commit"` / `"1,204 commits"`.
pub fn count(n: u64, one: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{} {many}", comma(n))
    }
}

fn plural(n: i64, one: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {many}")
    }
}

pub fn absolute(ts: i64, fmt: &str) -> String {
    Local
        .timestamp_opt(ts, 0)
        .single()
        .map(|dt| dt.format(fmt).to_string())
        .unwrap_or_else(|| "-".into())
}

/// Display width of a string in terminal columns.
pub fn display_width(s: &str) -> usize {
    s.width()
}

/// Truncate to `w` display columns, appending an ellipsis when shortened.
pub fn truncate_w(s: &str, w: usize) -> String {
    if s.width() <= w {
        return s.to_string();
    }
    if w == 0 {
        return String::new();
    }
    if w == 1 {
        return "…".into();
    }
    let mut out = String::new();
    let mut used = 0usize;
    for ch in s.chars() {
        let cw = ch.width().unwrap_or(0);
        if used + cw > w - 1 {
            break;
        }
        out.push(ch);
        used += cw;
    }
    out.push('…');
    out
}

/// Pad with spaces to `w` display columns (truncating when too long).
pub fn pad_w(s: &str, w: usize) -> String {
    let t = truncate_w(s, w);
    let pad = w.saturating_sub(t.width());
    let mut out = t;
    out.extend(std::iter::repeat_n(' ', pad));
    out
}

pub fn ansi_fg(rgb: Rgb, q: Quality) -> String {
    match q {
        Quality::True => format!("\x1b[38;2;{};{};{}m", rgb.0, rgb.1, rgb.2),
        Quality::Ansi256 => format!("\x1b[38;5;{}m", rgb.to_256()),
        Quality::Ansi16 => format!("\x1b[38;5;{}m", rgb.to_16()),
    }
}

/// Wrap `text` in a foreground colour (and optionally bold) when colours are enabled.
pub fn paint(text: &str, rgb: Rgb, q: Quality, enabled: bool, bold: bool) -> String {
    if !enabled {
        return text.to_string();
    }
    let mut s = String::new();
    if bold {
        s.push_str("\x1b[1m");
    }
    s.push_str(&ansi_fg(rgb, q));
    s.push_str(text);
    s.push_str("\x1b[0m");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commas() {
        assert_eq!(comma(0), "0");
        assert_eq!(comma(999), "999");
        assert_eq!(comma(1_284), "1,284");
        assert_eq!(comma(1_234_567), "1,234,567");
    }

    #[test]
    fn humanize_units() {
        assert_eq!(humanize(0, 10), "now");
        assert_eq!(humanize(0, 120), "2m");
        assert_eq!(humanize(0, 7_200), "2h");
        assert_eq!(humanize(0, 3 * 86_400), "3d");
        assert_eq!(humanize(0, 21 * 86_400), "3w");
        assert_eq!(humanize(0, 90 * 86_400), "3mo");
        assert_eq!(humanize(0, 800 * 86_400), "2y");
    }

    #[test]
    fn unicode_truncation() {
        assert_eq!(truncate_w("hello world", 8), "hello w…");
        assert_eq!(truncate_w("短提交信息", 5), "短提…");
        assert_eq!(truncate_w("ok", 4), "ok");
        assert_eq!(pad_w("ab", 4), "ab  ");
    }
}
