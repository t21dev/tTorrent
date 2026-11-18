//! Small building blocks: icons, buttons, separators, segmented controls.

use crate::theme::{FONT_NUM, Theme};
use gpui::{
    BoxShadow, Div, FontWeight, Hsla, ParentElement, SharedString, Styled, Svg, div, point, px, svg,
};

pub fn icon(name: &'static str, size: f32, color: Hsla, t: &Theme) -> Svg {
    svg()
        .path(SharedString::from(format!("icons/{name}.svg")))
        .size(t.px(size))
        .flex_none()
        .text_color(color)
}

/// The two-layer shadow of floating surfaces (menus, sheets, toasts).
pub fn pop_shadow(t: &Theme) -> Vec<BoxShadow> {
    vec![
        BoxShadow {
            color: t.c.shadow.opacity(0.5),
            offset: point(px(0.), px(1.)),
            blur_radius: px(1.),
            spread_radius: px(0.),
        },
        BoxShadow {
            color: t.c.shadow,
            offset: point(px(8.), px(18.)),
            blur_radius: px(28.),
            spread_radius: px(-14.),
        },
    ]
}

/// A moulded button: 1 px top highlight, 1 px bottom edge (DESIGN.md › Shape).
pub fn moulded(t: &Theme) -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: t.c.rule,
        offset: point(px(0.), px(1.)),
        blur_radius: px(0.),
        spread_radius: px(0.),
    }]
}

pub fn kbd(text: impl Into<SharedString>, t: &Theme) -> Div {
    div()
        .px(t.px(5.))
        .py(t.px(1.))
        .rounded(t.px(5.))
        .border_1()
        .border_color(t.c.rule)
        .bg(t.c.raise)
        .font_family(FONT_NUM)
        .text_size(t.fpx(11.))
        .text_color(t.c.muted)
        .child(text.into())
}

pub fn section_label(text: &'static str, t: &Theme) -> Div {
    div()
        .px(t.px(10.))
        .pt(t.px(14.))
        .pb(t.px(4.))
        .text_size(t.fpx(12.))
        .text_color(t.c.muted)
        .font_weight(FontWeight::MEDIUM)
        .child(text)
}

pub fn hairline_v(t: &Theme) -> Div {
    div().w(px(1.)).h(t.px(18.)).bg(t.c.rule).flex_none()
}

/// A `ctrl` or `cmd` label for shortcuts shown in menus and sheets.
pub fn shortcut(s: &str) -> String {
    if cfg!(target_os = "macos") {
        s.replace("Ctrl+Shift+", "⇧⌘")
            .replace("Ctrl+Alt+", "⌥⌘")
            .replace("Ctrl+", "⌘")
            .replace("Shift+", "⇧")
    } else {
        s.to_owned()
    }
}

pub fn percent_bar(p: f32, fill: Hsla, t: &Theme) -> Div {
    let p = p.clamp(0.0, 1.0);
    div()
        .h(t.px(7.))
        .flex_1()
        .min_w(t.px(40.))
        .rounded_full()
        .bg(t.c.track)
        .overflow_hidden()
        .child(div().h_full().w(gpui::relative(p)).rounded_full().bg(fill))
}
