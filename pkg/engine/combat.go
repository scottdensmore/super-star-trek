package engine

import (
	"errors"
	"math"
)

// ComputePhaserDamage calculates phaser beam damage delivered to a target based on
// fired energy and Euclidean distance. Damage attenuates inversely with distance.
func ComputePhaserDamage(energy float64, dist float64) float64 {
	if energy <= 0 {
		return 0
	}
	if dist <= 0 {
		dist = 0.1
	}
	// Classic formula: damage attenuates by distance factor
	return energy * (1.0 / dist)
}

// TraceTorpedoPath simulates photon torpedo ballistics along a given angle (in radians)
// originating from origin within a quadrant. Returns coordinates of the hit cell,
// the EntityType hit, and a boolean indicating whether a collision occurred.
func TraceTorpedoPath(origin Coord, angle float64, quad *QuadrantState) (Coord, EntityType, bool) {
	if quad == nil {
		return Coord{}, EntityEmpty, false
	}
	if origin[0] < 1 || origin[0] > 8 || origin[1] < 1 || origin[1] > 8 {
		return Coord{}, EntityEmpty, false
	}

	r := float64(origin[0])
	c := float64(origin[1])
	dr := -math.Sin(angle) * 0.25
	dc := math.Cos(angle) * 0.25

	for step := 0; step < 40; step++ {
		r += dr
		c += dc
		ir := int(math.Round(r))
		ic := int(math.Round(c))

		// Check out of bounds
		if ir < 1 || ir > 8 || ic < 1 || ic > 8 {
			return Coord{}, EntityEmpty, false
		}

		if ir == origin[0] && ic == origin[1] {
			continue
		}

		cell := quad.Grid[ir][ic]
		if cell != EntityEmpty {
			return Coord{ir, ic}, cell, true
		}
	}
	return Coord{}, EntityEmpty, false
}

// ResolveShieldHit resolves incoming damage to the Enterprise, applying damage first
// to shields and deflecting remainder into ship energy reserves (hull damage).
// Returns (shieldDamageAbsorbed, hullDamageTaken).
func ResolveShieldHit(enterprise *Enterprise, damage float64) (float64, float64) {
	if enterprise == nil || damage <= 0 {
		return 0, 0
	}
	if enterprise.Shields < 0 {
		enterprise.Shields = 0
	}
	if enterprise.Shields >= damage {
		enterprise.Shields -= damage
		return damage, 0
	}
	absorbed := enterprise.Shields
	remainder := damage - absorbed
	enterprise.Shields = 0
	enterprise.Energy -= remainder
	return absorbed, remainder
}

// ActionTorpedoDirect fires a photon torpedo with target lock on a specific sector.
type ActionTorpedoDirect struct {
	TargetSector Coord
}

// Execute applies target-lock torpedo firing to GameState, rejecting target lock on cloaked vessels.
func (a ActionTorpedoDirect) Execute(g *GameState) ([]Event, error) {
	for _, k := range g.CurrentQuad.Klingons {
		if k.Sector == a.TargetSector && k.IsCloaked {
			return nil, errors.New("TARGET LOCK FAILED: CLOAKED VESSEL")
		}
	}
	return ActionFireTorpedo{Target: a.TargetSector}.Execute(g)
}

// DecloakKlingon decloaks a Klingon vessel if currently cloaked and returns corresponding events.
func DecloakKlingon(g *GameState, k *Klingon) []Event {
	if k == nil || !k.IsCloaked {
		return nil
	}
	k.IsCloaked = false
	return []Event{
		EventKlingonCloakState{
			KlingonID: k.ID,
			Cloaked:   false,
		},
	}
}

// CloakKlingon cloaks a Klingon commander if rules permit and not already cloaked, returning corresponding events.
func CloakKlingon(g *GameState, k *Klingon) []Event {
	if k == nil || k.IsCloaked {
		return nil
	}
	if g != nil && !g.Rules.KlingonCloak {
		return nil
	}
	if !k.IsCommander {
		return nil
	}
	k.IsCloaked = true
	return []Event{
		EventKlingonCloakState{
			KlingonID: k.ID,
			Cloaked:   true,
		},
	}
}

// KlingonCounterAttack simulates return fire from a Klingon vessel against the Enterprise.
// If the attacking vessel is cloaked, it decloaks prior to firing.
func KlingonCounterAttack(g *GameState, k *Klingon, damage float64) []Event {
	if g == nil || k == nil || damage <= 0 {
		return nil
	}
	var events []Event
	if k.IsCloaked {
		events = append(events, DecloakKlingon(g, k)...)
	}
	_, hullDamage := ResolveShieldHit(&g.Enterprise, damage)
	if hullDamage > 0 {
		casualties := int(math.Ceil(hullDamage / 50.0))
		if casualties < 1 {
			casualties = 1
		}
		g.Metrics.Casualties += casualties
	}
	events = append(events, EventKlingonCounterAttack{
		EnemyID: k.ID,
		Damage:  damage,
	})
	return events
}

// MoveKlingon moves a Klingon vessel to a destination sector.
// If moved or pushed into EntityBlackHole, the vessel is absorbed into the singularity,
// eliminated from CurrentQuad.Klingons, and emits an EventSingularityAbsorption.
func MoveKlingon(g *GameState, k *Klingon, dest Coord) []Event {
	if g == nil || k == nil {
		return nil
	}
	if dest[0] < 1 || dest[0] > 8 || dest[1] < 1 || dest[1] > 8 {
		return nil
	}

	cell := g.CurrentQuad.Grid[dest[0]][dest[1]]
	// If destination is occupied by an obstacle or another entity (not EntityBlackHole), reject the move
	if cell != EntityEmpty && cell != EntityBlackHole {
		return nil
	}

	origSector := k.Sector
	var origEntity EntityType = EntityKlingon
	if origSector[0] >= 1 && origSector[0] <= 8 && origSector[1] >= 1 && origSector[1] <= 8 {
		gridEnt := g.CurrentQuad.Grid[origSector[0]][origSector[1]]
		if gridEnt == EntityKlingon || gridEnt == EntityCommander || gridEnt == EntitySuperCommander {
			origEntity = gridEnt
			g.CurrentQuad.Grid[origSector[0]][origSector[1]] = EntityEmpty
		} else if k.IsCommander {
			origEntity = EntityCommander
		}
	} else if k.IsCommander {
		origEntity = EntityCommander
	}

	if cell == EntityBlackHole {
		for i, klingon := range g.CurrentQuad.Klingons {
			if klingon.ID == k.ID {
				g.CurrentQuad.Klingons = append(g.CurrentQuad.Klingons[:i], g.CurrentQuad.Klingons[i+1:]...)
				break
			}
		}
		if g.RemainingKlingons > 0 {
			g.RemainingKlingons--
		}
		qr, qc := g.Enterprise.Quad[0], g.Enterprise.Quad[1]
		if qr >= 1 && qr <= 8 && qc >= 1 && qc <= 8 && g.GalaxyChart[qr][qc] >= 100 {
			g.GalaxyChart[qr][qc] -= 100
		}
		if origEntity == EntitySuperCommander {
			g.Metrics.SuperCommandersKilled++
		} else if origEntity == EntityCommander || k.IsCommander {
			g.Metrics.CommandersKilled++
		} else {
			g.Metrics.KlingonsKilled++
		}
		return []Event{
			EventSingularityAbsorption{
				Sector: dest,
				Target: origEntity,
				Weapon: "singularity",
			},
		}
	}

	g.CurrentQuad.Grid[dest[0]][dest[1]] = origEntity
	k.Sector = dest
	return nil
}

// KlingonTurn executes environmental and combat updates for Klingons in the current quadrant.
// In EnvNebula, Klingon shields collapse to 0 (symmetrical with Enterprise).
// If any Klingon occupies or is pulled into EntityBlackHole, it is absorbed and eliminated.
func KlingonTurn(g *GameState) []Event {
	if g == nil {
		return nil
	}
	var events []Event
	qr, qc := g.Enterprise.Quad[0], g.Enterprise.Quad[1]
	inNebula := qr >= 1 && qr <= 8 && qc >= 1 && qc <= 8 && g.QuadrantEnv[qr][qc] == EnvNebula
	if inNebula {
		for _, k := range g.CurrentQuad.Klingons {
			if k != nil {
				k.Shields = 0
			}
		}
	}

	var survivors []*Klingon
	for _, k := range g.CurrentQuad.Klingons {
		if k == nil {
			continue
		}
		if k.Sector[0] >= 1 && k.Sector[0] <= 8 && k.Sector[1] >= 1 && k.Sector[1] <= 8 &&
			g.CurrentQuad.Grid[k.Sector[0]][k.Sector[1]] == EntityBlackHole {
			if g.RemainingKlingons > 0 {
				g.RemainingKlingons--
			}
			if qr >= 1 && qr <= 8 && qc >= 1 && qc <= 8 && g.GalaxyChart[qr][qc] >= 100 {
				g.GalaxyChart[qr][qc] -= 100
			}
			if k.IsCommander {
				g.Metrics.CommandersKilled++
			} else {
				g.Metrics.KlingonsKilled++
			}
			targetEntity := EntityKlingon
			if k.IsCommander {
				targetEntity = EntityCommander
			}
			events = append(events, EventSingularityAbsorption{
				Sector: k.Sector,
				Target: targetEntity,
				Weapon: "singularity",
			})
		} else {
			survivors = append(survivors, k)
		}
	}
	g.CurrentQuad.Klingons = survivors

	return events
}


