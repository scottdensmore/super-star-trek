package engine

// Dispatch executes the specified action against the game state, applying mutations
// and returning an event slice detailing all resulting occurrences.
func (g *GameState) Dispatch(action Action) ([]Event, error) {
	return action.Execute(g)
}

// AdvanceRepairs repairs damaged subsystems according to elapsed time and the rules repair multiplier.
func AdvanceRepairs(g *GameState, elapsed float64) {
	if g == nil || elapsed <= 0 {
		return
	}
	mult := g.Rules.RepairMultiplier
	if mult <= 0 {
		mult = 1.0
	}
	repairStep := elapsed / mult
	for i := range g.Enterprise.Devices {
		if g.Enterprise.Devices[i] > 0 {
			g.Enterprise.Devices[i] -= repairStep
			if g.Enterprise.Devices[i] < 0 {
				g.Enterprise.Devices[i] = 0
			}
		}
	}
}

// AdvanceTurn advances time and subsystem repairs, allowing commanders to cloak when rules permit.
func (g *GameState) AdvanceTurn(elapsed float64) {
	AdvanceRepairs(g, elapsed)
	if g != nil && g.Rules.KlingonCloak {
		for _, k := range g.CurrentQuad.Klingons {
			if k != nil && k.IsCommander && !k.IsCloaked {
				CloakKlingon(g, k)
			}
		}
	}
}
