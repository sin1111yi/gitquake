use chrono::NaiveDate;
use chrono::{Datelike, Duration, Local, Months, TimeZone, Timelike};

use crate::config::HeatScale;
use crate::model::Commit;

/// Nearest-rank percentile. `p` is in 0..=100.
pub fn percentile(sorted: &[u64], p: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let p = p.clamp(0.0, 100.0);
    let idx = ((p / 100.0) * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

/// Map an absolute change volume onto 0..=1.
pub fn normalize(v: u64, reference: u64, scale: HeatScale, gamma: f64) -> f64 {
    if reference == 0 {
        return if v > 0 { 1.0 } else { 0.0 };
    }
    let raw = match scale {
        HeatScale::Log => ((1.0 + v as f64).ln()) / ((1.0 + reference as f64).ln()),
        HeatScale::Linear => v as f64 / reference as f64,
    };
    raw.clamp(0.0, 1.0).powf(gamma.clamp(0.05, 10.0))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    pub start: i64,
    pub end: i64,
}

impl Window {
    pub fn new(start: i64, end: i64) -> Window {
        Window {
            start: start.min(end),
            end: start.max(end),
        }
    }

    pub fn span(&self) -> i64 {
        (self.end - self.start).max(1)
    }

    /// Column for a timestamp, or `None` when outside the window.
    #[allow(dead_code)]
    pub fn x(&self, ts: i64, width: usize) -> Option<usize> {
        if width == 0 || ts < self.start || ts > self.end {
            return None;
        }
        let frac = (ts - self.start) as f64 / self.span() as f64;
        Some(((frac * (width.saturating_sub(1)) as f64).round() as usize).min(width - 1))
    }

    /// Timestamp at a column.
    #[allow(dead_code)]
    pub fn time_at(&self, x: usize, width: usize) -> i64 {
        if width <= 1 {
            return self.start;
        }
        let frac = x.min(width - 1) as f64 / (width - 1) as f64;
        self.start + (frac * self.span() as f64).round() as i64
    }

    /// Shrink the window by `factor` (factor > 1 means zoom in) around `at`.
    pub fn zoom(&self, at: i64, factor: f64, bounds: Window) -> Window {
        let span = self.span() as f64 / factor;
        let min_span: f64 = 60.0;
        let full = bounds.span() as f64;
        let span = span.clamp(min_span.min(full), full);
        let at = at.clamp(self.start, self.end);
        let frac = (at - self.start) as f64 / self.span() as f64;
        let mut start = at as f64 - span * frac;
        let mut end = start + span;
        if start < bounds.start as f64 {
            start = bounds.start as f64;
            end = start + span;
        }
        if end > bounds.end as f64 {
            end = bounds.end as f64;
            start = end - span;
        }
        Window::new(start.round() as i64, end.round() as i64)
    }

    /// Pan by a fraction of the current span (positive = newer).
    pub fn pan(&self, frac: f64, bounds: Window) -> Window {
        let delta = (self.span() as f64 * frac).round() as i64;
        let span = self.span();
        let mut start = self.start + delta;
        start = start.clamp(bounds.start, (bounds.end - span).max(bounds.start));
        Window::new(start, start + span)
    }

    pub fn is_full(&self, bounds: Window) -> bool {
        self.span() >= bounds.span() - 1
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Bucket {
    pub count: u32,
    /// Total insertions + deletions for every commit in this column.
    pub volume: u64,
    /// Largest single-commit volume, used to pick a representative commit.
    pub max_volume: u64,
    pub commit: Option<usize>,
}

/// Piecewise-linear scale for commit counts.
///
/// Bands are powers of two (`0-1`, `1-2`, `2-4`, `4-8`, ...), and every band
/// receives the same amount of vertical space. This keeps 100 and 150 commits
/// visibly apart instead of letting a smooth saturation curve flatten them.
/// Only the largest count among the visible columns is needed.
#[derive(Debug, Clone)]
pub struct CountScale {
    breaks: Vec<u64>,
}

impl CountScale {
    pub fn new(max_count: u64) -> CountScale {
        let max = max_count.max(1);
        let mut breaks = vec![0u64];
        let mut next = 1u64;
        while next < max {
            breaks.push(next);
            next = next.saturating_mul(2);
        }
        breaks.push(max);
        CountScale { breaks }
    }

    /// Map a count in `0..=max_count` to `0..=1`.
    pub fn height(&self, count: u64) -> f64 {
        if count == 0 {
            return 0.0;
        }
        let max = *self.breaks.last().unwrap_or(&1);
        let count = count.min(max);
        let bands = (self.breaks.len() - 1) as f64;
        let band = (self.breaks.partition_point(|&b| b <= count) - 1).min(self.breaks.len() - 2);
        let lo = self.breaks[band];
        let hi = self.breaks[band + 1];
        let frac = if hi > lo {
            (count - lo) as f64 / (hi - lo) as f64
        } else {
            1.0
        };
        ((band as f64 + frac) / bands).clamp(0.0, 1.0)
    }
}

/// Aggregate commits into one bucket per terminal column.
#[allow(dead_code)]
pub fn bucketize(commits: &[Commit], window: Window, width: usize) -> Vec<Bucket> {
    let mut cols = vec![
        Bucket {
            count: 0,
            volume: 0,
            max_volume: 0,
            commit: None,
        };
        width
    ];
    for (idx, c) in commits.iter().enumerate() {
        if let Some(x) = window.x(c.timestamp, width) {
            let col = &mut cols[x];
            let v = c.volume();
            col.count += 1;
            col.volume += v;
            if col.commit.is_none() || v >= col.max_volume {
                col.max_volume = v;
                col.commit = Some(idx);
            }
        }
    }
    cols
}

/// Maps timestamps to columns using a blend of wall-clock time and commit rank.
///
/// With `weight = 1.0` this is pure time: a week with two commits takes as much
/// room as a week with two hundred. With `weight = 0.5` commits also get a say,
/// so bursts of work stay legible while quiet gaps keep some width.
pub struct Projection {
    pub window: Window,
    pub width: usize,
    pub weight: f64,
    /// Commit indices inside the window, ascending by timestamp.
    pub order: Vec<usize>,
    times: Vec<i64>,
}

impl Projection {
    /// History from `git::load` is sorted newest-first, so the visible range
    /// can be found with binary search instead of walking the whole history on
    /// every frame. Unsorted input still works through the fallback path.
    pub fn new(window: Window, width: usize, commits: &[Commit], weight: f64) -> Projection {
        let sorted = commits.windows(2).all(|w| w[0].timestamp >= w[1].timestamp);
        let mut order: Vec<usize> = if sorted {
            let first = commits.partition_point(|c| c.timestamp > window.end);
            let last = commits.partition_point(|c| c.timestamp >= window.start);
            (first..last).rev().collect()
        } else {
            (0..commits.len())
                .filter(|&i| {
                    let t = commits[i].timestamp;
                    t >= window.start && t <= window.end
                })
                .collect()
        };
        order.sort_by_key(|&i| (commits[i].timestamp, i));
        let times = order.iter().map(|&i| commits[i].timestamp).collect();
        Projection {
            window,
            width,
            weight: weight.clamp(0.0, 1.0),
            order,
            times,
        }
    }

    fn frac_to_col(&self, frac: f64) -> usize {
        if self.width <= 1 {
            return 0;
        }
        let f = frac.clamp(0.0, 1.0);
        ((f * (self.width - 1) as f64).round() as usize).min(self.width - 1)
    }

    fn time_frac(&self, ts: i64) -> f64 {
        (ts - self.window.start) as f64 / self.window.span() as f64
    }

    /// Column for the commit at position `j` in `order`.
    pub fn x_commit(&self, j: usize) -> Option<usize> {
        if self.width == 0 || j >= self.order.len() {
            return None;
        }
        let n = self.order.len();
        let rank = if n <= 1 {
            0.5
        } else {
            j as f64 / (n - 1) as f64
        };
        let tf = self.time_frac(self.times[j]);
        Some(self.frac_to_col(self.weight * tf + (1.0 - self.weight) * rank))
    }

    /// Rank fraction for an arbitrary time, interpolated between commits.
    fn rank_time(&self, ts: i64) -> f64 {
        let n = self.times.len();
        if n <= 1 {
            return 0.5;
        }
        let i = self.times.partition_point(|&t| t < ts);
        if i == 0 {
            0.0
        } else if i >= n {
            1.0
        } else {
            let a = self.times[i - 1];
            let b = self.times[i];
            let f = if b > a {
                (ts - a) as f64 / (b - a) as f64
            } else {
                1.0
            };
            ((i - 1) as f64 + f) / (n - 1) as f64
        }
    }

    /// Column for an arbitrary timestamp (used for axis ticks and the cursor).
    pub fn x_time(&self, ts: i64) -> Option<usize> {
        if self.width == 0 || ts < self.window.start || ts > self.window.end {
            return None;
        }
        let tf = self.time_frac(ts);
        let rf = self.rank_time(ts);
        Some(self.frac_to_col(self.weight * tf + (1.0 - self.weight) * rf))
    }
}

/// Bucket commits using a [`Projection`] instead of raw time.
pub fn project_buckets(commits: &[Commit], proj: &Projection, width: usize) -> Vec<Bucket> {
    let mut cols = vec![
        Bucket {
            count: 0,
            volume: 0,
            max_volume: 0,
            commit: None,
        };
        width
    ];
    for (j, &ci) in proj.order.iter().enumerate() {
        let Some(x) = proj.x_commit(j) else { continue };
        if x >= width {
            continue;
        }
        let c = &commits[ci];
        let v = c.volume();
        let col = &mut cols[x];
        col.count += 1;
        col.volume += v;
        if col.commit.is_none() || v >= col.max_volume {
            col.max_volume = v;
            col.commit = Some(ci);
        }
    }
    cols
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickUnit {
    Hour,
    Day,
    Month,
    Year,
}

pub fn choose_unit(span_secs: i64, width: usize) -> TickUnit {
    let target = (width / 10).max(3) as i64;
    let candidates = [
        (TickUnit::Hour, 3_600i64),
        (TickUnit::Day, 86_400),
        (TickUnit::Month, 30 * 86_400),
        (TickUnit::Year, 365 * 86_400),
    ];
    for (unit, secs) in candidates {
        if span_secs / secs <= target {
            return unit;
        }
    }
    TickUnit::Year
}

/// Generate `(column, label)` ticks aligned to calendar boundaries.
#[allow(dead_code)]
pub fn ticks(window: Window, width: usize, unit: TickUnit) -> Vec<(usize, String)> {
    if width == 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut last_x = None;
    for (ts, label) in tick_stamps(window, unit) {
        if let Some(x) = window.x(ts, width) {
            if last_x == Some(x) {
                continue;
            }
            last_x = Some(x);
            out.push((x, label));
        }
    }
    out
}

/// Like [`ticks`], but positions come from a [`Projection`].
pub fn ticks_projected(proj: &Projection, unit: TickUnit) -> Vec<(usize, String)> {
    if proj.width == 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut last_x = None;
    for (ts, label) in tick_stamps(proj.window, unit) {
        if let Some(x) = proj.x_time(ts) {
            if last_x == Some(x) {
                continue;
            }
            last_x = Some(x);
            out.push((x, label));
        }
    }
    out
}

/// Ticks placed at evenly spaced commits, labelled with their dates.
///
/// Used when the horizontal mapping is a blend: calendar boundaries would
/// bunch up in sparse stretches, so showing "every nth commit" stays honest.
pub fn ticks_ranked(proj: &Projection, commits: &[Commit]) -> Vec<(usize, String)> {
    let n = proj.order.len();
    if n == 0 || proj.width == 0 {
        return Vec::new();
    }
    let target = (proj.width / 10).max(2);
    let step = ((n as f64 / target as f64).ceil() as usize).max(1);
    let span = proj.window.span();
    let mut out: Vec<(usize, String)> = Vec::new();
    let mut last_x = None;
    let mut last_label = String::new();
    let mut last_label_x = 0usize;
    let mut j = 0;
    while j < n {
        let ts = commits[proj.order[j]].timestamp;
        if let Some(x) = proj.x_commit(j) {
            let label = date_label(ts, span);
            let repeat_soon = label == last_label && x.saturating_sub(last_label_x) < 24;
            if last_x != Some(x) && !repeat_soon {
                last_x = Some(x);
                last_label = label.clone();
                last_label_x = x;
                out.push((x, label));
            }
        }
        j += step;
    }
    if let Some(x) = proj.x_commit(n - 1) {
        let label = date_label(commits[proj.order[n - 1]].timestamp, span);
        if last_x != Some(x) && !(label == last_label && x.saturating_sub(last_label_x) < 24) {
            out.push((x, label));
        }
    }
    out
}

fn date_label(ts: i64, span: i64) -> String {
    let fmt = if span < 2 * 86_400 {
        "%H:%M"
    } else if span < 90 * 86_400 {
        "%m-%d"
    } else if span < 2 * 365 * 86_400 {
        "%Y-%m"
    } else {
        "%Y"
    };
    Local
        .timestamp_opt(ts, 0)
        .single()
        .map(|d| d.format(fmt).to_string())
        .unwrap_or_default()
}

fn tick_stamps(window: Window, unit: TickUnit) -> Vec<(i64, String)> {
    let fmt = match unit {
        TickUnit::Hour => "%H:%M",
        TickUnit::Day => "%m-%d",
        TickUnit::Month => "%Y-%m",
        TickUnit::Year => "%Y",
    };
    let mut stamps = Vec::new();
    let Some(start_dt) = Local.timestamp_opt(window.start, 0).single() else {
        return Vec::new();
    };

    match unit {
        TickUnit::Hour => {
            let mut t = start_dt
                .with_minute(0)
                .and_then(|d| d.with_second(0))
                .and_then(|d| d.with_nanosecond(0))
                .map(|d| d + Duration::hours(1));
            let mut guard = 0;
            while let Some(dt) = t {
                if dt.timestamp() > window.end || guard > 4_000 {
                    break;
                }
                stamps.push((dt.timestamp(), dt.format(fmt).to_string()));
                t = Some(dt + Duration::hours(1));
                guard += 1;
            }
        }
        TickUnit::Day => {
            let mut t = start_dt
                .date_naive()
                .succ_opt()
                .and_then(|d| d.and_hms_opt(0, 0, 0))
                .and_then(|n| Local.from_local_datetime(&n).single());
            let mut guard = 0;
            while let Some(dt) = t {
                if dt.timestamp() > window.end || guard > 4_000 {
                    break;
                }
                stamps.push((dt.timestamp(), dt.format(fmt).to_string()));
                t = Some(dt + Duration::days(1));
                guard += 1;
            }
        }
        TickUnit::Month => {
            let mut date = NaiveDate::from_ymd_opt(start_dt.year(), start_dt.month(), 1)
                .and_then(|d| d.checked_add_months(Months::new(1)));
            let mut guard = 0;
            while let Some(d) = date {
                let Some(dt) = d
                    .and_hms_opt(0, 0, 0)
                    .and_then(|n| Local.from_local_datetime(&n).single())
                else {
                    break;
                };
                if dt.timestamp() > window.end || guard > 400 {
                    break;
                }
                stamps.push((dt.timestamp(), dt.format(fmt).to_string()));
                date = d.checked_add_months(Months::new(1));
                guard += 1;
            }
        }
        TickUnit::Year => {
            let mut date = NaiveDate::from_ymd_opt(start_dt.year() + 1, 1, 1);
            let mut guard = 0;
            while let Some(d) = date {
                let Some(dt) = d
                    .and_hms_opt(0, 0, 0)
                    .and_then(|n| Local.from_local_datetime(&n).single())
                else {
                    break;
                };
                if dt.timestamp() > window.end || guard > 400 {
                    break;
                }
                stamps.push((dt.timestamp(), dt.format(fmt).to_string()));
                date = NaiveDate::from_ymd_opt(d.year() + 1, 1, 1);
                guard += 1;
            }
        }
    }

    stamps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_extremes() {
        assert_eq!(normalize(0, 100, HeatScale::Log, 1.0), 0.0);
        assert!((normalize(100, 100, HeatScale::Log, 1.0) - 1.0).abs() < 1e-9);
        assert!(normalize(1_000, 100, HeatScale::Log, 1.0) <= 1.0);
        assert!((normalize(50, 100, HeatScale::Linear, 1.0) - 0.5).abs() < 1e-9);
        assert_eq!(normalize(5, 0, HeatScale::Log, 1.0), 1.0);
        assert_eq!(normalize(0, 0, HeatScale::Log, 1.0), 0.0);
    }

    #[test]
    fn percentile_works() {
        assert_eq!(percentile(&[], 95.0), 0);
        assert_eq!(percentile(&[1], 95.0), 1);
        let v: Vec<u64> = (1..=100).collect();
        assert_eq!(percentile(&v, 0.0), 1);
        assert_eq!(percentile(&v, 100.0), 100);
        assert_eq!(percentile(&v, 50.0), 51);
    }

    #[test]
    fn window_mapping() {
        let w = Window::new(0, 1000);
        assert_eq!(w.x(0, 11), Some(0));
        assert_eq!(w.x(500, 11), Some(5));
        assert_eq!(w.x(1000, 11), Some(10));
        assert_eq!(w.x(-1, 11), None);
        assert_eq!(w.x(1001, 11), None);
        assert_eq!(w.time_at(5, 11), 500);
    }

    #[test]
    fn zoom_and_pan_clamp() {
        let bounds = Window::new(0, 10_000);
        let w = Window::new(0, 10_000);
        let z = w.zoom(5_000, 2.0, bounds);
        assert!(z.span() < bounds.span());
        assert!(z.start >= 0 && z.end <= 10_000);
        let full = z.zoom(5_000, 0.001, bounds);
        assert_eq!(full, bounds);
        let p = z.pan(-10.0, bounds);
        assert_eq!(p.start, bounds.start);
        assert_eq!(p.span(), z.span());
        let p = z.pan(10.0, bounds);
        assert_eq!(p.end, bounds.end);
    }

    #[test]
    fn tick_units_scale_with_span() {
        assert_eq!(choose_unit(3_600, 80), TickUnit::Hour);
        assert_eq!(choose_unit(86_400 * 3, 80), TickUnit::Day);
        assert_eq!(choose_unit(86_400 * 200, 80), TickUnit::Month);
        assert_eq!(choose_unit(86_400 * 4_000, 80), TickUnit::Year);
    }

    #[test]
    fn ticks_are_inside_window() {
        let w = Window::new(0, 86_400 * 400);
        let mut width = 0;
        for (x, label) in ticks(w, 90, TickUnit::Month) {
            assert!(x < 90);
            assert!(!label.is_empty());
            width += 1;
        }
        assert!(width > 1);
    }

    #[test]
    fn count_scale_separates_large_counts() {
        let scale = CountScale::new(150);
        assert_eq!(scale.height(0), 0.0);
        assert!((scale.height(150) - 1.0).abs() < 1e-9);
        assert!(scale.height(100) > scale.height(64));
        assert!(scale.height(128) > scale.height(100));
        assert!(scale.height(150) > scale.height(128));
        // 100 and 150 must not collapse onto the same row.
        assert!(scale.height(150) - scale.height(100) > 0.1);

        let wide = CountScale::new(10_000);
        assert!(wide.height(150) > wide.height(100));
        assert!(wide.height(150) - wide.height(100) > 0.01);
    }

    #[test]
    fn count_scale_handles_tiny_maxima() {
        assert_eq!(CountScale::new(0).height(0), 0.0);
        assert!((CountScale::new(1).height(1) - 1.0).abs() < 1e-9);
        assert!((CountScale::new(2).height(1) - 0.5).abs() < 1e-9);
        assert!((CountScale::new(3).height(2) - 2.0 / 3.0).abs() < 1e-9);
    }

    fn bare_commit(timestamp: i64) -> Commit {
        Commit {
            hash: format!("{timestamp:040x}"),
            short: format!("{timestamp:x}"),
            author_name: "Test".into(),
            author_email: "test@example.com".into(),
            timestamp,
            subject: "test".into(),
            refs: Vec::new(),
            parents: Vec::new(),
            files: Vec::new(),
            insertions: 0,
            deletions: 0,
        }
    }

    #[test]
    fn projection_only_walks_the_visible_range() {
        let commits: Vec<Commit> = [100, 90, 80, 70, 60]
            .iter()
            .map(|&t| bare_commit(t))
            .collect();
        let proj = Projection::new(Window::new(75, 95), 80, &commits, 1.0);
        assert_eq!(proj.order, vec![2, 1]);
        assert_eq!(proj.times, vec![80, 90]);

        let buckets = project_buckets(&commits, &proj, 80);
        assert_eq!(buckets.iter().map(|b| b.count).sum::<u32>(), 2);
    }
}
