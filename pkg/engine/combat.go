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
