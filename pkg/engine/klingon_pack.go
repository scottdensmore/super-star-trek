package engine

import (
	"math"
)

// EventKlingonScreening signals that an escort raider moved into line-of-fire to shield its Commander.
type EventKlingonScreening struct {
	RaiderID    int
	CommanderID int
	Interposed  Coord
}

// EventType returns the type name for EventKlingonScreening.
func (e EventKlingonScreening) EventType() string { return "KlingonScreening" }

// DetectCrossfireBracket checks if 2 or more active Klingons bracket the Enterprise at an angle >= 60 degrees.
func DetectCrossfireBracket(g *GameState) bool {
	if g == nil || len(g.CurrentQuad.Enemies) < 2 {
		return false
	}
	var activeKlingons []*EnemyVessel
	for _, e := range g.CurrentQuad.Enemies {
		if e != nil && e.Faction == FactionKlingon && e.Energy > 0 {
			activeKlingons = append(activeKlingons, e)
		}
	}
	if len(activeKlingons) < 2 {
		return false
	}

	ent := g.Enterprise.Sector
	for i := 0; i < len(activeKlingons); i++ {
		for j := i + 1; j < len(activeKlingons); j++ {
			v1x := float64(activeKlingons[i].Sector[0] - ent[0])
			v1y := float64(activeKlingons[i].Sector[1] - ent[1])
			v2x := float64(activeKlingons[j].Sector[0] - ent[0])
			v2y := float64(activeKlingons[j].Sector[1] - ent[1])

			mag1 := math.Hypot(v1x, v1y)
			mag2 := math.Hypot(v2x, v2y)
			if mag1 == 0 || mag2 == 0 {
				continue
			}

			dot := v1x*v2x + v1y*v2y
			cosTheta := dot / (mag1 * mag2)
			if cosTheta > 1.0 {
				cosTheta = 1.0
			} else if cosTheta < -1.0 {
				cosTheta = -1.0
			}
			angle := math.Acos(cosTheta) * (180.0 / math.Pi)

			if angle >= 60.0 {
				return true
			}
		}
	}
	return false
}

// CalculatePackDamageMultiplier returns 1.35 if crossfire bracket is active, otherwise 1.0.
func CalculatePackDamageMultiplier(g *GameState) float64 {
	if DetectCrossfireBracket(g) {
		return 1.35
	}
	return 1.0
}

// ExecuteCommanderScreening moves eligible escort raiders into the torpedo line of fire to shield Commanders.
func ExecuteCommanderScreening(g *GameState) []Event {
	if g == nil {
		return nil
	}
	var events []Event
	ent := g.Enterprise.Sector
	screenedRaiders := make(map[int]bool)

	for _, cmd := range g.CurrentQuad.Enemies {
		if cmd == nil || !cmd.IsCommander || cmd.Faction != FactionKlingon {
			continue
		}
		if cmd.MaxEnergy > 0 && cmd.Energy <= 0 {
			continue
		}

		// Check if in direct horizontal or vertical line of fire with Enterprise
		if cmd.Sector[0] == ent[0] { // Same row
			startC, endC := ent[1], cmd.Sector[1]
			if startC > endC {
				startC, endC = endC, startC
			}

			// Find escort nearby that can step into (cmd.Sector[0], middle)
			targetC := int(math.Round(float64(startC+endC) / 2.0))
			targetCoord := Coord{cmd.Sector[0], targetC}

			for _, raider := range g.CurrentQuad.Enemies {
				if raider == nil || raider.IsCommander || raider.Faction != FactionKlingon || screenedRaiders[raider.ID] {
					continue
				}
				if raider.MaxEnergy > 0 && raider.Energy <= 0 {
					continue
				}
				if math.Abs(float64(raider.Sector[0]-targetCoord[0])) <= 1 &&
					math.Abs(float64(raider.Sector[1]-targetCoord[1])) <= 1 &&
					g.CurrentQuad.Grid[targetCoord[0]][targetCoord[1]] == EntityEmpty {

					g.CurrentQuad.Grid[raider.Sector[0]][raider.Sector[1]] = EntityEmpty
					g.CurrentQuad.Grid[targetCoord[0]][targetCoord[1]] = EntityKlingon
					raider.Sector = targetCoord
					screenedRaiders[raider.ID] = true

					events = append(events, EventKlingonScreening{
						RaiderID:    raider.ID,
						CommanderID: cmd.ID,
						Interposed:  targetCoord,
					})
					break
				}
			}
		} else if cmd.Sector[1] == ent[1] { // Same column
			startR, endR := ent[0], cmd.Sector[0]
			if startR > endR {
				startR, endR = endR, startR
			}

			// Find escort nearby that can step into (middle, cmd.Sector[1])
			targetR := int(math.Round(float64(startR+endR) / 2.0))
			targetCoord := Coord{targetR, cmd.Sector[1]}

			for _, raider := range g.CurrentQuad.Enemies {
				if raider == nil || raider.IsCommander || raider.Faction != FactionKlingon || screenedRaiders[raider.ID] {
					continue
				}
				if raider.MaxEnergy > 0 && raider.Energy <= 0 {
					continue
				}
				if math.Abs(float64(raider.Sector[0]-targetCoord[0])) <= 1 &&
					math.Abs(float64(raider.Sector[1]-targetCoord[1])) <= 1 &&
					g.CurrentQuad.Grid[targetCoord[0]][targetCoord[1]] == EntityEmpty {

					g.CurrentQuad.Grid[raider.Sector[0]][raider.Sector[1]] = EntityEmpty
					g.CurrentQuad.Grid[targetCoord[0]][targetCoord[1]] = EntityKlingon
					raider.Sector = targetCoord
					screenedRaiders[raider.ID] = true

					events = append(events, EventKlingonScreening{
						RaiderID:    raider.ID,
						CommanderID: cmd.ID,
						Interposed:  targetCoord,
					})
					break
				}
			}
		}
	}
	return events
}
