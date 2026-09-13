package galacticchart

import (
	"math"
	"testing"

	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

func TestCalculateTelemetry_CardinalDirections(t *testing.T) {
	from := engine.Coord{4, 4}

	tests := []struct {
		name       string
		to         engine.Coord
		wantDist   float64
		wantCourse float64
		wantName   string
		wantWarp   float64
	}{
		{"Same Quadrant", engine.Coord{4, 4}, 0.0, 0.0, "Current", 0.0},
		{"Due North", engine.Coord{2, 4}, 2.0, math.Pi / 2, "North", 2.0},
		{"Due East", engine.Coord{4, 7}, 3.0, 0.0, "East", 3.0},
		{"Due South", engine.Coord{7, 4}, 3.0, 3 * math.Pi / 2, "South", 3.0},
		{"Due West", engine.Coord{4, 1}, 3.0, math.Pi, "West", 3.0},
		{"North-East", engine.Coord{2, 6}, math.Sqrt(8), math.Pi / 4, "North-East", 2.8},
		{"North-West", engine.Coord{2, 2}, math.Sqrt(8), 3 * math.Pi / 4, "North-West", 2.8},
		{"South-West", engine.Coord{6, 2}, math.Sqrt(8), 5 * math.Pi / 4, "South-West", 2.8},
		{"South-East", engine.Coord{6, 6}, math.Sqrt(8), 7 * math.Pi / 4, "South-East", 2.8},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			res := CalculateTelemetry(from, tc.to)
			if math.Abs(res.Distance-tc.wantDist) > 0.05 {
				t.Errorf("Distance = %v, want %v", res.Distance, tc.wantDist)
			}
			if tc.to != from && math.Abs(res.Course-tc.wantCourse) > 0.05 {
				t.Errorf("Course = %v, want %v", res.Course, tc.wantCourse)
			}
			if res.Direction != tc.wantName {
				t.Errorf("Direction = %q, want %q", res.Direction, tc.wantName)
			}
			if math.Abs(res.RecommendedWarp-tc.wantWarp) > 0.05 {
				t.Errorf("RecommendedWarp = %v, want %v", res.RecommendedWarp, tc.wantWarp)
			}
		})
	}
}

func TestCalculateTelemetry_Fields(t *testing.T) {
	t.Run("Same quadrant fields", func(t *testing.T) {
		coord := engine.Coord{3, 5}
		res := CalculateTelemetry(coord, coord)
		if !res.IsCurrent {
			t.Errorf("IsCurrent = %v, want true", res.IsCurrent)
		}
		if res.From != coord || res.To != coord {
			t.Errorf("From=%v To=%v, want %v", res.From, res.To, coord)
		}
		if res.DeltaR != 0 || res.DeltaC != 0 {
			t.Errorf("DeltaR=%v DeltaC=%v, want 0, 0", res.DeltaR, res.DeltaC)
		}
		if res.Distance != 0.0 {
			t.Errorf("Distance = %v, want 0.0", res.Distance)
		}
		if res.RecommendedWarp != 0.0 {
			t.Errorf("RecommendedWarp = %v, want 0.0", res.RecommendedWarp)
		}
		if res.Direction != "Current" {
			t.Errorf("Direction = %q, want Current", res.Direction)
		}
	})

	t.Run("Remote quadrant fields and deltas", func(t *testing.T) {
		from := engine.Coord{1, 1}
		to := engine.Coord{2, 3}
		res := CalculateTelemetry(from, to)
		if res.IsCurrent {
			t.Errorf("IsCurrent = true, want false")
		}
		if res.From != from || res.To != to {
			t.Errorf("From=%v To=%v, want %v, %v", res.From, res.To, from, to)
		}
		if res.DeltaR != 1 || res.DeltaC != 2 {
			t.Errorf("DeltaR=%v DeltaC=%v, want 1, 2", res.DeltaR, res.DeltaC)
		}
		expectedDist := math.Sqrt(1 + 4)
		if math.Abs(res.Distance-expectedDist) > 0.001 {
			t.Errorf("Distance = %v, want %v", res.Distance, expectedDist)
		}
		expectedWarp := math.Round(expectedDist*10) / 10
		if res.RecommendedWarp != expectedWarp {
			t.Errorf("RecommendedWarp = %v, want %v", res.RecommendedWarp, expectedWarp)
		}
	})

	t.Run("Warp clamping minimum 1.0", func(t *testing.T) {
		from := engine.Coord{3, 3}
		to := engine.Coord{3, 4} // dist = 1.0
		res := CalculateTelemetry(from, to)
		if res.RecommendedWarp < 1.0 {
			t.Errorf("RecommendedWarp = %v, want at least 1.0", res.RecommendedWarp)
		}
	})
}
