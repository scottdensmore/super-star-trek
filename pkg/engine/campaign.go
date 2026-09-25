package engine

import (
	"fmt"
	"time"
)

type TourID string

type SectorObjectiveType string

const (
	ObjectiveBorderPatrol     SectorObjectiveType = "border_patrol"
	ObjectiveDeepSurveillance SectorObjectiveType = "deep_surveillance"
	ObjectiveConvoyEscort     SectorObjectiveType = "convoy_escort"
	ObjectiveStarbaseSiege    SectorObjectiveType = "starbase_siege"
)

type TourSectorConfig struct {
	Index         int                 `json:"index"`
	Name          string              `json:"name"`
	Objective     SectorObjectiveType `json:"objective"`
	Description   string              `json:"description"`
	InitialDays   float64             `json:"initial_days"`
	HostileCount  int                 `json:"hostile_count"`
	StarbaseCount int                 `json:"starbase_count"`
	Anomalies     bool                `json:"anomalies"`
	BonusBounty   int                 `json:"bonus_bounty"`
}

type TourState struct {
	ID                 TourID             `json:"id"`
	Active             bool               `json:"active"`
	CurrentSectorIndex int                `json:"current_sector_index"`
	Sectors            []TourSectorConfig `json:"sectors"`
	RequisitionPoints  int                `json:"requisition_points"`
	InstalledRefits    map[RefitID]int    `json:"installed_refits"`
	TotalTourScore     int                `json:"total_tour_score"`
	SectorsCompleted   int                `json:"sectors_completed"`
	HostilesDestroyed  int                `json:"hostiles_destroyed"`
	CurrentGameState   *GameState         `json:"current_game_state,omitempty"`
	InDrydock          bool               `json:"in_drydock"`
	Completed          bool               `json:"completed"`
	Failed             bool               `json:"failed"`
	FailureReason      GameOverReason     `json:"failure_reason,omitempty"`
	Medals             []string           `json:"medals"`
	Seed               int64              `json:"seed"`
}


func defaultSectors() []TourSectorConfig {
	return []TourSectorConfig{
		{
			Index:         1,
			Name:          "Vanguard Border Incursion",
			Objective:     ObjectiveBorderPatrol,
			Description:   "Eliminate Klingon vanguard battlecruisers infiltrating Federation border space.",
			InitialDays:   30.0,
			HostileCount:  4,
			StarbaseCount: 1,
			Anomalies:     false,
			BonusBounty:   250,
		},
		{
			Index:         2,
			Name:          "Mutara Deep Surveillance",
			Objective:     ObjectiveDeepSurveillance,
			Description:   "Chart unmapped anomaly quadrants in the Mutara rift and eliminate stealth scout vessels.",
			InitialDays:   32.0,
			HostileCount:  5,
			StarbaseCount: 1,
			Anomalies:     true,
			BonusBounty:   350,
		},
		{
			Index:         3,
			Name:          "Federation Convoy Escort",
			Objective:     ObjectiveConvoyEscort,
			Description:   "Protect Federation transport ships and eliminate marauder squadrons targeting medical convoys.",
			InitialDays:   35.0,
			HostileCount:  6,
			StarbaseCount: 2,
			Anomalies:     false,
			BonusBounty:   500,
		},
		{
			Index:         4,
			Name:          "Starbase 01 Final Siege",
			Objective:     ObjectiveStarbaseSiege,
			Description:   "Defend Starbase 01 against a concentrated hostile fleet armada. Hold the line at all costs.",
			InitialDays:   40.0,
			HostileCount:  8,
			StarbaseCount: 1,
			Anomalies:     true,
			BonusBounty:   750,
		},
	}
}

func NewTour(seed int64) *TourState {
	if seed == 0 {
		seed = time.Now().UnixNano()
	}
	return &TourState{
		ID:                 TourID(fmt.Sprintf("tour-%d", seed)),
		Active:             true,
		CurrentSectorIndex: 0,
		Sectors:            defaultSectors(),
		RequisitionPoints:  0,
		InstalledRefits:    make(map[RefitID]int),
		TotalTourScore:     0,
		SectorsCompleted:   0,
		InDrydock:          false,
		Completed:          false,
		Failed:             false,
		Medals:             make([]string, 0),
		Seed:               seed,
	}
}

func (t *TourState) CurrentSector() *TourSectorConfig {
	if t.CurrentSectorIndex < 0 || t.CurrentSectorIndex >= len(t.Sectors) {
		return nil
	}
	return &t.Sectors[t.CurrentSectorIndex]
}

func (t *TourState) StartCurrentSector() *GameState {
	sec := t.CurrentSector()
	if sec == nil {
		return nil
	}
	sectorSeed := t.Seed + int64(t.CurrentSectorIndex*1000)
	g := NewGameWithSeed(sectorSeed)
	g.Options.Difficulty = DifficultyNormal
	g.DaysRemaining = sec.InitialDays
	g.TimeRemaining = sec.InitialDays
	g.KlingonsRemaining = sec.HostileCount
	g.RemainingKlingons = sec.HostileCount

	// Apply any installed refits to starting game state
	ApplyRefits(g, t.InstalledRefits)

	t.CurrentGameState = g
	t.InDrydock = false
	return g
}

func (t *TourState) EvaluateSector() (cleared bool, failed bool, bounty int) {
	g := t.CurrentGameState
	if g == nil {
		return false, false, 0
	}

	// Permadeath check
	if g.GameOver && g.GameOverReason != GameOverWon {
		t.Active = false
		t.Failed = true
		t.FailureReason = g.GameOverReason
		return false, true, 0
	}

	sec := t.CurrentSector()
	if sec == nil {
		return false, false, 0
	}

	// Sector cleared when hostiles remaining reach zero
	if g.KlingonsRemaining <= 0 {
		baseBounty := 1000
		stardateBonus := int(g.DaysRemaining * 50)
		flawlessBonus := 0
		hasDamage := false
		for _, dev := range g.Enterprise.Damage {
			if dev > 0 {
				hasDamage = true
				break
			}
		}
		if !hasDamage {
			for _, dev := range g.Enterprise.Devices {
				if dev > 0 {
					hasDamage = true
					break
				}
			}
		}
		if !hasDamage {
			flawlessBonus = 250
		}
		bounty = baseBounty + stardateBonus + flawlessBonus + sec.BonusBounty
		return true, false, bounty
	}

	return false, false, 0
}

func (t *TourState) AdvanceToDrydock(bounty int) {
	t.InDrydock = true
	t.RequisitionPoints += bounty
	t.SectorsCompleted++
	if t.CurrentGameState != nil {
		t.TotalTourScore += t.CurrentGameState.Score
	}
}

func (t *TourState) DisembarkToNextSector() (*GameState, error) {
	if !t.InDrydock {
		return nil, fmt.Errorf("cannot disembark: ship is not docked in drydock")
	}
	if t.CurrentSectorIndex+1 >= len(t.Sectors) {
		t.Completed = true
		t.Active = false
		t.InDrydock = false
		return nil, nil
	}
	t.CurrentSectorIndex++
	return t.StartCurrentSector(), nil
}
