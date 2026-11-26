//! Deterministic PRNG (port of `pkg/engine/prng.go`), an exact
//! re-implementation of Go's `math/rand/v2` `PCG` source and the `rand.Rand`
//! methods built on top of it.

/// Go `math/rand/v2.PCG`: a 128-bit permuted congruential generator (DXSM output).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pcg {
    hi: u64,
    lo: u64,
}

impl Pcg {
    /// Go `rand.NewPCG(seed1, seed2)`.
    pub fn new(seed1: u64, seed2: u64) -> Self {
        Pcg {
            hi: seed1,
            lo: seed2,
        }
    }

    /// Go `PCG.Seed`.
    pub fn seed(&mut self, seed1: u64, seed2: u64) {
        self.hi = seed1;
        self.lo = seed2;
    }

    fn next(&mut self) -> (u64, u64) {
        const MUL_HI: u64 = 2549297995355413924;
        const MUL_LO: u64 = 4865540595714422341;
        const INC_HI: u64 = 6364136223846793005;
        const INC_LO: u64 = 1442695040888963407;
        let m = (self.lo as u128) * (MUL_LO as u128);
        let mut hi = (m >> 64) as u64;
        let lo = m as u64;
        hi = hi
            .wrapping_add(self.hi.wrapping_mul(MUL_LO))
            .wrapping_add(self.lo.wrapping_mul(MUL_HI));
        let (lo, c) = lo.overflowing_add(INC_LO);
        let hi = hi.wrapping_add(INC_HI).wrapping_add(c as u64);
        self.lo = lo;
        self.hi = hi;
        (hi, lo)
    }

    /// Internal state `(hi, lo)` (Go `PCG.MarshalBinary` payload).
    pub fn state(&self) -> (u64, u64) {
        (self.hi, self.lo)
    }

    /// Go `PCG.Uint64`.
    pub fn uint64(&mut self) -> u64 {
        let (mut hi, lo) = self.next();
        const CHEAP_MUL: u64 = 0xda942042e4dd58b5;
        hi ^= hi >> 32;
        hi = hi.wrapping_mul(CHEAP_MUL);
        hi ^= hi >> 48;
        hi = hi.wrapping_mul(lo | 1);
        hi
    }
}

/// Go `math/rand/v2.Rand` over a [`Pcg`] source (only the PCG source is used in this repo).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rand {
    pub src: Pcg,
}

impl Rand {
    pub fn new(src: Pcg) -> Self {
        Rand { src }
    }

    pub fn uint64(&mut self) -> u64 {
        self.src.uint64()
    }

    pub fn int64(&mut self) -> i64 {
        (self.src.uint64() & !(1u64 << 63)) as i64
    }

    pub fn uint32(&mut self) -> u32 {
        (self.src.uint64() >> 32) as u32
    }

    pub fn int32(&mut self) -> i32 {
        (self.src.uint64() >> 33) as i32
    }

    /// Go `Rand.Int` (64-bit `int`).
    pub fn int(&mut self) -> i64 {
        ((self.src.uint64() << 1) >> 1) as i64
    }

    fn uint64n(&mut self, n: u64) -> u64 {
        if n & n.wrapping_sub(1) == 0 {
            return self.uint64() & n.wrapping_sub(1);
        }
        let m = (self.uint64() as u128) * (n as u128);
        let mut hi = (m >> 64) as u64;
        let mut lo = m as u64;
        if lo < n {
            let thresh = n.wrapping_neg() % n;
            while lo < thresh {
                let m = (self.uint64() as u128) * (n as u128);
                hi = (m >> 64) as u64;
                lo = m as u64;
            }
        }
        hi
    }

    pub fn int64n(&mut self, n: i64) -> i64 {
        if n <= 0 {
            panic!("invalid argument to Int64N");
        }
        self.uint64n(n as u64) as i64
    }

    pub fn uint64n_checked(&mut self, n: u64) -> u64 {
        if n == 0 {
            panic!("invalid argument to Uint64N");
        }
        self.uint64n(n)
    }

    pub fn int32n(&mut self, n: i32) -> i32 {
        if n <= 0 {
            panic!("invalid argument to Int32N");
        }
        self.uint64n(n as u64) as i32
    }

    pub fn uint32n(&mut self, n: u32) -> u32 {
        if n == 0 {
            panic!("invalid argument to Uint32N");
        }
        self.uint64n(n as u64) as u32
    }

    /// Go `Rand.IntN` (`int` is 64-bit in Go).
    pub fn int_n(&mut self, n: i64) -> i64 {
        if n <= 0 {
            panic!("invalid argument to IntN");
        }
        self.uint64n(n as u64) as i64
    }

    pub fn float64(&mut self) -> f64 {
        ((self.uint64() << 11) >> 11) as f64 / (1u64 << 53) as f64
    }

    pub fn float32(&mut self) -> f32 {
        ((self.uint32() << 8) >> 8) as f32 / (1u32 << 24) as f32
    }

    pub fn shuffle(&mut self, n: i64, mut swap: impl FnMut(usize, usize)) {
        if n < 0 {
            panic!("invalid argument to Shuffle");
        }
        let mut i = n - 1;
        while i > 0 {
            let j = self.uint64n((i + 1) as u64) as usize;
            swap(i as usize, j);
            i -= 1;
        }
    }

    pub fn perm(&mut self, n: usize) -> Vec<i64> {
        let mut p: Vec<i64> = (0..n as i64).collect();
        self.shuffle(n as i64, |i, j| p.swap(i, j));
        p
    }
}

/// Go `engine.PRNG`.
///
/// `rng` is `None` for a PRNG that was decoded from JSON: Go's
/// `encoding/json` turns `"RNG": {}` into `&PRNG{}` whose unexported source is
/// nil, and any draw from it panics (nil pointer dereference). The Rust port
/// panics the same way.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PRNG {
    rng: Option<Rand>,
}

impl PRNG {
    /// Go `NewPRNG(seed)`: `rand.NewPCG(uint64(seed), uint64(seed^0x5DEECE66D))`.
    pub fn new(seed: i64) -> Self {
        let src = Pcg::new(seed as u64, (seed ^ 0x5DEECE66D) as u64);
        PRNG {
            rng: Some(Rand::new(src)),
        }
    }

    /// A PRNG with a nil source (what Go's JSON decoder produces).
    pub fn zero() -> Self {
        PRNG { rng: None }
    }

    /// True when the underlying source is nil (draws would panic).
    pub fn is_nil_source(&self) -> bool {
        self.rng.is_none()
    }

    fn r(&mut self) -> &mut Rand {
        self.rng
            .as_mut()
            .expect("runtime error: invalid memory address or nil pointer dereference")
    }

    /// Go `PRNG.Float64`.
    pub fn float64(&mut self) -> f64 {
        self.r().float64()
    }

    /// Go `PRNG.Intn`. Panics if `n <= 0`.
    pub fn intn(&mut self, n: i32) -> i32 {
        self.r().int_n(n as i64) as i32
    }

    /// Go `PRNG.RandReal`.
    pub fn rand_real(&mut self) -> f64 {
        self.r().float64()
    }

    /// PCG state `(hi, lo)`, or `None` for a nil source.
    pub fn state(&self) -> Option<(u64, u64)> {
        self.rng.as_ref().map(|r| r.src.state())
    }

    /// Access to the underlying `rand.Rand` (panics on a nil source).
    pub fn rand(&mut self) -> &mut Rand {
        self.r()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        let mut a = PRNG::new(12345);
        let mut b = PRNG::new(12345);
        for _ in 0..100 {
            assert_eq!(a.float64(), b.float64());
        }
    }
}
