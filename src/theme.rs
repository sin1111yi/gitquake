use std::collections::HashMap;

use crate::config::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub fn parse_hex(s: &str) -> Option<Rgb> {
        let s = s.trim().trim_matches('"').trim();
        let s = s.strip_prefix('#').unwrap_or(s);
        if s.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&s[0..2], 16).ok()?;
        let g = u8::from_str_radix(&s[2..4], 16).ok()?;
        let b = u8::from_str_radix(&s[4..6], 16).ok()?;
        Some(Rgb(r, g, b))
    }

    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }

    /// Linear blend in sRGB space.
    pub fn blend(self, other: Rgb, t: f64) -> Rgb {
        let t = t.clamp(0.0, 1.0);
        let mix = |a: u8, b: u8| (a as f64 + (b as f64 - a as f64) * t).round() as u8;
        Rgb(
            mix(self.0, other.0),
            mix(self.1, other.1),
            mix(self.2, other.2),
        )
    }

    pub fn to_256(self) -> u8 {
        // Find the closest colour in the 6x6x6 cube or the grey ramp.
        const LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];
        let nearest = |v: u8| -> (usize, u8) {
            let mut best = (0usize, u8::MAX);
            let mut dist = i32::MAX;
            for (i, l) in LEVELS.iter().enumerate() {
                let d = (v as i32 - *l as i32).abs();
                if d < dist {
                    dist = d;
                    best = (i, *l);
                }
            }
            best
        };
        let (ri, rv) = nearest(self.0);
        let (gi, gv) = nearest(self.1);
        let (bi, bv) = nearest(self.2);
        let cube_idx = 16 + 36 * ri + 6 * gi + bi;
        let cube = Rgb(rv, gv, bv);
        // Grey ramp 232..=255, step 10 starting at 8.
        let grey_i = ((self.0 as i32 + self.1 as i32 + self.2 as i32) / 3 - 8).clamp(0, 23) as u8;
        let gv2 = 8 + grey_i as u16 * 10;
        let grey = Rgb(gv2 as u8, gv2 as u8, gv2 as u8);
        if dist2(self, grey) < dist2(self, cube) {
            232 + grey_i
        } else {
            cube_idx as u8
        }
    }

    pub fn to_16(self) -> u8 {
        const PALETTE: [Rgb; 16] = [
            Rgb(0, 0, 0),
            Rgb(128, 0, 0),
            Rgb(0, 128, 0),
            Rgb(128, 128, 0),
            Rgb(0, 0, 128),
            Rgb(128, 0, 128),
            Rgb(0, 128, 128),
            Rgb(192, 192, 192),
            Rgb(128, 128, 128),
            Rgb(255, 0, 0),
            Rgb(0, 255, 0),
            Rgb(255, 255, 0),
            Rgb(0, 0, 255),
            Rgb(255, 0, 255),
            Rgb(0, 255, 255),
            Rgb(255, 255, 255),
        ];
        let mut best = 7u8;
        let mut dist = i32::MAX;
        for (i, p) in PALETTE.iter().enumerate() {
            let d = dist2(self, *p);
            if d < dist {
                dist = d;
                best = i as u8;
            }
        }
        best
    }
}

fn dist2(a: Rgb, b: Rgb) -> i32 {
    let dr = a.0 as i32 - b.0 as i32;
    let dg = a.1 as i32 - b.1 as i32;
    let db = a.2 as i32 - b.2 as i32;
    dr * dr + dg * dg + db * db
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quality {
    True,
    Ansi256,
    Ansi16,
}

impl Quality {
    pub fn detect() -> Quality {
        if let Ok(ct) = std::env::var("COLORTERM") {
            let ct = ct.to_ascii_lowercase();
            if ct.contains("truecolor") || ct.contains("24bit") {
                return Quality::True;
            }
        }
        match std::env::var("TERM") {
            Ok(t) if t == "dumb" => Quality::Ansi16,
            Ok(t) if t.contains("256") => Quality::Ansi256,
            Ok(_) => Quality::Ansi256,
            Err(_) => Quality::Ansi16,
        }
    }

    pub fn to_ratatui(self, rgb: Rgb) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self {
            Quality::True => Color::Rgb(rgb.0, rgb.1, rgb.2),
            Quality::Ansi256 => Color::Indexed(rgb.to_256()),
            Quality::Ansi16 => Color::Indexed(rgb.to_16()),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Roles {
    pub fg: Rgb,
    pub dim: Rgb,
    pub accent: Rgb,
    pub ok: Rgb,
    pub bad: Rgb,
    pub warn: Rgb,
    pub axis: Rgb,
    pub border: Rgb,
    pub selected_bg: Rgb,
    pub wave_peak: Rgb,
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub ramp: [Rgb; 5],
    pub roles: Roles,
}

struct Spec {
    name: &'static str,
    ramp: [&'static str; 5],
    accent: &'static str,
}

const SPECS: &[Spec] = &[
    Spec {
        name: "inferno",
        ramp: ["#1e1b4b", "#6d28d9", "#db2777", "#f97316", "#fde047"],
        accent: "#22d3ee",
    },
    Spec {
        name: "magma",
        ramp: ["#180f3d", "#6b1d70", "#d13b62", "#fb8b3c", "#fcf7b6"],
        accent: "#a5b4fc",
    },
    Spec {
        name: "viridis",
        ramp: ["#2a1e5c", "#3b528b", "#21918c", "#5ec962", "#fde725"],
        accent: "#5eead4",
    },
    Spec {
        name: "ice",
        ramp: ["#0b2545", "#155e8a", "#3fa7d6", "#8ed9f0", "#eafcff"],
        accent: "#67e8f9",
    },
    Spec {
        name: "mono",
        ramp: ["#3f3f46", "#52525b", "#78787f", "#b4b4bb", "#f5f5f7"],
        accent: "#e4e4e7",
    },
    Spec {
        name: "solarized",
        ramp: ["#073642", "#268bd2", "#2aa198", "#b58900", "#fdf6e3"],
        accent: "#2aa198",
    },
];

pub const BUILTIN: &[&str] = &["inferno", "magma", "viridis", "ice", "mono", "solarized"];

fn base_roles(accent: Rgb) -> Roles {
    Roles {
        fg: Rgb(0xcb, 0xd5, 0xe1),
        dim: Rgb(0x64, 0x74, 0x8b),
        accent,
        ok: Rgb(0x22, 0xc5, 0x5e),
        bad: Rgb(0xef, 0x44, 0x44),
        warn: Rgb(0xea, 0xb3, 0x08),
        axis: Rgb(0x3f, 0x47, 0x56),
        border: Rgb(0x2b, 0x32, 0x40),
        selected_bg: Rgb(0x1e, 0x26, 0x35),
        wave_peak: Rgb(0xff, 0xff, 0xff),
    }
}

pub fn builtin(name: &str) -> Option<Theme> {
    let spec = SPECS.iter().find(|s| s.name.eq_ignore_ascii_case(name))?;
    let mut ramp = [Rgb(0, 0, 0); 5];
    for (i, hex) in spec.ramp.iter().enumerate() {
        ramp[i] = Rgb::parse_hex(hex).expect("builtin hex");
    }
    let accent = Rgb::parse_hex(spec.accent).expect("builtin hex");
    Some(Theme {
        name: spec.name.to_string(),
        ramp,
        roles: base_roles(accent),
    })
}

impl Theme {
    /// Interpolate the heat ramp. `t` is clamped to 0..=1.
    pub fn heat(&self, t: f64) -> Rgb {
        let t = if t.is_nan() { 0.0 } else { t.clamp(0.0, 1.0) };
        let pos = t * (self.ramp.len() - 1) as f64;
        let i = pos.floor() as usize;
        if i >= self.ramp.len() - 1 {
            return self.ramp[self.ramp.len() - 1];
        }
        self.ramp[i].blend(self.ramp[i + 1], pos - i as f64)
    }

    pub fn apply_overrides(&mut self, map: &HashMap<String, String>, warnings: &mut Vec<String>) {
        for (key, value) in map {
            let Some(rgb) = Rgb::parse_hex(value) else {
                warnings.push(format!("invalid colour for `{key}`: {value}"));
                continue;
            };
            match key.as_str() {
                "heat_low" => self.ramp[0] = rgb,
                "heat_1" => self.ramp[1] = rgb,
                "heat_2" => self.ramp[2] = rgb,
                "heat_3" => self.ramp[3] = rgb,
                "heat_high" => self.ramp[4] = rgb,
                "fg" => self.roles.fg = rgb,
                "dim" => self.roles.dim = rgb,
                "accent" => self.roles.accent = rgb,
                "ok" | "added" => self.roles.ok = rgb,
                "bad" | "removed" => self.roles.bad = rgb,
                "warn" => self.roles.warn = rgb,
                "axis" => self.roles.axis = rgb,
                "border" => self.roles.border = rgb,
                "selected_bg" => self.roles.selected_bg = rgb,
                "wave_peak" => self.roles.wave_peak = rgb,
                other => warnings.push(format!("unknown colour token `{other}`")),
            }
        }
    }

    pub fn gradient_preview(&self, width: usize) -> String {
        let glyphs = ['░', '▒', '▓', '█'];
        let mut out = String::new();
        for i in 0..width {
            let t = if width <= 1 {
                1.0
            } else {
                i as f64 / (width - 1) as f64
            };
            out.push(glyphs[((t * glyphs.len() as f64) as usize).min(glyphs.len() - 1)]);
        }
        out
    }
}

pub fn resolve(cfg: &Config, warnings: &mut Vec<String>) -> Theme {
    let name = cfg.general.theme.trim();
    let mut theme = builtin(name)
        .or_else(|| builtin("inferno"))
        .expect("inferno builtin");

    // A non-builtin name may point at a theme file.
    if builtin(name).is_none() {
        let path = cfg.theme_file(name);
        match std::fs::read_to_string(&path) {
            Ok(src) => {
                theme.name = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| name.to_string());
                let mut map = HashMap::new();
                crate::config::parse_theme_overrides(&src, &mut map, warnings);
                theme.apply_overrides(&map, warnings);
            }
            Err(_) => {
                warnings.push(format!(
                    "theme `{name}` not found ({}), using inferno",
                    path.display()
                ));
            }
        }
    }

    theme.apply_overrides(&cfg.colors, warnings);
    theme
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_roundtrip() {
        assert_eq!(Rgb::parse_hex("#ff8800"), Some(Rgb(255, 136, 0)));
        assert_eq!(Rgb::parse_hex("ff8800"), Some(Rgb(255, 136, 0)));
        assert_eq!(Rgb::parse_hex("#f80"), None);
        assert_eq!(Rgb::parse_hex("#gg0000"), None);
    }

    #[test]
    fn ramp_endpoints_and_monotonic() {
        let t = builtin("inferno").unwrap();
        assert_eq!(t.heat(0.0), t.ramp[0]);
        assert_eq!(t.heat(1.0), t.ramp[4]);
        let mut prev = -1;
        for i in 0..=10 {
            let v = t.heat(i as f64 / 10.0);
            let lum = v.0 as i32 + v.1 as i32 + v.2 as i32;
            assert!(lum >= prev, "ramp should not get darker: {v:?}");
            prev = lum;
        }
        assert_eq!(t.heat(-1.0), t.ramp[0]);
        assert_eq!(t.heat(9.0), t.ramp[4]);
    }

    #[test]
    fn all_builtin_themes_exist() {
        for name in BUILTIN {
            let t = builtin(name).unwrap_or_else(|| panic!("missing {name}"));
            assert_eq!(t.ramp.len(), 5);
        }
    }

    #[test]
    fn xterm_indices_in_range() {
        for c in [Rgb(0, 0, 0), Rgb(255, 255, 255), Rgb(120, 30, 200)] {
            assert!(c.to_256() >= 16);
            assert!(c.to_16() < 16);
        }
    }
}
