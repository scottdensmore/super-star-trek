package engine

// Event represents a typed occurrence resulting from an executed Action.
type Event interface {
	EventType() string
}

// EventShieldTransfer is emitted when energy is transferred between ship's reserves and shields.
type EventShieldTransfer struct {
	NewShields float64
	NewEnergy  float64
}

// EventType returns the type name for EventShieldTransfer.
func (e EventShieldTransfer) EventType() string { return "ShieldTransfer" }

// EventTorpedoFired is emitted when a photon torpedo is launched.
type EventTorpedoFired struct {
	Origin Coord
	Angle  float64
}

// EventType returns the type name for EventTorpedoFired.
func (e EventTorpedoFired) EventType() string { return "TorpedoFired" }

// EventTorpedoHit is emitted when a photon torpedo strikes an entity or obstacle.
type EventTorpedoHit struct {
	Target    Coord
	Entity    EntityType
	Damage    float64
	Destroyed bool
}

// EventType returns the type name for EventTorpedoHit.
func (e EventTorpedoHit) EventType() string { return "TorpedoHit" }

// EventPhaserFired is emitted when ship phasers are discharged.
type EventPhaserFired struct {
	Energy float64
}

// EventType returns the type name for EventPhaserFired.
func (e EventPhaserFired) EventType() string { return "PhaserFired" }

// EventPhaserHit is emitted when phaser energy impacts a target vessel.
type EventPhaserHit struct {
	Target    Coord
	KlingonID int
	Damage    float64
	Destroyed bool
}

// EventType returns the type name for EventPhaserHit.
func (e EventPhaserHit) EventType() string { return "PhaserHit" }

// EventDocked is emitted when Enterprise docks at a starbase.
type EventDocked struct {
	Starbase Coord
}

// EventType returns the type name for EventDocked.
func (e EventDocked) EventType() string { return "Docked" }

// EventShipMoved is emitted when Enterprise completes a movement maneuver.
type EventShipMoved struct {
	FromQuad   Coord
	ToQuad     Coord
	FromSector Coord
	ToSector   Coord
	Warp       float64
	EnergyUsed float64
	TimeUsed   float64
}

// EventType returns the type name for EventShipMoved.
func (e EventShipMoved) EventType() string { return "ShipMoved" }

// EventObstacleEncountered is emitted when Enterprise movement is blocked by an obstacle.
type EventObstacleEncountered struct {
	Sector Coord
	Entity EntityType
}

// EventType returns the type name for EventObstacleEncountered.
func (e EventObstacleEncountered) EventType() string { return "ObstacleEncountered" }

// EventConditionChanged is emitted when Enterprise alert status changes.
type EventConditionChanged struct {
	From ConditionType
	To   ConditionType
}

// EventType returns the type name for EventConditionChanged.
func (e EventConditionChanged) EventType() string { return "ConditionChanged" }

// GameOverReason identifies the cause of game termination.
type GameOverReason int

const (
	GameOverWon GameOverReason = iota
	GameOverEnergy
	GameOverDestroyed
	GameOverTime
	GameOverStranded
	GameOverLost
)

// EventGameOver is emitted when the game ends.
type EventGameOver struct {
	Reason GameOverReason
	Score  float64
}

// EventType returns the type name for EventGameOver.
func (e EventGameOver) EventType() string { return "GameOver" }

// EventKlingonCounterAttack is emitted when enemy ships return fire.
type EventKlingonCounterAttack struct {
	EnemyID      int
	Damage       float64
	ShieldDamage float64
	HullDamage   float64
}

// EventType returns the type name for EventKlingonCounterAttack.
func (e EventKlingonCounterAttack) EventType() string { return "KlingonCounterAttack" }

// EventSubsystemDamaged is emitted when a ship subsystem is damaged.
type EventSubsystemDamaged struct {
	Device     DeviceID
	RepairTime float64
}

// EventType returns the type name for EventSubsystemDamaged.
func (e EventSubsystemDamaged) EventType() string { return "SubsystemDamaged" }

// EventSubsystemRepaired is emitted when a ship subsystem is repaired.
type EventSubsystemRepaired struct {
	Device DeviceID
}

// EventType returns the type name for EventSubsystemRepaired.
func (e EventSubsystemRepaired) EventType() string { return "SubsystemRepaired" }

// EventLRScanCompleted is emitted when a long-range scan completes.
type EventLRScanCompleted struct {
	CenterQuad    Coord
	ScannedQuads  []Coord
	RelayedByBase bool
	Degraded      bool
	Readings      [3][3]int
	ReadingsMap   map[Coord]int
}

// EventType returns the type name for EventLRScanCompleted.
func (e EventLRScanCompleted) EventType() string { return "LRScanCompleted" }

// EventStarbaseSurveillance is emitted when starbase records update the galactic star chart.
type EventStarbaseSurveillance struct {
	StarbaseCoord Coord
	UpdatedQuads  int
	Mode          SurveillanceMode
}

// EventType returns the type name for EventStarbaseSurveillance.
func (e EventStarbaseSurveillance) EventType() string { return "StarbaseSurveillance" }

// EventKlingonCloakState is emitted when a Klingon vessel cloaks or decloaks.
type EventKlingonCloakState struct {
	KlingonID int
	Cloaked   bool
}

// EventType returns the type name for EventKlingonCloakState.
func (e EventKlingonCloakState) EventType() string { return "KlingonCloakState" }

// EventHelpCalled is emitted when Enterprise calls for assistance.
type EventHelpCalled struct{}

// EventType returns the type name for EventHelpCalled.
func (e EventHelpCalled) EventType() string { return "HelpCalled" }

// EventHazardTriggered is emitted when an environmental hazard affects the ship.
type EventHazardTriggered struct {
	HazardType  string
	Description string
	EnergyDrain float64
}

// EventType returns the type name for EventHazardTriggered.
func (e EventHazardTriggered) EventType() string { return "HazardTriggered" }

// EventWormholeJump is emitted upon entering a wormhole transit rift.
type EventWormholeJump struct {
	FromQuad   Coord
	FromSector Coord
	ToQuad     Coord
	ToSector   Coord
	TimeDelta  float64
}

// EventType returns the type name for EventWormholeJump.
func (e EventWormholeJump) EventType() string { return "WormholeJump" }

// EventSingularityAbsorption is emitted when an entity or weapon is pulled into a black hole.
type EventSingularityAbsorption struct {
	Sector Coord
	Target EntityType
	Weapon string
}

// EventType returns the type name for EventSingularityAbsorption.
func (e EventSingularityAbsorption) EventType() string { return "SingularityAbsorption" }

// EventAnomalyDiscovered is emitted when an anomaly is encountered.
type EventAnomalyDiscovered struct {
	Quad Coord
	Env  EnvironmentType
}

// EventType returns the type name for EventAnomalyDiscovered.
func (e EventAnomalyDiscovered) EventType() string { return "AnomalyDiscovered" }
