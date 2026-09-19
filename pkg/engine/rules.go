package engine

// DifficultyProfile represents standard game difficulty presets.
type DifficultyProfile string

const (
	ProfileCasual    DifficultyProfile = "casual"
	ProfileNormal    DifficultyProfile = "normal"
	ProfileHardcore  DifficultyProfile = "hardcore"
	ProfileNightmare DifficultyProfile = "nightmare"
	ProfileCustom    DifficultyProfile = "custom"
)

// SurveillanceMode defines the extent of quadrant intelligence revealed upon starbase docking.
type SurveillanceMode string

const (
	SurveillanceFull     SurveillanceMode = "full"     // Reveals all 64 quadrants
	SurveillanceClassic  SurveillanceMode = "classic"  // Reveals 3x3 perimeters around all active starbases
	SurveillanceLocal    SurveillanceMode = "local"    // Reveals 3x3 perimeter around docked base only
	SurveillanceBlackout SurveillanceMode = "blackout" // Reveals no additional quadrants; starbases unmapped at start
)

// Animation speed constants for combat visual FX.
const (
	AnimSpeedOff       = 0
	AnimSpeedFast      = 1
	AnimSpeedNormal    = 2
	AnimSpeedCinematic = 3
)

// GameRules encapsulates difficulty and realism configuration parameters.
type GameRules struct {
	Profile           DifficultyProfile `json:"profile"`
	Surveillance      SurveillanceMode  `json:"surveillance"`
	SensorDegradation bool              `json:"sensor_degradation"`
	RepairMultiplier  float64           `json:"repair_multiplier"`
	KlingonCloak      bool              `json:"klingon_cloak"`
	SpatialAnomalies  bool              `json:"spatial_anomalies"`
	TimeMargin        float64           `json:"time_margin"`
	AnimSpeed         int               `json:"anim_speed"`
}

// DefaultRulesForProfile returns standard game rules for the specified difficulty profile.
func DefaultRulesForProfile(profile DifficultyProfile) GameRules {
	switch profile {
	case ProfileCasual:
		return GameRules{
			Profile:           ProfileCasual,
			Surveillance:      SurveillanceFull,
			SensorDegradation: false,
			RepairMultiplier:  0.75,
			KlingonCloak:      false,
			SpatialAnomalies:  false,
			TimeMargin:        1.25,
			AnimSpeed:         AnimSpeedNormal,
		}
	case ProfileHardcore:
		return GameRules{
			Profile:           ProfileHardcore,
			Surveillance:      SurveillanceLocal,
			SensorDegradation: true,
			RepairMultiplier:  1.50,
			KlingonCloak:      true,
			SpatialAnomalies:  true,
			TimeMargin:        0.80,
			AnimSpeed:         AnimSpeedNormal,
		}
	case ProfileNightmare:
		return GameRules{
			Profile:           ProfileNightmare,
			Surveillance:      SurveillanceBlackout,
			SensorDegradation: true,
			RepairMultiplier:  2.00,
			KlingonCloak:      true,
			SpatialAnomalies:  true,
			TimeMargin:        0.60,
			AnimSpeed:         AnimSpeedNormal,
		}
	case ProfileNormal:
		fallthrough
	default:
		return GameRules{
			Profile:           ProfileNormal,
			Surveillance:      SurveillanceClassic,
			SensorDegradation: true,
			RepairMultiplier:  1.00,
			KlingonCloak:      false,
			SpatialAnomalies:  false,
			TimeMargin:        1.00,
			AnimSpeed:         AnimSpeedNormal,
		}
	}
}
