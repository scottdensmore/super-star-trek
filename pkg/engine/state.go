package engine

// EntityType represents the type of object occupying a grid cell.
type EntityType int

const (
	EntityEmpty EntityType = iota
	EntityEnterprise
	EntityKlingon
	EntityCommander
	EntitySuperCommander
	EntityStarbase
	EntityStar
	EntityPlanet
	EntityBlackHole
)

// ConditionType represents the alert status of the Enterprise.
type ConditionType int

const (
	ConditionGreen ConditionType = iota
	ConditionYellow
	ConditionRed
	ConditionDocked
)

// DeviceID represents subsystem devices aboard the Enterprise.
type DeviceID int

const (
	DeviceWarp DeviceID = iota
	DeviceSRSensors
	DeviceLRSensors
	DevicePhasers
	DevicePhotonTubes
	DeviceDamageControl
	DeviceShields
	DeviceComputer
	NumDevices
)

// SkillLevel represents the game difficulty setting.
type SkillLevel int

const (
	SkillNovice SkillLevel = 1 + iota
	SkillFair
	SkillGood
	SkillExpert
	SkillEmeritus
)

// GameLength represents the duration/scope of the mission.
type GameLength int

const (
	LengthShort GameLength = 1 + iota
	LengthMedium
	LengthLong
)

// Enterprise holds the operational status, systems, and coordinates of USS Enterprise.
type Enterprise struct {
	Quad        Coord
	Sector      Coord
	Energy      float64
	Shields     float64
	Torpedoes   int
	Condition   ConditionType
	Devices     [NumDevices]float64 // 0 = operational, >0 = turns until repaired
	LifeSupport float64
}

// EnterpriseState is a type alias for Enterprise.
type EnterpriseState = Enterprise

// Klingon represents an enemy vessel within the current quadrant.
type Klingon struct {
	ID          int
	Sector      Coord
	Energy      float64
	IsCommander bool
	IsCloaked   bool
}

// QuadrantState stores the layout and entities within the currently occupied quadrant.
type QuadrantState struct {
	Grid     [9][9]EntityType // 1..8 indexed
	Klingons []*Klingon
	Starbase *Coord
	Stars    []Coord
}

// GameMetrics records cumulative mission counters for scoring.
type GameMetrics struct {
	KlingonsKilled        int `json:"klingons_killed"`         // Regular Klingons destroyed (+10 pts)
	CommandersKilled      int `json:"commanders_killed"`       // Commanders destroyed (+50 pts)
	SuperCommandersKilled int `json:"super_commanders_killed"` // Super-Commanders destroyed (+200 pts)
	RomulansKilled        int `json:"romulans_killed"`         // Romulans destroyed (+20 pts)
	RomulansSurrendered   int `json:"romulans_surrendered"`    // Romulans surrendered (+1 pt)
	StarbasesDestroyed    int `json:"starbases_destroyed"`     // Friendly starbases lost (-100 pts)
	StarsDestroyed        int `json:"stars_destroyed"`         // Stars destroyed by torpedoes (-5 pts)
	PlanetsDestroyed      int `json:"planets_destroyed"`       // Planets destroyed (-10 pts)
	Casualties            int `json:"casualties"`              // Crew casualties suffered (-1 pt)
	HelpCalls             int `json:"help_calls"`              // Distress calls to starbase (-45 pts)
	StarshipsLost         int `json:"starships_lost"`          // Starships lost (-100 pts each)
}

// GameState holds all mutable state for an active game session.
type GameState struct {
	RNG                *PRNG
	Rules              GameRules `json:"rules"`
	Skill              SkillLevel
	Length             GameLength
	Enterprise         Enterprise
	CurrentQuad        QuadrantState
	GalaxyChart        [9][9]int // Klingons*100 + Starbases*10 + Stars
	ChartDiscovered    [9][9]bool
	ChartKnownBases    [9][9]bool
	RemainingKlingons  int
	RemainingStarbases int
	Stardate           float64
	InitialStardate    float64
	TimeRemaining      float64
	Metrics            GameMetrics `json:"metrics"`
	GameWon            bool        `json:"game_won"`
}

// NewGame initializes a new game session with deterministic initial state from the given seed.
func NewGame(seed int64, skill SkillLevel, length GameLength) *GameState {
	return NewGameWithOptions(seed, skill, length, DefaultRulesForProfile(ProfileNormal))
}

// NewGameWithOptions initializes a new game session with specified rules and deterministic initial state from the given seed.
func NewGameWithOptions(seed int64, skill SkillLevel, length GameLength, rules GameRules) *GameState {
	rng := NewPRNG(seed)
	g := &GameState{
		RNG:                rng,
		Rules:              rules,
		Skill:              skill,
		Length:             length,
		Stardate:           float64(2000 + rng.Intn(1000)),
		TimeRemaining:      30.0 * rules.TimeMargin,
		RemainingKlingons:  15,
		RemainingStarbases: 3,
		Enterprise: Enterprise{
			Quad:      Coord{rng.Intn(8) + 1, rng.Intn(8) + 1},
			Sector:    Coord{rng.Intn(8) + 1, rng.Intn(8) + 1},
			Energy:    5000,
			Shields:   0,
			Torpedoes: 10,
			Condition: ConditionGreen,
		},
	}
	g.InitialStardate = g.Stardate

	// Procedural generation:
	// 1. Stars (1..9 in each quadrant)
	for r := 1; r <= 8; r++ {
		for c := 1; c <= 8; c++ {
			g.GalaxyChart[r][c] = rng.Intn(9) + 1
		}
	}

	// 2. Starbases across RemainingStarbases distinct quadrants
	placedBases := 0
	for placedBases < g.RemainingStarbases {
		r := rng.Intn(8) + 1
		c := rng.Intn(8) + 1
		if (g.GalaxyChart[r][c]%100)/10 == 0 {
			g.GalaxyChart[r][c] += 10
			if rules.Surveillance != SurveillanceBlackout {
				g.ChartKnownBases[r][c] = true
			}
			placedBases++
		}
	}

	// 3. Klingons distributed in clusters of 1..3
	klingonsToPlace := g.RemainingKlingons
	for klingonsToPlace > 0 {
		r := rng.Intn(8) + 1
		c := rng.Intn(8) + 1
		currentK := g.GalaxyChart[r][c] / 100
		if currentK < 9 {
			cluster := rng.Intn(3) + 1
			if cluster > klingonsToPlace {
				cluster = klingonsToPlace
			}
			if currentK+cluster > 9 {
				cluster = 9 - currentK
			}
			g.GalaxyChart[r][c] += cluster * 100
			klingonsToPlace -= cluster
		}
	}

	// 4. Starting quadrant discovered
	g.ChartDiscovered[g.Enterprise.Quad[0]][g.Enterprise.Quad[1]] = true

	return g
}

// PopulateQuadrant populates CurrentQuad with the entities (stars, starbases, Klingons)
// specified in GalaxyChart for the given quadrant coordinates, and places the Enterprise at entSector.
func (g *GameState) PopulateQuadrant(quad Coord, entSector Coord) {
	if g == nil {
		return
	}
	if quad[0] < 1 || quad[0] > 8 || quad[1] < 1 || quad[1] > 8 {
		return
	}
	if entSector[0] < 1 || entSector[0] > 8 || entSector[1] < 1 || entSector[1] > 8 {
		entSector = Coord{4, 4}
	}
	if g.RNG == nil {
		g.RNG = NewPRNG(12345)
	}

	g.CurrentQuad = QuadrantState{}
	g.CurrentQuad.Grid[entSector[0]][entSector[1]] = EntityEnterprise
	g.Enterprise.Quad = quad
	g.Enterprise.Sector = entSector

	val := g.GalaxyChart[quad[0]][quad[1]]
	numK := val / 100
	numB := (val % 100) / 10
	numS := val % 10

	findEmptySector := func() Coord {
		for {
			r := g.RNG.Intn(8) + 1
			c := g.RNG.Intn(8) + 1
			if g.CurrentQuad.Grid[r][c] == EntityEmpty {
				return Coord{r, c}
			}
		}
	}

	if numB > 0 {
		sb := findEmptySector()
		g.CurrentQuad.Starbase = &sb
		g.CurrentQuad.Grid[sb[0]][sb[1]] = EntityStarbase
	}

	if numK > 0 {
		g.CurrentQuad.Klingons = make([]*Klingon, 0, numK)
		for i := 0; i < numK; i++ {
			kCoord := findEmptySector()
			isCommander := false
			if g.Rules.KlingonCloak && i == 0 {
				isCommander = true
			}
			entType := EntityKlingon
			if isCommander {
				entType = EntityCommander
			}
			g.CurrentQuad.Grid[kCoord[0]][kCoord[1]] = entType
			energy := 300.0 + g.RNG.Float64()*150.0 + 25.0*float64(g.Skill)
			if isCommander {
				energy = 950.0 + 400.0*g.RNG.Float64() + 50.0*float64(g.Skill)
			}
			k := &Klingon{
				ID:          i + 1,
				Sector:      kCoord,
				Energy:      energy,
				IsCommander: isCommander,
				IsCloaked:   false,
			}
			g.CurrentQuad.Klingons = append(g.CurrentQuad.Klingons, k)
		}
	}

	if numS > 0 {
		g.CurrentQuad.Stars = make([]Coord, 0, numS)
		for i := 0; i < numS; i++ {
			sCoord := findEmptySector()
			g.CurrentQuad.Stars = append(g.CurrentQuad.Stars, sCoord)
			g.CurrentQuad.Grid[sCoord[0]][sCoord[1]] = EntityStar
		}
	}

	g.ChartDiscovered[quad[0]][quad[1]] = true

	if numK > 0 {
		g.Enterprise.Condition = ConditionRed
	} else {
		g.Enterprise.Condition = ConditionGreen
	}
}
