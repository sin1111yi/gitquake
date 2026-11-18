use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::config::Action;
use crate::ui::app::App;
use crate::util;

fn describe(a: Action) -> &'static str {
    match a {
        Action::Quit => "quit",
        Action::Help => "toggle this help",
        Action::SelectPrev => "select previous commit",
        Action::SelectNext => "select next commit",
        Action::PageUp => "page up",
        Action::PageDown => "page down",
        Action::SelectFirst => "jump to oldest",
        Action::SelectLast => "jump to newest",
        Action::PanLeft => "pan window left",
        Action::PanRight => "pan window right",
        Action::ZoomIn => "zoom in",
        Action::ZoomOut => "zoom out",
        Action::ResetZoom => "reset zoom",
        Action::ToggleDetail => "toggle detail pane",
        Action::ViewCommit => "view full commit (message + diff)",
        Action::Search => "search commits",
        Action::SearchNext => "next match",
        Action::SearchPrev => "previous match",
        Action::ToggleSort => "cycle sort order",
        Action::ToggleScale => "log / linear heat",
        Action::ToggleAllBranches => "all branches / current",
        Action::ToggleTheme => "cycle theme",
        Action::ToggleList => "toggle commit list",
        Action::Reload => "reload config + repo",
        Action::ToggleMouse => "toggle mouse",
    }
}

pub fn draw(f: &mut Frame, app: &App, frame_area: Rect) {
    if frame_area.width < 30 || frame_area.height < 10 {
        return;
    }
    let w = (frame_area.width.saturating_sub(8))
        .clamp(40, 96)
        .min(frame_area.width);
    let rows = Action::ALL.len() as u16;
    let h = (rows + 9).min(frame_area.height.saturating_sub(2));
    let x = frame_area.x + (frame_area.width - w) / 2;
    let y = frame_area.y + (frame_area.height - h) / 2;
    let area = Rect::new(x, y, w, h);

    f.render_widget(Clear, area);
    let block = Block::new()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.quality.to_ratatui(app.theme.roles.accent)))
        .title(Span::styled(
            " keys ",
            Style::default()
                .fg(app.quality.to_ratatui(app.theme.roles.fg))
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(app.quality.to_ratatui(app.theme.roles.selected_bg)));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let dim = Style::default().fg(app.quality.to_ratatui(app.theme.roles.dim));
    let accent = Style::default()
        .fg(app.quality.to_ratatui(app.theme.roles.accent))
        .add_modifier(Modifier::BOLD);

    let footer_rows = 4u16;
    let list_h = inner.height.saturating_sub(footer_rows);
    let cols =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(inner);

    let per_col = (Action::ALL.len() as u16).div_ceil(2).max(1) as usize;
    for (ci, col) in cols.iter().enumerate() {
        let mut lines: Vec<Line> = Vec::new();
        for (i, action) in Action::ALL.iter().enumerate() {
            if i / per_col != ci {
                continue;
            }
            let key = app.cfg.keymap.label(*action);
            lines.push(Line::from(vec![
                Span::styled(format!(" {:>6} ", key), accent),
                Span::styled(describe(*action), dim),
            ]));
        }
        let r = Rect::new(col.x, col.y, col.width, list_h);
        f.render_widget(Paragraph::new(lines), r);
    }

    let mut foot: Vec<Line> = Vec::new();
    foot.push(Line::from(Span::styled(
        " Esc  clear search / reset zoom      Ctrl-C  quit",
        dim,
    )));
    foot.push(Line::from(Span::styled(
        format!(
            " theme {} · scale {} · {}",
            app.theme.name,
            app.cfg.general.heat_scale.as_str(),
            app.cfg.general.wave_mode.as_str()
        ),
        dim,
    )));

    let pw = inner.width.saturating_sub(1) as usize;
    if pw > 2 {
        let preview = app.theme.gradient_preview(pw.saturating_sub(2));
        let n = preview.chars().count().max(2);
        let mut spans = vec![Span::styled(" ", dim)];
        for (i, ch) in preview.chars().enumerate() {
            let t = i as f64 / (n - 1) as f64;
            spans.push(Span::styled(
                ch.to_string(),
                Style::default().fg(app.quality.to_ratatui(app.theme.heat(t))),
            ));
        }
        foot.push(Line::from(spans));
    }
    let path = app
        .cfg
        .path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "(no config file)".into());
    foot.push(Line::from(Span::styled(
        format!(
            " {}",
            util::truncate_w(&path, inner.width.saturating_sub(2) as usize)
        ),
        dim,
    )));

    let foot_area = Rect::new(
        inner.x,
        inner.y + inner.height.saturating_sub(foot.len() as u16),
        inner.width,
        foot.len() as u16,
    );
    f.render_widget(Paragraph::new(foot), foot_area);
}
