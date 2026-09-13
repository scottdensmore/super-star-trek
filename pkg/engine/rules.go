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

// GameRules encapsulates difficulty and realism configuration parameters.
type GameRules struct {
	Profile           DifficultyProfile `json:"profile"`
	Surveillance      SurveillanceMode  `json:"surveillance"`
	SensorDegradation bool              `json:"sensor_degradation"`
	RepairMultiplier  float64           `json:"repair_multiplier"`
	KlingonCloak      bool              `json:"klingon_cloak"`
	TimeMargin        float64           `json:"time_margin"`
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
			TimeMargin:        1.25,
		}
	case ProfileHardcore:
		return GameRules{
			Profile:           ProfileHardcore,
			Surveillance:      SurveillanceLocal,
			SensorDegradation: true,
			RepairMultiplier:  1.50,
			KlingonCloak:      true,
			TimeMargin:        0.80,
		}
	case ProfileNightmare:
		return GameRules{
			Profile:           ProfileNightmare,
			Surveillance:      SurveillanceBlackout,
			SensorDegradation: true,
			RepairMultiplier:  2.00,
			KlingonCloak:      true,
			TimeMargin:        0.60,
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
			TimeMargin:        1.00,
		}
	}
}
