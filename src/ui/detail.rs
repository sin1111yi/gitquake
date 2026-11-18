use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::model::Commit;
use crate::rows;
use crate::stats;
use crate::ui::app::App;
use crate::util;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    if area.width < 8 || area.height < 3 {
        return;
    }
    let block = Block::new()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.quality.to_ratatui(app.theme.roles.border)))
        .title(Span::styled(
            " commit ",
            Style::default()
                .fg(app.quality.to_ratatui(app.theme.roles.dim))
                .add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let Some(c) = app.selected_commit() else {
        return;
    };
    let lines = build_lines(app, c, inner.width as usize);
    f.render_widget(Paragraph::new(lines), inner);
}

fn build_lines(app: &App, c: &Commit, width: usize) -> Vec<Line<'static>> {
    let fg = |rgb| Style::default().fg(app.quality.to_ratatui(rgb));
    let roles = &app.theme.roles;
    let dim = Style::default().fg(app.quality.to_ratatui(roles.dim));
    let strong = fg(roles.fg).add_modifier(Modifier::BOLD);
    let accent = fg(roles.accent).add_modifier(Modifier::BOLD);

    let mut lines: Vec<Line<'static>> = Vec::new();

    let mut head = vec![Span::styled(c.short.clone(), accent)];
    let refs = rows::refs_label(c, 4, width.saturating_sub(12));
    if !refs.trim().is_empty() {
        head.push(Span::styled(format!("  {refs}"), dim));
    }
    lines.push(Line::from(head));

    for chunk in wrap(&c.subject, width) {
        lines.push(Line::from(Span::styled(chunk, strong)));
    }

    lines.push(Line::from(Span::styled(
        util::truncate_w(
            &format!(
                "{} <{}>  ·  {}  ·  {}",
                c.author_name,
                c.author_email,
                util::absolute(c.timestamp, "%Y-%m-%d %H:%M"),
                util::humanize(c.timestamp, app.now())
            ),
            width,
        ),
        dim,
    )));

    if c.parents.len() > 1 {
        let parents: Vec<String> = c
            .parents
            .iter()
            .take(6)
            .map(|p| p[..7.min(p.len())].to_string())
            .collect();
        lines.push(Line::from(Span::styled(
            format!("merge of {}", parents.join(" ")),
            dim,
        )));
    }

    let mut stat = vec![
        Span::styled(format!("+{}", util::comma(c.insertions)), fg(roles.ok)),
        Span::styled(" ", dim),
        Span::styled(format!("-{}", util::comma(c.deletions)), fg(roles.bad)),
        Span::styled(
            format!("  {} files", util::comma(c.files.len() as u64)),
            dim,
        ),
    ];
    if c.is_merge() {
        stat.push(Span::styled("  (merge, no diff)", dim));
    }
    lines.push(Line::from(stat));
    lines.push(Line::from(""));

    if c.files.is_empty() {
        return lines;
    }

    let max_vol = c.files.iter().map(|f| f.volume()).max().unwrap_or(0).max(1);
    let bar_w = (width / 4).clamp(4, 18);
    let diff_w = 13usize;
    let path_w = width.saturating_sub(bar_w + 1 + diff_w + 1).max(6);

    for file in c.files.iter().take(400) {
        let vol = file.volume();
        let t = if file.is_binary() {
            0.0
        } else {
            stats::normalize(
                vol,
                max_vol,
                app.cfg.general.heat_scale,
                app.cfg.general.heat_gamma,
            )
        };
        let filled = if t <= 0.0 {
            0
        } else {
            ((t * bar_w as f64).round() as usize).max(1)
        };
        let color = if file.is_binary() {
            roles.dim
        } else {
            app.theme.heat(t)
        };

        let mut spans = vec![
            Span::styled("█".repeat(filled), fg(color)),
            Span::styled(
                "░".repeat(bar_w - filled),
                Style::default().fg(app.quality.to_ratatui(roles.axis)),
            ),
            Span::styled(" ", dim),
            Span::styled(
                util::pad_w(&util::truncate_w(&file.path, path_w), path_w),
                if file.is_binary() { dim } else { fg(roles.fg) },
            ),
        ];
        match (file.insertions, file.deletions) {
            (Some(a), Some(d)) => {
                spans.push(Span::styled(
                    format!("{:>8}", format!("+{}", rows::compact_number(a))),
                    fg(roles.ok),
                ));
                spans.push(Span::styled(
                    format!("{:>6}", format!("-{}", rows::compact_number(d))),
                    fg(roles.bad),
                ));
            }
            _ => spans.push(Span::styled("  binary", dim)),
        }
        lines.push(Line::from(spans));
    }
    if c.files.len() > 400 {
        lines.push(Line::from(Span::styled(
            format!("… {} more files", c.files.len() - 400),
            dim,
        )));
    }
    lines
}

fn wrap(s: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![];
    }
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut used = 0usize;
    for word in s.split(' ') {
        let ww = util::display_width(word);
        if used > 0 && used + 1 + ww > width {
            out.push(std::mem::take(&mut cur));
            used = 0;
        }
        if used > 0 {
            cur.push(' ');
            used += 1;
        }
        if ww > width {
            cur.push_str(&util::truncate_w(word, width));
            used = width;
        } else {
            cur.push_str(word);
            used += ww;
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::wrap;

    #[test]
    fn wraps_long_subjects() {
        let out = wrap(
            "fix the thing that broke everything in production today",
            20,
        );
        assert!(out.len() > 1);
        assert!(out.iter().all(|l| l.chars().count() <= 20));
    }
}
