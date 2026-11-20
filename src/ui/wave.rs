use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::config::GlyphSet;
use crate::rows::{self, heat_t};
use crate::stats::{self, Projection};
use crate::theme::Rgb;
use crate::ui::app::{App, RectI};
use crate::util;

const EIGHTHS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

#[derive(Clone, Copy, Debug)]
pub struct Cell {
    pub ch: char,
    pub fg: Rgb,
}

impl Cell {
    fn blank(fg: Rgb) -> Cell {
        Cell { ch: ' ', fg }
    }
}

pub struct Grid {
    pub rows: Vec<Vec<Cell>>,
    pub columns: Vec<Option<usize>>,
}

fn full_glyph(set: GlyphSet) -> char {
    match set {
        GlyphSet::Eighth | GlyphSet::Block => '█',
        GlyphSet::Ascii => '#',
    }
}

fn partial_glyph(set: GlyphSet, frac: usize) -> Option<char> {
    match set {
        GlyphSet::Eighth => Some(EIGHTHS[frac.clamp(1, 8) - 1]),
        GlyphSet::Block => (frac >= 5).then_some('█'),
        GlyphSet::Ascii => match frac {
            5..=8 => Some('#'),
            1..=4 => Some('.'),
            _ => None,
        },
    }
}

fn stamp(
    rows: &mut [Vec<Cell>],
    x: usize,
    top: usize,
    height: usize,
    eighths: usize,
    fg: Rgb,
    set: GlyphSet,
) {
    if height == 0 || eighths == 0 {
        return;
    }
    let eighths = eighths.min(height * 8);
    let full = eighths / 8;
    let frac = eighths % 8;
    let full_ch = full_glyph(set);
    for b in 0..full {
        if b >= height {
            break;
        }
        let y = top + height - 1 - b;
        if let Some(row) = rows.get_mut(y) {
            if let Some(cell) = row.get_mut(x) {
                cell.ch = full_ch;
                cell.fg = fg;
            }
        }
    }
    if frac > 0 && full < height {
        if let Some(g) = partial_glyph(set, frac) {
            let y = top + height - 1 - full;
            if let Some(row) = rows.get_mut(y) {
                if let Some(cell) = row.get_mut(x) {
                    cell.ch = g;
                    cell.fg = fg;
                }
            }
        }
    }
}

impl Grid {
    /// Build the waveform cell grid. `height` is the number of waveform rows.
    ///
    /// Height comes from the number of commits in a column; colour comes from
    /// the column's total insertions + deletions. Both are computed only for
    /// the columns currently inside the visible window.
    pub fn build(app: &App, width: usize, height: usize) -> Grid {
        let g = &app.cfg.general;
        let proj = Projection::new(app.window, width, &app.commits, g.wave_axis.weight());
        let buckets = stats::project_buckets(&app.commits, &proj, width);
        let mut columns = vec![None; width];
        let mut rows = vec![vec![Cell::blank(app.theme.roles.axis); width]; height];
        if width == 0 || height == 0 {
            return Grid { rows, columns };
        }

        let selected_col = app.selected_commit_index().and_then(|ci| {
            proj.order
                .iter()
                .position(|&i| i == ci)
                .and_then(|j| proj.x_commit(j))
        });

        // Search dims a column only when none of its commits match.
        let mut has_match = vec![false; width];
        if app.search.is_some() {
            for (j, &ci) in proj.order.iter().enumerate() {
                if !app.is_dimmed(ci) {
                    if let Some(x) = proj.x_commit(j).filter(|x| *x < width) {
                        has_match[x] = true;
                    }
                }
            }
        }

        let max_count = buckets.iter().map(|b| b.count as u64).max().unwrap_or(0);
        let count_scale = stats::CountScale::new(max_count);

        for (x, bucket) in buckets.iter().enumerate() {
            columns[x] = bucket.commit;
            if bucket.count == 0 {
                continue;
            }

            let h = count_scale.height(bucket.count as u64);
            let n8 = (h * (height * 8) as f64).round() as usize;
            let n8 = n8.max(1);

            let t = rows::heat_value(bucket.volume, app.p95, g);
            let base = if bucket.volume == 0 {
                app.theme.roles.dim
            } else {
                app.theme.heat(t)
            };
            let dimmed = app.search.is_some() && !has_match[x];
            let fg = if dimmed {
                app.theme.roles.dim
            } else if selected_col == Some(x) {
                base.blend(app.theme.roles.wave_peak, 0.45)
            } else {
                base
            };
            stamp(&mut rows, x, 0, height, n8, fg, g.glyphs);
        }

        Grid { rows, columns }
    }
}

fn cursor_column(app: &App, width: usize) -> Option<usize> {
    let ci = app.selected_commit_index()?;
    if let Some(x) = app.columns.iter().position(|c| *c == Some(ci)) {
        return Some(x);
    }
    let proj = Projection::new(
        app.window,
        width,
        &app.commits,
        app.cfg.general.wave_axis.weight(),
    );
    app.selected_commit().and_then(|c| proj.x_time(c.timestamp))
}

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    app.wave_region = Some(RectI {
        x: area.x,
        y: area.y,
        w: area.width,
        h: area.height,
    });

    let axis_h = if area.height >= 4 { 2 } else { 0 };
    let grid_h = area.height.saturating_sub(axis_h) as usize;

    if grid_h > 0 {
        let grid = Grid::build(app, area.width as usize, grid_h);
        app.columns = grid.columns.clone();
        let buf = f.buffer_mut();
        for (y, row) in grid.rows.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                if cell.ch == ' ' {
                    continue;
                }
                let bx = area.x + x as u16;
                let by = area.y + y as u16;
                let style = Style::default().fg(app.quality.to_ratatui(cell.fg));
                if let Some(c) = buf.cell_mut((bx, by)) {
                    let mut tmp = [0u8; 4];
                    c.set_symbol(cell.ch.encode_utf8(&mut tmp));
                    c.set_style(style);
                }
            }
        }
    }

    if axis_h > 0 {
        let proj = Projection::new(
            app.window,
            area.width as usize,
            &app.commits,
            app.cfg.general.wave_axis.weight(),
        );
        draw_axis(f, app, area, area.y + grid_h as u16, &proj);
    }

    draw_card(f, app, area);
}

fn draw_axis(f: &mut Frame, app: &App, area: Rect, y: u16, proj: &Projection) {
    let width = area.width as usize;
    if width == 0 {
        return;
    }
    let axis_color = app.quality.to_ratatui(app.theme.roles.axis);
    let cursor_color = app.quality.to_ratatui(app.theme.roles.accent);
    let dim = app.theme.roles.dim;

    let buf = f.buffer_mut();
    for x in 0..width {
        if let Some(c) = buf.cell_mut((area.x + x as u16, y)) {
            c.set_symbol("─");
            c.set_style(Style::default().fg(axis_color));
        }
    }

    let unit = stats::choose_unit(app.window.span(), width);
    let labels = if proj.weight >= 0.999 {
        stats::ticks_projected(proj, unit)
    } else {
        stats::ticks_ranked(proj, &app.commits)
    };
    let mut draw_labels = area.height > 2;
    let mut next_free = 0usize;
    for (col, label) in labels {
        if col >= width {
            continue;
        }
        if let Some(c) = buf.cell_mut((area.x + col as u16, y)) {
            c.set_symbol("┴");
            c.set_style(Style::default().fg(axis_color));
        }
        if !draw_labels {
            continue;
        }
        let lw = util::display_width(&label);
        if lw == 0 {
            continue;
        }
        let start = if col == 0 { 0 } else { col.max(next_free) };
        if start + lw > width {
            draw_labels = false;
            continue;
        }
        let ly = y + 1;
        for (i, ch) in label.chars().enumerate() {
            if let Some(c) = buf.cell_mut((area.x + (start + i) as u16, ly)) {
                c.set_symbol(ch.encode_utf8(&mut [0u8; 4]));
                c.set_style(Style::default().fg(app.quality.to_ratatui(dim)));
            }
        }
        next_free = start + lw + 2;
    }

    if let Some(cx) = cursor_column(app, width) {
        if cx < width {
            if let Some(c) = buf.cell_mut((area.x + cx as u16, y)) {
                c.set_symbol("┃");
                c.set_style(
                    Style::default()
                        .fg(cursor_color)
                        .add_modifier(Modifier::BOLD),
                );
            }
        }
    }
}

fn draw_card(f: &mut Frame, app: &App, area: Rect) {
    let Some(c) = app.selected_commit() else {
        return;
    };
    if area.width < 24 || area.height < 7 {
        return;
    }
    let g = &app.cfg.general;
    let heat = heat_t(c, app.p95, g);
    let total: u64 = c.volume();

    let mut line1 = vec![Span::styled(
        format!(" {}", c.short),
        Style::default()
            .fg(app.quality.to_ratatui(app.theme.roles.accent))
            .add_modifier(Modifier::BOLD),
    )];
    let refs = rows::refs_label(c, 2, 26);
    if !refs.trim().is_empty() {
        line1.push(Span::styled(
            format!(" {refs}"),
            Style::default().fg(app.quality.to_ratatui(app.theme.roles.dim)),
        ));
    }

    let subject = util::truncate_w(&c.subject, 60);
    let lines = vec![
        Line::from(line1),
        Line::from(Span::styled(
            format!(" {subject}"),
            Style::default()
                .fg(app.quality.to_ratatui(app.theme.roles.fg))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!(
                " {} · {}",
                c.author_name,
                util::humanize(c.timestamp, app.now())
            ),
            Style::default().fg(app.quality.to_ratatui(app.theme.roles.dim)),
        )),
        Line::from(vec![
            Span::styled(
                format!(" +{}", util::comma(c.insertions)),
                Style::default().fg(app.quality.to_ratatui(app.theme.roles.ok)),
            ),
            Span::styled(
                format!(" -{}", util::comma(c.deletions)),
                Style::default().fg(app.quality.to_ratatui(app.theme.roles.bad)),
            ),
            Span::styled(
                format!("  {} lines", util::comma(total)),
                Style::default().fg(app.quality.to_ratatui(app.theme.roles.dim)),
            ),
            Span::styled(
                format!("  heat {}%", (heat * 100.0).round() as u32),
                Style::default().fg(app.quality.to_ratatui(app.theme.heat(heat))),
            ),
        ]),
    ];

    let content_w = lines
        .iter()
        .map(|l| l.width())
        .max()
        .unwrap_or(20)
        .min(area.width as usize - 2);
    // One column of breathing room inside each side of the border.
    let pw = (content_w as u16 + 3).max(20).min(area.width - 2);
    let ph = 6u16.min(area.height);

    let cursor = cursor_column(app, area.width as usize).unwrap_or(area.width as usize / 2);
    let ideal = cursor as u16 + 2;
    let max_x = area.x + area.width - pw - 1;
    let px = (area.x + ideal).min(max_x).max(area.x + 1);
    let py = area.y + 1;

    let rect = Rect::new(px, py, pw, ph);
    f.render_widget(Clear, rect);
    let block = Block::new()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.quality.to_ratatui(app.theme.roles.accent)))
        .style(Style::default().bg(app.quality.to_ratatui(app.theme.roles.selected_bg)));
    let para = Paragraph::new(lines).block(block);
    f.render_widget(para, rect);
}
