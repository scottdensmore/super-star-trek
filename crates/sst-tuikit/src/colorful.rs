//! The subset of `github.com/lucasb-eyer/go-colorful` v1.4.1 that termenv
//! uses: hex parsing, HSL, and the HSLuv distance used to degrade colours
//! (`colors.go`, `hsluv.go`). All floating point follows Go's operation order
//! and goes through [`crate::gomath`].

#![allow(clippy::excessive_precision)]

use crate::gomath;

/// A colour with components in `[0, 1]` (`colorful.Color`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

/// Go untyped constant `6.0/29.0*6.0/29.0*6.0/29.0` (folded exactly).
const LUV_EPS: f64 = f64::from_bits(0x3f822354d28f7cd6);
/// Go untyped constant `29.0/3.0*29.0/3.0*29.0/3.0`.
const LUV_KAPPA: f64 = f64::from_bits(0x408c3a5ed097b426);

const HSLUV_D65: [f64; 3] = [0.95045592705167, 1.0, 1.089057750759878];

const M: [[f64; 3]; 3] = [
    [
        3.2409699419045214,
        -1.5373831775700935,
        -0.49861076029300328,
    ],
    [
        -0.96924363628087983,
        1.8759675015077207,
        0.041555057407175613,
    ],
    [
        0.055630079696993609,
        -0.20397695888897657,
        1.0569715142428786,
    ],
];
const KAPPA: f64 = 903.2962962962963;
const EPSILON: f64 = 0.0088564516790356308;

fn sq(v: f64) -> f64 {
    v * v
}

fn clamp01(v: f64) -> f64 {
    // math.Max(0.0, math.Min(v, 1.0))
    gomath::max(0.0, gomath::min(v, 1.0))
}

/// Go `strconv.ParseUint(s, 16, bits)` for the short strings colorful passes.
fn parse_hex_uint(s: &str, bits: u32) -> Option<u64> {
    if s.is_empty() {
        return None;
    }
    let mut v: u64 = 0;
    for c in s.chars() {
        let d = c.to_digit(16)? as u64;
        v = v.checked_mul(16)?.checked_add(d)?;
    }
    if bits < 64 && v >= (1u64 << bits) {
        return None;
    }
    Some(v)
}

fn parse_hex_color(r: &str, g: &str, b: &str, bits: u32, factor: f64) -> Option<Color> {
    Some(Color {
        r: parse_hex_uint(r, bits)? as f64 * factor,
        g: parse_hex_uint(g, bits)? as f64 * factor,
        b: parse_hex_uint(b, bits)? as f64 * factor,
    })
}

/// `colorful.Hex`: parses "#rgb" or "#rrggbb". `None` is Go's error case.
pub fn hex(scol: &str) -> Option<Color> {
    let b = scol.as_bytes();
    if b.is_empty() || b[0] != b'#' {
        return None;
    }
    // Go slices by bytes; a non-ASCII byte makes ParseUint fail anyway.
    let sub = |a: usize, z: usize| scol.get(a..z);
    match b.len() {
        4 => parse_hex_color(sub(1, 2)?, sub(2, 3)?, sub(3, 4)?, 4, 1.0 / 15.0),
        7 => parse_hex_color(sub(1, 3)?, sub(3, 5)?, sub(5, 7)?, 8, 1.0 / 255.0),
        _ => None,
    }
}

impl Color {
    /// `Color.RGBA` (16-bit components, alpha 0xFFFF).
    pub fn rgba(&self) -> (u32, u32, u32, u32) {
        (
            (self.r * 65535.0 + 0.5) as u32,
            (self.g * 65535.0 + 0.5) as u32,
            (self.b * 65535.0 + 0.5) as u32,
            0xFFFF,
        )
    }

    /// `Color.Hex`.
    pub fn hex(&self) -> String {
        format!(
            "#{:02x}{:02x}{:02x}",
            (self.r * 255.0 + 0.5) as u8,
            (self.g * 255.0 + 0.5) as u8,
            (self.b * 255.0 + 0.5) as u8
        )
    }

    /// `Color.Hsl`.
    pub fn hsl(&self) -> (f64, f64, f64) {
        let min = gomath::min(gomath::min(self.r, self.g), self.b);
        let max = gomath::max(gomath::max(self.r, self.g), self.b);
        let l = (max + min) / 2.0;
        let (mut h, s);
        if min == max {
            s = 0.0;
            h = 0.0;
        } else {
            if l < 0.5 {
                s = (max - min) / (max + min);
            } else {
                s = (max - min) / (2.0 - max - min);
            }
            if max == self.r {
                h = (self.g - self.b) / (max - min);
            } else if max == self.g {
                h = 2.0 + (self.b - self.r) / (max - min);
            } else {
                h = 4.0 + (self.r - self.g) / (max - min);
            }
            h *= 60.0;
            if h < 0.0 {
                h += 360.0;
            }
        }
        (h, s, l)
    }

    fn linear_rgb(&self) -> (f64, f64, f64) {
        (linearize(self.r), linearize(self.g), linearize(self.b))
    }

    fn xyz(&self) -> (f64, f64, f64) {
        let (r, g, b) = self.linear_rgb();
        linear_rgb_to_xyz(r, g, b)
    }

    fn luv_white_ref(&self, wref: [f64; 3]) -> (f64, f64, f64) {
        let (x, y, z) = self.xyz();
        xyz_to_luv_white_ref(x, y, z, wref)
    }

    fn luv_lch_white_ref(&self, wref: [f64; 3]) -> (f64, f64, f64) {
        let (l, u, v) = self.luv_white_ref(wref);
        luv_to_luv_lch(l, u, v)
    }

    /// `Color.HSLuv`.
    pub fn hsluv(&self) -> (f64, f64, f64) {
        let (l, c, h) = self.luv_lch_white_ref(HSLUV_D65);
        luv_lch_to_hsluv(l, c, h)
    }

    /// `Color.DistanceHSLuv`.
    pub fn distance_hsluv(&self, c2: &Color) -> f64 {
        let (h1, s1, l1) = self.hsluv();
        let (h2, s2, l2) = c2.hsluv();
        (sq((h1 - h2) / 100.0) + sq(s1 - s2) + sq(l1 - l2)).sqrt()
    }
}

fn linearize(v: f64) -> f64 {
    if v <= 0.04045 {
        return v / 12.92;
    }
    gomath::pow((v + 0.055) / 1.055, 2.4)
}

fn linear_rgb_to_xyz(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let x = 0.41239079926595948 * r + 0.35758433938387796 * g + 0.18048078840183429 * b;
    let y = 0.21263900587151036 * r + 0.71516867876775593 * g + 0.072192315360733715 * b;
    let z = 0.019330818715591851 * r + 0.11919477979462599 * g + 0.95053215224966058 * b;
    (x, y, z)
}

fn xyz_to_uv(x: f64, y: f64, z: f64) -> (f64, f64) {
    let denom = x + 15.0 * y + 3.0 * z;
    if denom == 0.0 {
        (0.0, 0.0)
    } else {
        (4.0 * x / denom, 9.0 * y / denom)
    }
}

fn xyz_to_luv_white_ref(x: f64, y: f64, z: f64, wref: [f64; 3]) -> (f64, f64, f64) {
    let l = if y / wref[1] <= LUV_EPS {
        y / wref[1] * LUV_KAPPA / 100.0
    } else {
        1.16 * gomath::cbrt(y / wref[1]) - 0.16
    };
    let (ubis, vbis) = xyz_to_uv(x, y, z);
    let (un, vn) = xyz_to_uv(wref[0], wref[1], wref[2]);
    let u = 13.0 * l * (ubis - un);
    let v = 13.0 * l * (vbis - vn);
    (l, u, v)
}

fn luv_to_luv_lch(l_in: f64, u: f64, v: f64) -> (f64, f64, f64) {
    let h = if (v - u).abs() > 1e-4 && u.abs() > 1e-4 {
        gomath::mod_(57.29577951308232087721 * gomath::atan2(v, u) + 360.0, 360.0)
    } else {
        0.0
    };
    let c = (sq(u) + sq(v)).sqrt();
    (l_in, c, h)
}

fn luv_lch_to_hsluv(l: f64, c: f64, h: f64) -> (f64, f64, f64) {
    let c = c * 100.0;
    let l = l * 100.0;
    let s = if l > 99.9999999 || l < 0.00000001 {
        0.0
    } else {
        let max = max_chroma_for_lh(l, h);
        c / max * 100.0
    };
    (h, clamp01(s / 100.0), clamp01(l / 100.0))
}

fn max_chroma_for_lh(l: f64, h: f64) -> f64 {
    let h_rad = h / 360.0 * gomath::PI * 2.0;
    let mut min_length = f64::MAX;
    for line in get_bounds(l) {
        let length = length_of_ray_until_intersect(h_rad, line[0], line[1]);
        if length > 0.0 && length < min_length {
            min_length = length;
        }
    }
    min_length
}

fn get_bounds(l: f64) -> [[f64; 2]; 6] {
    let mut ret = [[0.0; 2]; 6];
    let sub1 = gomath::pow(l + 16.0, 3.0) / 1560896.0;
    let sub2 = if sub1 > EPSILON { sub1 } else { l / KAPPA };
    for (i, m) in M.iter().enumerate() {
        for k in 0..2 {
            let kf = k as f64;
            let top1 = (284517.0 * m[0] - 94839.0 * m[2]) * sub2;
            let top2 = (838422.0 * m[2] + 769860.0 * m[1] + 731718.0 * m[0]) * l * sub2
                - 769860.0 * kf * l;
            let bottom = (632260.0 * m[2] - 126452.0 * m[1]) * sub2 + 126452.0 * kf;
            ret[i * 2 + k][0] = top1 / bottom;
            ret[i * 2 + k][1] = top2 / bottom;
        }
    }
    ret
}

fn length_of_ray_until_intersect(theta: f64, x: f64, y: f64) -> f64 {
    y / (gomath::sin(theta) - x * gomath::cos(theta))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_parsing() {
        assert_eq!(hex("#ffffff").unwrap().rgba(), (65535, 65535, 65535, 65535));
        assert!(hex("ffffff").is_none());
        assert!(hex("#ggg").is_none());
        assert!(hex("#12345").is_none());
        assert_eq!(hex("#fff").unwrap().hex(), "#ffffff");
    }
}
