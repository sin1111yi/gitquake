use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::config::Action;
use crate::rows;
use crate::ui::app::App;
use crate::util;

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
        Span::styled(" gitquake ", accent),
        Span::styled("· ", dim),
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
                "   {} · {} · count · {} heat · {} · {}",
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
        legend.push(Span::styled("  low → more lines → high", dim));
        f.render_widget(Paragraph::new(Line::from(legend)), rows[2]);
    }
}

struct Hint {
    keys: String,
    label: &'static str,
    group: u8,
    priority: u8,
}

fn hint_list(app: &App) -> Vec<Hint> {
    let km = &app.cfg.keymap;
    let pair = |a: Action, b: Action| format!("{}{}", km.label(a), km.label(b));
    vec![
        Hint {
            keys: pair(Action::SelectPrev, Action::SelectNext),
            label: "select",
            group: 0,
            priority: 1,
        },
        Hint {
            keys: pair(Action::PanLeft, Action::PanRight),
            label: "pan",
            group: 0,
            priority: 8,
        },
        Hint {
            keys: pair(Action::ZoomIn, Action::ZoomOut),
            label: "zoom",
            group: 0,
            priority: 5,
        },
        Hint {
            keys: km.label(Action::ToggleDetail),
            label: "detail",
            group: 1,
            priority: 3,
        },
        Hint {
            keys: km.label(Action::ViewCommit),
            label: "commit",
            group: 1,
            priority: 4,
        },
        Hint {
            keys: km.label(Action::Search),
            label: "search",
            group: 1,
            priority: 2,
        },
        Hint {
            keys: km.label(Action::ToggleSort),
            label: "sort",
            group: 2,
            priority: 6,
        },
        Hint {
            keys: km.label(Action::ToggleScale),
            label: "heat",
            group: 2,
            priority: 9,
        },
        Hint {
            keys: km.label(Action::ToggleTheme),
            label: "theme",
            group: 2,
            priority: 7,
        },
        Hint {
            keys: km.label(Action::ToggleList),
            label: "list",
            group: 2,
            priority: 10,
        },
        Hint {
            keys: km.label(Action::ToggleAllBranches),
            label: "branches",
            group: 3,
            priority: 11,
        },
        Hint {
            keys: km.label(Action::Reload),
            label: "reload",
            group: 3,
            priority: 12,
        },
        Hint {
            keys: km.label(Action::ToggleMouse),
            label: "mouse",
            group: 3,
            priority: 13,
        },
        Hint {
            keys: km.label(Action::ResetZoom),
            label: "reset",
            group: 3,
            priority: 14,
        },
        Hint {
            keys: km.label(Action::Help),
            label: "help",
            group: 4,
            priority: 0,
        },
        Hint {
            keys: km.label(Action::Quit),
            label: "quit",
            group: 4,
            priority: 0,
        },
    ]
}

fn chip_width(h: &Hint) -> usize {
    util::display_width(&h.keys) + 3 + util::display_width(h.label)
}

fn row_width(hints: &[Hint], included: &[bool]) -> usize {
    let mut width = 0;
    let mut prev_group: Option<u8> = None;
    for (h, &on) in hints.iter().zip(included) {
        if !on {
            continue;
        }
        if let Some(g) = prev_group {
            width += if g == h.group { 2 } else { 3 };
        }
        width += chip_width(h);
        prev_group = Some(h.group);
    }
    width
}

/// Pick the highest-priority subset of hints that fits `avail` columns.
fn fit_hints(hints: &[Hint], avail: usize) -> Vec<bool> {
    let mut included = vec![true; hints.len()];
    let mut order: Vec<usize> = (0..hints.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(hints[i].priority));
    for i in order {
        if hints[i].priority == 0 {
            continue;
        }
        if row_width(hints, &included) <= avail {
            break;
        }
        included[i] = false;
    }
    included
}

pub fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    if area.height == 0 {
        return;
    }
    let dim = Style::default().fg(app.quality.to_ratatui(app.theme.roles.dim));
    let axis = Style::default().fg(app.quality.to_ratatui(app.theme.roles.axis));
    let accent = Style::default()
        .fg(app.quality.to_ratatui(app.theme.roles.accent))
        .add_modifier(Modifier::BOLD);
    let chip = Style::default()
        .bg(app.quality.to_ratatui(app.theme.roles.selected_bg))
        .fg(app.quality.to_ratatui(app.theme.roles.accent))
        .add_modifier(Modifier::BOLD);
    let rows = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).split(area);

    // Row 1: btop-style key chips, packed by priority.
    let hints = hint_list(app);
    let avail = area.width.saturating_sub(1) as usize;
    let included = fit_hints(&hints, avail);
    let dropped = included.iter().any(|on| !on);
    let mut spans: Vec<Span> = vec![Span::styled(" ", dim)];
    let mut prev_group: Option<u8> = None;
    let mut any = false;
    for (h, &on) in hints.iter().zip(&included) {
        if !on {
            continue;
        }
        if let Some(g) = prev_group {
            if g == h.group {
                spans.push(Span::styled("  ", dim));
            } else {
                spans.push(Span::styled(" │ ", axis));
            }
        }
        spans.push(Span::styled(format!(" {} ", h.keys), chip));
        spans.push(Span::styled(format!(" {}", h.label), dim));
        prev_group = Some(h.group);
        any = true;
    }
    if any && dropped && row_width(&hints, &included) + 2 <= avail {
        spans.push(Span::styled(" …", axis));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), rows[0]);

    if area.height <= 1 {
        return;
    }

    // Row 2: prompt, transient status, or the current selection.
    let second = rows[1];
    if let Some(buf) = &app.input {
        let matches = app.search.as_ref().map(|s| s.matches.len()).unwrap_or(0);
        let line = Line::from(vec![
            Span::styled(" / ", chip),
            Span::styled(
                buf.clone(),
                Style::default().fg(app.quality.to_ratatui(app.theme.roles.fg)),
            ),
            Span::styled(
                "█",
                Style::default().fg(app.quality.to_ratatui(app.theme.roles.accent)),
            ),
            Span::styled(
                format!(
                    "   {}   {} apply · Esc cancel",
                    util::count(matches as u64, "match", "matches"),
                    app.cfg
                        .keymap
                        .label(Action::ToggleDetail)
                        .replace('↵', "Enter")
                ),
                dim,
            ),
        ]);
        f.render_widget(Paragraph::new(line), second);
        return;
    }

    let Some(c) = app.selected_commit() else {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(" no commits ", dim))),
            second,
        );
        return;
    };

    let plus = format!("+{}", rows::compact_number(c.insertions));
    let minus = format!("-{}", rows::compact_number(c.deletions));
    let age = util::humanize(c.timestamp, app.now());
    let right_w =
        (util::display_width(&plus) + util::display_width(&minus) + util::display_width(&age) + 6)
            as u16;
    let cols = Layout::horizontal([
        Constraint::Min(1),
        Constraint::Length(right_w.min(second.width)),
    ])
    .split(second);

    let left_line = if let Some((msg, _)) = &app.status {
        Line::from(vec![
            Span::styled(" ● ", accent),
            Span::styled(
                util::truncate_w(msg, cols[0].width.saturating_sub(3) as usize),
                Style::default().fg(app.quality.to_ratatui(app.theme.roles.ok)),
            ),
        ])
    } else if let Some(search) = &app.search {
        let km = &app.cfg.keymap;
        Line::from(Span::styled(
            util::truncate_w(
                &format!(
                    " {} · {} next · {} prev · Esc clear",
                    util::count(search.matches.len() as u64, "match", "matches"),
                    km.label(Action::SearchNext),
                    km.label(Action::SearchPrev)
                ),
                cols[0].width as usize,
            ),
            dim,
        ))
    } else {
        let pos = app.selected + 1;
        let prefix = format!(" {}/{}  {}  ", pos, app.commits.len(), c.short);
        let room = cols[0].width as usize;
        let subject_w = room.saturating_sub(util::display_width(&prefix));
        Line::from(vec![
            Span::styled(format!(" {}/{}  ", pos, app.commits.len()), dim),
            Span::styled(format!("{}  ", c.short), accent),
            Span::styled(
                util::truncate_w(&c.subject, subject_w),
                Style::default().fg(app.quality.to_ratatui(app.theme.roles.fg)),
            ),
        ])
    };
    f.render_widget(Paragraph::new(left_line), cols[0]);

    let right_line = Line::from(vec![
        Span::styled(
            plus,
            Style::default().fg(app.quality.to_ratatui(app.theme.roles.ok)),
        ),
        Span::styled(" ", dim),
        Span::styled(
            minus,
            Style::default().fg(app.quality.to_ratatui(app.theme.roles.bad)),
        ),
        Span::styled(format!("  {age} "), dim),
    ])
    .alignment(Alignment::Right);
    f.render_widget(Paragraph::new(right_line), cols[1]);
}
