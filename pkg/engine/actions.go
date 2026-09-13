package engine

import (
	"errors"
	"math"
)

// Action represents an executable game command that mutates GameState and emits Events.
type Action interface {
	Execute(g *GameState) ([]Event, error)
}

// ActionShields transfers energy between Enterprise main power banks and defensive shields.
// Positive amount transfers energy to shields; negative amount transfers energy from shields to engines.
type ActionShields struct {
	Amount float64
}

// ActionTransferShields is an alias for ActionShields.
type ActionTransferShields = ActionShields

// Execute applies the shield transfer action to GameState.
func (a ActionShields) Execute(g *GameState) ([]Event, error) {
	if a.Amount > 0 && g.Enterprise.Energy < a.Amount {
		return nil, errors.New("insufficient energy for shield transfer")
	}
	if a.Amount < 0 && g.Enterprise.Shields < -a.Amount {
		return nil, errors.New("insufficient shield energy to transfer to engines")
	}

	g.Enterprise.Energy -= a.Amount
	g.Enterprise.Shields += a.Amount

	return []Event{
		EventShieldTransfer{
			NewShields: g.Enterprise.Shields,
			NewEnergy:  g.Enterprise.Energy,
		},
	}, nil
}

// ActionDock docks Enterprise with an adjacent Starbase, replenishing energy,
// photon torpedoes, lowering and securing shields, and repairing ship systems.
type ActionDock struct{}

// Execute applies the dock action to GameState.
func (a ActionDock) Execute(g *GameState) ([]Event, error) {
	if g.Enterprise.Condition == ConditionDocked {
		return nil, errors.New("already docked")
	}
	if g.CurrentQuad.Starbase == nil {
		return nil, errors.New("no starbase in current quadrant")
	}

	sb := *g.CurrentQuad.Starbase
	ent := g.Enterprise.Sector
	dr := ent[0] - sb[0]
	dc := ent[1] - sb[1]
	if dr < -1 || dr > 1 || dc < -1 || dc > 1 || (dr == 0 && dc == 0) {
		return nil, errors.New("enterprise not adjacent to starbase")
	}

	g.Enterprise.Condition = ConditionDocked
	if g.Enterprise.Energy < 5000 {
		g.Enterprise.Energy = 5000
	}
	g.Enterprise.Torpedoes = 10
	for i := range g.Enterprise.Devices {
		g.Enterprise.Devices[i] = 0
	}

	// Starbase surveillance download
	updatedQuads := 0
	for r := 1; r <= 8; r++ {
		for c := 1; c <= 8; c++ {
			if (g.GalaxyChart[r][c]%100)/10 > 0 {
				g.ChartKnownBases[r][c] = true
				for dr := -1; dr <= 1; dr++ {
					for dc := -1; dc <= 1; dc++ {
						nr := r + dr
						nc := c + dc
						if nr >= 1 && nr <= 8 && nc >= 1 && nc <= 8 {
							if !g.ChartDiscovered[nr][nc] {
								g.ChartDiscovered[nr][nc] = true
								updatedQuads++
							}
						}
					}
				}
			}
		}
	}

	return []Event{
		EventDocked{
			Starbase: sb,
		},
		EventStarbaseSurveillance{
			StarbaseCoord: sb,
			UpdatedQuads:  updatedQuads,
		},
	}, nil
}

// ActionFireTorpedo launches a photon torpedo along a given angle or bearing to target.
type ActionFireTorpedo struct {
	Target Coord
	Angle  float64
}

// Execute applies the photon torpedo firing action to GameState.
func (a ActionFireTorpedo) Execute(g *GameState) ([]Event, error) {
	if g.Enterprise.Torpedoes <= 0 {
		return nil, errors.New("no photon torpedoes left")
	}

	if g.Enterprise.Condition != ConditionDocked {
		g.Enterprise.Torpedoes--
	}

	angle := a.Angle
	if angle == 0 && a.Target != (Coord{}) {
		angle = Bearing(g.Enterprise.Sector, a.Target)
	}

	events := []Event{
		EventTorpedoFired{
			Origin: g.Enterprise.Sector,
			Angle:  angle,
		},
	}

	hitCoord, hitEntity, hit := TraceTorpedoPath(g.Enterprise.Sector, angle, &g.CurrentQuad)
	if !hit {
		return events, nil
	}

	damage := 500.0
	destroyed := false

	switch hitEntity {
	case EntityKlingon, EntityCommander, EntitySuperCommander:
		var targetKlingon *Klingon
		var targetIndex = -1
		for i, k := range g.CurrentQuad.Klingons {
			if k.Sector == hitCoord {
				targetKlingon = k
				targetIndex = i
				break
			}
		}

		if targetKlingon != nil {
			if damage >= targetKlingon.Energy {
				destroyed = true
				damage = targetKlingon.Energy
				targetKlingon.Energy = 0
				g.CurrentQuad.Klingons = append(g.CurrentQuad.Klingons[:targetIndex], g.CurrentQuad.Klingons[targetIndex+1:]...)
			} else {
				targetKlingon.Energy -= damage
			}
		} else {
			destroyed = true
		}

		if destroyed {
			g.CurrentQuad.Grid[hitCoord[0]][hitCoord[1]] = EntityEmpty
			if g.RemainingKlingons > 0 {
				g.RemainingKlingons--
			}
		}

	case EntityStar:
		damage = 0
		destroyed = false

	case EntityStarbase:
		damage = 500.0
		destroyed = true
		g.CurrentQuad.Starbase = nil
		g.CurrentQuad.Grid[hitCoord[0]][hitCoord[1]] = EntityEmpty
		if g.RemainingStarbases > 0 {
			g.RemainingStarbases--
		}

	default:
		damage = 0
		destroyed = false
	}

	events = append(events, EventTorpedoHit{
		Target:    hitCoord,
		Entity:    hitEntity,
		Damage:    damage,
		Destroyed: destroyed,
	})

	return events, nil
}

// ActionFirePhasers discharges ship phaser banks either automatically (equal distribution
// across all enemy ships in quadrant) or manually (energy allocated per target Klingon ID).
type ActionFirePhasers struct {
	Energy           float64
	ManualAllocation map[int]float64
}

// Execute applies the phaser firing action to GameState.
func (a ActionFirePhasers) Execute(g *GameState) ([]Event, error) {
	if a.Energy <= 0 && len(a.ManualAllocation) == 0 {
		return nil, errors.New("phaser energy must be positive")
	}

	totalEnergy := a.Energy
	if len(a.ManualAllocation) > 0 {
		totalEnergy = 0
		for _, e := range a.ManualAllocation {
			if e > 0 {
				totalEnergy += e
			}
		}
	}

	if totalEnergy <= 0 {
		return nil, errors.New("phaser energy must be positive")
	}

	if g.Enterprise.Energy < totalEnergy {
		return nil, errors.New("insufficient energy to fire phasers")
	}

	if g.Enterprise.Condition == ConditionDocked {
		return nil, errors.New("cannot fire phasers while docked")
	}

	if len(g.CurrentQuad.Klingons) == 0 {
		return nil, errors.New("no enemy ships in quadrant")
	}

	g.Enterprise.Energy -= totalEnergy

	events := []Event{
		EventPhaserFired{
			Energy: totalEnergy,
		},
	}

	if len(a.ManualAllocation) > 0 {
		for _, k := range g.CurrentQuad.Klingons {
			alloc, ok := a.ManualAllocation[k.ID]
			if !ok || alloc <= 0 {
				continue
			}
			dist := Distance(g.Enterprise.Sector, k.Sector)
			damage := ComputePhaserDamage(alloc, dist)
			destroyed := false
			if damage >= k.Energy {
				destroyed = true
				damage = k.Energy
				k.Energy = 0
				g.CurrentQuad.Grid[k.Sector[0]][k.Sector[1]] = EntityEmpty
				if g.RemainingKlingons > 0 {
					g.RemainingKlingons--
				}
			} else {
				k.Energy -= damage
			}
			events = append(events, EventPhaserHit{
				Target:    k.Sector,
				KlingonID: k.ID,
				Damage:    damage,
				Destroyed: destroyed,
			})
		}
	} else {
		numEnemies := float64(len(g.CurrentQuad.Klingons))
		energyPerTarget := totalEnergy / numEnemies
		for _, k := range g.CurrentQuad.Klingons {
			dist := Distance(g.Enterprise.Sector, k.Sector)
			damage := ComputePhaserDamage(energyPerTarget, dist)
			destroyed := false
			if damage >= k.Energy {
				destroyed = true
				damage = k.Energy
				k.Energy = 0
				g.CurrentQuad.Grid[k.Sector[0]][k.Sector[1]] = EntityEmpty
				if g.RemainingKlingons > 0 {
					g.RemainingKlingons--
				}
			} else {
				k.Energy -= damage
			}
			events = append(events, EventPhaserHit{
				Target:    k.Sector,
				KlingonID: k.ID,
				Damage:    damage,
				Destroyed: destroyed,
			})
		}
	}

	// Filter surviving Klingons
	var survivors []*Klingon
	for _, k := range g.CurrentQuad.Klingons {
		if k.Energy > 0 {
			survivors = append(survivors, k)
		}
	}
	g.CurrentQuad.Klingons = survivors

	return events, nil
}

// ActionMove navigates the Enterprise using warp engines.
// Course is specified in radians (0 = East, Pi/2 = North), and Warp is the speed/distance factor.
// DestSector may optionally specify a direct destination sector.
type ActionMove struct {
	Course     float64
	Warp       float64
	DestSector Coord
	DestQuad   Coord
}

// Execute applies the movement action to GameState.
func (a ActionMove) Execute(g *GameState) ([]Event, error) {
	if a.Warp <= 0 {
		return nil, errors.New("warp factor must be positive")
	}
	if g.Enterprise.Devices[DeviceWarp] >= 10.0 {
		return nil, errors.New("warp engines inoperative")
	}
	if g.Enterprise.Devices[DeviceWarp] > 0 && a.Warp > 4.0 {
		return nil, errors.New("warp engines damaged; maximum speed is warp 4")
	}

	quadrants := a.Warp
	factor := a.Warp
	if factor < 1.0 {
		factor = 1.0
	}
	shieldsMultiplier := 1.0
	if g.Enterprise.Shields > 0 {
		shieldsMultiplier = 2.0
	}
	energyNeeded := quadrants * factor * factor * factor * shieldsMultiplier
	if g.Enterprise.Energy < energyNeeded {
		return nil, errors.New("insufficient energy for warp movement")
	}

	timeUsed := 10.0 * quadrants / (factor * factor)

	fromQuad := g.Enterprise.Quad
	fromSector := g.Enterprise.Sector
	toQuad := fromQuad
	toSector := fromSector

	var events []Event

	if a.DestQuad != (Coord{}) {
		if a.DestQuad[0] < 1 || a.DestQuad[0] > 8 || a.DestQuad[1] < 1 || a.DestQuad[1] > 8 {
			return nil, errors.New("destination quadrant out of bounds")
		}
		toQuad = a.DestQuad
		if a.DestSector != (Coord{}) {
			if a.DestSector[0] < 1 || a.DestSector[0] > 8 || a.DestSector[1] < 1 || a.DestSector[1] > 8 {
				return nil, errors.New("destination sector out of bounds")
			}
			if toQuad == fromQuad && g.CurrentQuad.Grid[a.DestSector[0]][a.DestSector[1]] != EntityEmpty && a.DestSector != fromSector {
				return nil, errors.New("destination sector is occupied")
			}
			toSector = a.DestSector
		} else {
			toSector = Coord{4, 4}
			if toQuad == fromQuad && g.CurrentQuad.Grid[toSector[0]][toSector[1]] != EntityEmpty && toSector != fromSector {
				return nil, errors.New("destination sector is occupied")
			}
		}
	} else if a.DestSector != (Coord{}) {
		if a.DestSector[0] < 1 || a.DestSector[0] > 8 || a.DestSector[1] < 1 || a.DestSector[1] > 8 {
			return nil, errors.New("destination sector out of bounds")
		}
		if g.CurrentQuad.Grid[a.DestSector[0]][a.DestSector[1]] != EntityEmpty {
			return nil, errors.New("destination sector is occupied")
		}
		toSector = a.DestSector
	} else {
		// Vector movement
		dr := -math.Sin(a.Course)
		dc := math.Cos(a.Course)
		numSteps := int(math.Round(a.Warp * 8.0))
		if numSteps < 1 {
			numSteps = 1
		}

		currentR := float64(fromSector[0])
		currentC := float64(fromSector[1])
		hitObstacle := false
		exitedQuad := false

		for step := 1; step <= numSteps; step++ {
			nextR := int(math.Round(currentR + float64(step)*dr))
			nextC := int(math.Round(currentC + float64(step)*dc))

			if nextR < 1 || nextR > 8 || nextC < 1 || nextC > 8 {
				// Quadrant transition or edge of quadrant
				exitedQuad = true
				break
			}

			// Check obstacle in current quadrant
			cell := g.CurrentQuad.Grid[nextR][nextC]
			if cell != EntityEmpty && cell != EntityEnterprise {
				events = append(events, EventObstacleEncountered{
					Sector: Coord{nextR, nextC},
					Entity: cell,
				})
				hitObstacle = true
				break
			}
			toSector = Coord{nextR, nextC}
		}

		if !hitObstacle && exitedQuad {
			totalR := 8*(fromQuad[0]-1) + fromSector[0]
			totalC := 8*(fromQuad[1]-1) + fromSector[1]
			destR := int(math.Round(float64(totalR) + float64(numSteps)*dr))
			destC := int(math.Round(float64(totalC) + float64(numSteps)*dc))
			if destR < 1 {
				destR = 1
			} else if destR > 64 {
				destR = 64
			}
			if destC < 1 {
				destC = 1
			} else if destC > 64 {
				destC = 64
			}
			toQuad = Coord{(destR-1)/8 + 1, (destC-1)/8 + 1}
			toSector = Coord{(destR-1)%8 + 1, (destC-1)%8 + 1}
		}
	}

	// Apply movement mutations
	g.Enterprise.Energy -= energyNeeded
	g.Stardate += timeUsed
	g.TimeRemaining -= timeUsed

	if g.Enterprise.Condition == ConditionDocked {
		g.Enterprise.Condition = ConditionGreen
	}

	g.CurrentQuad.Grid[fromSector[0]][fromSector[1]] = EntityEmpty
	if toQuad == fromQuad {
		g.CurrentQuad.Grid[toSector[0]][toSector[1]] = EntityEnterprise
	}
	g.Enterprise.Sector = toSector
	g.Enterprise.Quad = toQuad

	shipMovedEvt := EventShipMoved{
		FromQuad:   fromQuad,
		ToQuad:     toQuad,
		FromSector: fromSector,
		ToSector:   toSector,
		Warp:       a.Warp,
		EnergyUsed: energyNeeded,
		TimeUsed:   timeUsed,
	}
	events = append([]Event{shipMovedEvt}, events...)

	return events, nil
}

// ActionLRScan initiates a long-range sensor scan of the quadrants immediately surrounding the Enterprise.
type ActionLRScan struct{}

// Execute applies the long-range scan action to GameState.
func (a ActionLRScan) Execute(g *GameState) ([]Event, error) {
	if g.Enterprise.Devices[DeviceLRSensors] > 0 && g.Enterprise.Condition != ConditionDocked {
		return nil, errors.New("long-range sensors damaged")
	}

	relayed := (g.Enterprise.Condition == ConditionDocked && g.Enterprise.Devices[DeviceLRSensors] > 0)
	center := g.Enterprise.Quad
	var scanned []Coord

	for dr := -1; dr <= 1; dr++ {
		for dc := -1; dc <= 1; dc++ {
			r := center[0] + dr
			c := center[1] + dc
			if r >= 1 && r <= 8 && c >= 1 && c <= 8 {
				g.ChartDiscovered[r][c] = true
				scanned = append(scanned, Coord{r, c})
			}
		}
	}

	return []Event{
		EventLRScanCompleted{
			CenterQuad:    center,
			ScannedQuads:  scanned,
			RelayedByBase: relayed,
		},
	}, nil
}

