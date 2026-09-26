package engine

import (
	"math"
)

// EventRomulanDecloak signals that a Romulan vessel has deactivated its cloaking device.
type EventRomulanDecloak struct {
	RomulanID int
	Sector    Coord
}

// EventType returns the type name for EventRomulanDecloak.
func (e EventRomulanDecloak) EventType() string { return "RomulanDecloak" }

// EventPlasmaLaunched signals the firing of an in-flight tracking plasma torpedo.
type EventPlasmaLaunched struct {
	TorpedoID int
	SourceID  int
	Sector    Coord
	Energy    float64
}

// EventType returns the type name for EventPlasmaLaunched.
func (e EventPlasmaLaunched) EventType() string { return "PlasmaLaunched" }

// EventPlasmaMoved signals that an in-flight plasma torpedo advanced across the sector grid.
type EventPlasmaMoved struct {
	TorpedoID int
	From      Coord
	To        Coord
	Yield     float64
}

// EventType returns the type name for EventPlasmaMoved.
func (e EventPlasmaMoved) EventType() string { return "PlasmaMoved" }

// EventPlasmaImpact signals that a plasma torpedo collided with the Enterprise.
type EventPlasmaImpact struct {
	TorpedoID    int
	ShieldDamage float64
	HullDamage   float64
}

// EventType returns the type name for EventPlasmaImpact.
func (e EventPlasmaImpact) EventType() string { return "PlasmaImpact" }

// EventPlasmaDissipated signals that a plasma torpedo ran out of thermal energy and dissipated.
type EventPlasmaDissipated struct {
	TorpedoID int
	Sector    Coord
}

// EventType returns the type name for EventPlasmaDissipated.
func (e EventPlasmaDissipated) EventType() string { return "PlasmaDissipated" }

// EventPlasmaIntercepted signals that a plasma torpedo was destroyed by point-defense weapons.
type EventPlasmaIntercepted struct {
	TorpedoID int
	Sector    Coord
	Weapon    string
}

// EventType returns the type name for EventPlasmaIntercepted.
func (e EventPlasmaIntercepted) EventType() string { return "PlasmaIntercepted" }

// EventEntityMove signals that an entity moved across the sector grid.
type EventEntityMove struct {
	Target EntityType
	From   Coord
	To     Coord
}

// EventType returns the type name for EventEntityMove.
func (e EventEntityMove) EventType() string { return "EntityMove" }

// RomulanTurn executes tactical decision-making for a Romulan vessel.
func RomulanTurn(g *GameState, r *EnemyVessel) []Event {
	if g == nil || r == nil || r.Faction != FactionRomulan {
		return nil
	}
	var events []Event

	if r.IsCloaked {
		if r.CloakTurns > 0 {
			r.CloakTurns--
		}
		if r.CloakTurns == 0 {
			// Decloak and launch plasma torpedo
			r.IsCloaked = false
			g.CurrentQuad.Grid[r.Sector[0]][r.Sector[1]] = EntityRomulan
			events = append(events, EventRomulanDecloak{
				RomulanID: r.ID,
				Sector:    r.Sector,
			})

			// Launch plasma torpedo
			torpID := 1000 + len(g.CurrentQuad.PlasmaTorpedoes) + 1
			initialYield := 1000.0
			plasma := &PlasmaTorpedo{
				ID:            torpID,
				SourceID:      r.ID,
				Sector:        r.Sector,
				Energy:        initialYield,
				TargetSector:  g.Enterprise.Sector,
				TurnsInFlight: 0,
			}
			g.CurrentQuad.PlasmaTorpedoes = append(g.CurrentQuad.PlasmaTorpedoes, plasma)
			events = append(events, EventPlasmaLaunched{
				TorpedoID: torpID,
				SourceID:  r.ID,
				Sector:    r.Sector,
				Energy:    initialYield,
			})
			return events
		}

		// Reposition stealthily 1 sector closer or laterally
		dr := 0
		dc := 0
		if g.Enterprise.Sector[0] > r.Sector[0] {
			dr = 1
		} else if g.Enterprise.Sector[0] < r.Sector[0] {
			dr = -1
		}
		if g.Enterprise.Sector[1] > r.Sector[1] {
			dc = 1
		} else if g.Enterprise.Sector[1] < r.Sector[1] {
			dc = -1
		}

		oldSector := r.Sector
		newSector := Coord{r.Sector[0] + dr, r.Sector[1] + dc}
		if newSector[0] >= 1 && newSector[0] <= 8 && newSector[1] >= 1 && newSector[1] <= 8 {
			if g.CurrentQuad.Grid[newSector[0]][newSector[1]] == EntityEmpty {
				if g.CurrentQuad.Grid[r.Sector[0]][r.Sector[1]] == EntityRomulan {
					g.CurrentQuad.Grid[r.Sector[0]][r.Sector[1]] = EntityEmpty
				}
				r.Sector = newSector
			}
		}
		return []Event{EventEntityMove{Target: EntityRomulan, From: oldSector, To: r.Sector}}
	}

	// When decloaked, fire disruptors if plasma already launched
	dist := math.Hypot(float64(r.Sector[0]-g.Enterprise.Sector[0]), float64(r.Sector[1]-g.Enterprise.Sector[1]))
	dmg := ComputePhaserDamage(r.Energy*0.4, dist)
	events = append(events, KlingonCounterAttack(g, &Klingon{ID: r.ID, Sector: r.Sector}, dmg)...)

	// Re-cloak after firing if energy permits
	r.IsCloaked = true
	r.CloakTurns = 3
	if g.CurrentQuad.Grid[r.Sector[0]][r.Sector[1]] == EntityRomulan {
		g.CurrentQuad.Grid[r.Sector[0]][r.Sector[1]] = EntityEmpty
	}
	return events
}

// AdvancePlasmaTorpedoes moves in-flight plasma torpedoes 1-2 sectors toward the Enterprise and applies dissipation or impact.
func AdvancePlasmaTorpedoes(g *GameState) []Event {
	if g == nil {
		return nil
	}
	var events []Event
	var survivors []*PlasmaTorpedo

	for _, pt := range g.CurrentQuad.PlasmaTorpedoes {
		pt.TurnsInFlight++
		pt.Energy *= 0.80 // Cools by 20% each turn

		if pt.Energy < 200.0 || pt.TurnsInFlight >= 5 {
			if g.CurrentQuad.Grid[pt.Sector[0]][pt.Sector[1]] == EntityPlasmaTorpedo {
				g.CurrentQuad.Grid[pt.Sector[0]][pt.Sector[1]] = EntityEmpty
			}
			events = append(events, EventPlasmaDissipated{
				TorpedoID: pt.ID,
				Sector:    pt.Sector,
			})
			continue
		}

		pt.TargetSector = g.Enterprise.Sector
		oldCoord := pt.Sector
		dr := 0
		dc := 0
		if g.Enterprise.Sector[0] > pt.Sector[0] {
			dr = 1
		} else if g.Enterprise.Sector[0] < pt.Sector[0] {
			dr = -1
		}
		if g.Enterprise.Sector[1] > pt.Sector[1] {
			dc = 1
		} else if g.Enterprise.Sector[1] < pt.Sector[1] {
			dc = -1
		}

		newCoord := Coord{pt.Sector[0] + dr, pt.Sector[1] + dc}
		if newCoord == g.Enterprise.Sector {
			// Direct impact: 50% absorbed by shields, 50% penetrating to hull/systems
			if g.CurrentQuad.Grid[oldCoord[0]][oldCoord[1]] == EntityPlasmaTorpedo {
				g.CurrentQuad.Grid[oldCoord[0]][oldCoord[1]] = EntityEmpty
			}
			half := pt.Energy * 0.5
			shieldDmg, overflow := ResolveShieldHit(&g.Enterprise, half)
			g.Enterprise.Energy -= half
			hullDmg := half + overflow
			events = append(events, EventPlasmaImpact{
				TorpedoID:    pt.ID,
				ShieldDamage: shieldDmg,
				HullDamage:   hullDmg,
			})
			continue
		}

		// Move projectile
		if newCoord[0] >= 1 && newCoord[0] <= 8 && newCoord[1] >= 1 && newCoord[1] <= 8 && g.CurrentQuad.Grid[newCoord[0]][newCoord[1]] == EntityEmpty {
			if g.CurrentQuad.Grid[oldCoord[0]][oldCoord[1]] == EntityPlasmaTorpedo {
				g.CurrentQuad.Grid[oldCoord[0]][oldCoord[1]] = EntityEmpty
			}
			g.CurrentQuad.Grid[newCoord[0]][newCoord[1]] = EntityPlasmaTorpedo
			pt.Sector = newCoord
		} else {
			// Try routing around obstacle: horizontal or vertical step
			alt1 := Coord{pt.Sector[0] + dr, pt.Sector[1]}
			alt2 := Coord{pt.Sector[0], pt.Sector[1] + dc}
			if dr != 0 && alt1[0] >= 1 && alt1[0] <= 8 && g.CurrentQuad.Grid[alt1[0]][alt1[1]] == EntityEmpty {
				if g.CurrentQuad.Grid[oldCoord[0]][oldCoord[1]] == EntityPlasmaTorpedo {
					g.CurrentQuad.Grid[oldCoord[0]][oldCoord[1]] = EntityEmpty
				}
				g.CurrentQuad.Grid[alt1[0]][alt1[1]] = EntityPlasmaTorpedo
				pt.Sector = alt1
			} else if dc != 0 && alt2[1] >= 1 && alt2[1] <= 8 && g.CurrentQuad.Grid[alt2[0]][alt2[1]] == EntityEmpty {
				if g.CurrentQuad.Grid[oldCoord[0]][oldCoord[1]] == EntityPlasmaTorpedo {
					g.CurrentQuad.Grid[oldCoord[0]][oldCoord[1]] = EntityEmpty
				}
				g.CurrentQuad.Grid[alt2[0]][alt2[1]] = EntityPlasmaTorpedo
				pt.Sector = alt2
			}
		}

		events = append(events, EventPlasmaMoved{
			TorpedoID: pt.ID,
			From:      oldCoord,
			To:        pt.Sector,
			Yield:     pt.Energy,
		})
		survivors = append(survivors, pt)
	}
	g.CurrentQuad.PlasmaTorpedoes = survivors
	return events
}

// InterceptPlasmaTorpedo attempts to detonate an in-flight plasma torpedo using phasers or torpedoes.
func InterceptPlasmaTorpedo(g *GameState, target Coord, damage float64) (bool, []Event) {
	if g == nil || damage <= 0 {
		return false, nil
	}
	for i, pt := range g.CurrentQuad.PlasmaTorpedoes {
		if pt.Sector == target {
			if damage >= pt.Energy*0.75 { // Sufficient point defense destroys the torpedo
				if g.CurrentQuad.Grid[pt.Sector[0]][pt.Sector[1]] == EntityPlasmaTorpedo {
					g.CurrentQuad.Grid[pt.Sector[0]][pt.Sector[1]] = EntityEmpty
				}
				g.CurrentQuad.PlasmaTorpedoes = append(g.CurrentQuad.PlasmaTorpedoes[:i], g.CurrentQuad.PlasmaTorpedoes[i+1:]...)
				return true, []Event{
					EventPlasmaIntercepted{
						TorpedoID: pt.ID,
						Sector:    target,
						Weapon:    "point-defense",
					},
				}
			}
		}
	}
	return false, nil
}
