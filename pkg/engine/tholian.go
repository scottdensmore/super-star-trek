package engine

// EventWebSegmentLaid signals that a Tholian spinner laid a web segment.
type EventWebSegmentLaid struct {
	SpinnerID int
	Coord     Coord
}

// EventType returns the type name for EventWebSegmentLaid.
func (e EventWebSegmentLaid) EventType() string { return "WebSegmentLaid" }

// EventWebBreached signals that an energy web segment was destroyed by weapons fire.
type EventWebBreached struct {
	Coord Coord
}

// EventType returns the type name for EventWebBreached.
func (e EventWebBreached) EventType() string { return "WebBreached" }

// EventWebCollapsed signals that all web segments dissipated because spinners were eliminated.
type EventWebCollapsed struct {
	SegmentsCount int
}

// EventType returns the type name for EventWebCollapsed.
func (e EventWebCollapsed) EventType() string { return "WebCollapsed" }

// TholianTurn executes perimeter movement and web weaving for all active Tholian spinners.
func TholianTurn(g *GameState) []Event {
	if g == nil {
		return nil
	}
	var events []Event
	var tholians []*EnemyVessel
	for _, e := range g.CurrentQuad.Enemies {
		if e.Faction == FactionTholian {
			tholians = append(tholians, e)
		}
	}

	// If no Tholians left, all web segments instantly collapse
	if len(tholians) == 0 {
		if len(g.CurrentQuad.WebSegments) > 0 {
			count := len(g.CurrentQuad.WebSegments)
			for _, ws := range g.CurrentQuad.WebSegments {
				if g.CurrentQuad.Grid[ws.Coord[0]][ws.Coord[1]] == EntityTholianWeb {
					g.CurrentQuad.Grid[ws.Coord[0]][ws.Coord[1]] = EntityEmpty
				}
			}
			g.CurrentQuad.WebSegments = nil
			return []Event{EventWebCollapsed{SegmentsCount: count}}
		}
		return nil
	}

	// Move spinners around perimeter and leave web in departed cells
	for _, t := range tholians {
		oldCoord := t.Sector
		// Clockwise patrol step
		dr, dc := getPerimeterStep(t.Sector, t.SpecialState)
		newR := t.Sector[0] + dr
		newC := t.Sector[1] + dc
		if newR < 1 {
			newR = 1
		} else if newR > 8 {
			newR = 8
		}
		if newC < 1 {
			newC = 1
		} else if newC > 8 {
			newC = 8
		}

		newCoord := Coord{newR, newC}
		if g.CurrentQuad.Grid[newCoord[0]][newCoord[1]] == EntityEmpty {
			g.CurrentQuad.Grid[oldCoord[0]][oldCoord[1]] = EntityTholianWeb
			g.CurrentQuad.WebSegments = append(g.CurrentQuad.WebSegments, &TholianWebSegment{
				Coord:    oldCoord,
				Strength: 250.0,
			})
			g.CurrentQuad.Grid[newCoord[0]][newCoord[1]] = EntityTholian
			t.Sector = newCoord

			events = append(events, EventWebSegmentLaid{
				SpinnerID: t.ID,
				Coord:     oldCoord,
			})
		}
	}

	return events
}

func getPerimeterStep(current Coord, phase int) (int, int) {
	if phase == 0 { // Alpha: clockwise
		if current[0] == 1 && current[1] < 8 {
			return 0, 1
		}
		if current[1] == 8 && current[0] < 8 {
			return 1, 0
		}
		if current[0] == 8 && current[1] > 1 {
			return 0, -1
		}
		return -1, 0
	}
	// Beta: counter-clockwise
	if current[0] == 1 && current[1] > 1 {
		return 0, -1
	}
	if current[1] == 1 && current[0] < 8 {
		return 1, 0
	}
	if current[0] == 8 && current[1] < 8 {
		return 0, 1
	}
	return -1, 0
}

// CalculateWebContainment computes the enclosure percentage of the web perimeter.
func CalculateWebContainment(g *GameState) float64 {
	if g == nil {
		return 0.0
	}
	count := len(g.CurrentQuad.WebSegments)
	// Full perimeter of an 8x8 quadrant is 28 cells
	containment := (float64(count) / 20.0) * 100.0
	if containment > 100.0 {
		containment = 100.0
	}
	return containment
}

// DamageWebSegment applies weapons damage to a web segment, destroying it if strength reaches 0.
func DamageWebSegment(g *GameState, coord Coord, damage float64) (bool, []Event) {
	if g == nil || damage <= 0 {
		return false, nil
	}
	for i, ws := range g.CurrentQuad.WebSegments {
		if ws.Coord == coord {
			ws.Strength -= damage
			if ws.Strength <= 0 {
				g.CurrentQuad.Grid[coord[0]][coord[1]] = EntityEmpty
				g.CurrentQuad.WebSegments = append(g.CurrentQuad.WebSegments[:i], g.CurrentQuad.WebSegments[i+1:]...)
				return true, []Event{EventWebBreached{Coord: coord}}
			}
			return false, nil
		}
	}
	return false, nil
}
