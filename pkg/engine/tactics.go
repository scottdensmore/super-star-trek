package engine

import (
	"errors"
)

// ComputeDistance returns Euclidean distance between two coordinates.
func ComputeDistance(c1, c2 Coord) float64 {
	return Distance(c1, c2)
}

// UnifiedAdversaryTurn resolves all tactical adversary factions in strict sequence:
// 1. Advance in-flight plasma torpedoes
// 2. Klingon pack screening & crossfire counter-attacks
// 3. Romulan stealth & decloak actions
// 4. Tholian binary spinners & web construction
func UnifiedAdversaryTurn(g *GameState) []Event {
	if g == nil {
		return nil
	}
	var events []Event

	// 1. Projectiles
	events = append(events, AdvancePlasmaTorpedoes(g)...)

	// 2. Klingon Pack Screening
	events = append(events, ExecuteCommanderScreening(g)...)

	// Klingon counter attacks with crossfire multiplier
	mult := CalculatePackDamageMultiplier(g)
	for _, e := range g.CurrentQuad.Enemies {
		if e != nil && e.Faction == FactionKlingon && e.Energy > 0 {
			k := &Klingon{ID: e.ID, Sector: e.Sector, IsCommander: e.IsCommander, IsCloaked: e.IsCloaked}
			baseDmg := e.Energy * 0.35 * mult
			events = append(events, KlingonCounterAttack(g, k, baseDmg)...)
			e.IsCloaked = k.IsCloaked
		}
	}

	// 3. Romulan Faction
	for _, e := range g.CurrentQuad.Enemies {
		if e != nil && e.Faction == FactionRomulan && e.Energy > 0 {
			events = append(events, RomulanTurn(g, e)...)
		}
	}

	// 4. Tholian Faction
	events = append(events, TholianTurn(g)...)

	// Synchronize legacy slice
	SyncQuadrantEnemies(&g.CurrentQuad)
	return events
}

// ActionPhaserDirect fires focused phaser energy at a specific sector (ship, torpedo, or web filament).
type ActionPhaserDirect struct {
	TargetSector Coord
	Energy       float64
}

// Execute applies targeted phaser energy to the specified sector.
func (a ActionPhaserDirect) Execute(g *GameState) ([]Event, error) {
	if g == nil {
		return nil, errors.New("nil game state")
	}
	if a.Energy <= 0 {
		return nil, errors.New("phaser energy must be positive")
	}
	if g.Enterprise.Energy < a.Energy {
		return nil, errors.New("insufficient energy for phasers")
	}
	if a.TargetSector[0] < 1 || a.TargetSector[0] > 8 || a.TargetSector[1] < 1 || a.TargetSector[1] > 8 {
		return nil, errors.New("target sector out of bounds")
	}

	g.Enterprise.Energy -= a.Energy
	var events []Event

	cell := g.CurrentQuad.Grid[a.TargetSector[0]][a.TargetSector[1]]
	switch cell {
	case EntityPlasmaTorpedo:
		intercepted, intEvents := InterceptPlasmaTorpedo(g, a.TargetSector, a.Energy)
		if intercepted {
			events = append(events, intEvents...)
			g.Metrics.RomulansSurrendered++ // metric count for point-defense
		}
	case EntityTholianWeb:
		breached, breachEvents := DamageWebSegment(g, a.TargetSector, a.Energy)
		if breached {
			events = append(events, breachEvents...)
		}
	default:
		// Standard vessel phaser attack
		dist := ComputeDistance(g.Enterprise.Sector, a.TargetSector)
		dmg := ComputePhaserDamage(a.Energy, dist)
		for _, e := range g.CurrentQuad.Enemies {
			if e != nil && e.Sector == a.TargetSector && e.Energy > 0 {
				e.Shields -= dmg
				if e.Shields < 0 {
					e.Energy += e.Shields
					e.Shields = 0
				}
				if e.Energy <= 0 {
					e.Energy = 0
					g.CurrentQuad.Grid[e.Sector[0]][e.Sector[1]] = EntityEmpty
					if e.Faction == FactionRomulan {
						g.Metrics.RomulansKilled++
					}
					var remaining []*EnemyVessel
					for _, other := range g.CurrentQuad.Enemies {
						if other != nil && other.ID != e.ID && other.Energy > 0 {
							remaining = append(remaining, other)
						}
					}
					g.CurrentQuad.Enemies = remaining
					SyncQuadrantEnemies(&g.CurrentQuad)
					if g.RemainingKlingons > 0 {
						g.RemainingKlingons--
					}
					g.KlingonsRemaining = g.RemainingKlingons
				}
				break
			}
		}
	}

	return events, nil
}
