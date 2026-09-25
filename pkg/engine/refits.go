package engine

import (
	"fmt"
)

type RefitID string

const (
	RefitDilithiumCore  RefitID = "dilithium_core"
	RefitDeflectorGrid  RefitID = "deflector_grid"
	RefitTorpedoCasings RefitID = "torpedo_casings"
	RefitTorpedoBays    RefitID = "torpedo_bays"
	RefitSensorMatrix   RefitID = "sensor_matrix"
	RefitDamageNanites  RefitID = "damage_nanites"
)

type RefitDefinition struct {
	ID          RefitID   `json:"id"`
	Name        string    `json:"name"`
	Description string    `json:"description"`
	TierCosts   [3]int    `json:"tier_costs"`
	TierEffects [3]string `json:"tier_effects"`
}

var RefitCatalog = []RefitDefinition{
	{
		ID:          RefitDilithiumCore,
		Name:        "Dilithium Core Tuning",
		Description: "Increases primary warp core energy storage capacity.",
		TierCosts:   [3]int{500, 1000, 1750},
		TierEffects: [3]string{
			"Max Energy: 3,500 (+500)",
			"Max Energy: 4,000 (+1,000)",
			"Max Energy: 4,500 (+1,500)",
		},
	},
	{
		ID:          RefitDeflectorGrid,
		Name:        "Reinforced Deflectors",
		Description: "Enhances deflector shield absorption efficiency under attack.",
		TierCosts:   [3]int{450, 900, 1500},
		TierEffects: [3]string{
			"Shield drain reduced by 15%",
			"Shield drain reduced by 30%",
			"Shield drain reduced by 45%",
		},
	},
	{
		ID:          RefitTorpedoCasings,
		Name:        "High-Yield Torpedoes",
		Description: "Upgrades photon warhead casing density and antimatter payload.",
		TierCosts:   [3]int{400, 800, 1400},
		TierEffects: [3]string{
			"Torpedo damage +25%",
			"Torpedo damage +50%",
			"Torpedo damage +75%",
		},
	},
	{
		ID:          RefitTorpedoBays,
		Name:        "Auxiliary Torpedo Magazine",
		Description: "Expands physical torpedo storage capacity in forward ordnance deck.",
		TierCosts:   [3]int{350, 700, 1200},
		TierEffects: [3]string{
			"Max Torpedoes: 14 (+4)",
			"Max Torpedoes: 18 (+8)",
			"Max Torpedoes: 22 (+12)",
		},
	},
	{
		ID:          RefitSensorMatrix,
		Name:        "Subspace Sensor Matrix",
		Description: "Extends long-range scan radius and reveals cloaked silhouettes.",
		TierCosts:   [3]int{300, 600, 1000},
		TierEffects: [3]string{
			"LRS scan radius +1 quadrant",
			"Cloaked hostile silhouettes revealed",
			"Automatic free LRS on quadrant entry",
		},
	},
	{
		ID:          RefitDamageNanites,
		Name:        "Automated Repair Bots",
		Description: "Deployable micro-drones accelerate damage control during warp transit.",
		TierCosts:   [3]int{400, 850, 1450},
		TierEffects: [3]string{
			"Warp movement passive repair +0.5d",
			"Warp movement passive repair +1.0d",
			"Warp movement passive repair +1.5d",
		},
	},
}

func GetRefitDefinition(id RefitID) *RefitDefinition {
	for i := range RefitCatalog {
		if RefitCatalog[i].ID == id {
			return &RefitCatalog[i]
		}
	}
	return nil
}

func ApplyRefits(g *GameState, refits map[RefitID]int) {
	if g == nil || refits == nil {
		return
	}
	g.ActiveRefits = make(map[RefitID]int)
	for k, v := range refits {
		g.ActiveRefits[k] = v
	}

	// Dilithium Core
	if tier := refits[RefitDilithiumCore]; tier > 0 {
		bonus := float64(tier * 500)
		g.Enterprise.MaxEnergy = 3000.0 + bonus
	} else {
		g.Enterprise.MaxEnergy = 3000.0
	}
	if g.Enterprise.Energy > g.Enterprise.MaxEnergy {
		g.Enterprise.Energy = g.Enterprise.MaxEnergy
	}

	// Torpedo Bays
	if tier := refits[RefitTorpedoBays]; tier > 0 {
		bonus := tier * 4
		g.Enterprise.MaxTorpedoes = 10 + bonus
	} else {
		g.Enterprise.MaxTorpedoes = 10
	}
	if g.Enterprise.Torpedoes > g.Enterprise.MaxTorpedoes {
		g.Enterprise.Torpedoes = g.Enterprise.MaxTorpedoes
	}
}

func PurchaseRefit(tour *TourState, id RefitID) error {
	if tour == nil {
		return fmt.Errorf("tour is nil")
	}
	def := GetRefitDefinition(id)
	if def == nil {
		return fmt.Errorf("unknown refit module: %s", id)
	}
	if tour.InstalledRefits == nil {
		tour.InstalledRefits = make(map[RefitID]int)
	}

	currentTier := tour.InstalledRefits[id]
	if currentTier >= 3 {
		return fmt.Errorf("%s is already at maximum tier 3", def.Name)
	}

	nextTier := currentTier + 1
	cost := def.TierCosts[nextTier-1]
	if tour.RequisitionPoints < cost {
		return fmt.Errorf("insufficient requisition: requires %d, have %d", cost, tour.RequisitionPoints)
	}

	tour.RequisitionPoints -= cost
	tour.InstalledRefits[id] = nextTier
	return nil
}

func CalculateTorpedoDamage(g *GameState, baseDamage float64) float64 {
	if g == nil || g.ActiveRefits == nil {
		return baseDamage
	}
	tier := g.ActiveRefits[RefitTorpedoCasings]
	if tier < 0 {
		tier = 0
	} else if tier > 3 {
		tier = 3
	}
	if tier <= 0 {
		return baseDamage
	}
	multiplier := 1.0 + (float64(tier) * 0.25)
	return baseDamage * multiplier
}

func CalculateShieldDamageAbsorption(g *GameState, incomingDamage float64) float64 {
	if g == nil || g.ActiveRefits == nil {
		return incomingDamage
	}
	tier := g.ActiveRefits[RefitDeflectorGrid]
	if tier < 0 {
		tier = 0
	} else if tier > 3 {
		tier = 3
	}
	if tier <= 0 {
		return incomingDamage
	}
	reductionPercent := float64(tier) * 0.15
	return incomingDamage * (1.0 - reductionPercent)
}
