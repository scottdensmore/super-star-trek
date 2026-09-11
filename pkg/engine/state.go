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

// Klingon represents an enemy vessel within the current quadrant.
type Klingon struct {
	ID          int
	Sector      Coord
	Energy      float64
	IsCommander bool
}

// QuadrantState stores the layout and entities within the currently occupied quadrant.
type QuadrantState struct {
	Grid     [9][9]EntityType // 1..8 indexed
	Klingons []*Klingon
	Starbase *Coord
	Stars    []Coord
}

// GameState holds all mutable state for an active game session.
type GameState struct {
	RNG                *PRNG
	Skill              SkillLevel
	Length             GameLength
	Enterprise         Enterprise
	CurrentQuad        QuadrantState
	GalaxyChart        [9][9]int // Klingons*100 + Starbases*10 + Stars
	ChartDiscovered    [9][9]bool
	RemainingKlingons  int
	RemainingStarbases int
	Stardate           float64
	InitialStardate    float64
	TimeRemaining      float64
}

// NewGame initializes a new game session with deterministic initial state from the given seed.
func NewGame(seed int64, skill SkillLevel, length GameLength) *GameState {
	rng := NewPRNG(seed)
	g := &GameState{
		RNG:                rng,
		Skill:              skill,
		Length:             length,
		Stardate:           float64(2000 + rng.Intn(1000)),
		TimeRemaining:      30.0,
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
	return g
}
