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
			Name:          "Neutral Zone Patrol",
			Objective:     ObjectiveBorderPatrol,
			Description:   "Eliminate cloaked Romulan incursions infiltrating Federation border space.",
			InitialDays:   30.0,
			HostileCount:  2,
			StarbaseCount: 1,
			Anomalies:     false,
			BonusBounty:   250,
		},
		{
			Index:         2,
			Name:          "Border Outpost Defense",
			Objective:     ObjectiveDeepSurveillance,
			Description:   "Neutralize Tholian web spinners constructing energy barriers around border outposts.",
			InitialDays:   32.0,
			HostileCount:  2,
			StarbaseCount: 1,
			Anomalies:     true,
			BonusBounty:   350,
		},
		{
			Index:         3,
			Name:          "Commander Decapitation",
			Objective:     ObjectiveConvoyEscort,
			Description:   "Eliminate the Klingon commander flagship and its escort wolf-pack.",
			InitialDays:   35.0,
			HostileCount:  3,
			StarbaseCount: 2,
			Anomalies:     false,
			BonusBounty:   500,
		},
		{
			Index:         4,
			Name:          "Invasion Fleet Interception",
			Objective:     ObjectiveStarbaseSiege,
			Description:   "Defend against a multi-faction armada of Klingon, Romulan, and Tholian warships.",
			InitialDays:   40.0,
			HostileCount:  4,
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

func (t *TourState) StartCurrentSector(indices ...int) (*GameState, error) {
	if len(indices) > 0 {
		t.CurrentSectorIndex = indices[0]
	}
	sec := t.CurrentSector()
	if sec == nil {
		return nil, fmt.Errorf("invalid sector index: %d", t.CurrentSectorIndex)
	}
	index := t.CurrentSectorIndex
	sectorSeed := t.Seed + int64(index*1000)
	g := NewGameWithSeed(sectorSeed)
	g.Options.Difficulty = DifficultyNormal
	g.DaysRemaining = sec.InitialDays
	g.TimeRemaining = sec.InitialDays
	g.KlingonsRemaining = sec.HostileCount
	g.RemainingKlingons = sec.HostileCount

	// Apply any installed refits to starting game state
	ApplyRefits(g, t.InstalledRefits)

	g.Enterprise.Energy = g.Enterprise.MaxEnergy
	g.Enterprise.Torpedoes = g.Enterprise.MaxTorpedoes
	g.Enterprise.Sector = Coord{4, 4}
	g.CurrentQuad.Grid[4][4] = EntityEnterprise

	switch index {
	case 0: // Sector 1: Neutral Zone (Romulans)
		g.CurrentQuad.Enemies = []*EnemyVessel{
			{ID: 1, Faction: FactionRomulan, Sector: Coord{2, 3}, Energy: 1000.0, Shields: 400.0, MaxEnergy: 1000.0, IsCloaked: true, CloakTurns: 2},
			{ID: 2, Faction: FactionRomulan, Sector: Coord{6, 7}, Energy: 1000.0, Shields: 400.0, MaxEnergy: 1000.0, IsCloaked: true, CloakTurns: 3},
		}
		g.CurrentQuad.Grid[2][3] = EntityEmpty // Cloaked
		g.CurrentQuad.Grid[6][7] = EntityEmpty // Cloaked
	case 1: // Sector 2: Border Outpost (Tholians)
		g.CurrentQuad.Enemies = []*EnemyVessel{
			{ID: 11, Faction: FactionTholian, Sector: Coord{1, 1}, Energy: 800.0, Shields: 300.0, SpecialState: 0},
			{ID: 12, Faction: FactionTholian, Sector: Coord{8, 8}, Energy: 800.0, Shields: 300.0, SpecialState: 1},
		}
		g.CurrentQuad.Grid[1][1] = EntityTholian
		g.CurrentQuad.Grid[8][8] = EntityTholian
	case 2: // Sector 3: Commander Decapitation (Klingon wolf-pack)
		g.CurrentQuad.Enemies = []*EnemyVessel{
			{ID: 21, Faction: FactionKlingon, Sector: Coord{4, 7}, Energy: 1200.0, Shields: 600.0, IsCommander: true},
			{ID: 22, Faction: FactionKlingon, Sector: Coord{3, 5}, Energy: 600.0, Shields: 250.0},
			{ID: 23, Faction: FactionKlingon, Sector: Coord{5, 5}, Energy: 600.0, Shields: 250.0},
		}
		g.CurrentQuad.Grid[4][7] = EntityCommander
		g.CurrentQuad.Grid[3][5] = EntityKlingon
		g.CurrentQuad.Grid[5][5] = EntityKlingon
	case 3: // Sector 4: Invasion Fleet (Multi-faction coalition)
		g.CurrentQuad.Enemies = []*EnemyVessel{
			{ID: 31, Faction: FactionKlingon, Sector: Coord{4, 8}, Energy: 1800.0, Shields: 800.0, IsCommander: true},
			{ID: 32, Faction: FactionRomulan, Sector: Coord{2, 6}, Energy: 1200.0, Shields: 500.0, IsCloaked: true, CloakTurns: 2},
			{ID: 33, Faction: FactionTholian, Sector: Coord{1, 2}, Energy: 800.0, Shields: 300.0, SpecialState: 0},
			{ID: 34, Faction: FactionTholian, Sector: Coord{8, 2}, Energy: 800.0, Shields: 300.0, SpecialState: 1},
		}
		g.CurrentQuad.Grid[4][8] = EntityCommander
		g.CurrentQuad.Grid[2][6] = EntityEmpty
		g.CurrentQuad.Grid[1][2] = EntityTholian
		g.CurrentQuad.Grid[8][2] = EntityTholian
	}
	SyncQuadrantEnemies(&g.CurrentQuad)

	if len(g.CurrentQuad.Enemies) > 0 {
		g.KlingonsRemaining = len(g.CurrentQuad.Enemies)
		g.RemainingKlingons = len(g.CurrentQuad.Enemies)
		g.Enterprise.Condition = ConditionRed
	}

	t.CurrentGameState = g
	t.InDrydock = false
	return g, nil
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
	return t.StartCurrentSector()
}
