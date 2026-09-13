package galacticchart

import (
	"math"

	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

type Telemetry struct {
	From            engine.Coord
	To              engine.Coord
	DeltaR          int
	DeltaC          int
	Distance        float64
	Course          float64
	Direction       string
	RecommendedWarp float64
	IsCurrent       bool
}

func CalculateTelemetry(from, to engine.Coord) Telemetry {
	if from == to {
		return Telemetry{
			From:            from,
			To:              to,
			Direction:       "Current",
			IsCurrent:       true,
			RecommendedWarp: 0.0,
		}
	}

	dr := to[0] - from[0]
	dc := to[1] - from[1]
	dist := math.Sqrt(float64(dr*dr + dc*dc))

	// In Super Star Trek geometry, dr is negative North, positive South:
	// dr = -sin(theta), dc = cos(theta) => theta = atan2(-dr, dc)
	angle := math.Atan2(float64(-dr), float64(dc))
	if angle < 0 {
		angle += 2 * math.Pi
	}

	dir := bearingDirection(angle)
	warp := math.Round(dist*10) / 10
	if warp < 1.0 {
		warp = 1.0
	}

	return Telemetry{
		From:            from,
		To:              to,
		DeltaR:          dr,
		DeltaC:          dc,
		Distance:        dist,
		Course:          angle,
		Direction:       dir,
		RecommendedWarp: warp,
		IsCurrent:       false,
	}
}

func bearingDirection(angle float64) string {
	deg := angle * 180.0 / math.Pi
	switch {
	case deg >= 337.5 || deg < 22.5:
		return "East"
	case deg >= 22.5 && deg < 67.5:
		return "North-East"
	case deg >= 67.5 && deg < 112.5:
		return "North"
	case deg >= 112.5 && deg < 157.5:
		return "North-West"
	case deg >= 157.5 && deg < 202.5:
		return "West"
	case deg >= 202.5 && deg < 247.5:
		return "South-West"
	case deg >= 247.5 && deg < 292.5:
		return "South"
	default:
		return "South-East"
	}
}
