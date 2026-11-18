use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;

use crate::rows::{self, RowCtx};
use crate::stats;
use crate::ui::app::App;
use crate::ui::wave::Grid;
use crate::util;

/// Render the whole view as plain ANSI text (for pipes, `--plain`, or `$PAGER`).
pub fn print(app: &App, color: bool) -> Result<()> {
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    let width = crossterm::terminal::size()
        .map(|(w, _)| w as usize)
        .unwrap_or(120)
        .clamp(40, 180);
    let q = app.quality;
    let accent = app.theme.roles.accent;
    let dim = app.theme.roles.dim;

    writeln!(
        out,
        "{}",
        util::paint(&format!("ghg · {}", app.repo.name), accent, q, color, true)
    )?;
    writeln!(
        out,
        "{}",
        util::paint(
            &format!(
                "{} · {} · {} · {}{} · {} axis · {} heat",
                app.repo.branch,
                util::count(app.commits.len() as u64, "commit", "commits"),
                util::count(app.repo.authors as u64, "author", "authors"),
                if app.opts.all { "all branches" } else { "HEAD" },
                if app.opts.no_merges {
                    " · no merges"
                } else {
                    ""
                },
                app.cfg.general.wave_axis.as_str(),
                app.cfg.general.heat_scale.as_str()
            ),
            dim,
            q,
            color,
            false
        )
    )?;

    // heat legend
    let mut legend = String::new();
    legend.push_str(&util::paint("heat ", dim, q, color, false));
    for i in 0..24 {
        let t = i as f64 / 23.0;
        legend.push_str(&util::paint("█", app.theme.heat(t), q, color, false));
    }
    legend.push_str(&util::paint(
        &format!("  low → {} lines → high", util::comma(app.p95)),
        dim,
        q,
        color,
        false,
    ));
    writeln!(out, "{legend}")?;
    if !app.warnings.is_empty() {
        writeln!(
            out,
            "{}",
            util::paint(
                &format!(
                    "⚠ {} config warning{}: {}",
                    app.warnings.len(),
                    if app.warnings.len() == 1 { "" } else { "s" },
                    app.warnings[0]
                ),
                app.theme.roles.warn,
                q,
                color,
                false
            )
        )?;
    }
    writeln!(out)?;

    let wave_h = 10usize;
    let grid = Grid::build(app, width, wave_h);
    for row in &grid.rows {
        let mut line = String::new();
        for cell in row {
            if cell.ch == ' ' {
                line.push(' ');
            } else {
                line.push_str(&util::paint(&cell.ch.to_string(), cell.fg, q, color, false));
            }
        }
        writeln!(out, "{}", line.trim_end())?;
    }

    // axis
    let unit = stats::choose_unit(app.window.span(), width);
    let proj = stats::Projection::new(
        app.window,
        width,
        &app.commits,
        app.cfg.general.wave_axis.weight(),
    );
    let labels = if proj.weight >= 0.999 {
        stats::ticks_projected(&proj, unit)
    } else {
        stats::ticks_ranked(&proj, &app.commits)
    };
    let mut axis = vec![' '; width];
    let mut text = vec![' '; width];
    let mut next_free = 0usize;
    for (col, label) in &labels {
        if *col < width {
            axis[*col] = '┴';
        }
        let lw = util::display_width(label);
        let start = if *col == 0 { 0 } else { (*col).max(next_free) };
        if start + lw <= width {
            for (i, ch) in label.chars().enumerate() {
                text[start + i] = ch;
            }
            next_free = start + lw + 2;
        }
    }
    for c in axis {
        if c == '┴' {
            write!(
                out,
                "{}",
                util::paint("┴", app.theme.roles.axis, q, color, false)
            )?;
        } else {
            write!(out, "─")?;
        }
    }
    writeln!(out)?;
    let label_line: String = text.iter().collect();
    writeln!(
        out,
        "{}",
        util::paint(label_line.trim_end(), dim, q, color, false)
    )?;
    writeln!(out)?;

    // list
    let ctx = RowCtx {
        cfg: &app.cfg.general,
        theme: &app.theme,
        now: app.now(),
        p95: app.p95,
        width,
    };
    let mut shown = 0u64;
    for item in &app.display {
        match *item {
            crate::ui::app::DisplayItem::Commit(ci) => {
                let c = &app.commits[ci];
                let pieces = rows::row_pieces(c, &ctx, false);
                let mut line = String::new();
                for p in pieces {
                    match p.rgb {
                        Some(rgb) => line.push_str(&util::paint(&p.text, rgb, q, color, p.bold)),
                        None => line.push_str(&p.text),
                    }
                }
                writeln!(out, "{}", line.trim_end())?;
                shown += 1;
            }
            crate::ui::app::DisplayItem::Gap(secs) => {
                writeln!(
                    out,
                    "{}",
                    util::paint(
                        &format!("  ┄┄ {} ┄┄", util::humanize_long(secs)),
                        dim,
                        q,
                        color,
                        false
                    )
                )?;
            }
        }
    }

    // summary
    let total_ins: u64 = app.commits.iter().map(|c| c.insertions).sum();
    let total_del: u64 = app.commits.iter().map(|c| c.deletions).sum();
    let biggest = app.commits.iter().max_by_key(|c| c.volume());
    writeln!(out)?;
    let mut summary = vec![
        util::paint(" ", dim, q, color, false),
        util::paint(
            &format!("+{}", util::comma(total_ins)),
            app.theme.roles.ok,
            q,
            color,
            false,
        ),
        util::paint(" / ", dim, q, color, false),
        util::paint(
            &format!("-{}", util::comma(total_del)),
            app.theme.roles.bad,
            q,
            color,
            false,
        ),
        util::paint(
            &format!(
                " across {} · {} files touched",
                util::count(shown, "commit", "commits"),
                util::comma(app.commits.iter().map(|c| c.files.len() as u64).sum())
            ),
            dim,
            q,
            color,
            false,
        ),
    ];
    if let Some(b) = biggest {
        summary.push(util::paint(
            &format!(" · largest {} ({} lines)", b.short, util::comma(b.volume())),
            dim,
            q,
            color,
            false,
        ));
    }
    writeln!(out, "{}", summary.concat())?;

    if let Some(p) = &app.cfg.path {
        writeln!(
            out,
            "{}",
            util::paint(
                &format!(" config {}", p.display()),
                app.theme.roles.axis,
                q,
                color,
                false
            )
        )?;
    }
    out.flush()?;
    Ok(())
}

/// Print all configured themes with their ramps.
pub fn list_themes(cfg: &crate::config::Config) {
    println!("built-in themes:");
    for name in crate::theme::BUILTIN {
        if let Some(t) = crate::theme::builtin(name) {
            let ramp: Vec<String> = t.ramp.iter().map(|c| c.to_hex()).collect();
            println!("  {:<10} {}", name, ramp.join(" "));
        }
    }
    let dir = cfg.themes_dir();
    println!("theme directory: {}", dir.display());
    if let Ok(entries) = std::fs::read_dir(&dir) {
        let mut names: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "theme"))
            .collect();
        names.sort();
        if !names.is_empty() {
            println!("custom themes:");
            for p in names {
                println!("  {}", p.display());
            }
        }
    }
}
