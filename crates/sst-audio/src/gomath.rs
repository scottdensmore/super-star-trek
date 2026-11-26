//! Bit-exact ports of the Go `math` functions used by the WAV synthesizer
//! (`$GOROOT/src/math`: sin.go, trig_reduce.go, exp_amd64.s, log_amd64.s,
//! pow.go, frexp.go, ldexp.go, modf.go, bits.go).
//!
//! The parity reference is recorded on darwin/amd64, where Go dispatches
//! `math.Exp` and `math.Log` to the assembly implementations; those are
//! replicated here (including the FMA branch of `Exp`, selected by
//! [`EXP_USE_FMA`] to match the recording CPU). The pure-Go fallbacks used on
//! other architectures (e.g. Go's own wasm build) are provided as
//! [`exp_pure`] and [`log_pure`].

#![allow(
    clippy::excessive_precision,
    clippy::approx_constant,
    clippy::neg_cmp_op_on_partial_ord
)]

const SHIFT: u32 = 52;

/// Go's `math.NaN()` bit pattern.
#[inline]
fn nan() -> f64 {
    f64::from_bits(0x7FF8000000000001)
}

const MASK: u64 = 0x7FF;
const BIAS: i64 = 1023;

/// `math.Pi` rounded to float64.
pub const PI: f64 = std::f64::consts::PI;

/// Whether the reference machine took the AVX/FMA path in `exp_amd64.s`
/// (`useFMA = cpu.X86.HasAVX && cpu.X86.HasFMA`).
pub const EXP_USE_FMA: bool = true;

#[inline]
fn is_inf(x: f64, sign: i32) -> bool {
    (sign >= 0 && x == f64::INFINITY) || (sign <= 0 && x == f64::NEG_INFINITY)
}

/// Go `math.Abs`.
#[inline]
pub fn abs(x: f64) -> f64 {
    x.abs()
}

/// Go `math.Trunc`.
#[inline]
pub fn trunc(x: f64) -> f64 {
    x.trunc()
}

/// Go `math.Copysign`.
#[inline]
pub fn copysign(f: f64, sign: f64) -> f64 {
    f.copysign(sign)
}

/// Go `math.Modf`.
pub fn modf(f: f64) -> (f64, f64) {
    let integer = trunc(f);
    (integer, copysign(f - integer, f))
}

fn normalize(x: f64) -> (f64, i64) {
    const SMALLEST_NORMAL: f64 = 2.2250738585072014e-308;
    if abs(x) < SMALLEST_NORMAL {
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
        return if frac < 0.0 {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
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
/// Go constant `4 / Pi` rounded to float64.
const FOUR_OVER_PI: f64 = f64::from_bits(0x3ff45f306dc9c883);
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

#[inline]
fn mul64(a: u64, b: u64) -> (u64, u64) {
    let p = (a as u128) * (b as u128);
    ((p >> 64) as u64, p as u64)
}

/// Go `trigReduce` (Payne-Hanek reduction by Pi/4).
fn trig_reduce(x: f64) -> (u64, f64) {
    const PI4: f64 = PI / 4.0;
    if x < PI4 {
        return (0, x);
    }
    let mut ix = x.to_bits();
    let exp = ((ix >> SHIFT) & MASK) as i64 - BIAS - SHIFT as i64;
    ix &= !(MASK << SHIFT);
    ix |= 1 << SHIFT;
    let digit = ((exp + 61) as u64 / 64) as usize;
    let bitshift = ((exp + 61) as u64 % 64) as u32;
    // Go's shifts by >= 64 yield 0.
    let shr = |v: u64, s: u32| if s >= 64 { 0 } else { v >> s };
    let shl = |v: u64, s: u32| if s >= 64 { 0 } else { v << s };
    let z0 = shl(M_PI4[digit], bitshift) | shr(M_PI4[digit + 1], 64 - bitshift);
    let z1 = shl(M_PI4[digit + 1], bitshift) | shr(M_PI4[digit + 2], 64 - bitshift);
    let z2 = shl(M_PI4[digit + 2], bitshift) | shr(M_PI4[digit + 3], 64 - bitshift);
    let (z2hi, _) = mul64(z2, ix);
    let (z1hi, z1lo) = mul64(z1, ix);
    let z0lo = z0.wrapping_mul(ix);
    let (lo, c) = z1lo.overflowing_add(z2hi);
    let hi = z0lo.wrapping_add(z1hi).wrapping_add(c as u64);
    let mut j = hi >> 61;
    let mut hi = (hi << 3) | (lo >> 61);
    let lz = hi.leading_zeros();
    let e = (BIAS as u64).wrapping_sub(lz as u64 + 1);
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

#[inline]
fn sin_poly(z: f64, zz: f64) -> f64 {
    z + z
        * zz
        * ((((((SIN[0] * zz) + SIN[1]) * zz + SIN[2]) * zz + SIN[3]) * zz + SIN[4]) * zz + SIN[5])
}

#[inline]
fn cos_poly(zz: f64) -> f64 {
    1.0 - 0.5 * zz
        + zz * zz
            * ((((((COS[0] * zz) + COS[1]) * zz + COS[2]) * zz + COS[3]) * zz + COS[4]) * zz
                + COS[5])
}

#[inline]
fn reduce(x: f64) -> (u64, f64) {
    if x >= REDUCE_THRESHOLD {
        trig_reduce(x)
    } else {
        let mut j = (x * FOUR_OVER_PI) as u64;
        let mut y = j as f64;
        if j & 1 == 1 {
            j += 1;
            y += 1.0;
        }
        j &= 7;
        (j, ((x - y * PI4A) - y * PI4B) - y * PI4C)
    }
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
    let (mut j, z) = reduce(x);
    if j > 3 {
        sign = !sign;
        j -= 4;
    }
    let zz = z * z;
    let y = if j == 1 || j == 2 {
        cos_poly(zz)
    } else {
        sin_poly(z, zz)
    };
    if sign {
        -y
    } else {
        y
    }
}

/// Go `math.Cos`.
pub fn cos(x: f64) -> f64 {
    if x.is_nan() || x.is_infinite() {
        return nan();
    }
    let mut sign = false;
    let x = abs(x);
    let (mut j, z) = reduce(x);
    if j > 3 {
        j -= 4;
        sign = !sign;
    }
    if j > 1 {
        sign = !sign;
    }
    let zz = z * z;
    let y = if j == 1 || j == 2 {
        sin_poly(z, zz)
    } else {
        cos_poly(zz)
    };
    if sign {
        -y
    } else {
        y
    }
}

/// x86 `CVTSD2SI` (32-bit): round to nearest even; out of range -> i32::MIN.
#[inline]
fn cvtsd2sl(x: f64) -> i32 {
    let r = x.round_ties_even();
    if r.is_nan() || r >= 2147483648.0 || r < -2147483648.0 {
        i32::MIN
    } else {
        r as i32
    }
}

/// Go `math.Exp` as executed on amd64 (`exp_amd64.s`).
pub fn exp(x: f64) -> f64 {
    exp_amd64(x, EXP_USE_FMA)
}

/// `exp_amd64.s` with an explicit choice of the FMA or SSE2 code path.
pub fn exp_amd64(x: f64, use_fma: bool) -> f64 {
    const LOG2E: f64 = 1.4426950408889634073599246810018920;
    const LN2U: f64 = 0.69314718055966295651160180568695068359375;
    const LN2L: f64 = 0.28235290563031577122588448175013436025525412068e-12;
    const OVERFLOW: f64 = 7.09782712893384e+02;
    const C: [f64; 9] = [
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
    const POS_INF: u64 = 0x7FF0000000000000;
    const NEG_INF: u64 = 0xFFF0000000000000;

    let bx_bits = x.to_bits();
    if (bx_bits & !(1u64 << 63)) as i64 >= POS_INF as i64 {
        if bx_bits == NEG_INF {
            return 0.0;
        }
        return x;
    }
    if x > OVERFLOW {
        return f64::INFINITY;
    }
    let mut bx = cvtsd2sl(LOG2E * x);
    let k = bx as f64;
    let mut x0;
    if use_fma {
        x0 = (-LN2U).mul_add(k, x);
        x0 = (-LN2L).mul_add(k, x0);
        x0 *= 0.0625;
        let mut x1 = C[8];
        for c in [C[7], C[6], C[5], C[4], C[3], C[0], C[1]] {
            x1 = x1.mul_add(x0, c);
        }
        x0 *= x1;
        for _ in 0..3 {
            let t = x0 + C[2];
            x0 *= t;
        }
        let t = x0 + C[2];
        x0 = x0.mul_add(t, C[1]);
    } else {
        x0 = x - LN2U * k;
        x0 -= LN2L * k;
        x0 *= 0.0625;
        let mut x1 = C[8] * x0;
        for c in [C[7], C[6], C[5], C[4], C[3], C[0]] {
            x1 += c;
            x1 *= x0;
        }
        x1 += C[1];
        x0 *= x1;
        for _ in 0..4 {
            let t = C[2] + x0;
            x0 *= t;
        }
        x0 += C[1];
    }
    // ldexp
    bx = bx.wrapping_add(0x3FF);
    if bx <= 0 {
        // denormal
        if bx < -52 {
            return 0.0;
        }
        bx += 0x3FE;
        x0 *= f64::from_bits((bx as u32 as u64) << 52);
        bx = 1;
    } else if bx >= 0x7FF {
        return f64::INFINITY;
    }
    x0 * f64::from_bits((bx as u32 as u64) << 52)
}

/// Pure-Go `math.exp` (exp.go), used by Go on architectures without assembly.
pub fn exp_pure(x: f64) -> f64 {
    const LOG2E: f64 = 1.44269504088896338700e+00;
    const OVERFLOW: f64 = 7.09782712893383973096e+02;
    const UNDERFLOW: f64 = -7.45133219101941108420e+02;
    const NEAR_ZERO: f64 = 1.0 / (1u64 << 28) as f64;
    if x.is_nan() {
        return x;
    }
    if x > OVERFLOW {
        return f64::INFINITY;
    }
    if x < UNDERFLOW {
        return 0.0;
    }
    if -NEAR_ZERO < x && x < NEAR_ZERO {
        return 1.0 + x;
    }
    let k: i64 = if x < 0.0 {
        (LOG2E * x - 0.5) as i64
    } else if x > 0.0 {
        (LOG2E * x + 0.5) as i64
    } else {
        0
    };
    let hi = x - k as f64 * LN2HI;
    let lo = k as f64 * LN2LO;
    const P1: f64 = 1.66666666666666657415e-01;
    const P2: f64 = -2.77777777770155933842e-03;
    const P3: f64 = 6.61375632143793436117e-05;
    const P4: f64 = -1.65339022054652515390e-06;
    const P5: f64 = 4.13813679705723846039e-08;
    let r = hi - lo;
    let t = r * r;
    let c = r - t * (P1 + t * (P2 + t * (P3 + t * (P4 + t * P5))));
    let y = 1.0 - ((lo - (r * c) / (2.0 - c)) - hi);
    ldexp(y, k)
}

const LN2HI: f64 = 6.93147180369123816490e-01;
const LN2LO: f64 = 1.90821492927058770002e-10;
const L1: f64 = 6.666666666666735130e-01;
const L2: f64 = 3.999999999940941908e-01;
const L3: f64 = 2.857142874366239149e-01;
const L4: f64 = 2.222219843214978396e-01;
const L5: f64 = 1.818357216161805012e-01;
const L6: f64 = 1.531383769920937332e-01;
const L7: f64 = 1.479819860511658591e-01;

/// Go `math.Log` as executed on amd64 (`log_amd64.s`).
pub fn log(x: f64) -> f64 {
    const HSQRT2: f64 = 7.07106781186547524401e-01;
    let bx = x.to_bits();
    if bx & !(1u64 << 63) == 0 {
        return f64::NEG_INFINITY;
    }
    if (bx as i64) < 0 {
        return f64::from_bits(0x7FF8000000000001);
    }
    if bx >= 0x7FF0000000000000 {
        return x;
    }
    let mut f1 = f64::from_bits((bx & 0x000FFFFFFFFFFFFF) | 0.5f64.to_bits());
    let mut k = (((bx >> 52) & 0x7FF) as i32 - 0x3FE) as f64;
    // CMPSD not-less-than: HSqrt2 >= f1 (or unordered).
    let x3 = if !(HSQRT2 < f1) { 1.0 } else { 0.0 };
    k -= x3;
    f1 *= x3 + 1.0;
    let f = f1 - 1.0;
    let s = f / (2.0 + f);
    let s2 = s * s;
    let s4 = s2 * s2;
    let t1 = s2 * ((((L7 * s4 + L5) * s4) + L3) * s4 + L1);
    let t2 = s4 * ((L6 * s4 + L4) * s4 + L2);
    let r = t1 + t2;
    let hfsq = 0.5 * f * f;
    let x3 = s * (r + hfsq) + LN2LO * k;
    let x0 = (hfsq - x3) - f;
    k * LN2HI - x0
}

/// Pure-Go `math.log` (log.go).
pub fn log_pure(x: f64) -> f64 {
    if x.is_nan() || is_inf(x, 1) {
        return x;
    }
    if x < 0.0 {
        return nan();
    }
    if x == 0.0 {
        return f64::NEG_INFINITY;
    }
    let (mut f1, mut ki) = frexp(x);
    if f1 < std::f64::consts::SQRT_2 / 2.0 {
        f1 *= 2.0;
        ki -= 1;
    }
    let f = f1 - 1.0;
    let k = ki as f64;
    let s = f / (2.0 + f);
    let s2 = s * s;
    let s4 = s2 * s2;
    let t1 = s2 * (L1 + s4 * (L3 + s4 * (L5 + s4 * L7)));
    let t2 = s4 * (L2 + s4 * (L4 + s4 * L6));
    let r = t1 + t2;
    let hfsq = 0.5 * f * f;
    k * LN2HI - ((hfsq - (s * (hfsq + r) + k * LN2LO)) - f)
}

fn is_odd_int(x: f64) -> bool {
    if abs(x) >= (1u64 << 53) as f64 {
        return false;
    }
    let (xi, xf) = modf(x);
    xf == 0.0 && (xi as i64) & 1 == 1
}

/// Go `math.Pow` (pow.go; uses the amd64 `Exp`/`Log`).
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
            if x.is_sign_negative() && is_odd_int(y) {
                return f64::NEG_INFINITY;
            }
            return f64::INFINITY;
        } else if y > 0.0 {
            if x.is_sign_negative() && is_odd_int(y) {
                return x;
            }
            return 0.0;
        }
    } else if y.is_infinite() {
        if x == -1.0 {
            return 1.0;
        } else if (abs(x) < 1.0) == is_inf(y, 1) {
            return 0.0;
        } else {
            return f64::INFINITY;
        }
    } else if x.is_infinite() {
        if is_inf(x, -1) {
            return pow(1.0 / x, -y);
        }
        if y < 0.0 {
            return 0.0;
        } else if y > 0.0 {
            return f64::INFINITY;
        }
    } else if y == 0.5 {
        return x.sqrt();
    } else if y == -0.5 {
        return 1.0 / x.sqrt();
    }

    let (mut yi, mut yf) = modf(abs(y));
    if yf != 0.0 && x < 0.0 {
        return nan();
    }
    if yi >= 9.223_372_036_854_776e18 {
        if x == -1.0 {
            return 1.0;
        } else if (abs(x) < 1.0) == (y > 0.0) {
            return 0.0;
        } else {
            return f64::INFINITY;
        }
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
        if !(-(1 << 12)..=(1 << 12)).contains(&xe) {
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

    fn data() -> String {
        std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../testdata/parity/audio/gomath.txt"
        ))
        .unwrap()
    }

    fn hex(s: &str) -> u64 {
        u64::from_str_radix(s, 16).unwrap()
    }

    fn check(exp_fn: fn(f64) -> f64) -> (usize, Vec<String>) {
        let mut n = 0;
        let mut bad = Vec::new();
        for line in data().lines() {
            let p: Vec<&str> = line.split(' ').collect();
            let x = f64::from_bits(hex(p[1]));
            let (got, want) = match p[0] {
                "exp" => (exp_fn(x), hex(p[2])),
                "log" => (log(x), hex(p[2])),
                "sin" => (sin(x), hex(p[2])),
                "cos" => (cos(x), hex(p[2])),
                "pow" => (pow(x, f64::from_bits(hex(p[2]))), hex(p[3])),
                _ => unreachable!(),
            };
            n += 1;
            let ok = got.to_bits() == want;
            if !ok {
                bad.push(format!("{line} got {:016x}", got.to_bits()));
            }
        }
        (n, bad)
    }

    #[test]
    fn gomath_parity() {
        let (n, bad) = check(exp);
        assert!(n > 40000);
        assert!(
            bad.is_empty(),
            "{} mismatches, first: {:?}",
            bad.len(),
            &bad[..bad.len().min(10)]
        );
    }

    #[test]
    fn nan_bits_match_go_asm() {
        assert_eq!(log(-1.0).to_bits(), 0x7FF8000000000001);
    }
}
