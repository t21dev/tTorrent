//! tt-theme tokens as GPUI colours and sizes. The UI reads every colour,
//! size and duration from here (AGENTS.md: never literals).

use gpui::{Hsla, Pixels, Rgba, px};
use tt_ipc::UiPrefs;
use tt_theme::{Accent, Density, Metrics, Mode, Palette, Tint};

#[derive(Clone, Copy)]
pub struct C {
    pub bg: Hsla,
    pub side: Hsla,
    pub panel: Hsla,
    pub raise: Hsla,
    pub well: Hsla,
    pub ink: Hsla,
    pub ink_2: Hsla,
    pub muted: Hsla,
    pub rule: Hsla,
    pub row_rule: Hsla,
    pub hover: Hsla,
    pub track: Hsla,
    pub ok: Hsla,
    pub warn: Hsla,
    pub bad: Hsla,
    pub accent: Hsla,
    pub accent_fg: Hsla,
    pub accent_ink: Hsla,
    pub accent_hover: Hsla,
    pub accent_soft: Hsla,
    pub accent_line: Hsla,
    pub shadow: Hsla,
    pub categories: [Hsla; 6],
}

fn h(c: tt_theme::Rgba) -> Hsla {
    Rgba {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
    .into()
}

impl From<Palette> for C {
    fn from(p: Palette) -> C {
        C {
            bg: h(p.bg),
            side: h(p.side),
            panel: h(p.panel),
            raise: h(p.raise),
            well: h(p.well),
            ink: h(p.ink),
            ink_2: h(p.ink_2),
            muted: h(p.muted),
            rule: h(p.rule),
            row_rule: h(p.row_rule),
            hover: h(p.hover),
            track: h(p.track),
            ok: h(p.ok),
            warn: h(p.warn),
            bad: h(p.bad),
            accent: h(p.accent),
            accent_fg: h(p.accent_fg),
            accent_ink: h(p.accent_ink),
            accent_hover: h(p.accent_hover),
            accent_soft: h(p.accent_soft),
            accent_line: h(p.accent_line),
            shadow: h(p.shadow),
            categories: p.categories.map(h),
        }
    }
}

/// The resolved theme: colours, sizes, and the UI scale applied to them.
#[derive(Clone, Copy)]
pub struct Theme {
    pub c: C,
    pub m: Metrics,
    pub mode: Mode,
    pub tint: Tint,
    pub scale: f32,
}

pub fn parse_mode(s: &str) -> Mode {
    if s == "dark" { Mode::Dark } else { Mode::Light }
}
pub fn parse_tint(s: &str) -> Tint {
    if s == "tinted" {
        Tint::Tinted
    } else {
        Tint::Neutral
    }
}
pub fn parse_density(s: &str) -> Density {
    match s {
        "compact" => Density::Compact,
        "comfortable" => Density::Comfortable,
        _ => Density::Cozy,
    }
}
pub fn parse_accent(s: &str) -> Accent {
    match s {
        "blue" => Accent::Blue,
        "violet" => Accent::Violet,
        "amber" => Accent::Amber,
        "rose" => Accent::Rose,
        "ink" => Accent::Ink,
        _ => Accent::Green,
    }
}

impl Theme {
    pub fn from_prefs(p: &UiPrefs) -> Theme {
        let (mode, tint, accent, density) = (
            parse_mode(&p.mode),
            parse_tint(&p.tint),
            parse_accent(&p.accent),
            parse_density(&p.density),
        );
        Theme {
            c: Palette::new(mode, tint, accent).into(),
            m: density.metrics(),
            mode,
            tint,
            scale: p.scale.clamp(0.8, 1.5),
        }
    }

    /// A token size in px at the current UI scale.
    pub fn px(&self, v: f32) -> Pixels {
        px((v * self.scale).round())
    }

    /// Like `px` without rounding (font sizes).
    pub fn fpx(&self, v: f32) -> Pixels {
        px(v * self.scale)
    }
}

pub const FONT_UI: &str = tt_theme::fonts::UI;
pub const FONT_NUM: &str = tt_theme::fonts::NUM;
pub const FONT_HEAD: &str = tt_theme::fonts::HEAD;
