package engine

import (
	"testing"
)

func TestDeterministicPRNG(t *testing.T) {
	rng1 := NewPRNG(12345)
	rng2 := NewPRNG(12345)

	for i := 0; i < 100; i++ {
		val1 := rng1.Float64()
		val2 := rng2.Float64()
		if val1 != val2 {
			t.Fatalf("Float64 mismatch at step %d: %f != %f", i, val1, val2)
		}
	}
}

func TestPRNGIntnDeterministic(t *testing.T) {
	rng1 := NewPRNG(42)
	rng2 := NewPRNG(42)

	for i := 0; i < 100; i++ {
		val1 := rng1.Intn(10)
		val2 := rng2.Intn(10)
		if val1 != val2 {
			t.Fatalf("Intn mismatch at step %d: %d != %d", i, val1, val2)
		}
		if val1 < 0 || val1 >= 10 {
			t.Fatalf("Intn out of range [0, 10): %d", val1)
		}
	}
}

func TestPRNGRandReal(t *testing.T) {
	rng1 := NewPRNG(999)
	rng2 := NewPRNG(999)

	for i := 0; i < 100; i++ {
		val1 := rng1.RandReal()
		val2 := rng2.RandReal()
		if val1 != val2 {
			t.Fatalf("RandReal mismatch at step %d: %f != %f", i, val1, val2)
		}
		if val1 < 0.0 || val1 >= 1.0 {
			t.Fatalf("RandReal out of range [0.0, 1.0): %f", val1)
		}
	}
}

func TestPRNGDifferentSeeds(t *testing.T) {
	rng1 := NewPRNG(111)
	rng2 := NewPRNG(222)

	mismatch := false
	for i := 0; i < 10; i++ {
		if rng1.Float64() != rng2.Float64() {
			mismatch = true
			break
		}
	}
	if !mismatch {
		t.Fatal("expected different seeds to produce different sequences")
	}
}
