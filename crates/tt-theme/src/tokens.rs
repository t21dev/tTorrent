use crate::color::{Oklch, Rgba, contrast, mix, ok};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Light,
    Dark,
}

/// Surface tint (DESIGN.md › Themes).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Tint {
    #[default]
    Neutral,
    Tinted,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase", tag = "kind")]
pub enum Accent {
    #[default]
    Green,
    Blue,
    Violet,
    Amber,
    Rose,
    Ink,
    /// A custom colour: its hue and chroma are kept, its lightness is adjusted per mode.
    Custom {
        l: f32,
        c: f32,
        h: f32,
    },
}

impl Accent {
    pub const PRESETS: [Accent; 6] = [
        Accent::Green,
        Accent::Blue,
        Accent::Violet,
        Accent::Amber,
        Accent::Rose,
        Accent::Ink,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Accent::Green => "Green",
            Accent::Blue => "Blue",
            Accent::Violet => "Violet",
            Accent::Amber => "Amber",
            Accent::Rose => "Rose",
            Accent::Ink => "Ink",
            Accent::Custom { .. } => "Custom",
        }
    }

    /// The base colour for a mode (presets from DESIGN.md › Accent).
    pub fn base(self, mode: Mode) -> Oklch {
        let (l, d) = match self {
            Accent::Green => (ok(50.0, 0.13, 160.0), ok(70.0, 0.11, 160.0)),
            Accent::Blue => (ok(52.0, 0.15, 250.0), ok(72.0, 0.12, 250.0)),
            Accent::Violet => (ok(50.0, 0.2, 292.0), ok(72.0, 0.15, 292.0)),
            Accent::Amber => (ok(57.0, 0.14, 62.0), ok(78.0, 0.13, 70.0)),
            Accent::Rose => (ok(54.0, 0.18, 15.0), ok(72.0, 0.15, 15.0)),
            Accent::Ink => (ok(28.0, 0.02, 170.0), ok(90.0, 0.012, 160.0)),
            Accent::Custom { l, c, h } => {
                let base = Oklch { l, c, h, a: 1.0 };
                (base, base)
            }
        };
        match mode {
            Mode::Light => l,
            Mode::Dark => d,
        }
    }
}

/// Every colour the window uses, resolved to sRGB for one theme.
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub mode: Mode,
    pub bg: Rgba,
    pub side: Rgba,
    pub panel: Rgba,
    pub raise: Rgba,
    pub well: Rgba,
    pub ink: Rgba,
    pub ink_2: Rgba,
    pub muted: Rgba,
    pub rule: Rgba,
    pub row_rule: Rgba,
    pub hover: Rgba,
    pub track: Rgba,
    pub ok: Rgba,
    pub warn: Rgba,
    pub bad: Rgba,
    /// Primary button, progress fills, switches (3:1 against `bg`).
    pub accent: Rgba,
    /// Accent used as text, focus rings, tab underline (4.5:1 against `bg`).
    pub accent_fg: Rgba,
    /// Text on an accent fill.
    pub accent_ink: Rgba,
    /// Primary button hover: accent mixed 88 % with ink.
    pub accent_hover: Rgba,
    /// Selection fill (accent at 15 %).
    pub accent_soft: Rgba,
    /// Partial pieces and focus borders (accent at 50 %).
    pub accent_line: Rgba,
    /// Shadow colour for floating surfaces.
    pub shadow: Rgba,
    /// Category colour squares.
    pub categories: [Rgba; 6],
}

struct Surface {
    bg: Oklch,
    side: Oklch,
    panel: Oklch,
    raise: Oklch,
    well: Oklch,
    ink: Oklch,
    ink_2: Oklch,
    muted: Oklch,
    rule: Oklch,
    row_rule: Oklch,
    hover: Oklch,
    track: Oklch,
    ok: Oklch,
    warn: Oklch,
    bad: Oklch,
}

fn surface(mode: Mode, tint: Tint) -> Surface {
    match (tint, mode) {
        (Tint::Neutral, Mode::Light) => Surface {
            bg: ok(98.7, 0.002, 260.0),
            side: ok(96.6, 0.003, 260.0),
            panel: ok(99.5, 0.001, 260.0),
            raise: ok(99.7, 0.001, 260.0),
            well: ok(96.4, 0.003, 260.0),
            ink: ok(20.0, 0.006, 260.0),
            ink_2: ok(30.0, 0.006, 260.0),
            muted: ok(48.0, 0.008, 260.0),
            rule: ok(88.5, 0.004, 260.0),
            row_rule: ok(93.5, 0.003, 260.0),
            hover: ok(95.2, 0.004, 260.0),
            track: ok(92.0, 0.004, 260.0),
            ok: ok(52.0, 0.12, 150.0),
            warn: ok(58.0, 0.13, 65.0),
            bad: ok(55.0, 0.18, 25.0),
        },
        (Tint::Neutral, Mode::Dark) => Surface {
            bg: ok(17.0, 0.003, 260.0),
            side: ok(19.5, 0.004, 260.0),
            panel: ok(20.5, 0.004, 260.0),
            raise: ok(24.0, 0.005, 260.0),
            well: ok(15.5, 0.003, 260.0),
            ink: ok(94.5, 0.003, 260.0),
            ink_2: ok(85.0, 0.004, 260.0),
            muted: ok(70.0, 0.006, 260.0),
            rule: ok(30.0, 0.005, 260.0),
            row_rule: ok(23.0, 0.004, 260.0),
            hover: ok(23.5, 0.005, 260.0),
            track: ok(28.0, 0.005, 260.0),
            ok: ok(76.0, 0.12, 150.0),
            warn: ok(80.0, 0.12, 70.0),
            bad: ok(72.0, 0.15, 25.0),
        },
        (Tint::Tinted, Mode::Light) => Surface {
            bg: ok(98.6, 0.006, 88.0),
            side: ok(96.8, 0.012, 120.0),
            panel: ok(99.4, 0.004, 88.0),
            raise: ok(99.7, 0.006, 95.0),
            well: ok(96.5, 0.01, 120.0),
            ink: ok(21.0, 0.025, 170.0),
            ink_2: ok(31.0, 0.02, 170.0),
            muted: ok(49.0, 0.024, 165.0),
            rule: ok(88.0, 0.02, 140.0),
            row_rule: ok(93.0, 0.014, 140.0),
            hover: ok(95.0, 0.02, 150.0),
            track: ok(92.0, 0.02, 150.0),
            ok: ok(52.0, 0.12, 150.0),
            warn: ok(58.0, 0.13, 65.0),
            bad: ok(55.0, 0.18, 25.0),
        },
        (Tint::Tinted, Mode::Dark) => Surface {
            bg: ok(17.0, 0.012, 165.0),
            side: ok(19.5, 0.014, 160.0),
            panel: ok(20.5, 0.014, 165.0),
            raise: ok(24.0, 0.016, 165.0),
            well: ok(15.5, 0.012, 165.0),
            ink: ok(94.0, 0.012, 160.0),
            ink_2: ok(85.0, 0.015, 165.0),
            muted: ok(71.0, 0.02, 165.0),
            rule: ok(30.0, 0.02, 160.0),
            row_rule: ok(23.0, 0.016, 160.0),
            hover: ok(23.5, 0.018, 160.0),
            track: ok(28.0, 0.02, 160.0),
            ok: ok(76.0, 0.12, 150.0),
            warn: ok(80.0, 0.12, 70.0),
            bad: ok(72.0, 0.15, 25.0),
        },
    }
}

/// Move `c`'s lightness away from `bg` until it reaches `ratio` against it
/// (Kitab's rule for custom accents). Hue and chroma are kept.
pub fn reach_contrast(c: Oklch, bg: Rgba, ratio: f32, mode: Mode) -> Oklch {
    let mut x = c;
    let step = match mode {
        Mode::Light => -0.01,
        Mode::Dark => 0.01,
    };
    for _ in 0..100 {
        if contrast(x.to_rgba(), bg) >= ratio {
            return x;
        }
        x = x.with_l((x.l + step).clamp(0.0, 1.0));
    }
    x
}

impl Palette {
    pub fn new(mode: Mode, tint: Tint, accent: Accent) -> Palette {
        let s = surface(mode, tint);
        let bg = s.bg.to_rgba();
        let accent_base = reach_contrast(accent.base(mode), bg, 3.0, mode);
        let accent_fg = reach_contrast(accent_base, bg, 4.5, mode);
        // Text on the accent fill: whichever of near-white and near-black reads better,
        // then the fill moves away from it until the label reaches 4.5:1.
        let light_ink = ok(99.0, 0.006, accent_base.h).to_rgba();
        let dark_ink = s.bg.with_l(0.17).to_rgba();
        let base_rgba = accent_base.to_rgba();
        let light = contrast(light_ink, base_rgba) >= contrast(dark_ink, base_rgba);
        let accent_ink = if light { light_ink } else { dark_ink };
        let mut accent_base = accent_base;
        for _ in 0..40 {
            if contrast(accent_ink, accent_base.to_rgba()) >= 4.5 {
                break;
            }
            let dl = if light { -0.005 } else { 0.005 };
            accent_base = accent_base.with_l((accent_base.l + dl).clamp(0.0, 1.0));
        }
        let accent_rgba = accent_base.to_rgba();
        let cats = match mode {
            Mode::Light => [
                ok(70.0, 0.11, 22.0),
                ok(68.0, 0.11, 160.0),
                ok(68.0, 0.09, 230.0),
                ok(72.0, 0.12, 80.0),
                ok(64.0, 0.13, 300.0),
                ok(66.0, 0.06, 260.0),
            ],
            Mode::Dark => [
                ok(70.0, 0.08, 22.0),
                ok(72.0, 0.09, 160.0),
                ok(72.0, 0.07, 230.0),
                ok(76.0, 0.09, 80.0),
                ok(70.0, 0.1, 300.0),
                ok(70.0, 0.04, 260.0),
            ],
        };
        Palette {
            mode,
            bg,
            side: s.side.to_rgba(),
            panel: s.panel.to_rgba(),
            raise: s.raise.to_rgba(),
            well: s.well.to_rgba(),
            ink: s.ink.to_rgba(),
            ink_2: s.ink_2.to_rgba(),
            muted: s.muted.to_rgba(),
            rule: s.rule.to_rgba(),
            row_rule: s.row_rule.to_rgba(),
            hover: s.hover.to_rgba(),
            track: s.track.to_rgba(),
            ok: s.ok.to_rgba(),
            warn: s.warn.to_rgba(),
            bad: s.bad.to_rgba(),
            accent: accent_rgba,
            accent_fg: accent_fg.to_rgba(),
            accent_ink,
            accent_hover: mix(accent_base, s.ink, 0.88),
            accent_soft: accent_base.alpha(0.15).to_rgba(),
            accent_line: accent_base.alpha(0.5).to_rgba(),
            shadow: match mode {
                Mode::Light => ok(30.0, 0.01, 260.0).alpha(0.16).to_rgba(),
                Mode::Dark => ok(5.0, 0.01, 260.0).alpha(0.7).to_rgba(),
            },
            categories: cats.map(|c| c.to_rgba()),
        }
    }
}

/// Row, toolbar, button and sidebar sizes (DESIGN.md › Density and scale).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Density {
    Compact,
    #[default]
    Cozy,
    Comfortable,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Metrics {
    pub row: f32,
    pub toolbar: f32,
    pub button: f32,
    pub sidebar: f32,
    pub ui_font: f32,
    pub num_font: f32,
    pub head_font: f32,
    pub small_font: f32,
    pub status_bar: f32,
    pub tab_bar: f32,
    pub title_bar: f32,
    pub header: f32,
    pub sidebar_item: f32,
    pub icon: f32,
}

impl Density {
    pub const ALL: [Density; 3] = [Density::Compact, Density::Cozy, Density::Comfortable];

    pub fn name(self) -> &'static str {
        match self {
            Density::Compact => "Compact",
            Density::Cozy => "Cozy",
            Density::Comfortable => "Comfortable",
        }
    }

    pub fn metrics(self) -> Metrics {
        let (row, toolbar, button, sidebar, ui_font, item) = match self {
            Density::Compact => (26.0, 38.0, 24.0, 196.0, 12.5, 24.0),
            Density::Cozy => (34.0, 44.0, 28.0, 216.0, 13.0, 28.0),
            Density::Comfortable => (42.0, 50.0, 32.0, 236.0, 13.5, 32.0),
        };
        Metrics {
            row,
            toolbar,
            button,
            sidebar,
            ui_font,
            num_font: 12.0,
            head_font: 16.0,
            small_font: 12.0,
            status_bar: 30.0,
            tab_bar: 40.0,
            title_bar: 34.0,
            header: 32.0,
            sidebar_item: item,
            icon: 16.0,
        }
    }
}

/// UI scale steps, independent of the OS scale.
pub const SCALES: [f32; 6] = [0.8, 0.9, 1.0, 1.1, 1.25, 1.5];

/// Next scale step up (`dir` > 0) or down; clamps at the ends.
pub fn step_scale(current: f32, dir: i32) -> f32 {
    let i = SCALES
        .iter()
        .position(|s| (s - current).abs() < 0.001)
        .unwrap_or(2) as i32;
    let j = (i + dir.signum()).clamp(0, SCALES.len() as i32 - 1) as usize;
    SCALES[j]
}

/// Corner radii in px (DESIGN.md › Shape and depth).
pub mod radius {
    pub const WINDOW: f32 = 12.0;
    pub const BUTTON: f32 = 10.0;
    pub const INPUT: f32 = 10.0;
    pub const ITEM: f32 = 8.0;
    pub const MENU: f32 = 12.0;
    pub const TOAST: f32 = 12.0;
    pub const PIECE: f32 = 2.0;
    pub const BAR: f32 = 999.0;
}

/// Durations in ms and easing control points (DESIGN.md › Motion).
pub mod motion {
    pub const DUR_MICRO_MS: u64 = 120;
    pub const DUR_SHORT_MS: u64 = 220;
    pub const DUR_LONG_MS: u64 = 420;
    pub const EASE_OUT: [f32; 4] = [0.16, 1.0, 0.3, 1.0];
    pub const EASE_SPRING: [f32; 4] = [0.34, 1.56, 0.64, 1.0];
}

/// Font families (DESIGN.md › Type).
pub mod fonts {
    pub const UI: &str = "Instrument Sans";
    pub const NUM: &str = "DM Mono";
    pub const HEAD: &str = "Schibsted Grotesk";
}

/// The piece map in Details › General has 48 columns.
pub const PIECE_COLUMNS: usize = 48;

#[cfg(test)]
mod tests {
    use super::*;

    fn all() -> Vec<Palette> {
        let mut v = Vec::new();
        for mode in [Mode::Light, Mode::Dark] {
            for tint in [Tint::Neutral, Tint::Tinted] {
                for accent in Accent::PRESETS {
                    v.push(Palette::new(mode, tint, accent));
                }
            }
        }
        v
    }

    /// TESTING.md › Accessibility: 4.5:1 text, 3:1 marks, in every theme, mode and accent.
    #[test]
    fn contrast_everywhere() {
        for p in all() {
            for (name, text) in [("ink", p.ink), ("ink_2", p.ink_2), ("muted", p.muted)] {
                for (sname, s) in [("bg", p.bg), ("side", p.side), ("panel", p.panel)] {
                    let r = contrast(text, s);
                    assert!(r >= 4.5, "{name} on {sname} {:?}: {r}", p.mode);
                }
            }
            assert!(contrast(p.accent, p.bg) >= 3.0, "accent mark {:?}", p.mode);
            assert!(
                contrast(p.accent_fg, p.bg) >= 4.5,
                "accent text {:?}",
                p.mode
            );
            assert!(
                contrast(p.accent_ink, p.accent) >= 4.5,
                "accent ink {:?} {}",
                p.mode,
                contrast(p.accent_ink, p.accent)
            );
            for s in [p.ok, p.warn, p.bad] {
                assert!(contrast(s, p.bg) >= 3.0, "status mark {:?}", p.mode);
            }
        }
    }

    #[test]
    fn custom_accent_reaches_contrast() {
        // A very light yellow on a light background must be darkened until it reads.
        let a = Accent::Custom {
            l: 0.95,
            c: 0.1,
            h: 100.0,
        };
        let p = Palette::new(Mode::Light, Tint::Neutral, a);
        assert!(contrast(p.accent, p.bg) >= 3.0);
        assert!(contrast(p.accent_fg, p.bg) >= 4.5);
    }

    #[test]
    fn scale_steps() {
        assert_eq!(step_scale(1.0, 1), 1.1);
        assert_eq!(step_scale(1.5, 1), 1.5);
        assert_eq!(step_scale(0.8, -1), 0.8);
    }

    #[test]
    fn density_table() {
        assert_eq!(Density::Cozy.metrics().row, 34.0);
        assert_eq!(Density::Compact.metrics().toolbar, 38.0);
        assert_eq!(Density::Comfortable.metrics().sidebar, 236.0);
    }
}
