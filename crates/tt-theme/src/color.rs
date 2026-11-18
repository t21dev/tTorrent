//! OKLCH colours and the maths the theme needs: conversion to sRGB, mixing in
//! Oklab, and WCAG contrast.

/// A colour in OKLCH: lightness 0..1, chroma, hue in degrees, alpha 0..1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Oklch {
    pub l: f32,
    pub c: f32,
    pub h: f32,
    pub a: f32,
}

/// Shorthand matching the CSS in DESIGN.md: `oklch(98.7% 0.002 260)` is `ok(98.7, 0.002, 260.0)`.
pub const fn ok(l_percent: f32, c: f32, h: f32) -> Oklch {
    Oklch {
        l: l_percent / 100.0,
        c,
        h,
        a: 1.0,
    }
}

/// An sRGB colour with straight alpha, each channel 0..1.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Rgba {
    /// `0xRRGGBBAA`.
    pub fn to_u32(self) -> u32 {
        let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u32;
        (q(self.r) << 24) | (q(self.g) << 16) | (q(self.b) << 8) | q(self.a)
    }

    pub fn with_alpha(self, a: f32) -> Self {
        Self { a, ..self }
    }

    /// WCAG relative luminance.
    pub fn luminance(self) -> f32 {
        let lin = |c: f32| {
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * lin(self.r) + 0.7152 * lin(self.g) + 0.0722 * lin(self.b)
    }

    /// Opaque colour of `self` drawn over `under` (alpha compositing).
    pub fn over(self, under: Rgba) -> Rgba {
        let a = self.a;
        Rgba {
            r: self.r * a + under.r * (1.0 - a),
            g: self.g * a + under.g * (1.0 - a),
            b: self.b * a + under.b * (1.0 - a),
            a: 1.0,
        }
    }
}

/// WCAG contrast ratio between two opaque colours (1..21).
pub fn contrast(a: Rgba, b: Rgba) -> f32 {
    let (la, lb) = (a.luminance(), b.luminance());
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

#[derive(Clone, Copy, Debug)]
struct Oklab {
    l: f32,
    a: f32,
    b: f32,
}

impl Oklch {
    fn to_oklab(self) -> Oklab {
        let h = self.h.to_radians();
        Oklab {
            l: self.l,
            a: self.c * h.cos(),
            b: self.c * h.sin(),
        }
    }

    /// Converted to sRGB, gamut-clipped by reducing chroma (hue and lightness kept).
    pub fn to_rgba(self) -> Rgba {
        let mut c = self.c;
        for _ in 0..24 {
            let (r, g, b) = oklab_to_linear(Oklch { c, ..self }.to_oklab());
            let eps = 2e-3;
            if (-eps..=1.0 + eps).contains(&r)
                && (-eps..=1.0 + eps).contains(&g)
                && (-eps..=1.0 + eps).contains(&b)
            {
                return Rgba {
                    r: encode(r),
                    g: encode(g),
                    b: encode(b),
                    a: self.a,
                };
            }
            c *= 0.9;
        }
        let (r, g, b) = oklab_to_linear(Oklch { c: 0.0, ..self }.to_oklab());
        Rgba {
            r: encode(r),
            g: encode(g),
            b: encode(b),
            a: self.a,
        }
    }

    pub fn with_l(self, l: f32) -> Self {
        Self { l, ..self }
    }

    pub fn alpha(self, a: f32) -> Self {
        Self { a, ..self }
    }
}

/// Mix two colours in Oklab, like CSS `color-mix(in oklab, a t, b)` with `t` the share of `a`.
pub fn mix(a: Oklch, b: Oklch, t: f32) -> Rgba {
    let (x, y) = (a.to_oklab(), b.to_oklab());
    let lab = Oklab {
        l: x.l * t + y.l * (1.0 - t),
        a: x.a * t + y.a * (1.0 - t),
        b: x.b * t + y.b * (1.0 - t),
    };
    let c = (lab.a * lab.a + lab.b * lab.b).sqrt();
    let h = lab.b.atan2(lab.a).to_degrees();
    Oklch {
        l: lab.l,
        c,
        h,
        a: a.a * t + b.a * (1.0 - t),
    }
    .to_rgba()
}

fn oklab_to_linear(lab: Oklab) -> (f32, f32, f32) {
    let l_ = lab.l + 0.396_337_78 * lab.a + 0.215_803_76 * lab.b;
    let m_ = lab.l - 0.105_561_346 * lab.a - 0.063_854_17 * lab.b;
    let s_ = lab.l - 0.089_484_18 * lab.a - 1.291_485_5 * lab.b;
    let (l, m, s) = (l_ * l_ * l_, m_ * m_ * m_, s_ * s_ * s_);
    (
        4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_94 * s,
        -1.268_438 * l + 2.609_757_4 * m - 0.341_319_38 * s,
        -0.004_196_086_3 * l - 0.703_418_6 * m + 1.707_614_7 * s,
    )
}

fn encode(c: f32) -> f32 {
    let c = c.clamp(0.0, 1.0);
    if c <= 0.003_130_8 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn white_and_black() {
        assert_eq!(ok(100.0, 0.0, 0.0).to_rgba().to_u32(), 0xffffffff);
        assert_eq!(ok(0.0, 0.0, 0.0).to_rgba().to_u32(), 0x000000ff);
    }

    #[test]
    fn known_colour() {
        // oklch(62.8% 0.2577 29.23) is CSS red.
        let c = ok(62.8, 0.2577, 29.23).to_rgba();
        assert!(
            (c.r - 1.0).abs() < 0.01 && c.g < 0.02 && c.b < 0.02,
            "{c:?}"
        );
    }

    #[test]
    fn contrast_extremes() {
        let w = ok(100.0, 0.0, 0.0).to_rgba();
        let b = ok(0.0, 0.0, 0.0).to_rgba();
        assert!((contrast(w, b) - 21.0).abs() < 0.05);
        assert!((contrast(w, w) - 1.0).abs() < 1e-4);
    }

    proptest::proptest! {
        #[test]
        fn always_in_gamut(l in 0.0f32..1.0, c in 0.0f32..0.4, h in 0.0f32..360.0) {
            let x = Oklch { l, c, h, a: 1.0 }.to_rgba();
            for v in [x.r, x.g, x.b] {
                proptest::prop_assert!((0.0..=1.0).contains(&v));
            }
        }
    }
}
