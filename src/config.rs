use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crossterm::event::KeyCode;

use crate::theme::Rgb;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaveMode {
    Volume,
    Density,
    Both,
}
impl WaveMode {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "volume" => Some(WaveMode::Volume),
            "density" => Some(WaveMode::Density),
            "both" => Some(WaveMode::Both),
            _ => None,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            WaveMode::Volume => "volume",
            WaveMode::Density => "density",
            WaveMode::Both => "both",
        }
    }
}

/// How commit timestamps map to horizontal position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaveAxis {
    /// Strictly proportional to wall-clock time: quiet months take up space.
    Time,
    /// Half wall-clock, half commit order: bursts stay readable, spacing stays honest.
    Blend,
}

impl WaveAxis {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "time" | "linear" => Some(WaveAxis::Time),
            "blend" | "balanced" => Some(WaveAxis::Blend),
            _ => None,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            WaveAxis::Time => "time",
            WaveAxis::Blend => "blend",
        }
    }
    pub fn weight(self) -> f64 {
        match self {
            WaveAxis::Time => 1.0,
            WaveAxis::Blend => 0.5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeatScale {
    Log,
    Linear,
}

impl HeatScale {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "log" => Some(HeatScale::Log),
            "linear" => Some(HeatScale::Linear),
            _ => None,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            HeatScale::Log => "log",
            HeatScale::Linear => "linear",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlyphSet {
    Eighth,
    Block,
    Ascii,
}

impl GlyphSet {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "eighth" => Some(GlyphSet::Eighth),
            "block" => Some(GlyphSet::Block),
            "ascii" => Some(GlyphSet::Ascii),
            _ => None,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            GlyphSet::Eighth => "eighth",
            GlyphSet::Block => "block",
            GlyphSet::Ascii => "ascii",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailLayout {
    Auto,
    Side,
    Bottom,
}

impl DetailLayout {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "auto" => Some(DetailLayout::Auto),
            "side" => Some(DetailLayout::Side),
            "bottom" => Some(DetailLayout::Bottom),
            _ => None,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            DetailLayout::Auto => "auto",
            DetailLayout::Side => "side",
            DetailLayout::Bottom => "bottom",
        }
    }
}

#[derive(Debug, Clone)]
pub struct General {
    pub theme: String,
    pub limit: usize,
    pub all_branches: bool,
    pub relative_time: bool,
    pub date_format: String,
    pub mouse: bool,
    pub wave_height: f64,
    pub wave_mode: WaveMode,
    pub wave_axis: WaveAxis,
    pub density_strip: bool,
    pub heat_scale: HeatScale,
    pub heat_percentile: f64,
    pub heat_gamma: f64,
    pub glyphs: GlyphSet,
    pub bar_width: usize,
    pub row_fields: Vec<String>,
    pub detail_layout: DetailLayout,
    pub gap_hours: i64,
}

impl Default for General {
    fn default() -> Self {
        General {
            theme: "inferno".into(),
            limit: 5000,
            all_branches: false,
            relative_time: true,
            date_format: "%Y-%m-%d".into(),
            mouse: true,
            wave_height: 0.55,
            wave_mode: WaveMode::Volume,
            wave_axis: WaveAxis::Blend,
            density_strip: true,
            heat_scale: HeatScale::Log,
            heat_percentile: 95.0,
            heat_gamma: 1.0,
            glyphs: GlyphSet::Eighth,
            bar_width: 20,
            row_fields: vec![
                "heat".into(),
                "bar".into(),
                "hash".into(),
                "refs".into(),
                "subject".into(),
                "author".into(),
                "time".into(),
                "diff".into(),
            ],
            detail_layout: DetailLayout::Auto,
            gap_hours: 12,
        }
    }
}

pub const ROW_FIELDS: &[&str] = &[
    "heat", "bar", "hash", "subject", "author", "time", "diff", "refs",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Quit,
    Help,
    SelectPrev,
    SelectNext,
    PageUp,
    PageDown,
    SelectFirst,
    SelectLast,
    PanLeft,
    PanRight,
    ZoomIn,
    ZoomOut,
    ResetZoom,
    ToggleDetail,
    ViewCommit,
    Search,
    SearchNext,
    SearchPrev,
    ToggleSort,
    ToggleScale,
    ToggleAllBranches,
    ToggleTheme,
    ToggleList,
    Reload,
    ToggleMouse,
}

impl Action {
    pub const ALL: &'static [Action] = &[
        Action::Quit,
        Action::Help,
        Action::SelectPrev,
        Action::SelectNext,
        Action::PageUp,
        Action::PageDown,
        Action::SelectFirst,
        Action::SelectLast,
        Action::PanLeft,
        Action::PanRight,
        Action::ZoomIn,
        Action::ZoomOut,
        Action::ResetZoom,
        Action::ToggleDetail,
        Action::ViewCommit,
        Action::Search,
        Action::SearchNext,
        Action::SearchPrev,
        Action::ToggleSort,
        Action::ToggleScale,
        Action::ToggleAllBranches,
        Action::ToggleTheme,
        Action::ToggleList,
        Action::Reload,
        Action::ToggleMouse,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Action::Quit => "quit",
            Action::Help => "help",
            Action::SelectPrev => "select_prev",
            Action::SelectNext => "select_next",
            Action::PageUp => "page_up",
            Action::PageDown => "page_down",
            Action::SelectFirst => "select_first",
            Action::SelectLast => "select_last",
            Action::PanLeft => "pan_left",
            Action::PanRight => "pan_right",
            Action::ZoomIn => "zoom_in",
            Action::ZoomOut => "zoom_out",
            Action::ResetZoom => "reset_zoom",
            Action::ToggleDetail => "toggle_detail",
            Action::ViewCommit => "view_commit",
            Action::Search => "search",
            Action::SearchNext => "search_next",
            Action::SearchPrev => "search_prev",
            Action::ToggleSort => "toggle_sort",
            Action::ToggleScale => "toggle_scale",
            Action::ToggleAllBranches => "toggle_all_branches",
            Action::ToggleTheme => "toggle_theme",
            Action::ToggleList => "toggle_list",
            Action::Reload => "reload",
            Action::ToggleMouse => "toggle_mouse",
        }
    }

    pub fn from_name(name: &str) -> Option<Action> {
        Action::ALL.iter().copied().find(|a| a.name() == name)
    }
}

#[derive(Debug, Clone)]
pub struct KeyMap {
    pub map: HashMap<Action, Vec<KeyCode>>,
}

fn bind(map: &mut HashMap<Action, Vec<KeyCode>>, action: Action, keys: &[KeyCode]) {
    map.insert(action, keys.to_vec());
}

impl Default for KeyMap {
    fn default() -> Self {
        use KeyCode::*;
        let mut map = HashMap::new();
        bind(&mut map, Action::Quit, &[Char('q')]);
        bind(&mut map, Action::Help, &[Char('?')]);
        bind(&mut map, Action::SelectPrev, &[Up, Char('k')]);
        bind(&mut map, Action::SelectNext, &[Down, Char('j')]);
        bind(&mut map, Action::PageUp, &[PageUp]);
        bind(&mut map, Action::PageDown, &[PageDown]);
        bind(&mut map, Action::SelectFirst, &[Char('g'), Home]);
        bind(&mut map, Action::SelectLast, &[Char('G'), End]);
        bind(&mut map, Action::PanLeft, &[Left, Char('h')]);
        bind(&mut map, Action::PanRight, &[Right, Char('l')]);
        bind(&mut map, Action::ZoomIn, &[Char('+'), Char('=')]);
        bind(&mut map, Action::ZoomOut, &[Char('-'), Char('_')]);
        bind(&mut map, Action::ResetZoom, &[Char('0')]);
        bind(&mut map, Action::ToggleDetail, &[Enter]);
        bind(&mut map, Action::ViewCommit, &[KeyCode::Char('v')]);
        bind(&mut map, Action::Search, &[Char('/')]);
        bind(&mut map, Action::SearchNext, &[Char('n')]);
        bind(&mut map, Action::SearchPrev, &[Char('N')]);
        bind(&mut map, Action::ToggleSort, &[Char('t')]);
        bind(&mut map, Action::ToggleScale, &[Char('s')]);
        bind(&mut map, Action::ToggleAllBranches, &[Char('a')]);
        bind(&mut map, Action::ToggleTheme, &[Char('T')]);
        bind(&mut map, Action::ToggleList, &[Char('L')]);
        bind(&mut map, Action::Reload, &[Char('R')]);
        bind(&mut map, Action::ToggleMouse, &[Char('m')]);
        KeyMap { map }
    }
}

pub fn parse_key(s: &str) -> Option<KeyCode> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let lower = s.to_ascii_lowercase();
    let named = match lower.as_str() {
        "space" => Some(KeyCode::Char(' ')),
        "enter" | "return" => Some(KeyCode::Enter),
        "esc" | "escape" => Some(KeyCode::Esc),
        "tab" => Some(KeyCode::Tab),
        "backspace" => Some(KeyCode::Backspace),
        "up" => Some(KeyCode::Up),
        "down" => Some(KeyCode::Down),
        "left" => Some(KeyCode::Left),
        "right" => Some(KeyCode::Right),
        "pageup" | "pgup" => Some(KeyCode::PageUp),
        "pagedown" | "pgdn" => Some(KeyCode::PageDown),
        "home" => Some(KeyCode::Home),
        "end" => Some(KeyCode::End),
        _ => None,
    };
    if named.is_some() {
        return named;
    }
    let mut chars = s.chars();
    let first = chars.next()?;
    if chars.next().is_none() {
        return Some(KeyCode::Char(first));
    }
    None
}

pub fn key_label(code: KeyCode) -> String {
    match code {
        KeyCode::Char(' ') => "space".into(),
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "↵".into(),
        KeyCode::Esc => "esc".into(),
        KeyCode::Tab => "tab".into(),
        KeyCode::Backspace => "⌫".into(),
        KeyCode::Up => "↑".into(),
        KeyCode::Down => "↓".into(),
        KeyCode::Left => "←".into(),
        KeyCode::Right => "→".into(),
        KeyCode::PageUp => "PgUp".into(),
        KeyCode::PageDown => "PgDn".into(),
        KeyCode::Home => "Home".into(),
        KeyCode::End => "End".into(),
        other => format!("{other:?}"),
    }
}

impl KeyMap {
    pub fn apply(&mut self, overrides: &HashMap<String, String>, warnings: &mut Vec<String>) {
        for (name, raw) in overrides {
            let Some(action) = Action::from_name(name) else {
                warnings.push(format!("unknown key action `{name}`"));
                continue;
            };
            let Some(code) = parse_key(raw) else {
                warnings.push(format!("cannot parse key `{raw}` for `{name}`"));
                continue;
            };
            self.map.insert(action, vec![code]);
        }
    }

    pub fn action(&self, code: KeyCode) -> Option<Action> {
        for (action, keys) in &self.map {
            if keys.contains(&code) {
                return Some(*action);
            }
        }
        None
    }

    pub fn label(&self, action: Action) -> String {
        self.map
            .get(&action)
            .and_then(|keys| keys.first().copied())
            .map(key_label)
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub general: General,
    pub colors: HashMap<String, String>,
    pub keys: HashMap<String, String>,
    pub keymap: KeyMap,
    pub warnings: Vec<String>,
    pub path: Option<PathBuf>,
}

impl Config {
    pub fn base_dir(&self) -> Option<&Path> {
        self.path.as_deref().and_then(Path::parent)
    }

    pub fn themes_dir(&self) -> PathBuf {
        self.base_dir()
            .map(|d| d.to_path_buf())
            .unwrap_or_else(|| {
                default_config_path()
                    .parent()
                    .unwrap_or(Path::new("."))
                    .to_path_buf()
            })
            .join("themes")
    }

    pub fn theme_file(&self, name: &str) -> PathBuf {
        let expanded = expand_tilde(name);
        let looks_like_path = expanded.components().count() > 1
            || name.ends_with(".toml")
            || name.ends_with(".theme");
        if looks_like_path {
            if expanded.is_absolute() {
                return expanded;
            }
            if let Some(base) = self.base_dir() {
                let candidate = base.join(&expanded);
                if candidate.exists() {
                    return candidate;
                }
            }
            return expanded;
        }
        self.themes_dir().join(format!("{name}.toml"))
    }

    pub fn dump(&self) -> String {
        let g = &self.general;
        let mut out = String::new();
        out.push_str("[general]\n");
        out.push_str(&format!("theme = {}\n", toml_quote(&g.theme)));
        out.push_str(&format!("limit = {}\n", g.limit));
        out.push_str(&format!("all_branches = {}\n", g.all_branches));
        out.push_str(&format!("relative_time = {}\n", g.relative_time));
        out.push_str(&format!("date_format = {}\n", toml_quote(&g.date_format)));
        out.push_str(&format!("mouse = {}\n", g.mouse));
        out.push_str(&format!("wave_height = {:.2}\n", g.wave_height));
        out.push_str(&format!(
            "wave_mode = {}\n",
            toml_quote(g.wave_mode.as_str())
        ));
        out.push_str(&format!(
            "wave_axis = {}\n",
            toml_quote(g.wave_axis.as_str())
        ));
        out.push_str(&format!("density_strip = {}\n", g.density_strip));
        out.push_str(&format!(
            "heat_scale = {}\n",
            toml_quote(g.heat_scale.as_str())
        ));
        out.push_str(&format!("heat_percentile = {:.0}\n", g.heat_percentile));
        out.push_str(&format!("heat_gamma = {:.2}\n", g.heat_gamma));
        out.push_str(&format!("glyphs = {}\n", toml_quote(g.glyphs.as_str())));
        out.push_str(&format!("bar_width = {}\n", g.bar_width));
        let fields: Vec<String> = g.row_fields.iter().map(|f| toml_quote(f)).collect();
        out.push_str(&format!("row_fields = [{}]\n", fields.join(", ")));
        out.push_str(&format!(
            "detail_layout = {}\n",
            toml_quote(g.detail_layout.as_str())
        ));
        out.push_str(&format!("gap_hours = {}\n", g.gap_hours));
        if !self.colors.is_empty() {
            out.push_str("\n[colors]\n");
            let mut keys: Vec<_> = self.colors.keys().collect();
            keys.sort();
            for k in keys {
                out.push_str(&format!(
                    "{} = {}\n",
                    toml_key(k),
                    toml_quote(&self.colors[k])
                ));
            }
        }
        if !self.keys.is_empty() {
            out.push_str("\n[keys]\n");
            let mut keys: Vec<_> = self.keys.keys().collect();
            keys.sort();
            for k in keys {
                out.push_str(&format!(
                    "{} = {}\n",
                    toml_key(k),
                    toml_quote(&self.keys[k])
                ));
            }
        }
        out
    }
}

fn toml_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn toml_key(k: &str) -> String {
    if !k.is_empty()
        && k.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        k.to_string()
    } else {
        toml_quote(k)
    }
}

pub fn default_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("ghg")
        .join("ghg.toml")
}

pub fn expand_tilde(p: &str) -> PathBuf {
    if let Some(rest) = p.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(p)
}

fn parse_bool(v: &str) -> Option<bool> {
    match v.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" | "1" => Some(true),
        "false" | "no" | "off" | "0" => Some(false),
        _ => None,
    }
}

fn set_bool(target: &mut bool, key: &str, value: &str, warnings: &mut Vec<String>) {
    match parse_bool(value) {
        Some(b) => *target = b,
        None => warnings.push(format!("invalid boolean for `{key}`: {value}")),
    }
}

fn set_usize(target: &mut usize, key: &str, value: &str, warnings: &mut Vec<String>) {
    match value.trim().parse::<usize>() {
        Ok(n) => *target = n,
        Err(_) => warnings.push(format!("invalid number for `{key}`: {value}")),
    }
}

fn set_f64(target: &mut f64, key: &str, value: &str, warnings: &mut Vec<String>) {
    match value.trim().parse::<f64>() {
        Ok(n) if n.is_finite() => *target = n,
        _ => warnings.push(format!("invalid number for `{key}`: {value}")),
    }
}

fn set_enum<T>(
    target: &mut T,
    key: &str,
    value: &str,
    parse: impl Fn(&str) -> Option<T>,
    warnings: &mut Vec<String>,
) {
    match parse(value) {
        Some(v) => *target = v,
        None => warnings.push(format!("invalid value for `{key}`: {value}")),
    }
}

fn validate_date_format(fmt: &str) -> bool {
    use chrono::format::{Item, StrftimeItems};
    !StrftimeItems::new(fmt).any(|item| matches!(item, Item::Error))
}

fn apply_general(key: &str, value: &str, cfg: &mut Config, warnings: &mut Vec<String>) {
    let g = &mut cfg.general;
    match key {
        "theme" => g.theme = value.trim().to_string(),
        "limit" => set_usize(&mut g.limit, key, value, warnings),
        "all_branches" => set_bool(&mut g.all_branches, key, value, warnings),
        "relative_time" => set_bool(&mut g.relative_time, key, value, warnings),
        "date_format" => {
            let v = value.trim().to_string();
            if validate_date_format(&v) {
                g.date_format = v;
            } else {
                warnings.push(format!("invalid date_format: {v}"));
            }
        }
        "mouse" => set_bool(&mut g.mouse, key, value, warnings),
        "wave_height" => {
            let mut v = g.wave_height;
            set_f64(&mut v, key, value, warnings);
            g.wave_height = v.clamp(0.2, 0.85);
        }
        "wave_mode" => set_enum(&mut g.wave_mode, key, value, WaveMode::parse, warnings),
        "wave_axis" => set_enum(&mut g.wave_axis, key, value, WaveAxis::parse, warnings),
        "density_strip" => set_bool(&mut g.density_strip, key, value, warnings),
        "heat_scale" => set_enum(&mut g.heat_scale, key, value, HeatScale::parse, warnings),
        "heat_percentile" => {
            let mut v = g.heat_percentile;
            set_f64(&mut v, key, value, warnings);
            g.heat_percentile = v.clamp(50.0, 100.0);
        }
        "heat_gamma" => {
            let mut v = g.heat_gamma;
            set_f64(&mut v, key, value, warnings);
            g.heat_gamma = v.clamp(0.2, 5.0);
        }
        "glyphs" => set_enum(&mut g.glyphs, key, value, GlyphSet::parse, warnings),
        "bar_width" => {
            let mut v = g.bar_width;
            set_usize(&mut v, key, value, warnings);
            g.bar_width = v.clamp(4, 60);
        }
        "row_fields" => {
            let fields: Vec<String> = value
                .split(',')
                .map(|f| f.trim().to_ascii_lowercase())
                .filter(|f| !f.is_empty())
                .collect();
            let unknown: Vec<_> = fields
                .iter()
                .filter(|f| !ROW_FIELDS.contains(&f.as_str()))
                .collect();
            for u in unknown {
                warnings.push(format!("unknown row field `{u}`"));
            }
            let fields: Vec<String> = fields
                .into_iter()
                .filter(|f| ROW_FIELDS.contains(&f.as_str()))
                .collect();
            if !fields.iter().any(|f| f == "subject") {
                warnings.push("row_fields should include `subject`".into());
            }
            g.row_fields = fields;
        }
        "detail_layout" => set_enum(
            &mut g.detail_layout,
            key,
            value,
            DetailLayout::parse,
            warnings,
        ),
        "gap_hours" => match value.trim().parse::<i64>() {
            Ok(n) => g.gap_hours = n.clamp(0, 24 * 365),
            Err(_) => warnings.push(format!("invalid number for `{key}`: {value}")),
        },
        other => warnings.push(format!("unknown setting `{other}`")),
    }
}

fn first_line(s: &str) -> &str {
    s.lines().next().unwrap_or(s)
}

fn setting_from_value(v: &toml::Value) -> Option<String> {
    match v {
        toml::Value::String(s) => Some(s.clone()),
        toml::Value::Integer(i) => Some(i.to_string()),
        toml::Value::Float(f) => Some(f.to_string()),
        toml::Value::Boolean(b) => Some(b.to_string()),
        toml::Value::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(item.as_str()?.to_string());
            }
            Some(out.join(","))
        }
        _ => None,
    }
}

/// Parse a TOML config document into `cfg`, collecting warnings instead of failing.
pub fn parse_into(src: &str, cfg: &mut Config, warnings: &mut Vec<String>) {
    let table: toml::Table = match src.parse() {
        Ok(t) => t,
        Err(err) => {
            warnings.push(format!("TOML error: {}", first_line(&err.to_string())));
            return;
        }
    };
    for (section, value) in &table {
        match (section.as_str(), value) {
            ("general", toml::Value::Table(t)) => {
                for (key, v) in t {
                    match setting_from_value(v) {
                        Some(text) => apply_general(key, &text, cfg, warnings),
                        None => warnings.push(format!("invalid value for `{key}`: {v:?}")),
                    }
                }
            }
            ("colors" | "theme", toml::Value::Table(t)) => {
                for (key, v) in t {
                    match v.as_str() {
                        Some(text) => {
                            cfg.colors
                                .insert(key.to_ascii_lowercase(), text.to_string());
                        }
                        None => warnings.push(format!("colour `{key}` must be a string")),
                    }
                }
            }
            ("keys", toml::Value::Table(t)) => {
                for (key, v) in t {
                    match v.as_str() {
                        Some(text) => {
                            cfg.keys.insert(key.to_ascii_lowercase(), text.to_string());
                        }
                        None => warnings.push(format!("key binding `{key}` must be a string")),
                    }
                }
            }
            (other, toml::Value::Table(_)) => {
                warnings.push(format!("unknown section `[{other}]`"));
            }
            (key, _) => warnings.push(format!("unknown top-level key `{key}`")),
        }
    }
}

/// Parse a TOML theme file into colour overrides.
///
/// Accepts both
/// ```toml
/// [colors]
/// heat_high = "#fde047"
/// ```
/// and top-level colour keys.
pub fn parse_theme_overrides(
    src: &str,
    map: &mut HashMap<String, String>,
    warnings: &mut Vec<String>,
) {
    let table: toml::Table = match src.parse() {
        Ok(t) => t,
        Err(err) => {
            warnings.push(format!(
                "TOML error in theme file: {}",
                first_line(&err.to_string())
            ));
            return;
        }
    };
    for (key, value) in &table {
        if key == "colors" || key == "theme" {
            match value.as_table() {
                Some(t) => {
                    for (k, v) in t {
                        match v.as_str() {
                            Some(text) => {
                                map.insert(k.to_ascii_lowercase(), text.to_string());
                            }
                            None => warnings.push(format!("theme colour `{k}` must be a string")),
                        }
                    }
                }
                None => warnings.push(format!("`[{key}]` must be a table")),
            }
        } else if let Some(text) = value.as_str() {
            map.insert(key.to_ascii_lowercase(), text.to_string());
        } else if value.is_table() {
            warnings.push(format!("unknown section `[{key}]` in theme file"));
        } else {
            warnings.push(format!("theme colour `{key}` must be a string"));
        }
    }
}

pub struct Loaded {
    pub config: Config,
}

pub fn load(explicit: Option<&Path>, no_config: bool) -> Loaded {
    let mut cfg = Config::default();
    if no_config {
        return Loaded { config: cfg };
    }

    let env_path = std::env::var_os("GHG_CONFIG").map(PathBuf::from);
    let chosen = explicit.map(|p| p.to_path_buf()).or(env_path);
    let explicit_requested = chosen.is_some();
    let path = chosen.unwrap_or_else(default_config_path);

    if !path.exists() {
        if explicit_requested {
            cfg.warnings
                .push(format!("config file not found: {}", path.display()));
        } else if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
            let _ = std::fs::write(&path, DEFAULT_CONFIG);
        }
        cfg.path = if path.exists() { Some(path) } else { None };
    } else {
        match std::fs::read_to_string(&path) {
            Ok(src) => {
                let mut warnings = Vec::new();
                parse_into(&src, &mut cfg, &mut warnings);
                cfg.warnings.extend(warnings);
                cfg.path = Some(path.clone());
            }
            Err(err) => cfg
                .warnings
                .push(format!("cannot read {}: {err}", path.display())),
        }
    }

    let mut warnings = std::mem::take(&mut cfg.warnings);
    apply_settings(&mut cfg, &mut warnings);
    cfg.keymap.apply(&cfg.keys, &mut warnings);
    cfg.warnings = warnings;
    Loaded { config: cfg }
}

fn apply_settings(cfg: &mut Config, warnings: &mut Vec<String>) {
    // Clamp / validate derived values once more so programmatic construction is safe.
    cfg.general.wave_height = cfg.general.wave_height.clamp(0.2, 0.85);
    cfg.general.bar_width = cfg.general.bar_width.clamp(4, 60);
    cfg.general.heat_percentile = cfg.general.heat_percentile.clamp(50.0, 100.0);
    cfg.general.heat_gamma = cfg.general.heat_gamma.clamp(0.2, 5.0);
    if cfg.general.row_fields.is_empty() {
        cfg.general.row_fields = vec!["heat".into(), "bar".into(), "hash".into(), "subject".into()];
    }
    if !validate_date_format(&cfg.general.date_format) {
        warnings.push("invalid date_format, falling back to %Y-%m-%d".into());
        cfg.general.date_format = "%Y-%m-%d".into();
    }
    for (key, value) in &cfg.colors {
        let valid = if key.starts_with("heat")
            || key.ends_with("_bg")
            || key == "fg"
            || key == "dim"
            || key == "accent"
            || key == "ok"
            || key == "bad"
            || key == "warn"
            || key == "axis"
            || key == "border"
            || key == "added"
            || key == "removed"
            || key == "wave_peak"
        {
            Rgb::parse_hex(value).is_some()
        } else {
            false
        };
        if !valid {
            warnings.push(format!("invalid colour for `{key}`: {value}"));
        }
    }
}

pub const DEFAULT_CONFIG: &str = r##"# ghg — a waveform view of your git history
# TOML config. Colours are "#rrggbb" strings; arrays are TOML arrays.

[general]
# Built-in: inferno | magma | viridis | ice | mono | solarized
# Or a theme name found in ./themes/<name>.toml, or an absolute path.
theme = "inferno"

# Maximum commits to load (0 = everything).
limit = 5000

# Include commits from all branches.
all_branches = false

# Relative timestamps ("3d"); false uses date_format.
relative_time = true
date_format = "%Y-%m-%d"

# Mouse click / wheel support.
mouse = true

# Fraction of the body height given to the waveform (0.20 - 0.85).
wave_height = 0.55

# Waveform height source: volume | density | both
wave_mode = "volume"

# Horizontal mapping: blend (readable, default) | time (strict wall clock).
wave_axis = "blend"

# Show the commit-count sparkline under the waveform.
density_strip = true

# Change-volume mapping: log | linear. Height and colour share this value.
heat_scale = "log"

# Normalisation percentile (50 - 100). Keeps outliers from flattening the ramp.
heat_percentile = 95

# Ramp exponent (0.2 - 5.0). > 1 darkens small changes, < 1 lights them up.
heat_gamma = 1.0

# Sub-cell glyphs: eighth | block | ascii
glyphs = "eighth"

# Width of the per-commit heat bar, in columns (4 - 60).
bar_width = 20

# Which fields appear in the commit list, in order.
row_fields = ["heat", "bar", "hash", "refs", "subject", "author", "time", "diff"]

# Details pane placement: auto | side | bottom
detail_layout = "auto"

# Insert a gap separator when commits are this many hours apart (0 disables).
gap_hours = 12

# Colour overrides, applied on top of the selected theme.
# [colors]
# heat_low = "#1e1b4b"
# heat_1 = "#6d28d9"
# heat_2 = "#db2777"
# heat_3 = "#f97316"
# heat_high = "#fde047"
# fg = "#cbd5e1"
# dim = "#64748b"
# accent = "#38bdf8"
# ok = "#22c55e"
# bad = "#ef4444"
# warn = "#eab308"
# axis = "#3f4756"
# border = "#2b3240"
# selected_bg = "#1e2635"
# wave_peak = "#ffffff"

# Key bindings, using key names like "q", "?", "enter", "up", "page_down", "+".
# [keys]
# zoom_in = "+"
# zoom_out = "-"
# toggle_detail = "enter"
# view_commit = "v"
# toggle_list = "L"
# reload = "R"
# quit = "q"
"##;

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(src: &str) -> (Config, Vec<String>) {
        let mut cfg = Config::default();
        let mut warnings = Vec::new();
        parse_into(src, &mut cfg, &mut warnings);
        (cfg, warnings)
    }

    #[test]
    fn defaults_are_sane() {
        let cfg = Config::default();
        assert_eq!(cfg.general.theme, "inferno");
        assert_eq!(cfg.general.heat_percentile, 95.0);
        assert!(cfg.general.row_fields.iter().any(|f| f == "subject"));
        let g = cfg.general;
        assert!(g.wave_height > 0.0 && g.wave_height < 1.0);
    }

    #[test]
    fn parses_general_and_colors() {
        let (cfg, warnings) = parsed(
            r##"
            [general]
            theme = "ice"      # inline comment
            limit = 42
            wave_height = 0.7
            heat_scale = "linear"
            row_fields = ["hash", "subject", "diff"]

            [colors]
            heat_high = "#ff0000"
            accent = "#00ff00"
            "##,
        );
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(cfg.general.theme, "ice");
        assert_eq!(cfg.general.limit, 42);
        assert!((cfg.general.wave_height - 0.7).abs() < 1e-9);
        assert_eq!(cfg.general.heat_scale, HeatScale::Linear);
        assert_eq!(cfg.general.row_fields, vec!["hash", "subject", "diff"]);
        assert_eq!(cfg.colors.get("heat_high").unwrap(), "#ff0000");
        assert_eq!(cfg.colors.get("accent").unwrap(), "#00ff00");
    }

    #[test]
    fn invalid_values_warn_but_do_not_fail() {
        let (cfg, warnings) = parsed(
            r#"
            [general]
            limit = "nope"
            wave_mode = "sideways"
            row_fields = ["hash", "bogus"]
            date_format = "%Q"
            unknown_key = 1

            [colors]
            accent = "not-a-colour"

            [keys]
            zoom_in = "++++"
            nope = "q"
            "#,
        );
        assert_eq!(cfg.general.limit, 5000);
        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| w.contains("limit")));
        assert!(warnings.iter().any(|w| w.contains("sideways")));
        assert!(warnings.iter().any(|w| w.contains("bogus")));
        assert!(warnings.iter().any(|w| w.contains("unknown_key")));
    }

    #[test]
    fn broken_toml_warns_and_keeps_defaults() {
        let (cfg, warnings) = parsed("[general\nlimit = 3\n");
        assert_eq!(cfg.general.limit, 5000);
        assert!(warnings.iter().any(|w| w.contains("TOML")), "{warnings:?}");
    }

    #[test]
    fn clamps_out_of_range_numbers() {
        let (cfg, _) =
            parsed("[general]\nwave_height = 9.0\nheat_percentile = 10.0\nbar_width = 500\n");
        assert!((cfg.general.wave_height - 0.85).abs() < 1e-9);
        assert_eq!(cfg.general.heat_percentile, 50.0);
        assert_eq!(cfg.general.bar_width, 60);
    }

    #[test]
    fn default_config_parses_without_warnings() {
        let (cfg, warnings) = parsed(DEFAULT_CONFIG);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(cfg.general.theme, "inferno");
        assert_eq!(cfg.general.limit, 5000);
    }

    #[test]
    fn keys_can_be_overridden() {
        let (mut cfg, mut warnings) = parsed("[keys]\nquit = \"x\"\nzoom_in = \"=\"\n");
        cfg.keymap.apply(&cfg.keys, &mut warnings);
        assert_eq!(cfg.keymap.action(KeyCode::Char('x')), Some(Action::Quit));
        assert_eq!(cfg.keymap.action(KeyCode::Char('q')), None);
        assert_eq!(cfg.keymap.action(KeyCode::Char('=')), Some(Action::ZoomIn));
    }

    #[test]
    fn dump_roundtrips() {
        let (cfg, _) = parsed(
            "[general]\ntheme = \"mono\"\nrow_fields = [\"hash\", \"subject\"]\n\n[colors]\naccent = \"#010203\"\n",
        );
        let dumped = cfg.dump();
        let (cfg2, warnings) = parsed(&dumped);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(cfg2.general.theme, "mono");
        assert_eq!(cfg2.general.row_fields, vec!["hash", "subject"]);
        assert_eq!(cfg2.colors.get("accent").unwrap(), "#010203");
    }

    #[test]
    fn dump_is_valid_toml() {
        let cfg = Config::default();
        let dumped = cfg.dump();
        let table: toml::Table = dumped.parse().expect("dump must parse as TOML");
        assert!(table.contains_key("general"));
    }

    #[test]
    fn theme_file_overrides_parse() {
        let mut map = HashMap::new();
        let mut warnings = Vec::new();
        parse_theme_overrides(
            "# comment\nheat_low = \"#000000\"\n\n[colors]\naccent = \"#123456\"\n",
            &mut map,
            &mut warnings,
        );
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(map.get("heat_low").unwrap(), "#000000");
        assert_eq!(map.get("accent").unwrap(), "#123456");
    }
}
