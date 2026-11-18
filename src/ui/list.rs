use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::rows::{self, RowCtx};
use crate::ui::app::{App, DisplayItem, RectI};
use crate::util;

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    if area.width < 4 || area.height < 2 {
        app.list_region = None;
        return;
    }

    let g = &app.cfg.general;
    let count = app.commits.len();
    let matches: Option<usize> = app.search.as_ref().map(|s| s.matches.len());
    let right = match matches {
        Some(m) => format!(" {} / {} ", m, count),
        None => format!(
            " {} · {} ",
            util::count(count as u64, "commit", "commits"),
            app.sort.label()
        ),
    };

    let block = Block::new()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.quality.to_ratatui(app.theme.roles.border)))
        .title(Span::styled(
            " commits ",
            Style::default()
                .fg(app.quality.to_ratatui(app.theme.roles.dim))
                .add_modifier(Modifier::BOLD),
        ))
        .title(
            Line::from(Span::styled(
                right,
                Style::default().fg(app.quality.to_ratatui(app.theme.roles.dim)),
            ))
            .right_aligned(),
        );
    let inner = block.inner(area);
    f.render_widget(block, area);
    app.list_region = Some(RectI {
        x: inner.x,
        y: inner.y,
        w: inner.width,
        h: inner.height,
    });
    app.list_rows = inner.height.max(1);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let h = inner.height as usize;
    let sel = app.selected_display_index();
    if let Some(sel) = sel {
        if sel < app.list_offset {
            app.list_offset = sel;
        } else if sel >= app.list_offset + h {
            app.list_offset = sel + 1 - h;
        }
    }
    let max_offset = app.display.len().saturating_sub(h);
    app.list_offset = app.list_offset.min(max_offset);

    let ctx = RowCtx {
        cfg: g,
        theme: &app.theme,
        now: app.now(),
        p95: app.p95,
        width: inner.width as usize,
    };

    if app.display.is_empty() {
        let msg = Line::from(Span::styled(
            " no commits in range ",
            Style::default().fg(app.quality.to_ratatui(app.theme.roles.dim)),
        ));
        f.render_widget(Paragraph::new(msg), inner);
        return;
    }

    for (i, item) in app.display.iter().skip(app.list_offset).take(h).enumerate() {
        let y = inner.y + i as u16;
        let row = Rect::new(inner.x, y, inner.width, 1);
        let selected = sel == Some(app.list_offset + i);

        match *item {
            DisplayItem::Commit(ci) => {
                let c = &app.commits[ci];
                let dimmed = app.is_dimmed(ci);
                let pieces = rows::row_pieces(c, &ctx, dimmed);
                let spans: Vec<Span> = pieces
                    .iter()
                    .map(|p| {
                        let mut st = Style::default();
                        if let Some(rgb) = p.rgb {
                            st = st.fg(app.quality.to_ratatui(rgb));
                        }
                        if p.bold {
                            st = st.add_modifier(Modifier::BOLD);
                        }
                        Span::styled(p.text.clone(), st)
                    })
                    .collect();
                let mut line = Line::from(spans);
                if selected {
                    line = line.style(
                        Style::default().bg(app.quality.to_ratatui(app.theme.roles.selected_bg)),
                    );
                }
                f.render_widget(Paragraph::new(line), row);
            }
            DisplayItem::Gap(secs) => {
                let label = format!("┄┄ {} ┄┄", util::humanize_long(secs));
                let line = Line::from(Span::styled(
                    format!(" {label}"),
                    Style::default().fg(app.quality.to_ratatui(app.theme.roles.dim)),
                ));
                f.render_widget(Paragraph::new(line), row);
            }
        }
    }
}
