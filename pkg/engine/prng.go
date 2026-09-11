package engine

import (
	"math/rand/v2"
)

// PRNG is a deterministic pseudo-random number generator wrapper around math/rand/v2.
type PRNG struct {
	src rand.Source
	rng *rand.Rand
}

// NewPRNG creates a new PRNG initialized with a deterministic PCG source.
func NewPRNG(seed int64) *PRNG {
	src := rand.NewPCG(uint64(seed), uint64(seed^0x5DEECE66D))
	return &PRNG{
		src: src,
		rng: rand.New(src),
	}
}

// Float64 returns a pseudo-random float64 in the half-open interval [0.0, 1.0).
func (p *PRNG) Float64() float64 {
	return p.rng.Float64()
}

// Intn returns a non-negative pseudo-random number in the half-open interval [0, n).
// It panics if n <= 0.
func (p *PRNG) Intn(n int) int {
	return p.rng.IntN(n)
}

// RandReal replicates the original C ranf() function, returning [0.0, 1.0).
func (p *PRNG) RandReal() float64 {
	return p.rng.Float64()
}
