use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::config::Action;
use crate::ui::app::App;

pub fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    if area.height == 0 {
        return;
    }
    let accent = Style::default()
        .fg(app.quality.to_ratatui(app.theme.roles.accent))
        .add_modifier(Modifier::BOLD);
    let strong = Style::default()
        .fg(app.quality.to_ratatui(app.theme.roles.fg))
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(app.quality.to_ratatui(app.theme.roles.dim));
    let warn = Style::default().fg(app.quality.to_ratatui(app.theme.roles.warn));

    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(area);

    let mut title = vec![
        Span::styled(" ghg ", accent),
        Span::styled(app.repo.name.clone(), strong),
        Span::styled("  ", dim),
        Span::styled(app.repo.branch.clone(), accent),
        Span::styled(
            if app.repo.total_commits > app.commits.len() {
                format!(
                    "  {} / {} · {}",
                    crate::util::comma(app.commits.len() as u64),
                    crate::util::count(app.repo.total_commits as u64, "commit", "commits"),
                    crate::util::count(app.repo.authors as u64, "author", "authors")
                )
            } else {
                format!(
                    "  {} · {}",
                    crate::util::count(app.commits.len() as u64, "commit", "commits"),
                    crate::util::count(app.repo.authors as u64, "author", "authors")
                )
            },
            dim,
        ),
    ];
    if !app.warnings.is_empty() {
        title.push(Span::styled(
            format!("   ⚠ {} config warnings", app.warnings.len()),
            warn,
        ));
    }
    f.render_widget(Paragraph::new(Line::from(title)), rows[0]);

    if area.height > 1 {
        let scope = if app.opts.all { "all branches" } else { "HEAD" };
        let p_label = format!("p{}", app.cfg.general.heat_percentile as u32);
        let mut line = vec![Span::styled(format!(" {}", app.window_label()), dim)];
        line.push(Span::styled(
            format!(
                "   {} · {} · {} · {} · {}",
                scope,
                app.theme.name,
                app.cfg.general.heat_scale.as_str(),
                app.cfg.general.wave_axis.as_str(),
                p_label
            ),
            dim,
        ));
        f.render_widget(Paragraph::new(Line::from(line)), rows[1]);
    }

    if area.height > 2 {
        let mut legend = vec![Span::styled(" heat ", dim)];
        let steps = 16usize;
        for i in 0..steps {
            let t = i as f64 / (steps - 1) as f64;
            legend.push(Span::styled(
                "█",
                Style::default().fg(app.quality.to_ratatui(app.theme.heat(t))),
            ));
        }
        legend.push(Span::styled(
            format!("  low → {} lines → high", crate::util::comma(app.p95)),
            dim,
        ));
        f.render_widget(Paragraph::new(Line::from(legend)), rows[2]);
    }
}

pub fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    if area.height == 0 {
        return;
    }
    let dim = Style::default().fg(app.quality.to_ratatui(app.theme.roles.dim));
    let rows = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).split(area);

    let km = &app.cfg.keymap;
    let hints = [
        ("select", Action::SelectNext),
        ("pan", Action::PanRight),
        ("zoom", Action::ZoomIn),
        ("detail", Action::ToggleDetail),
        ("view", Action::ViewCommit),
        ("search", Action::Search),
        ("sort", Action::ToggleSort),
        ("theme", Action::ToggleTheme),
        ("help", Action::Help),
    ];
    let mut line: Vec<Span> = vec![Span::styled(" ", dim)];
    for (i, (label, action)) in hints.iter().enumerate() {
        if i > 0 {
            line.push(Span::styled("  ", dim));
        }
        line.push(Span::styled(
            km.label(*action),
            Style::default().fg(app.quality.to_ratatui(app.theme.roles.accent)),
        ));
        line.push(Span::styled(format!(" {label}"), dim));
    }
    line.push(Span::styled(
        format!("  {} quit", km.label(Action::Quit)),
        dim,
    ));
    f.render_widget(Paragraph::new(Line::from(line)), rows[0]);

    if area.height > 1 {
        let second = if let Some(buf) = &app.input {
            let cursor = "█";
            let matches = app.search.as_ref().map(|s| s.matches.len()).unwrap_or(0);
            Line::from(vec![
                Span::styled(
                    " /",
                    Style::default()
                        .fg(app.quality.to_ratatui(app.theme.roles.accent))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    buf.clone(),
                    Style::default().fg(app.quality.to_ratatui(app.theme.roles.fg)),
                ),
                Span::styled(
                    cursor,
                    Style::default().fg(app.quality.to_ratatui(app.theme.roles.accent)),
                ),
                Span::styled(
                    format!(
                        "   {} · Enter apply · Esc cancel",
                        crate::util::count(matches as u64, "match", "matches")
                    ),
                    dim,
                ),
            ])
        } else if let Some((msg, _)) = &app.status {
            Line::from(vec![
                Span::styled(" ", dim),
                Span::styled(
                    msg.clone(),
                    Style::default().fg(app.quality.to_ratatui(app.theme.roles.ok)),
                ),
            ])
        } else {
            Line::from(Span::styled(
                format!(
                    " {} reload · {} scale · {} all branches · Esc reset",
                    km.label(Action::Reload),
                    km.label(Action::ToggleScale),
                    km.label(Action::ToggleAllBranches)
                ),
                dim,
            ))
        };
        f.render_widget(Paragraph::new(second), rows[1]);
    }
}
