use unicode_width::UnicodeWidthStr;

use crate::config::General;
use crate::model::Commit;
use crate::stats::normalize;
use crate::theme::{Rgb, Theme};
use crate::util::{humanize, pad_w, truncate_w};

#[derive(Debug, Clone)]
pub struct Piece {
    pub text: String,
    pub rgb: Option<Rgb>,
    pub bold: bool,
}

impl Piece {
    fn new(text: impl Into<String>, rgb: Rgb) -> Piece {
        Piece {
            text: text.into(),
            rgb: Some(rgb),
            bold: false,
        }
    }

    fn dim(text: impl Into<String>, theme: &Theme) -> Piece {
        Piece {
            text: text.into(),
            rgb: Some(theme.roles.dim),
            bold: false,
        }
    }

    fn plain(text: impl Into<String>) -> Piece {
        Piece {
            text: text.into(),
            rgb: None,
            bold: false,
        }
    }

    #[allow(dead_code)]
    pub fn width(&self) -> usize {
        self.text.width()
    }
}

#[derive(Clone, Copy)]
pub struct RowCtx<'a> {
    pub cfg: &'a General,
    pub theme: &'a Theme,
    pub now: i64,
    pub p95: u64,
    pub width: usize,
}

/// The single heat value used for both colour and bar height.
pub fn heat_value(volume: u64, reference: u64, cfg: &General) -> f64 {
    normalize(volume, reference.max(1), cfg.heat_scale, cfg.heat_gamma)
}

pub fn heat_t(c: &Commit, p95: u64, cfg: &General) -> f64 {
    if c.is_merge() {
        0.0
    } else {
        heat_value(c.heat_volume(), p95, cfg)
    }
}

pub fn heat_glyph(t: f64) -> char {
    if t < 0.20 {
        '░'
    } else if t < 0.45 {
        '▒'
    } else if t < 0.72 {
        '▓'
    } else {
        '█'
    }
}

pub fn compact_number(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 10_000 {
        format!("{:.0}k", n as f64 / 1_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

pub fn diff_text(c: &Commit, width: usize) -> String {
    if c.is_merge() {
        return pad_w("merge", width);
    }
    if c.volume() == 0 && c.files.iter().any(|f| f.is_binary()) {
        return pad_w("binary", width);
    }
    let s = format!(
        "+{} -{}",
        compact_number(c.insertions),
        compact_number(c.deletions)
    );
    if s.width() >= width {
        truncate_w(&s, width)
    } else {
        format!("{}{}", " ".repeat(width - s.width()), s)
    }
}

fn diff_pieces(c: &Commit, width: usize, theme: &Theme) -> Vec<Piece> {
    if c.is_merge() || (c.volume() == 0 && c.files.iter().any(|f| f.is_binary())) {
        return vec![Piece {
            text: diff_text(c, width),
            rgb: Some(theme.roles.dim),
            bold: false,
        }];
    }
    let plus = format!("{:>6}", format!("+{}", compact_number(c.insertions)));
    let minus = format!("{:>6}", format!("-{}", compact_number(c.deletions)));
    let total = plus.width() + 1 + minus.width();
    let lag = width.saturating_sub(total);
    let mut out = Vec::with_capacity(4);
    if lag > 0 {
        out.push(Piece::plain(" ".repeat(lag)));
    }
    out.push(Piece {
        text: plus,
        rgb: Some(theme.roles.ok),
        bold: true,
    });
    out.push(Piece::plain(" "));
    out.push(Piece {
        text: minus,
        rgb: Some(theme.roles.bad),
        bold: true,
    });
    out
}

pub fn refs_label(c: &Commit, max_refs: usize, max_width: usize) -> String {
    if c.refs.is_empty() {
        return String::new();
    }
    let mut refs = c.refs.clone();
    refs.sort_by_key(|r| match r.kind {
        crate::model::RefKind::Head => 0,
        crate::model::RefKind::Tag => 1,
        crate::model::RefKind::Branch => 2,
        crate::model::RefKind::Remote => 3,
        crate::model::RefKind::Other => 4,
    });
    let mut parts: Vec<String> = Vec::new();
    for r in refs.iter().take(max_refs) {
        let label = format!("[{}]", r.name);
        let candidate = if parts.is_empty() {
            label.clone()
        } else {
            format!("{} {}", parts.join(" "), label)
        };
        if candidate.width() > max_width {
            break;
        }
        parts.push(label);
    }
    parts.join(" ")
}

fn field_width(field: &str, bar_w: usize, refs_w: usize) -> usize {
    match field {
        "heat" => 2,
        "bar" => bar_w + 1,
        "hash" => 8,
        "refs" => {
            if refs_w == 0 {
                0
            } else {
                refs_w + 1
            }
        }
        "author" => 11,
        "time" => 11,
        _ => 0,
    }
}

pub fn row_pieces(c: &Commit, ctx: &RowCtx, dimmed: bool) -> Vec<Piece> {
    let g = ctx.cfg;
    let theme = ctx.theme;
    let mut fields: Vec<String> = g.row_fields.clone();
    if !fields.iter().any(|f| f == "subject") {
        fields.push("subject".into());
    }

    let bar_w = (ctx.width / 8).clamp(4, g.bar_width);
    let refs_w = refs_label(c, 2, 22).width();
    let refs_w = if ctx.width < 64 { 0 } else { refs_w };

    let diff_w = 14;
    let fixed = |fields: &[String]| -> usize {
        fields
            .iter()
            .map(|f| {
                if f == "subject" {
                    0
                } else if f == "diff" {
                    diff_w
                } else {
                    field_width(f, bar_w, refs_w)
                }
            })
            .sum()
    };

    // Ensure the subject keeps a usable width; drop low-priority fields first.
    loop {
        let subject_w = ctx.width.saturating_sub(fixed(&fields));
        if subject_w >= 12 || fields.len() <= 2 {
            break;
        }
        let mut dropped = false;
        for candidate in ["refs", "author", "time", "bar", "diff"] {
            if let Some(pos) = fields.iter().position(|f| f == candidate) {
                fields.remove(pos);
                dropped = true;
                break;
            }
        }
        if !dropped {
            break;
        }
    }

    let subject_w = ctx.width.saturating_sub(fixed(&fields)).max(8);
    let t = heat_t(c, ctx.p95, g);
    let heat_color = if dimmed {
        theme.roles.dim
    } else {
        theme.heat(t)
    };
    let time = if g.relative_time {
        humanize(c.timestamp, ctx.now)
    } else {
        crate::util::absolute(c.timestamp, &g.date_format)
    };

    let mut out: Vec<Piece> = Vec::new();

    for field in &fields {
        match field.as_str() {
            "heat" => {
                let glyph = if c.volume() == 0 && c.is_merge() {
                    '·'
                } else {
                    heat_glyph(t)
                };
                out.push(Piece::new(format!("{glyph} "), heat_color));
            }
            "bar" => {
                let bar = if c.is_merge() || c.heat_volume() == 0 {
                    "·".to_string()
                } else {
                    let n = ((t * bar_w as f64).round() as usize).clamp(1, bar_w);
                    "█".repeat(n)
                };
                out.push(Piece::new(pad_w(&bar, bar_w + 1), heat_color));
            }
            "hash" => out.push(Piece::dim(format!("{} ", c.short), theme)),
            "refs" => {
                if refs_w == 0 {
                    continue;
                }
                let label = refs_label(c, 2, 22);
                if !label.is_empty() {
                    out.push(Piece {
                        text: format!("{label} "),
                        rgb: Some(if dimmed {
                            theme.roles.dim
                        } else {
                            theme.roles.accent
                        }),
                        bold: !dimmed,
                    });
                }
            }
            "subject" => {
                let prefix = if c.is_merge() { "⑂ " } else { "" };
                let avail = subject_w.saturating_sub(prefix.width()).max(1);
                let subject = truncate_w(&c.subject, avail);
                let used = prefix.width() + subject.width();
                let text = format!(
                    "{prefix}{subject}{}",
                    " ".repeat(subject_w.saturating_sub(used))
                );
                out.push(Piece {
                    text,
                    rgb: Some(if dimmed {
                        theme.roles.dim
                    } else {
                        theme.roles.fg
                    }),
                    bold: false,
                });
            }
            "author" => out.push(Piece::dim(format!("{} ", pad_w(&c.author_name, 10)), theme)),
            "time" => out.push(Piece::dim(format!("{} ", pad_w(&time, 10)), theme)),
            "diff" => {
                let mut pieces = diff_pieces(c, diff_w, theme);
                if dimmed {
                    for p in &mut pieces {
                        p.rgb = Some(theme.roles.dim);
                        p.bold = false;
                    }
                }
                out.extend(pieces);
            }
            _ => {}
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::HeatScale;
    use crate::model::{FileStat, GitRef, RefKind};
    use crate::theme::builtin;

    fn commit(ins: u64, del: u64) -> Commit {
        Commit {
            hash: "a".repeat(40),
            short: "a1b2c3d".into(),
            author_name: "Ada Lovelace".into(),
            author_email: "ada@x".into(),
            timestamp: 1_700_000_000,
            subject: "feat: add a streaming parser for the waveform view".into(),
            refs: vec![GitRef {
                kind: RefKind::Head,
                name: "main".into(),
            }],
            parents: vec!["b".repeat(40)],
            files: vec![FileStat {
                path: "src/main.rs".into(),
                insertions: Some(ins),
                deletions: Some(del),
            }],
            insertions: ins,
            deletions: del,
        }
    }

    #[test]
    fn heat_value_is_the_single_mapping() {
        let mut g = General::default();
        assert_eq!(heat_value(0, 100, &g), 0.0);
        assert!((heat_value(100, 100, &g) - 1.0).abs() < 1e-9);
        assert!(heat_value(10, 100, &g) < heat_value(50, 100, &g));
        // same reference and flags as the commit colour/height path
        let c = commit(30, 20);
        assert!((heat_t(&c, 100, &g) - heat_value(50, 100, &g)).abs() < 1e-12);
        g.heat_scale = HeatScale::Linear;
        assert!((heat_value(50, 100, &g) - 0.5).abs() < 1e-9);
        g.heat_gamma = 2.0;
        assert!((heat_value(50, 100, &g) - 0.25).abs() < 1e-9);
    }

    #[test]
    fn row_never_exceeds_width() {
        let theme = builtin("inferno").unwrap();
        for width in [40usize, 60, 80, 120, 200] {
            let g = General::default();
            let ctx = RowCtx {
                cfg: &g,
                theme: &theme,
                now: 1_800_000_000,
                p95: 500,
                width,
            };
            let pieces = row_pieces(&commit(412, 88), &ctx, false);
            let total: usize = pieces.iter().map(|p| p.width()).sum();
            assert!(total <= width, "width {width}: total {total}");
        }
    }

    #[test]
    fn tiny_width_still_keeps_subject() {
        let theme = builtin("inferno").unwrap();
        let g = General::default();
        let ctx = RowCtx {
            cfg: &g,
            theme: &theme,
            now: 1_800_000_000,
            p95: 500,
            width: 24,
        };
        let pieces = row_pieces(&commit(412, 88), &ctx, false);
        let text: String = pieces.iter().map(|p| p.text.as_str()).collect();
        assert!(text.contains('…'));
        assert!(!text.contains("+412"));
    }

    #[test]
    fn heat_gradient_ordering() {
        let g = General {
            heat_scale: HeatScale::Log,
            ..General::default()
        };
        let small = heat_t(&commit(5, 0), 1000, &g);
        let medium = heat_t(&commit(200, 0), 1000, &g);
        let large = heat_t(&commit(900, 100), 1000, &g);
        assert!(small < medium && medium < large);
        assert!(large <= 1.0);
    }

    #[test]
    fn merge_rows_are_dim() {
        let theme = builtin("inferno").unwrap();
        let g = General::default();
        let ctx = RowCtx {
            cfg: &g,
            theme: &theme,
            now: 1_800_000_000,
            p95: 500,
            width: 100,
        };
        let mut c = commit(0, 0);
        c.parents.push("c".repeat(40));
        let pieces = row_pieces(&c, &ctx, false);
        let text: String = pieces.iter().map(|p| p.text.as_str()).collect();
        assert!(text.contains("⑂"));
        assert!(text.contains("merge"));
    }

    #[test]
    fn compact_numbers() {
        assert_eq!(compact_number(999), "999");
        assert_eq!(compact_number(1_500), "1.5k");
        assert_eq!(compact_number(25_000), "25k");
        assert_eq!(compact_number(2_500_000), "2.5M");
    }
}
