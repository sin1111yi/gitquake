use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

use crate::config::{Action, Config, DetailLayout, HeatScale, KeyMap};
use crate::git::{self, LoadOptions};
use crate::model::{Commit, Repo};
use crate::stats::{Window, percentile};
use crate::theme::{self, BUILTIN, Quality, Theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    Time,
    SizeDesc,
    SizeAsc,
}

impl SortMode {
    pub fn label(self) -> &'static str {
        match self {
            SortMode::Time => "newest first",
            SortMode::SizeDesc => "largest first",
            SortMode::SizeAsc => "smallest first",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Search {
    pub query: String,
    pub matches: HashSet<usize>,
}

#[derive(Debug, Clone, Copy)]
pub enum DisplayItem {
    Commit(usize),
    Gap(i64),
}

#[derive(Debug, Clone, Copy)]
pub struct RectI {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

impl RectI {
    pub fn contains(&self, x: u16, y: u16) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

#[derive(Debug, Clone)]
pub struct ReloadCtx {
    pub config_path: Option<PathBuf>,
    pub no_config: bool,
    pub theme: Option<String>,
    pub scale: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct Viewer {
    pub title: String,
    pub lines: Vec<String>,
    pub scroll: usize,
    /// Visible content rows, refreshed by the renderer.
    pub view_h: usize,
}

pub struct App {
    pub repo: Repo,
    pub cfg: Config,
    pub theme: Theme,
    pub quality: Quality,
    pub commits: Vec<Commit>,
    pub order: Vec<usize>,
    pub display: Vec<DisplayItem>,
    pub selected: usize,
    pub window: Window,
    pub full_window: Window,
    pub p95: u64,
    pub sort: SortMode,
    pub detail_open: bool,
    pub list_open: bool,
    pub help_open: bool,
    pub viewer: Option<Viewer>,
    pub search: Option<Search>,
    pub input: Option<String>,
    pub status: Option<(String, Instant)>,
    pub list_offset: usize,
    pub opts: LoadOptions,
    pub quit: bool,
    pub mouse_dirty: bool,
    pub warnings: Vec<String>,
    pub reload_ctx: ReloadCtx,
    pub wave_region: Option<RectI>,
    pub list_region: Option<RectI>,
    pub columns: Vec<Option<usize>>,
    pub list_rows: u16,
}

pub fn now() -> i64 {
    chrono::Local::now().timestamp()
}

fn windows_for(commits: &[Commit]) -> Window {
    if commits.is_empty() {
        let n = now();
        return Window::new(n - 86_400, n);
    }
    let mut min = i64::MAX;
    let mut max = i64::MIN;
    for c in commits {
        min = min.min(c.timestamp);
        max = max.max(c.timestamp);
    }
    if max - min < 60 {
        Window::new(min - 3_600, max + 3_600)
    } else {
        Window::new(min, max)
    }
}

fn reference_volume(commits: &[Commit], pct: f64) -> u64 {
    let mut vols: Vec<u64> = commits.iter().map(|c| c.heat_volume()).collect();
    vols.sort_unstable();
    percentile(&vols, pct).max(1)
}

impl App {
    pub fn new(
        repo: Repo,
        cfg: Config,
        theme: Theme,
        quality: Quality,
        opts: LoadOptions,
        reload_ctx: ReloadCtx,
        warnings: Vec<String>,
    ) -> App {
        let commits = repo.commits.clone();
        let full = windows_for(&commits);
        let p95 = reference_volume(&commits, cfg.general.heat_percentile);
        let mut app = App {
            display: Vec::new(),
            order: Vec::new(),
            commits,
            repo,
            p95,
            cfg,
            theme,
            quality,
            selected: 0,
            window: full,
            full_window: full,
            sort: SortMode::Time,
            detail_open: false,
            list_open: true,
            help_open: false,
            viewer: None,
            search: None,
            input: None,
            status: None,
            list_offset: 0,
            opts,
            quit: false,
            mouse_dirty: false,
            warnings,
            reload_ctx,
            wave_region: None,
            list_region: None,
            columns: Vec::new(),
            list_rows: 1,
        };
        app.rebuild_display();
        app
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status = Some((msg.into(), Instant::now()));
    }

    /// Returns true when something time-based changed.
    pub fn tick(&mut self) -> bool {
        if let Some((_, at)) = &self.status {
            if at.elapsed() > Duration::from_secs(3) {
                self.status = None;
                return true;
            }
        }
        false
    }

    pub fn is_dimmed(&self, commit_index: usize) -> bool {
        self.search
            .as_ref()
            .is_some_and(|s| !s.matches.contains(&commit_index))
    }

    fn rebuild_display(&mut self) {
        let mut order: Vec<usize> = (0..self.commits.len()).collect();
        match self.sort {
            SortMode::Time => {}
            SortMode::SizeDesc => order.sort_by(|&a, &b| {
                self.commits[b]
                    .volume()
                    .cmp(&self.commits[a].volume())
                    .then(self.commits[a].timestamp.cmp(&self.commits[b].timestamp))
            }),
            SortMode::SizeAsc => order.sort_by(|&a, &b| {
                self.commits[a]
                    .volume()
                    .cmp(&self.commits[b].volume())
                    .then(self.commits[b].timestamp.cmp(&self.commits[a].timestamp))
            }),
        }

        let gap = self.cfg.general.gap_hours.max(0) * 3_600;
        let mut display = Vec::with_capacity(order.len());
        if self.sort == SortMode::Time {
            let mut prev: Option<i64> = None;
            for &ci in &order {
                if let Some(p) = prev {
                    let diff = p - self.commits[ci].timestamp;
                    if gap > 0 && diff >= gap {
                        display.push(DisplayItem::Gap(diff));
                    }
                }
                display.push(DisplayItem::Commit(ci));
                prev = Some(self.commits[ci].timestamp);
            }
        } else {
            for &ci in &order {
                display.push(DisplayItem::Commit(ci));
            }
        }

        self.order = order;
        self.display = display;
        if self.selected >= self.commits.len() {
            self.selected = self.commits.len().saturating_sub(1);
        }
    }

    pub fn selected_commit_index(&self) -> Option<usize> {
        self.order.get(self.selected).copied()
    }

    pub fn selected_commit(&self) -> Option<&Commit> {
        self.selected_commit_index().map(|i| &self.commits[i])
    }

    pub fn selected_display_index(&self) -> Option<usize> {
        let ci = self.selected_commit_index()?;
        self.display
            .iter()
            .position(|d| matches!(d, DisplayItem::Commit(c) if *c == ci))
    }

    fn select_commit_index(&mut self, ci: usize) {
        if let Some(pos) = self.order.iter().position(|&c| c == ci) {
            self.selected = pos;
            self.follow_selection();
        }
    }

    fn follow_selection(&mut self) {
        if self.window.is_full(self.full_window) {
            return;
        }
        let Some(c) = self.selected_commit() else {
            return;
        };
        if c.timestamp < self.window.start || c.timestamp > self.window.end {
            let span = self.window.span();
            let center = c.timestamp;
            let start = center - span / 2;
            self.window = Window::new(start, start + span);
        }
    }

    pub fn move_selection(&mut self, delta: isize) {
        if self.order.is_empty() {
            return;
        }
        let max = self.order.len() - 1;
        let next = (self.selected as isize + delta).clamp(0, max as isize) as usize;
        if next != self.selected {
            self.selected = next;
            self.follow_selection();
        }
    }

    pub fn select_first(&mut self) {
        self.selected = 0;
        self.follow_selection();
    }

    pub fn select_last(&mut self) {
        self.selected = self.order.len().saturating_sub(1);
        self.follow_selection();
    }

    pub fn zoom(&mut self, factor: f64) {
        let at = self
            .selected_commit()
            .map(|c| c.timestamp)
            .unwrap_or(self.window.start + self.window.span() / 2);
        let next = self.window.zoom(at, factor, self.full_window);
        self.window = if next.span() >= self.full_window.span() - 1 {
            self.full_window
        } else {
            next
        };
        let pct =
            (self.window.span() as f64 / self.full_window.span() as f64 * 100.0).round() as i64;
        if pct >= 100 {
            self.set_status("zoom: full history");
        } else {
            self.set_status(format!("zoom: {pct}% of history"));
        }
    }

    pub fn pan(&mut self, frac: f64) {
        if self.window.is_full(self.full_window) {
            self.set_status("zoom in first (+/-) to pan");
            return;
        }
        self.window = self.window.pan(frac, self.full_window);
    }

    pub fn reset_zoom(&mut self) {
        self.window = self.full_window;
        self.set_status("zoom: full history");
    }

    pub fn toggle_sort(&mut self) {
        self.sort = match self.sort {
            SortMode::Time => SortMode::SizeDesc,
            SortMode::SizeDesc => SortMode::SizeAsc,
            SortMode::SizeAsc => SortMode::Time,
        };
        let keep = self.selected_commit_index();
        self.rebuild_display();
        if let Some(ci) = keep {
            self.select_commit_index(ci);
        }
        let label = self.sort.label();
        self.set_status(format!("sort: {label}"));
    }

    pub fn toggle_scale(&mut self) {
        self.cfg.general.heat_scale = match self.cfg.general.heat_scale {
            HeatScale::Log => HeatScale::Linear,
            HeatScale::Linear => HeatScale::Log,
        };
        self.p95 = reference_volume(&self.commits, self.cfg.general.heat_percentile);
        self.set_status(format!(
            "heat scale: {}",
            self.cfg.general.heat_scale.as_str()
        ));
    }

    pub fn toggle_theme(&mut self) {
        let current = self.theme.name.clone();
        let idx = BUILTIN
            .iter()
            .position(|n| *n == current.as_str())
            .unwrap_or(BUILTIN.len() - 1);
        let next = BUILTIN[(idx + 1) % BUILTIN.len()];
        self.cfg.general.theme = next.to_string();
        let mut warnings = Vec::new();
        self.theme = theme::resolve(&self.cfg, &mut warnings);
        self.warnings.extend(warnings);
        let name = self.theme.name.clone();
        self.set_status(format!("theme: {name}"));
    }

    pub fn toggle_detail(&mut self) {
        self.detail_open = !self.detail_open;
    }

    pub fn toggle_list(&mut self) {
        self.list_open = !self.list_open;
    }

    pub fn toggle_help(&mut self) {
        self.help_open = !self.help_open;
    }

    pub fn toggle_mouse(&mut self) {
        self.cfg.general.mouse = !self.cfg.general.mouse;
        self.mouse_dirty = true;
        let state = if self.cfg.general.mouse { "on" } else { "off" };
        self.set_status(format!("mouse: {state}"));
    }

    pub fn start_search(&mut self) {
        let existing = self
            .search
            .as_ref()
            .map(|s| s.query.clone())
            .unwrap_or_default();
        self.input = Some(existing);
    }

    fn update_matches(&mut self, query: &str) {
        let q = query.trim().to_ascii_lowercase();
        if q.is_empty() {
            self.search = None;
            return;
        }
        let mut matches = HashSet::new();
        for (i, c) in self.commits.iter().enumerate() {
            if c.subject.to_ascii_lowercase().contains(&q)
                || c.author_name.to_ascii_lowercase().contains(&q)
                || c.hash.to_ascii_lowercase().contains(&q)
                || c.short.to_ascii_lowercase().contains(&q)
            {
                matches.insert(i);
            }
        }
        let count = matches.len();
        self.search = Some(Search {
            query: query.trim().to_string(),
            matches,
        });
        let word = if count == 1 { "match" } else { "matches" };
        self.set_status(format!("{count} {word} for `{}`", query.trim()));
    }

    fn handle_input(&mut self, key: KeyEvent) {
        let mut buffer = self.input.clone().unwrap_or_default();
        match key.code {
            KeyCode::Esc => {
                self.input = None;
                self.search = None;
                return;
            }
            KeyCode::Enter => {
                self.input = None;
                self.update_matches(&buffer);
                let already = self
                    .selected_commit_index()
                    .zip(self.search.as_ref())
                    .is_some_and(|(ci, s)| s.matches.contains(&ci));
                if !already {
                    self.jump_to_match(1);
                }
                return;
            }
            KeyCode::Backspace => {
                buffer.pop();
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                buffer.push(c);
            }
            _ => return,
        }
        self.input = Some(buffer.clone());
        if buffer.trim().is_empty() {
            self.search = None;
        } else {
            self.update_matches(&buffer);
        }
    }

    fn jump_to_match(&mut self, dir: isize) {
        let Some(search) = &self.search else {
            return;
        };
        if search.matches.is_empty() {
            return;
        }
        let len = self.order.len();
        if len == 0 {
            return;
        }
        let mut pos = self.selected as isize;
        for _ in 0..len {
            pos = (pos + dir).rem_euclid(len as isize);
            if let Some(&ci) = self.order.get(pos as usize) {
                if search.matches.contains(&ci) {
                    self.selected = pos as usize;
                    self.follow_selection();
                    return;
                }
            }
        }
    }

    pub fn search_next(&mut self) {
        self.jump_to_match(1);
    }

    pub fn search_prev(&mut self) {
        self.jump_to_match(-1);
    }

    pub fn clear_search(&mut self) {
        self.input = None;
        self.search = None;
    }

    pub fn dispatch(&mut self, action: Action) {
        match action {
            Action::Quit => self.quit = true,
            Action::Help => self.toggle_help(),
            Action::SelectPrev => self.move_selection(-1),
            Action::SelectNext => self.move_selection(1),
            Action::PageUp => self.move_selection(-(self.list_rows.max(1) as isize)),
            Action::PageDown => self.move_selection(self.list_rows.max(1) as isize),
            Action::SelectFirst => self.select_first(),
            Action::SelectLast => self.select_last(),
            Action::PanLeft => self.pan(-0.18),
            Action::PanRight => self.pan(0.18),
            Action::ZoomIn => self.zoom(1.7),
            Action::ZoomOut => self.zoom(1.0 / 1.7),
            Action::ResetZoom => self.reset_zoom(),
            Action::ToggleDetail => self.toggle_detail(),
            Action::ViewCommit => self.open_viewer(),
            Action::Search => self.start_search(),
            Action::SearchNext => self.search_next(),
            Action::SearchPrev => self.search_prev(),
            Action::ToggleSort => self.toggle_sort(),
            Action::ToggleScale => self.toggle_scale(),
            Action::ToggleAllBranches => self.toggle_all_branches(),
            Action::ToggleTheme => self.toggle_theme(),
            Action::ToggleList => self.toggle_list(),
            Action::Reload => self.reload_all(),
            Action::ToggleMouse => self.toggle_mouse(),
        }
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.quit = true;
            return;
        }
        if self.input.is_some() {
            self.handle_input(key);
            return;
        }
        if self.viewer.is_some() {
            match key.code {
                KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char('v') => {
                    self.close_viewer()
                }
                KeyCode::Down | KeyCode::Char('j') => self.viewer_scroll(1),
                KeyCode::Up | KeyCode::Char('k') => self.viewer_scroll(-1),
                KeyCode::PageDown | KeyCode::Char(' ') => self.viewer_page(1),
                KeyCode::PageUp => self.viewer_page(-1),
                KeyCode::Home | KeyCode::Char('g') => self.viewer_top(),
                KeyCode::End | KeyCode::Char('G') => self.viewer_bottom(),
                _ => {}
            }
            return;
        }
        if key.code == KeyCode::Esc {
            if self.help_open {
                self.help_open = false;
            } else if self.search.is_some() {
                self.clear_search();
            } else if !self.window.is_full(self.full_window) {
                self.reset_zoom();
            }
            return;
        }
        if let Some(action) = self.cfg.keymap.action(key.code) {
            self.dispatch(action);
        }
    }

    pub fn on_mouse(&mut self, ev: MouseEvent) {
        if !self.cfg.general.mouse {
            return;
        }
        if self.viewer.is_some() {
            match ev.kind {
                MouseEventKind::ScrollUp => self.viewer_scroll(-3),
                MouseEventKind::ScrollDown => self.viewer_scroll(3),
                _ => {}
            }
            return;
        }
        match ev.kind {
            MouseEventKind::ScrollUp => self.move_selection(-1),
            MouseEventKind::ScrollDown => self.move_selection(1),
            MouseEventKind::Down(MouseButton::Left) => {
                if let Some(r) = self.wave_region {
                    if r.contains(ev.column, ev.row) {
                        let col = (ev.column - r.x) as usize;
                        if let Some(Some(ci)) = self.columns.get(col).copied() {
                            self.select_commit_index(ci);
                        }
                        return;
                    }
                }
                if let Some(r) = self.list_region {
                    if r.contains(ev.column, ev.row) {
                        let row = (ev.row - r.y) as usize;
                        let idx = self.list_offset + row;
                        if let Some(DisplayItem::Commit(ci)) = self.display.get(idx).copied() {
                            self.select_commit_index(ci);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub fn open_viewer(&mut self) {
        let Some(c) = self.selected_commit() else {
            return;
        };
        let root = self.repo.root.clone();
        let hash = c.hash.clone();
        let title = format!(" {}  {} ", c.short, c.subject);
        let lines = match git::show(&root, &hash) {
            Ok(lines) => lines,
            Err(err) => vec![format!("could not load commit {hash}: {err}")],
        };
        self.viewer = Some(Viewer {
            title,
            lines,
            scroll: 0,
            view_h: 0,
        });
    }

    pub fn close_viewer(&mut self) {
        self.viewer = None;
    }

    fn viewer_max_scroll(&self) -> usize {
        match &self.viewer {
            Some(v) => v.lines.len().saturating_sub(v.view_h.max(1)),
            None => 0,
        }
    }

    pub fn viewer_scroll(&mut self, delta: isize) {
        let max = self.viewer_max_scroll() as isize;
        if let Some(v) = &mut self.viewer {
            v.scroll = (v.scroll as isize + delta).clamp(0, max) as usize;
        }
    }

    pub fn viewer_page(&mut self, dir: isize) {
        let page = self
            .viewer
            .as_ref()
            .map(|v| (v.view_h.max(1) as isize - 1).max(1))
            .unwrap_or(10);
        self.viewer_scroll(dir * page);
    }

    pub fn viewer_top(&mut self) {
        if let Some(v) = &mut self.viewer {
            v.scroll = 0;
        }
    }

    pub fn viewer_bottom(&mut self) {
        let max = self.viewer_max_scroll();
        if let Some(v) = &mut self.viewer {
            v.scroll = max;
        }
    }

    pub fn toggle_all_branches(&mut self) {
        self.opts.all = !self.opts.all;
        self.cfg.general.all_branches = self.opts.all;
        let state = if self.opts.all {
            "all branches"
        } else {
            "current branch"
        };
        self.set_status(format!("scope: {state}"));
        self.reload_repo(true);
    }

    pub fn reload_all(&mut self) {
        let loaded = crate::config::load(
            self.reload_ctx.config_path.as_deref(),
            self.reload_ctx.no_config,
        );
        let mut warnings = loaded.config.warnings.clone();
        let mut cfg = loaded.config;
        if let Some(theme) = &self.reload_ctx.theme {
            cfg.general.theme = theme.clone();
        }
        if let Some(scale) = &self.reload_ctx.scale {
            if let Some(v) = HeatScale::parse(scale) {
                cfg.general.heat_scale = v;
            }
        }
        let mut keymap = KeyMap::default();
        keymap.apply(&cfg.keys, &mut warnings);
        cfg.keymap = keymap;
        self.cfg = cfg;
        self.theme = theme::resolve(&self.cfg, &mut warnings);
        self.warnings = warnings;
        self.opts.limit = self.reload_ctx.limit.unwrap_or(self.cfg.general.limit);
        self.reload_repo(false);
        self.set_status("reloaded config and history");
    }

    fn reload_repo(&mut self, keep_window: bool) {
        let path = self.repo.root.join(".");
        match git::load(&path, &self.opts) {
            Ok(repo) => {
                let keep_hash = self.selected_commit().map(|c| c.hash.clone());
                let old_full = self.full_window;
                self.commits = repo.commits.clone();
                self.repo = repo;
                self.full_window = windows_for(&self.commits);
                self.window = if keep_window {
                    let rel_start =
                        (self.window.start - old_full.start) as f64 / old_full.span() as f64;
                    let rel_end =
                        (self.window.end - old_full.start) as f64 / old_full.span() as f64;
                    Window::new(
                        self.full_window.start
                            + (rel_start * self.full_window.span() as f64) as i64,
                        self.full_window.start + (rel_end * self.full_window.span() as f64) as i64,
                    )
                } else {
                    self.full_window
                };
                self.p95 = reference_volume(&self.commits, self.cfg.general.heat_percentile);
                self.rebuild_display();
                if let Some(hash) = keep_hash {
                    if let Some(pos) = self
                        .order
                        .iter()
                        .position(|&ci| self.commits[ci].hash == hash)
                    {
                        self.selected = pos;
                    }
                }
                if self.selected >= self.order.len() {
                    self.selected = self.order.len().saturating_sub(1);
                }
            }
            Err(err) => {
                self.set_status(format!("reload failed: {err}"));
            }
        }
    }

    pub fn now(&self) -> i64 {
        now()
    }

    pub fn window_label(&self) -> String {
        let fmt = &self.cfg.general.date_format;
        let a = crate::util::absolute(self.window.start, fmt);
        let b = crate::util::absolute(self.window.end, fmt);
        let pct =
            (self.window.span() as f64 / self.full_window.span() as f64 * 100.0).round() as i64;
        if pct >= 100 {
            format!("{a} → {b}  ·  full history")
        } else {
            format!("{a} → {b}  ·  {pct}% of history")
        }
    }

    pub fn detail_mode(&self) -> DetailLayout {
        self.cfg.general.detail_layout
    }
}
