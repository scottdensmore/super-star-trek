package engine

import (
	"math"
)

// ScoreBreakdown contains the itemized scoring metrics, penalties, and rank evaluation.
type ScoreBreakdown struct {
	KlingonsKilled        int
	KlingonPoints         int
	CommandersKilled      int
	CommanderPoints       int
	SuperCommandersKilled int
	SuperCommanderPoints  int
	RomulansKilled        int
	RomulanPoints         int
	RomulansSurrendered   int
	SurrenderedPoints     int
	ElapsedStardates      float64
	KillRate              float64
	KillRatePoints        int
	GameWon               bool
	WinBonus              int
	StarbasesLost         int
	StarbasePenalty       int
	StarshipsLost         int
	StarshipPenalty       int
	HelpCalls             int
	HelpPenalty           int
	PlanetsDestroyed      int
	PlanetPenalty         int
	StarsDestroyed        int
	StarPenalty           int
	Casualties            int
	CasualtyPenalty       int
	TotalScore            int
	RankTitle             string
	RankBadge             string
}

// ComputeScore calculates the authentic 15-rule score breakdown matching classic rules.c:score_compute.
func ComputeScore(g *GameState, won bool) ScoreBreakdown {
	if g == nil {
		return ScoreBreakdown{}
	}

	elapsed := g.Stardate - g.InitialStardate
	// Classic rules: if game was not won (or elapsed <= 0) and elapsed < 5.0, clamp to 5.0
	if (!won || elapsed <= 0) && elapsed < 5.0 {
		elapsed = 5.0
	}

	klingonPts := g.Metrics.KlingonsKilled * 10
	commanderPts := g.Metrics.CommandersKilled * 50
	superCommanderPts := g.Metrics.SuperCommandersKilled * 200
	romulanPts := g.Metrics.RomulansKilled * 20
	surrenderedPts := g.Metrics.RomulansSurrendered * 1

	totalKills := g.Metrics.KlingonsKilled + g.Metrics.CommandersKilled + g.Metrics.SuperCommandersKilled
	var killRate float64
	if elapsed > 0 {
		killRate = float64(totalKills) / elapsed
	}
	killRatePts := int(math.Round(500.0 * killRate))

	winBonus := 0
	if won {
		winBonus = 100 * int(g.Skill)
	}

	starbasePenalty := g.Metrics.StarbasesDestroyed * 100
	starshipPenalty := g.Metrics.StarshipsLost * 100
	helpPenalty := g.Metrics.HelpCalls * 45
	planetPenalty := g.Metrics.PlanetsDestroyed * 10
	starPenalty := g.Metrics.StarsDestroyed * 5
	casualtyPenalty := g.Metrics.Casualties * 1

	total := klingonPts + commanderPts + superCommanderPts + romulanPts + surrenderedPts +
		killRatePts + winBonus - starbasePenalty - starshipPenalty - helpPenalty -
		planetPenalty - starPenalty - casualtyPenalty

	title, badge := RankForScore(total)

	return ScoreBreakdown{
		KlingonsKilled:        g.Metrics.KlingonsKilled,
		KlingonPoints:         klingonPts,
		CommandersKilled:      g.Metrics.CommandersKilled,
		CommanderPoints:       commanderPts,
		SuperCommandersKilled: g.Metrics.SuperCommandersKilled,
		SuperCommanderPoints:  superCommanderPts,
		RomulansKilled:        g.Metrics.RomulansKilled,
		RomulanPoints:         romulanPts,
		RomulansSurrendered:   g.Metrics.RomulansSurrendered,
		SurrenderedPoints:     surrenderedPts,
		ElapsedStardates:      elapsed,
		KillRate:              killRate,
		KillRatePoints:        killRatePts,
		GameWon:               won,
		WinBonus:              winBonus,
		StarbasesLost:         g.Metrics.StarbasesDestroyed,
		StarbasePenalty:       starbasePenalty,
		StarshipsLost:         g.Metrics.StarshipsLost,
		StarshipPenalty:       starshipPenalty,
		HelpCalls:             g.Metrics.HelpCalls,
		HelpPenalty:           helpPenalty,
		PlanetsDestroyed:      g.Metrics.PlanetsDestroyed,
		PlanetPenalty:         planetPenalty,
		StarsDestroyed:        g.Metrics.StarsDestroyed,
		StarPenalty:           starPenalty,
		Casualties:            g.Metrics.Casualties,
		CasualtyPenalty:       casualtyPenalty,
		TotalScore:            total,
		RankTitle:             title,
		RankBadge:             badge,
	}
}

// RankForScore determines the Starfleet officer rank title and badge based on the player's net score.
func RankForScore(score int) (title string, badge string) {
	switch {
	case score < 0:
		return "Dishonorable Discharge", "[DISHONOR]"
	case score < 100:
		return "Starfleet Cadet", "[CADET]"
	case score < 200:
		return "Lieutenant", "[LT]"
	case score < 350:
		return "Commander", "[CDR]"
	case score < 500:
		return "Captain", "[CAPT]"
	case score < 750:
		return "Commodore", "[COMM]"
	case score < 1000:
		return "Rear Admiral", "[RADM]"
	default:
		return "Fleet Admiral", "[FADM]"
	}
}
