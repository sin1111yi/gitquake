use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::theme::{Quality, Theme};
use crate::ui::app::App;
use crate::util;

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    if area.width < 20 || area.height < 6 {
        return;
    }
    let Some(viewer) = app.viewer.as_mut() else {
        return;
    };
    let theme = &app.theme;
    let quality = app.quality;

    f.render_widget(Clear, area);
    let title = util::truncate_w(viewer.title.trim(), area.width.saturating_sub(6) as usize);
    let block = Block::new()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(quality.to_ratatui(theme.roles.accent)))
        .style(Style::default().bg(quality.to_ratatui(theme.roles.selected_bg)))
        .title(Span::styled(
            format!(" {title} "),
            Style::default()
                .fg(quality.to_ratatui(theme.roles.fg))
                .add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let footer_h = 1u16.min(inner.height);
    let content_h = inner.height.saturating_sub(footer_h) as usize;
    viewer.view_h = content_h.max(1);
    let max_scroll = viewer.lines.len().saturating_sub(viewer.view_h);
    viewer.scroll = viewer.scroll.min(max_scroll);

    let visible: Vec<Line> = viewer
        .lines
        .iter()
        .skip(viewer.scroll)
        .take(content_h)
        .map(|l| styled_line(l, theme, quality))
        .collect();
    f.render_widget(
        Paragraph::new(visible),
        Rect::new(inner.x, inner.y, inner.width, content_h as u16),
    );

    if footer_h > 0 {
        let pct = if max_scroll == 0 {
            100
        } else {
            (viewer.scroll as f64 / max_scroll as f64 * 100.0).round() as u32
        };
        let position = if viewer.lines.is_empty() {
            "0/0".to_string()
        } else {
            format!(
                "{}/{}",
                (viewer.scroll + 1).min(viewer.lines.len()),
                viewer.lines.len()
            )
        };
        let dim = Style::default().fg(quality.to_ratatui(theme.roles.dim));
        let footer = Line::from(vec![
            Span::styled(" j/k scroll · PgUp/PgDn · g/G · v/q close   ", dim),
            Span::styled(format!("{position}  {pct}%"), dim),
        ]);
        f.render_widget(
            Paragraph::new(footer),
            Rect::new(inner.x, inner.y + content_h as u16, inner.width, 1),
        );
    }
}

fn styled_line(line: &str, theme: &Theme, quality: Quality) -> Line<'static> {
    let fg = Style::default().fg(quality.to_ratatui(theme.roles.fg));
    let dim = Style::default().fg(quality.to_ratatui(theme.roles.dim));
    let accent = Style::default()
        .fg(quality.to_ratatui(theme.roles.accent))
        .add_modifier(Modifier::BOLD);
    let ok = Style::default().fg(quality.to_ratatui(theme.roles.ok));
    let bad = Style::default().fg(quality.to_ratatui(theme.roles.bad));

    if line.starts_with("commit ") {
        return Line::from(Span::styled(line.to_string(), accent));
    }
    if let Some(idx) = line.find(':') {
        let label = &line[..idx];
        if matches!(
            label,
            "Author" | "AuthorDate" | "Commit" | "CommitDate" | "Date" | "Merge"
        ) {
            return Line::from(vec![
                Span::styled(format!("{label}:"), dim),
                Span::styled(line[idx + 1..].to_string(), fg),
            ]);
        }
    }
    let style = if line.starts_with("diff --git") {
        accent
    } else if line.starts_with("index ")
        || line.starts_with("---")
        || line.starts_with("+++")
        || line.starts_with("new file")
        || line.starts_with("deleted file")
        || line.starts_with("similarity")
        || line.starts_with("rename ")
    {
        dim
    } else if line.starts_with("@@") {
        accent
    } else if line.starts_with('+') {
        ok
    } else if line.starts_with('-') {
        bad
    } else {
        fg
    };
    Line::from(Span::styled(line.to_string(), style))
}

#[cfg(test)]
mod tests {
    use super::styled_line;
    use crate::theme::{Quality, builtin};

    #[test]
    fn diff_line_colours_differ() {
        let theme = builtin("inferno").unwrap();
        let plus = styled_line("+added", &theme, Quality::True);
        let minus = styled_line("-removed", &theme, Quality::True);
        assert_ne!(plus.spans[0].style, minus.spans[0].style);
        let header = styled_line("diff --git a/x b/x", &theme, Quality::True);
        assert_ne!(header.spans[0].style, plus.spans[0].style);
    }
}
