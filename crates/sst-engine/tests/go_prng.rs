//! Port of pkg/engine/prng_test.go.
#![allow(clippy::needless_range_loop)] // loops mirror the Go indexing

use sst_engine::PRNG;

#[test]
fn test_deterministic_prng() {
    let mut r1 = PRNG::new(12345);
    let mut r2 = PRNG::new(12345);
    for i in 0..100 {
        let (a, b) = (r1.float64(), r2.float64());
        assert_eq!(a, b, "Float64 mismatch at step {i}");
    }
}

#[test]
fn test_prng_intn_deterministic() {
    let mut r1 = PRNG::new(42);
    let mut r2 = PRNG::new(42);
    for i in 0..100 {
        let (a, b) = (r1.intn(10), r2.intn(10));
        assert_eq!(a, b, "Intn mismatch at step {i}");
        assert!((0..10).contains(&a), "Intn out of range: {a}");
    }
}

#[test]
fn test_prng_rand_real() {
    let mut r1 = PRNG::new(999);
    let mut r2 = PRNG::new(999);
    for i in 0..100 {
        let (a, b) = (r1.rand_real(), r2.rand_real());
        assert_eq!(a, b, "RandReal mismatch at step {i}");
        assert!((0.0..1.0).contains(&a), "RandReal out of range: {a}");
    }
}

#[test]
fn test_prng_different_seeds() {
    let mut r1 = PRNG::new(111);
    let mut r2 = PRNG::new(222);
    let mismatch = (0..10).any(|_| r1.float64() != r2.float64());
    assert!(
        mismatch,
        "expected different seeds to produce different sequences"
    );
}
