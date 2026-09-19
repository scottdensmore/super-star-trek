package main

import (
	"fmt"
	"strings"

	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

const (
	ansiReset  = "\x1b[0m"
	ansiGreen  = "\x1b[32m"
	ansiYellow = "\x1b[33m"
	ansiRed    = "\x1b[31;1m"
	ansiCyan   = "\x1b[36m"
	ansiBold    = "\x1b[1m"
	ansiDim     = "\x1b[2m"
	ansiMagenta = "\x1b[35m"
)

var deviceNames = [engine.NumDevices]string{
	engine.DeviceWarp:          "Warp Engines",
	engine.DeviceSRSensors:     "Short Range Sensors",
	engine.DeviceLRSensors:     "Long Range Sensors",
	engine.DevicePhasers:       "Phaser Control",
	engine.DevicePhotonTubes:   "Photon Tubes",
	engine.DeviceDamageControl: "Damage Control",
	engine.DeviceShields:       "Shield Control",
	engine.DeviceComputer:      "Library Computer",
}

// FormatSRS renders an 8x8 sector grid paired with a telemetry status panel.
func FormatSRS(g *engine.GameState) string {
	if g == nil {
		return ""
	}
	var b strings.Builder
	fmt.Fprintf(&b, "%s=== SHORT RANGE SENSOR SCAN [%d,%d] ===%s\r\n", ansiCyan, g.Enterprise.Quad.Row(), g.Enterprise.Quad.Col(), ansiReset)

	for r := 1; r <= 8; r++ {
		fmt.Fprintf(&b, "%s%d%s ", ansiDim, r, ansiReset)
		for c := 1; c <= 8; c++ {
			coord := engine.Coord{r, c}
			if coord == g.Enterprise.Sector {
				fmt.Fprintf(&b, "%s<E>%s ", ansiCyan, ansiReset)
			} else {
				switch g.CurrentQuad.Grid[r][c] {
				case engine.EntityEnterprise:
					fmt.Fprintf(&b, "%s<E>%s ", ansiCyan, ansiReset)
				case engine.EntityKlingon, engine.EntityCommander, engine.EntitySuperCommander:
					fmt.Fprintf(&b, "%s+K+%s ", ansiRed, ansiReset)
				case engine.EntityStarbase:
					fmt.Fprintf(&b, "%s>B<%s ", ansiYellow, ansiReset)
				case engine.EntityStar:
					fmt.Fprintf(&b, "%s * %s ", ansiGreen, ansiReset)
				case engine.EntityPlanet:
					fmt.Fprintf(&b, "%s O %s ", ansiGreen, ansiReset)
				case engine.EntityBlackHole:
					fmt.Fprintf(&b, "%s @ %s ", ansiCyan, ansiReset)
				case engine.EntityWormhole:
					fmt.Fprintf(&b, "%s>W<%s ", ansiCyan, ansiReset)
				default:
					b.WriteString(" .  ")
				}
			}
		}

		// Append telemetry sidebar
		switch r {
		case 1:
			fmt.Fprintf(&b, "  Stardate:   %.1f", g.Stardate)
		case 2:
			condColor := ansiGreen
			condWord := "GREEN"
			switch g.Enterprise.Condition {
			case engine.ConditionYellow:
				condColor = ansiYellow
				condWord = "YELLOW"
			case engine.ConditionRed:
				condColor = ansiRed
				condWord = "RED"
			case engine.ConditionDocked:
				condColor = ansiCyan
				condWord = "DOCKED"
			}
			fmt.Fprintf(&b, "  CONDITION:  %s%s%s", condColor, condWord, ansiReset)
		case 3:
			fmt.Fprintf(&b, "  Sector:     [%d,%d]", g.Enterprise.Sector.Row(), g.Enterprise.Sector.Col())
		case 4:
			fmt.Fprintf(&b, "  Energy:     %.0f / 5000", g.Enterprise.Energy)
		case 5:
			fmt.Fprintf(&b, "  Shields:    %.0f / 2500", g.Enterprise.Shields)
		case 6:
			fmt.Fprintf(&b, "  Torpedoes:  %d", g.Enterprise.Torpedoes)
		case 7:
			fmt.Fprintf(&b, "  Klingons:   %d", g.RemainingKlingons)
		case 8:
			fmt.Fprintf(&b, "  Starbases:  %d", g.RemainingStarbases)
		}
		b.WriteString("\r\n")
	}
	return b.String()
}

// FormatLRS renders a 3x3 surrounding quadrant radar scan.
func FormatLRS(g *engine.GameState) string {
	if g == nil {
		return ""
	}
	var b strings.Builder
	fmt.Fprintf(&b, "%s=== LONG RANGE SENSOR SCAN ===%s\r\n", ansiCyan, ansiReset)
	b.WriteString("-------------------\r\n")
	eq := g.Enterprise.Quad
	for dr := -1; dr <= 1; dr++ {
		b.WriteString(": ")
		for dc := -1; dc <= 1; dc++ {
			qr := eq.Row() + dr
			qc := eq.Col() + dc
			if qr < 1 || qr > 8 || qc < 1 || qc > 8 {
				b.WriteString("*** : ")
			} else {
				val := g.GalaxyChart[qr][qc]
				fmt.Fprintf(&b, "%03d : ", val)
			}
		}
		b.WriteString("\r\n-------------------\r\n")
	}
	return b.String()
}

// FormatChart renders the 8x8 galactic star chart.
func FormatChart(g *engine.GameState) string {
	if g == nil {
		return ""
	}
	var b strings.Builder
	fmt.Fprintf(&b, "%s=== GALACTIC STAR CHART ===%s\r\n", ansiCyan, ansiReset)
	b.WriteString("    1   2   3   4   5   6   7   8\r\n")
	b.WriteString("  ---------------------------------\r\n")
	for r := 1; r <= 8; r++ {
		fmt.Fprintf(&b, "%d |", r)
		for c := 1; c <= 8; c++ {
			if r == g.Enterprise.Quad.Row() && c == g.Enterprise.Quad.Col() {
				fmt.Fprintf(&b, "%s%03d%s|", ansiCyan, g.GalaxyChart[r][c], ansiReset)
			} else if g.ChartDiscovered[r][c] {
				fmt.Fprintf(&b, "%03d|", g.GalaxyChart[r][c])
			} else if g.ChartKnownBases[r][c] {
				fmt.Fprintf(&b, "%s.B.%s|", ansiYellow, ansiReset)
			} else {
				b.WriteString("...|")
			}
		}
		b.WriteString("\r\n  ---------------------------------\r\n")
	}
	return b.String()
}

// FormatDamages renders damaged subsystem statuses.
func FormatDamages(g *engine.GameState) string {
	if g == nil {
		return ""
	}
	var b strings.Builder
	fmt.Fprintf(&b, "%s=== DAMAGE CONTROL REPORT ===%s\r\n", ansiCyan, ansiReset)
	for dev := engine.DeviceID(0); dev < engine.NumDevices; dev++ {
		status := fmt.Sprintf("%sOPERATIONAL%s", ansiGreen, ansiReset)
		dmg := g.Enterprise.Devices[dev]
		if dmg > 0 {
			status = fmt.Sprintf("%sDAMAGED (Repair in %.1f stardates)%s", ansiRed, dmg, ansiReset)
		}
		name := deviceNames[dev]
		fmt.Fprintf(&b, "%-22s : %s\r\n", name, status)
	}
	return b.String()
}

// FormatCombatEvents formats combat events into teletype output.
func FormatCombatEvents(events []engine.Event) string {
	var b strings.Builder
	for _, ev := range events {
		switch e := ev.(type) {
		case engine.EventTorpedoFired:
			fmt.Fprintf(&b, "%s[TORPEDO TRACK]%s Fired on course %.2f from [%d,%d]\r\n", ansiYellow, ansiReset, e.Angle, e.Origin.Row(), e.Origin.Col())
		case engine.EventTorpedoHit:
			if e.Destroyed {
				fmt.Fprintf(&b, "%s*** KLINGON WARSHIP DESTROYED AT [%d,%d] ***%s\r\n", ansiRed, e.Target.Row(), e.Target.Col(), ansiReset)
			} else {
				fmt.Fprintf(&b, "Torpedo hit Klingon at [%d,%d]: %.0f units damage\r\n", e.Target.Row(), e.Target.Col(), e.Damage)
			}
		case engine.EventPhaserFired:
			fmt.Fprintf(&b, "%s[PHASERS FIRED]%s Allocated %.0f units\r\n", ansiYellow, ansiReset, e.Energy)
		case engine.EventPhaserHit:
			if e.Destroyed {
				fmt.Fprintf(&b, "%s*** KLINGON DESTROYED BY PHASER FIRE ***%s\r\n", ansiRed, ansiReset)
			} else {
				fmt.Fprintf(&b, "Phaser beam hit target at [%d,%d]: %.0f units\r\n", e.Target.Row(), e.Target.Col(), e.Damage)
			}
		case engine.EventShieldTransfer:
			fmt.Fprintf(&b, "Deflector Shields: %.0f  |  Total energy: %.0f\r\n", e.NewShields, e.NewEnergy)
		case engine.EventShipMoved:
			if e.FromQuad != e.ToQuad {
				fmt.Fprintf(&b, "%s[WARP ENGINES ENGAGED]%s Arrived at quadrant [%d,%d] sector [%d,%d]\r\n", ansiGreen, ansiReset, e.ToQuad.Row(), e.ToQuad.Col(), e.ToSector.Row(), e.ToSector.Col())
			} else {
				fmt.Fprintf(&b, "%s[WARP ENGINES ENGAGED]%s Arrived at sector [%d,%d]\r\n", ansiGreen, ansiReset, e.ToSector.Row(), e.ToSector.Col())
			}
		case engine.EventObstacleEncountered:
			fmt.Fprintf(&b, "%s[NAVIGATION HAZARD]%s Blocked at sector [%d,%d]\r\n", ansiRed, ansiReset, e.Sector.Row(), e.Sector.Col())
		case engine.EventDocked:
			fmt.Fprintf(&b, "%s[STARBASE DOCKING]%s Docked with Starbase at [%d,%d]. Shields dropped, energy and torpedoes replenished.\r\n", ansiCyan, ansiReset, e.Starbase.Row(), e.Starbase.Col())
		case engine.EventKlingonCounterAttack:
			fmt.Fprintf(&b, "%s[RETURN FIRE]%s Klingon returns fire: %.0f units damage\r\n", ansiRed, ansiReset, e.Damage)
		case engine.EventSubsystemDamaged:
			name := "Subsystem"
			if e.Device >= 0 && int(e.Device) < len(deviceNames) {
				name = deviceNames[e.Device]
			}
			fmt.Fprintf(&b, "%s[DAMAGE]%s %s damaged! Repair in %.1f stardates\r\n", ansiRed, ansiReset, name, e.RepairTime)
		case engine.EventSubsystemRepaired:
			name := "Subsystem"
			if e.Device >= 0 && int(e.Device) < len(deviceNames) {
				name = deviceNames[e.Device]
			}
			fmt.Fprintf(&b, "%s[REPAIR]%s %s has been repaired.\r\n", ansiGreen, ansiReset, name)
		case engine.EventHazardTriggered:
			fmt.Fprintf(&b, "%s*** HAZARD: %s ***%s\r\n", ansiYellow, e.Description, ansiReset)
		case engine.EventSingularityAbsorption:
			fmt.Fprintf(&b, "%s*** GRAVITATIONAL SINGULARITY: Photon torpedo absorbed into event horizon! ***%s\r\n", ansiMagenta, ansiReset)
		case engine.EventWormholeJump:
			fmt.Fprintf(&b, "%s*** SUBSPACE RIFT: Wormhole transit completed to Quadrant [%d, %d] Sector [%d, %d]! ***%s\r\n", ansiCyan, e.ToQuad[0], e.ToQuad[1], e.ToSector[0], e.ToSector[1], ansiReset)
		case engine.EventAnomalyDiscovered:
			switch e.Env {
			case engine.EnvNebula:
				fmt.Fprintf(&b, "%s*** ENVIRONMENT ALERT: Entering Mutara Nebula. Electromagnetic dispersion drops shields to 0! ***%s\r\n", ansiMagenta, ansiReset)
			case engine.EnvIonStorm:
				fmt.Fprintf(&b, "%s*** ENVIRONMENT ALERT: Entering Ion Storm! High energy particle flux detected. ***%s\r\n", ansiYellow, ansiReset)
			default:
				fmt.Fprintf(&b, "%s*** ENVIRONMENT ALERT: Spatial anomaly detected! ***%s\r\n", ansiMagenta, ansiReset)
			}
		case engine.EventGameOver:
			if e.Reason == engine.GameOverWon {
				fmt.Fprintf(&b, "%s*** FEDERATION MISSION ACCOMPLISHED ***%s (Score: %.0f)\r\n", ansiGreen, ansiReset, e.Score)
			} else {
				fmt.Fprintf(&b, "%s*** MISSION TERMINATED ***%s (Score: %.0f)\r\n", ansiRed, ansiReset, e.Score)
			}
		}
	}
	return b.String()
}

// FormatScenarios formats the catalog of available tactical scenarios.
func FormatScenarios() string {
	var b strings.Builder
	fmt.Fprintf(&b, "%s=== TACTICAL SCENARIOS ===%s\r\n", ansiCyan, ansiReset)
	for _, sc := range engine.ListScenarios() {
		fmt.Fprintf(&b, "  %s%-16s%s [%s] - %s\r\n", ansiYellow, sc.ID, ansiReset, sc.Difficulty, sc.Name)
		fmt.Fprintf(&b, "    %s%s%s\r\n", ansiDim, sc.Description, ansiReset)
	}
	b.WriteString("\r\nType 'scenario <id>' to launch.\r\n")
	return b.String()
}

// FormatScenarioBriefing formats a tactical mission briefing for terminal output.
func FormatScenarioBriefing(sc *engine.Scenario) string {
	if sc == nil {
		return ""
	}
	var b strings.Builder
	fmt.Fprintf(&b, "%s=== MISSION BRIEFING: %s ===%s\r\n", ansiCyan, strings.ToUpper(sc.Name), ansiReset)
	if sc.Subtitle != "" {
		fmt.Fprintf(&b, "%s%s%s\r\n\r\n", ansiYellow, sc.Subtitle, ansiReset)
	}
	for _, line := range sc.Briefing {
		fmt.Fprintf(&b, "%s\r\n", line)
	}
	b.WriteString("\r\n")
	return b.String()
}

