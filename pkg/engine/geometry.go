package engine

import (
	"math"
)

// Coord represents a 1-indexed coordinate [row, col] (1..8) within a quadrant or galaxy.
type Coord [2]int // 1-indexed: [row, col] (1..8)

// Distance calculates Euclidean distance between two coordinates.
func Distance(c1, c2 Coord) float64 {
	dr := float64(c1[0] - c2[0])
	dc := float64(c1[1] - c2[1])
	return math.Hypot(dr, dc)
}

// Bearing computes the angle in radians from one coordinate to another,
// where 0 radians points due east (increasing column) and pi/2 radians points due north (decreasing row).
func Bearing(from, to Coord) float64 {
	dr := float64(to[0] - from[0])
	dc := float64(to[1] - from[1])
	angle := math.Atan2(-dr, dc) // radians
	if angle < 0 {
		angle += 2 * math.Pi
	}
	return angle
}
