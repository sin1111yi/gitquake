pub mod app;
pub mod chrome;
pub mod detail;
pub mod help;
pub mod list;
pub mod viewer;
pub mod wave;

use std::io::stdout;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event};
use crossterm::execute;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};

use crate::config::DetailLayout;
use app::App;

pub fn ui(f: &mut Frame, app: &mut App) {
    let area = f.area();
    if area.width < 20 || area.height < 8 {
        return;
    }

    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(4),
        Constraint::Length(2),
    ])
    .split(area);

    chrome::draw_header(f, app, chunks[0]);
    draw_body(f, app, chunks[1]);
    chrome::draw_footer(f, app, chunks[2]);

    if app.help_open {
        help::draw(f, app, area);
    }
    if app.viewer.is_some() {
        viewer::draw(f, app, area);
    }
}

fn draw_body(f: &mut Frame, app: &mut App, body: ratatui::layout::Rect) {
    if body.height < 3 || body.width < 20 {
        return;
    }
    let show_list = app.list_open;
    let show_detail = app.detail_open;
    if !show_list && !show_detail {
        wave::draw(f, app, body);
        app.list_region = None;
        return;
    }

    let want_wave = (body.height as f64 * app.cfg.general.wave_height).round() as u16;
    let max_wave = body.height.saturating_sub(3);
    if max_wave < 5 {
        wave::draw(f, app, body);
        app.list_region = None;
        return;
    }
    let wave_h = want_wave.clamp(5, max_wave);
    let rows = Layout::vertical([Constraint::Length(wave_h), Constraint::Min(1)]).split(body);
    wave::draw(f, app, rows[0]);
    let lower = rows[1];
    if lower.height == 0 {
        return;
    }

    if show_detail {
        let mode = match app.detail_mode() {
            DetailLayout::Auto if lower.width >= 110 => DetailLayout::Side,
            DetailLayout::Auto => DetailLayout::Bottom,
            other => other,
        };
        if !show_list {
            app.list_region = None;
            detail::draw(f, app, lower);
            return;
        }
        match mode {
            DetailLayout::Side if lower.width >= 110 => {
                let side_w = (lower.width as f64 * 0.36) as u16;
                let cols = Layout::horizontal([
                    Constraint::Min(40),
                    Constraint::Length(side_w.clamp(34, 64)),
                ])
                .split(lower);
                if show_list {
                    list::draw(f, app, cols[0]);
                } else {
                    app.list_region = None;
                }
                detail::draw(f, app, cols[1]);
            }
            _ => {
                if lower.height < 10 {
                    if show_list {
                        list::draw(f, app, lower);
                    } else {
                        detail::draw(f, app, lower);
                    }
                    return;
                }
                let detail_h = (lower.height / 2)
                    .max(10)
                    .min(lower.height.saturating_sub(2));
                let parts = Layout::vertical([Constraint::Min(4), Constraint::Length(detail_h)])
                    .split(lower);
                if show_list {
                    list::draw(f, app, parts[0]);
                } else {
                    app.list_region = None;
                }
                detail::draw(f, app, parts[1]);
            }
        }
    } else if show_list {
        list::draw(f, app, lower);
    }
}

pub fn run(app: &mut App) -> Result<()> {
    let mut terminal = ratatui::init();
    if app.cfg.general.mouse {
        let _ = execute!(stdout(), EnableMouseCapture);
    }
    let result = event_loop(&mut terminal, app);
    if app.cfg.general.mouse {
        let _ = execute!(stdout(), DisableMouseCapture);
    }
    ratatui::restore();
    result
}

fn event_loop(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> Result<()> {
    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| ui(f, app))?;
        if app.quit {
            break;
        }

        if event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(key) => app.on_key(key),
                Event::Mouse(ev) => app.on_mouse(ev),
                _ => {}
            }
        } else if last_tick.elapsed() >= Duration::from_secs(30) {
            last_tick = Instant::now();
        }
        app.tick();

        if app.mouse_dirty {
            if app.cfg.general.mouse {
                let _ = execute!(stdout(), EnableMouseCapture);
            } else {
                let _ = execute!(stdout(), DisableMouseCapture);
            }
            app.mouse_dirty = false;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::config::Config;
    use crate::git::LoadOptions;
    use crate::model::{Commit, FileStat, GitRef, RefKind, Repo};
    use crate::theme::{self, Quality};
    use crate::ui::app::{App, ReloadCtx};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use std::path::PathBuf;

    fn commit(i: usize, ts: i64, vol: u64, subject: &str) -> Commit {
        let hash = format!("{:040x}", i + 1);
        let files = vec![FileStat {
            path: format!("src/file_{i}.rs"),
            insertions: Some(vol / 2 + 1),
            deletions: Some(vol / 2),
        }];
        Commit {
            hash,
            short: format!("{:07x}", i + 1),
            author_name: if i % 3 == 0 {
                "Ada Lovelace".into()
            } else {
                "Linus".into()
            },
            author_email: "a@example.com".into(),
            timestamp: ts,
            subject: subject.into(),
            refs: if i == 0 {
                vec![GitRef {
                    kind: RefKind::Head,
                    name: "main".into(),
                }]
            } else {
                Vec::new()
            },
            parents: if i % 5 == 0 {
                vec!["1".repeat(40), "2".repeat(40)]
            } else {
                vec!["0".repeat(40)]
            },
            files,
            insertions: vol / 2 + 1,
            deletions: vol / 2,
        }
    }

    fn app_with(n: usize, detail: bool) -> App {
        let base = 1_760_000_000i64;
        let commits: Vec<Commit> = (0..n)
            .map(|i| {
                commit(
                    i,
                    base + (i as i64) * 5_000,
                    if i == 2 { 4_000 } else { (i as u64 + 1) * 7 },
                    &format!("commit number {i} with a reasonably long subject"),
                )
            })
            .collect();
        let repo = Repo {
            root: PathBuf::from("/tmp/demo"),
            name: "demo".into(),
            branch: "main".into(),
            total_commits: n,
            authors: 2,
            commits,
        };
        let cfg = Config::default();
        let theme = theme::builtin("inferno").unwrap();
        let mut app = App::new(
            repo,
            cfg,
            theme,
            Quality::True,
            LoadOptions::default(),
            ReloadCtx {
                config_path: None,
                no_config: true,
                theme: None,
                scale: None,
                limit: None,
            },
            Vec::new(),
        );
        app.detail_open = detail;
        app
    }

    fn render(app: &mut App, w: u16, h: u16) -> String {
        let backend = TestBackend::new(w, h);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| super::ui(f, app)).unwrap();
        let buf = terminal.backend().buffer().clone();
        let mut out = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                out.push_str(buf[(x, y)].symbol());
            }
            out.push('\n');
        }
        out
    }

    #[test]
    fn renders_header_wave_list_and_footer() {
        let mut app = app_with(14, false);
        let text = render(&mut app, 120, 40);
        assert!(text.contains("ghg"));
        assert!(text.contains("demo"));
        assert!(text.contains("commits"));
        assert!(text.contains("heat"));
        assert!(text.contains("commit number 0"));
        assert!(text.contains('█'));
        assert!(app.wave_region.is_some());
        assert!(app.list_region.is_some());
    }

    #[test]
    fn renders_detail_side_by_side() {
        let mut app = app_with(10, true);
        let text = render(&mut app, 130, 40);
        assert!(text.contains("commit "));
        assert!(text.contains("src/file_0.rs"), "{text}");
        assert!(text.contains("Ada Lovelace"));
    }

    #[test]
    fn renders_detail_bottom_on_narrow_screens() {
        let mut app = app_with(8, true);
        let text = render(&mut app, 90, 34);
        assert!(text.contains("src/file_0.rs"), "{text}");
    }

    #[test]
    fn help_overlay_lists_keys() {
        let mut app = app_with(6, false);
        app.help_open = true;
        let text = render(&mut app, 100, 36);
        assert!(text.contains("keys"));
        assert!(text.contains("search commits"));
        assert!(text.contains("zoom in"));
    }

    #[test]
    fn search_prompt_renders() {
        let mut app = app_with(8, false);
        app.input = Some("commit 3".into());
        app.search = Some(super::app::Search {
            query: "commit 3".into(),
            matches: [3usize].into_iter().collect(),
        });
        let text = render(&mut app, 110, 34);
        assert!(text.contains("commit 3"));
        assert!(text.contains("match"));
    }

    #[test]
    fn footer_uses_key_chips_and_selection_status() {
        let mut app = app_with(20, false);
        let text = render(&mut app, 120, 40);
        assert!(text.contains("select"), "{text}");
        assert!(text.contains("search"), "{text}");
        assert!(text.contains("help"), "{text}");
        assert!(text.contains("quit"), "{text}");
        assert!(text.contains('│'), "expected group separators:\n{text}");
        assert!(
            text.contains("1/20"),
            "expected position indicator:\n{text}"
        );
    }

    #[test]
    fn footer_keeps_help_and_quit_when_narrow() {
        let mut app = app_with(8, false);
        let text = render(&mut app, 58, 24);
        assert!(text.contains("help"), "{text}");
        assert!(text.contains("quit"), "{text}");
    }

    #[test]
    fn footer_shows_status_and_search() {
        let mut app = app_with(10, false);
        app.set_status("scope: all branches");
        let text = render(&mut app, 110, 34);
        assert!(text.contains("scope: all branches"), "{text}");

        app.status = None;
        app.search = Some(super::app::Search {
            query: "commit 3".into(),
            matches: [3usize].into_iter().collect(),
        });
        let text = render(&mut app, 110, 34);
        assert!(text.contains("1 match"), "{text}");
        assert!(text.contains("Esc clear"), "{text}");
    }

    #[test]
    fn viewer_overlay_renders_and_scrolls() {
        let mut app = app_with(8, false);
        app.viewer = Some(crate::ui::app::Viewer {
            title: " abc1234 add the thing ".into(),
            lines: (0..40).map(|i| format!("line {i:02}")).collect(),
            scroll: 0,
            view_h: 0,
        });
        let text = render(&mut app, 100, 30);
        assert!(text.contains("abc1234"), "{text}");
        assert!(text.contains("line 00"), "{text}");
        assert!(text.contains("j/k scroll"), "{text}");
        app.viewer_page(1);
        assert!(app.viewer.as_ref().unwrap().scroll > 0);
        app.viewer_bottom();
        assert!(app.viewer.as_ref().unwrap().scroll > 0);
        app.viewer_top();
        assert_eq!(app.viewer.as_ref().unwrap().scroll, 0);
        app.close_viewer();
        assert!(app.viewer.is_none());
    }

    #[test]
    fn tiny_terminal_does_not_panic() {
        let mut app = app_with(3, false);
        let _ = render(&mut app, 24, 10);
        let _ = render(&mut app, 40, 12);
    }
}
