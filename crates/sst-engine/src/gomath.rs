//! Bit-exact ports of the Go `math` functions used by the game (Go 1.27,
//! `GOARCH=amd64`, `GOAMD64=v1`). On amd64 Go uses assembly for `Exp`, `Log`,
//! `Hypot`, `Floor`/`Ceil`/`Trunc` and `Max`/`Min`; those are replicated here.
//! Everything else is the pure-Go algorithm.
//!
//! `Exp` on amd64 picks an FMA code path at run time when the CPU has
//! AVX+FMA. [`exp`] uses the FMA path (what the reference machine runs, and
//! what `Pow` uses there); [`exp_no_fma`] is the other path.

#![allow(
    clippy::excessive_precision,
    clippy::approx_constant,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::manual_range_contains
)]

pub const PI: f64 = std::f64::consts::PI;
pub const E: f64 = std::f64::consts::E;
pub const SQRT2: f64 = std::f64::consts::SQRT_2;
pub const LN2: f64 = std::f64::consts::LN_2;
pub const MAX_FLOAT64: f64 = f64::MAX;
pub const SMALLEST_NONZERO_FLOAT64: f64 = 4.9406564584124654e-324;

const MASK: u64 = 0x7FF;
const SHIFT: u64 = 64 - 11 - 1;
const BIAS: i64 = 1023;

/// Go constant `4 / math.Pi` (folded exactly by the Go compiler).
pub const FOUR_OVER_PI: f64 = f64::from_bits(0x3ff45f306dc9c883);
/// Go constant `3 * math.Pi / 4`.
pub const THREE_PI_OVER_4: f64 = f64::from_bits(0x4002d97c7f3321d2);
/// Go constant `180.0 / math.Pi`.
pub const DEG_PER_RAD: f64 = f64::from_bits(0x404ca5dc1a63c1f8);

#[inline]
pub fn float64bits(f: f64) -> u64 {
    f.to_bits()
}

#[inline]
pub fn float64frombits(b: u64) -> f64 {
    f64::from_bits(b)
}

#[inline]
pub fn abs(x: f64) -> f64 {
    x.abs()
}

#[inline]
pub fn sqrt(x: f64) -> f64 {
    x.sqrt()
}

#[inline]
pub fn floor(x: f64) -> f64 {
    x.floor()
}

#[inline]
pub fn ceil(x: f64) -> f64 {
    x.ceil()
}

#[inline]
pub fn trunc(x: f64) -> f64 {
    x.trunc()
}

/// Go `math.Round` (half away from zero).
#[inline]
pub fn round(x: f64) -> f64 {
    x.round()
}

/// Go `int(x)` / `int64(x)` on amd64 (CVTTSD2SQ): truncation toward zero;
/// NaN and out-of-range values yield `i64::MIN` (Rust `as` would saturate).
#[inline]
pub fn go_int(x: f64) -> i64 {
    if x.is_nan() || x >= 9.223_372_036_854_775_808e18 || x < -9.223_372_036_854_775_808e18 {
        i64::MIN
    } else {
        x as i64
    }
}

/// Go `math.RoundToEven`.
#[inline]
pub fn round_to_even(x: f64) -> f64 {
    x.round_ties_even()
}

#[inline]
pub fn copysign(f: f64, sign: f64) -> f64 {
    f.copysign(sign)
}

#[inline]
pub fn signbit(x: f64) -> bool {
    x.is_sign_negative()
}

#[inline]
pub fn is_nan(x: f64) -> bool {
    x.is_nan()
}

/// Go `math.IsInf(f, sign)`.
pub fn is_inf(f: f64, sign: i32) -> bool {
    (sign >= 0 && f == f64::INFINITY) || (sign <= 0 && f == f64::NEG_INFINITY)
}

/// Go `math.Inf(sign)`.
pub fn inf(sign: i32) -> f64 {
    if sign >= 0 {
        f64::INFINITY
    } else {
        f64::NEG_INFINITY
    }
}

/// Go `math.NaN()` (bit pattern 0x7FF8000000000001).
pub fn nan() -> f64 {
    f64::from_bits(0x7FF8000000000001)
}

/// Go `math.Max`.
pub fn max(x: f64, y: f64) -> f64 {
    if is_inf(x, 1) || is_inf(y, 1) {
        return inf(1);
    }
    if x.is_nan() || y.is_nan() {
        return nan();
    }
    if x == 0.0 && x == y {
        if signbit(x) {
            return y;
        }
        return x;
    }
    if x > y {
        x
    } else {
        y
    }
}

/// Go `math.Min`.
pub fn min(x: f64, y: f64) -> f64 {
    if is_inf(x, -1) || is_inf(y, -1) {
        return inf(-1);
    }
    if x.is_nan() || y.is_nan() {
        return nan();
    }
    if x == 0.0 && x == y {
        if signbit(x) {
            return x;
        }
        return y;
    }
    if x < y {
        x
    } else {
        y
    }
}

/// Go builtin `max(a, b)` for floats (Go 1.21+): NaN-propagating, -0 < +0.
pub fn builtin_max(x: f64, y: f64) -> f64 {
    if x.is_nan() || y.is_nan() {
        return f64::NAN;
    }
    if x == 0.0 && y == 0.0 {
        if signbit(x) {
            return y;
        }
        return x;
    }
    if x > y {
        x
    } else {
        y
    }
}

fn normalize(x: f64) -> (f64, i64) {
    const SMALLEST_NORMAL: f64 = 2.2250738585072014e-308;
    if x.abs() < SMALLEST_NORMAL {
        return (x * (1u64 << 52) as f64, -52);
    }
    (x, 0)
}

/// Go `math.Frexp`.
pub fn frexp(f: f64) -> (f64, i64) {
    if f == 0.0 || f.is_infinite() || f.is_nan() {
        return (f, 0);
    }
    let (f, mut exp) = normalize(f);
    let mut x = f.to_bits();
    exp += ((x >> SHIFT) & MASK) as i64 - BIAS + 1;
    x &= !(MASK << SHIFT);
    x |= ((-1 + BIAS) as u64) << SHIFT;
    (f64::from_bits(x), exp)
}

/// Go `math.Ldexp`.
pub fn ldexp(frac: f64, exp: i64) -> f64 {
    if frac == 0.0 || frac.is_infinite() || frac.is_nan() {
        return frac;
    }
    let (frac, e) = normalize(frac);
    let mut exp = exp + e;
    let mut x = frac.to_bits();
    exp += ((x >> SHIFT) & MASK) as i64 - BIAS;
    if exp < -1075 {
        return copysign(0.0, frac);
    }
    if exp > 1023 {
        if frac < 0.0 {
            return inf(-1);
        }
        return inf(1);
    }
    let mut m = 1.0;
    if exp < -1022 {
        exp += 53;
        m = 1.0 / (1u64 << 53) as f64;
    }
    x &= !(MASK << SHIFT);
    x |= ((exp + BIAS) as u64) << SHIFT;
    m * f64::from_bits(x)
}

/// Go `math.Modf`.
pub fn modf(f: f64) -> (f64, f64) {
    let int = f.trunc();
    let frac = copysign(f - int, f);
    (int, frac)
}

/// Go `math.Mod`.
pub fn mod_(x: f64, y: f64) -> f64 {
    if y == 0.0 || x.is_infinite() || x.is_nan() || y.is_nan() {
        return nan();
    }
    let y = y.abs();
    let (yfr, yexp) = frexp(y);
    let mut r = x;
    if x < 0.0 {
        r = -x;
    }
    while r >= y {
        let (rfr, mut rexp) = frexp(r);
        if rfr < yfr {
            rexp -= 1;
        }
        r -= ldexp(y, rexp - yexp);
    }
    if x < 0.0 {
        r = -r;
    }
    r
}

/// Go `math.Hypot` (amd64 assembly: `max * sqrt(1 + (min/max)^2)`).
pub fn hypot(p: f64, q: f64) -> f64 {
    let pb = p.to_bits() & !(1u64 << 63);
    let qb = q.to_bits() & !(1u64 << 63);
    const POS_INF: u64 = 0x7FF0000000000000;
    if pb >= POS_INF || qb >= POS_INF {
        if pb == POS_INF || qb == POS_INF {
            return f64::INFINITY;
        }
        return nan();
    }
    if (pb | qb) == 0 {
        return 0.0;
    }
    let p = f64::from_bits(pb);
    let q = f64::from_bits(qb);
    // MAXSD X1, X0 / MINSD X2, X1 (operands are finite here)
    let x0 = if p > q { p } else { q };
    let x1 = if q < p { q } else { p };
    let mut t = x1 / x0;
    t *= t;
    t += 1.0;
    t = t.sqrt();
    t * x0
}

/// Pure-Go `math.Hypot` (used on non-amd64, e.g. the Go WASM build).
pub fn hypot_pure(p: f64, q: f64) -> f64 {
    let mut p = p.abs();
    let mut q = q.abs();
    if is_inf(p, 1) || is_inf(q, 1) {
        return inf(1);
    }
    if p.is_nan() || q.is_nan() {
        return nan();
    }
    if p < q {
        std::mem::swap(&mut p, &mut q);
    }
    if p == 0.0 {
        return 0.0;
    }
    q /= p;
    p * (1.0 + q * q).sqrt()
}

const SIN: [f64; 6] = [
    1.58962301576546568060e-10,
    -2.50507477628578072866e-8,
    2.75573136213857245213e-6,
    -1.98412698295895385996e-4,
    8.33333333332211858878e-3,
    -1.66666666666666307295e-1,
];
const COS: [f64; 6] = [
    -1.13585365213876817300e-11,
    2.08757008419747316778e-9,
    -2.75573141792967388112e-7,
    2.48015872888517045348e-5,
    -1.38888888888730564116e-3,
    4.16666666666665929218e-2,
];
const PI4A: f64 = 7.85398125648498535156e-1;
const PI4B: f64 = 3.77489470793079817668e-8;
const PI4C: f64 = 2.69515142907905952645e-15;
const REDUCE_THRESHOLD: f64 = (1u64 << 29) as f64;

const M_PI4: [u64; 20] = [
    0x0000000000000001,
    0x45f306dc9c882a53,
    0xf84eafa3ea69bb81,
    0xb6c52b3278872083,
    0xfca2c757bd778ac3,
    0x6e48dc74849ba5c0,
    0x0c925dd413a32439,
    0xfc3bd63962534e7d,
    0xd1046bea5d768909,
    0xd338e04d68befc82,
    0x7323ac7306a673e9,
    0x3908bf177bf25076,
    0x3ff12fffbc0b301f,
    0xde5e2316b414da3e,
    0xda6cfd9e4f96136e,
    0x9e8c7ecd3cbfd45a,
    0xea4f758fd7cbe2f6,
    0x7a0e73ef14a525d4,
    0xd7f6bf623f1aba10,
    0xac06608df8f6d757,
];

fn trig_reduce(x: f64) -> (u64, f64) {
    const PI4: f64 = PI / 4.0;
    if x < PI4 {
        return (0, x);
    }
    let mut ix = x.to_bits();
    let exp = ((ix >> SHIFT) & MASK) as i64 - BIAS - SHIFT as i64;
    ix &= !(MASK << SHIFT);
    ix |= 1 << SHIFT;
    let digit = ((exp + 61) / 64) as usize;
    let bitshift = ((exp + 61) % 64) as u32;
    let shl = |a: u64, s: u32| if s >= 64 { 0 } else { a << s };
    let shr = |a: u64, s: u32| if s >= 64 { 0 } else { a >> s };
    let z0 = shl(M_PI4[digit], bitshift) | shr(M_PI4[digit + 1], 64 - bitshift);
    let z1 = shl(M_PI4[digit + 1], bitshift) | shr(M_PI4[digit + 2], 64 - bitshift);
    let z2 = shl(M_PI4[digit + 2], bitshift) | shr(M_PI4[digit + 3], 64 - bitshift);
    let z2hi = (((z2 as u128) * (ix as u128)) >> 64) as u64;
    let m1 = (z1 as u128) * (ix as u128);
    let z1hi = (m1 >> 64) as u64;
    let z1lo = m1 as u64;
    let z0lo = z0.wrapping_mul(ix);
    let (lo, c) = z1lo.overflowing_add(z2hi);
    let hi = z0lo.wrapping_add(z1hi).wrapping_add(c as u64);
    let mut j = hi >> 61;
    let mut hi = (hi << 3) | (lo >> 61);
    let lz = hi.leading_zeros();
    let e = (BIAS as u64).wrapping_sub((lz + 1) as u64);
    hi = shl(hi, lz + 1) | shr(lo, 64 - (lz + 1));
    hi >>= 64 - SHIFT;
    hi |= e << SHIFT;
    let mut z = f64::from_bits(hi);
    if j & 1 == 1 {
        j += 1;
        j &= 7;
        z -= 1.0;
    }
    (j, z * PI4)
}

/// Go `math.Cos`.
pub fn cos(x: f64) -> f64 {
    if x.is_nan() || x.is_infinite() {
        return nan();
    }
    let mut sign = false;
    let x = x.abs();
    let mut j: u64;
    let mut y: f64;
    let z: f64;
    if x >= REDUCE_THRESHOLD {
        let r = trig_reduce(x);
        j = r.0;
        z = r.1;
    } else {
        j = (x * FOUR_OVER_PI) as u64;
        y = j as f64;
        if j & 1 == 1 {
            j += 1;
            y += 1.0;
        }
        j &= 7;
        z = ((x - y * PI4A) - y * PI4B) - y * PI4C;
    }
    if j > 3 {
        j -= 4;
        sign = !sign;
    }
    if j > 1 {
        sign = !sign;
    }
    let zz = z * z;
    if j == 1 || j == 2 {
        y = z + z
            * zz
            * ((((((SIN[0] * zz) + SIN[1]) * zz + SIN[2]) * zz + SIN[3]) * zz + SIN[4]) * zz
                + SIN[5]);
    } else {
        y = 1.0 - 0.5 * zz
            + zz * zz
                * ((((((COS[0] * zz) + COS[1]) * zz + COS[2]) * zz + COS[3]) * zz + COS[4]) * zz
                    + COS[5]);
    }
    if sign {
        y = -y;
    }
    y
}

/// Go `math.Sin`.
pub fn sin(x: f64) -> f64 {
    if x == 0.0 || x.is_nan() {
        return x;
    }
    if x.is_infinite() {
        return nan();
    }
    let mut sign = false;
    let mut x = x;
    if x < 0.0 {
        x = -x;
        sign = true;
    }
    let mut j: u64;
    let mut y: f64;
    let z: f64;
    if x >= REDUCE_THRESHOLD {
        let r = trig_reduce(x);
        j = r.0;
        z = r.1;
    } else {
        j = (x * FOUR_OVER_PI) as u64;
        y = j as f64;
        if j & 1 == 1 {
            j += 1;
            y += 1.0;
        }
        j &= 7;
        z = ((x - y * PI4A) - y * PI4B) - y * PI4C;
    }
    if j > 3 {
        sign = !sign;
        j -= 4;
    }
    let zz = z * z;
    if j == 1 || j == 2 {
        y = 1.0 - 0.5 * zz
            + zz * zz
                * ((((((COS[0] * zz) + COS[1]) * zz + COS[2]) * zz + COS[3]) * zz + COS[4]) * zz
                    + COS[5]);
    } else {
        y = z + z
            * zz
            * ((((((SIN[0] * zz) + SIN[1]) * zz + SIN[2]) * zz + SIN[3]) * zz + SIN[4]) * zz
                + SIN[5]);
    }
    if sign {
        y = -y;
    }
    y
}

/// Go `math.Sincos`.
pub fn sincos(x: f64) -> (f64, f64) {
    (sin(x), cos(x))
}

fn xatan(x: f64) -> f64 {
    const P0: f64 = -8.750608600031904122785e-01;
    const P1: f64 = -1.615753718733365076637e+01;
    const P2: f64 = -7.500855792314704667340e+01;
    const P3: f64 = -1.228866684490136173410e+02;
    const P4: f64 = -6.485021904942025371773e+01;
    const Q0: f64 = 2.485846490142306297962e+01;
    const Q1: f64 = 1.650270098316988542046e+02;
    const Q2: f64 = 4.328810604912902668951e+02;
    const Q3: f64 = 4.853903996359136964868e+02;
    const Q4: f64 = 1.945506571482613964425e+02;
    let mut z = x * x;
    z = z * ((((P0 * z + P1) * z + P2) * z + P3) * z + P4)
        / (((((z + Q0) * z + Q1) * z + Q2) * z + Q3) * z + Q4);
    x * z + x
}

fn satan(x: f64) -> f64 {
    const MOREBITS: f64 = 6.123233995736765886130e-17;
    const TAN3PIO8: f64 = 2.41421356237309504880;
    if x <= 0.66 {
        return xatan(x);
    }
    if x > TAN3PIO8 {
        return PI / 2.0 - xatan(1.0 / x) + MOREBITS;
    }
    PI / 4.0 + xatan((x - 1.0) / (x + 1.0)) + 0.5 * MOREBITS
}

/// Go `math.Atan`.
pub fn atan(x: f64) -> f64 {
    if x == 0.0 {
        return x;
    }
    if x > 0.0 {
        return satan(x);
    }
    -satan(-x)
}

/// Go `math.Atan2`.
pub fn atan2(y: f64, x: f64) -> f64 {
    if y.is_nan() || x.is_nan() {
        return nan();
    }
    if y == 0.0 {
        if x >= 0.0 && !signbit(x) {
            return copysign(0.0, y);
        }
        return copysign(PI, y);
    }
    if x == 0.0 {
        return copysign(PI / 2.0, y);
    }
    if x.is_infinite() {
        if is_inf(x, 1) {
            if y.is_infinite() {
                return copysign(PI / 4.0, y);
            }
            return copysign(0.0, y);
        }
        if y.is_infinite() {
            return copysign(THREE_PI_OVER_4, y);
        }
        return copysign(PI, y);
    }
    if y.is_infinite() {
        return copysign(PI / 2.0, y);
    }
    let q = atan(y / x);
    if x < 0.0 {
        if q <= 0.0 {
            return q + PI;
        }
        return q - PI;
    }
    q
}

/// Go `math.Asin`.
pub fn asin(x: f64) -> f64 {
    if x == 0.0 {
        return x;
    }
    let mut sign = false;
    let mut x = x;
    if x < 0.0 {
        x = -x;
        sign = true;
    }
    if x > 1.0 {
        return nan();
    }
    let mut temp = (1.0 - x * x).sqrt();
    if x > 0.7 {
        temp = PI / 2.0 - satan(temp / x);
    } else {
        temp = satan(x / temp);
    }
    if sign {
        temp = -temp;
    }
    temp
}

/// Go `math.Acos`.
pub fn acos(x: f64) -> f64 {
    PI / 2.0 - asin(x)
}

/// Go `math.Exp` as computed on the reference machine (amd64 assembly,
/// AVX+FMA path; verified against Go output).
pub fn exp(x: f64) -> f64 {
    exp_impl(x, true)
}

/// Go `math.Exp` (amd64 assembly, AVX+FMA path).
pub fn exp_fma(x: f64) -> f64 {
    exp_impl(x, true)
}

/// Go `math.Exp` (amd64 assembly, path for CPUs without FMA).
pub fn exp_no_fma(x: f64) -> f64 {
    exp_impl(x, false)
}

fn exp_impl(x: f64, fma: bool) -> f64 {
    const LOG2E: f64 = 1.4426950408889634073599246810018920;
    const LN2U: f64 = 0.69314718055966295651160180568695068359375;
    const LN2L: f64 = 0.28235290563031577122588448175013436025525412068e-12;
    const OVERFLOW: f64 = 7.09782712893384e+02;
    const D: [f64; 9] = [
        0.5,
        1.0,
        2.0,
        1.6666666666666666667e-1,
        4.1666666666666666667e-2,
        8.3333333333333333333e-3,
        1.3888888888888888889e-3,
        1.9841269841269841270e-4,
        2.4801587301587301587e-5,
    ];
    let bx = x.to_bits();
    let dx = bx & !(1u64 << 63);
    if dx >= 0x7FF0000000000000 {
        // not finite
        if bx == 0xFFF0000000000000 {
            return 0.0;
        }
        return x;
    }
    if x > OVERFLOW {
        return f64::INFINITY;
    }
    let t = LOG2E * x;
    // CVTSD2SL: round to nearest even, out of range -> 0x80000000
    let mut e: i32 = if t.is_nan() || !(-2147483648.5..2147483647.5).contains(&t) {
        i32::MIN
    } else {
        t.round_ties_even() as i32
    };
    let k = e as f64;
    let mut x0 = x;
    let mut x1: f64;
    if fma {
        x0 = (-LN2U).mul_add(k, x0);
        x0 = (-LN2L).mul_add(k, x0);
        x0 *= 0.0625;
        x1 = D[8];
        x1 = x1.mul_add(x0, D[7]);
        x1 = x1.mul_add(x0, D[6]);
        x1 = x1.mul_add(x0, D[5]);
        x1 = x1.mul_add(x0, D[4]);
        x1 = x1.mul_add(x0, D[3]);
        x1 = x1.mul_add(x0, D[0]);
        x1 = x1.mul_add(x0, D[1]);
        x0 *= x1;
        x1 = x0 + D[2];
        x0 *= x1;
        x1 = x0 + D[2];
        x0 *= x1;
        x1 = x0 + D[2];
        x0 *= x1;
        x1 = x0 + D[2];
        x0 = x0.mul_add(x1, D[1]);
    } else {
        x0 -= LN2U * k;
        x0 -= LN2L * k;
        x0 *= 0.0625;
        x1 = D[8] * x0;
        x1 += D[7];
        x1 *= x0;
        x1 += D[6];
        x1 *= x0;
        x1 += D[5];
        x1 *= x0;
        x1 += D[4];
        x1 *= x0;
        x1 += D[3];
        x1 *= x0;
        x1 += D[0];
        x1 *= x0;
        x1 += D[1];
        x0 *= x1;
        for _ in 0..4 {
            x1 = D[2] + x0;
            x0 *= x1;
        }
        x0 += D[1];
    }
    // ldexp
    e = e.wrapping_add(0x3FF);
    if e <= 0 {
        // denormal
        if e < -52 {
            return 0.0;
        }
        e += 0x3FE;
        let m = f64::from_bits((e as u64) << 52);
        x0 *= m;
        e = 1;
    } else if e >= 0x7FF {
        return f64::INFINITY;
    }
    let m = f64::from_bits((e as u64) << 52);
    x0 * m
}

/// Go `math.Log` (amd64 assembly).
pub fn log(x: f64) -> f64 {
    const HSQRT2: f64 = 7.07106781186547524401e-01;
    const LN2HI: f64 = 6.93147180369123816490e-01;
    const LN2LO: f64 = 1.90821492927058770002e-10;
    const L1: f64 = 6.666666666666735130e-01;
    const L2: f64 = 3.999999999940941908e-01;
    const L3: f64 = 2.857142874366239149e-01;
    const L4: f64 = 2.222219843214978396e-01;
    const L5: f64 = 1.818357216161805012e-01;
    const L6: f64 = 1.531383769920937332e-01;
    const L7: f64 = 1.479819860511658591e-01;
    let bx = x.to_bits();
    if bx & !(1u64 << 63) == 0 {
        return f64::NEG_INFINITY;
    }
    if (bx as i64) < 0 {
        return nan();
    }
    if bx >= 0x7FF0000000000000 {
        return x;
    }
    let mut f1 = f64::from_bits((bx & 0x000FFFFFFFFFFFFF) | 0x3FE0000000000000);
    let mut k = (((bx >> 52) & 0x7FF) as i32 - 0x3FE) as f64;
    // CMPSD X2, X0, 5 (not less than): HSqrt2 >= f1 ? -> mask
    let adj = if !(HSQRT2 < f1) { 1.0 } else { 0.0 };
    k -= adj;
    f1 *= adj + 1.0;
    let f = f1 - 1.0;
    let s = f / (2.0 + f);
    let s2 = s * s;
    let s4 = s2 * s2;
    let t1 = s2 * (((L7 * s4 + L5) * s4 + L3) * s4 + L1);
    let t2 = s4 * ((L6 * s4 + L4) * s4 + L2);
    let r = t1 + t2;
    let hfsq = 0.5 * f * f;
    let a = s * (hfsq + r) + k * LN2LO;
    let b = (hfsq - a) - f;
    k * LN2HI - b
}

/// Go `math.Log10`.
pub fn log10(x: f64) -> f64 {
    log(x) * f64::from_bits(0x3fdbcb7b1526e50e)
}

fn is_odd_int(x: f64) -> bool {
    if x.abs() >= (1u64 << 53) as f64 {
        return false;
    }
    let (xi, xf) = modf(x);
    xf == 0.0 && (xi as i64) & 1 == 1
}

/// Go `math.Pow`.
pub fn pow(x: f64, y: f64) -> f64 {
    if y == 0.0 || x == 1.0 {
        return 1.0;
    }
    if y == 1.0 {
        return x;
    }
    if x.is_nan() || y.is_nan() {
        return nan();
    }
    if x == 0.0 {
        if y < 0.0 {
            if signbit(x) && is_odd_int(y) {
                return inf(-1);
            }
            return inf(1);
        } else if y > 0.0 {
            if signbit(x) && is_odd_int(y) {
                return x;
            }
            return 0.0;
        }
    }
    if y.is_infinite() {
        if x == -1.0 {
            return 1.0;
        }
        if (x.abs() < 1.0) == is_inf(y, 1) {
            return 0.0;
        }
        return inf(1);
    }
    if x.is_infinite() {
        if is_inf(x, -1) {
            return pow(1.0 / x, -y);
        }
        if y < 0.0 {
            return 0.0;
        } else if y > 0.0 {
            return inf(1);
        }
    }
    if y == 0.5 {
        return x.sqrt();
    }
    if y == -0.5 {
        return 1.0 / x.sqrt();
    }
    let (mut yi, mut yf) = modf(y.abs());
    if yf != 0.0 && x < 0.0 {
        return nan();
    }
    if yi >= 9.223_372_036_854_775_808e18 {
        if x == -1.0 {
            return 1.0;
        }
        if (x.abs() < 1.0) == (y > 0.0) {
            return 0.0;
        }
        return inf(1);
    }
    let mut a1 = 1.0;
    let mut ae: i64 = 0;
    if yf != 0.0 {
        if yf > 0.5 {
            yf -= 1.0;
            yi += 1.0;
        }
        a1 = exp(yf * log(x));
    }
    let (mut x1, mut xe) = frexp(x);
    let mut i = yi as i64;
    while i != 0 {
        if xe < -(1 << 12) || (1 << 12) < xe {
            ae += xe;
            break;
        }
        if i & 1 == 1 {
            a1 *= x1;
            ae += xe;
        }
        x1 *= x1;
        xe <<= 1;
        if x1 < 0.5 {
            x1 += x1;
            xe -= 1;
        }
        i >>= 1;
    }
    if y < 0.0 {
        a1 = 1.0 / a1;
        ae = -ae;
    }
    ldexp(a1, ae)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_values() {
        assert_eq!(hypot(3.0, 4.0), 5.0);
        assert!((sin(1.0) - 1.0f64.sin()).abs() < 1e-15);
        assert!((cos(1.0) - 1.0f64.cos()).abs() < 1e-15);
        assert!((atan2(1.0, 2.0) - 1.0f64.atan2(2.0)).abs() < 1e-15);
        assert!((exp(1.0) - std::f64::consts::E).abs() < 1e-15);
        assert!((log(10.0) - 10.0f64.ln()).abs() < 1e-15);
        assert!((pow(0.8, 3.5) - 0.8f64.powf(3.5)).abs() < 1e-15);
        assert_eq!(mod_(7.5, 2.0), 1.5);
    }
}
