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
	ansiBold   = "\x1b[1m"
	ansiDim    = "\x1b[2m"
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
	b.WriteString(fmt.Sprintf("%s=== SHORT RANGE SENSOR SCAN [%d,%d] ===%s\r\n", ansiCyan, g.Enterprise.Quad.Row(), g.Enterprise.Quad.Col(), ansiReset))

	for r := 1; r <= 8; r++ {
		b.WriteString(fmt.Sprintf("%s%d%s ", ansiDim, r, ansiReset))
		for c := 1; c <= 8; c++ {
			coord := engine.Coord{r, c}
			if coord == g.Enterprise.Sector {
				b.WriteString(fmt.Sprintf("%s<E>%s ", ansiCyan, ansiReset))
			} else {
				switch g.CurrentQuad.Grid[r][c] {
				case engine.EntityEnterprise:
					b.WriteString(fmt.Sprintf("%s<E>%s ", ansiCyan, ansiReset))
				case engine.EntityKlingon, engine.EntityCommander, engine.EntitySuperCommander:
					b.WriteString(fmt.Sprintf("%s+K+%s ", ansiRed, ansiReset))
				case engine.EntityStarbase:
					b.WriteString(fmt.Sprintf("%s>B<%s ", ansiYellow, ansiReset))
				case engine.EntityStar:
					b.WriteString(fmt.Sprintf("%s * %s ", ansiGreen, ansiReset))
				case engine.EntityPlanet:
					b.WriteString(fmt.Sprintf("%s O %s ", ansiGreen, ansiReset))
				case engine.EntityBlackHole:
					b.WriteString(fmt.Sprintf("%s @ %s ", ansiCyan, ansiReset))
				default:
					b.WriteString(" .  ")
				}
			}
		}

		// Append telemetry sidebar
		switch r {
		case 1:
			b.WriteString(fmt.Sprintf("  Stardate:   %.1f", g.Stardate))
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
			b.WriteString(fmt.Sprintf("  CONDITION:  %s%s%s", condColor, condWord, ansiReset))
		case 3:
			b.WriteString(fmt.Sprintf("  Sector:     [%d,%d]", g.Enterprise.Sector.Row(), g.Enterprise.Sector.Col()))
		case 4:
			b.WriteString(fmt.Sprintf("  Energy:     %.0f / 5000", g.Enterprise.Energy))
		case 5:
			b.WriteString(fmt.Sprintf("  Shields:    %.0f / 2500", g.Enterprise.Shields))
		case 6:
			b.WriteString(fmt.Sprintf("  Torpedoes:  %d", g.Enterprise.Torpedoes))
		case 7:
			b.WriteString(fmt.Sprintf("  Klingons:   %d", g.RemainingKlingons))
		case 8:
			b.WriteString(fmt.Sprintf("  Starbases:  %d", g.RemainingStarbases))
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
	b.WriteString(fmt.Sprintf("%s=== LONG RANGE SENSOR SCAN ===%s\r\n", ansiCyan, ansiReset))
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
				b.WriteString(fmt.Sprintf("%03d : ", val))
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
	b.WriteString(fmt.Sprintf("%s=== GALACTIC STAR CHART ===%s\r\n", ansiCyan, ansiReset))
	b.WriteString("    1   2   3   4   5   6   7   8\r\n")
	b.WriteString("  ---------------------------------\r\n")
	for r := 1; r <= 8; r++ {
		b.WriteString(fmt.Sprintf("%d |", r))
		for c := 1; c <= 8; c++ {
			if r == g.Enterprise.Quad.Row() && c == g.Enterprise.Quad.Col() {
				b.WriteString(fmt.Sprintf("%s%03d%s|", ansiCyan, g.GalaxyChart[r][c], ansiReset))
			} else if g.ChartDiscovered[r][c] {
				b.WriteString(fmt.Sprintf("%03d|", g.GalaxyChart[r][c]))
			} else if g.ChartKnownBases[r][c] {
				b.WriteString(fmt.Sprintf("%s.B.%s|", ansiYellow, ansiReset))
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
	b.WriteString(fmt.Sprintf("%s=== DAMAGE CONTROL REPORT ===%s\r\n", ansiCyan, ansiReset))
	for dev := engine.DeviceID(0); dev < engine.NumDevices; dev++ {
		status := fmt.Sprintf("%sOPERATIONAL%s", ansiGreen, ansiReset)
		dmg := g.Enterprise.Devices[dev]
		if dmg > 0 {
			status = fmt.Sprintf("%sDAMAGED (Repair in %.1f stardates)%s", ansiRed, dmg, ansiReset)
		}
		name := deviceNames[dev]
		b.WriteString(fmt.Sprintf("%-22s : %s\r\n", name, status))
	}
	return b.String()
}

// FormatCombatEvents formats combat events into teletype output.
func FormatCombatEvents(events []engine.Event) string {
	var b strings.Builder
	for _, ev := range events {
		switch e := ev.(type) {
		case engine.EventTorpedoFired:
			b.WriteString(fmt.Sprintf("%s[TORPEDO TRACK]%s Fired on course %.2f from [%d,%d]\r\n", ansiYellow, ansiReset, e.Angle, e.Origin.Row(), e.Origin.Col()))
		case engine.EventTorpedoHit:
			if e.Destroyed {
				b.WriteString(fmt.Sprintf("%s*** KLINGON WARSHIP DESTROYED AT [%d,%d] ***%s\r\n", ansiRed, e.Target.Row(), e.Target.Col(), ansiReset))
			} else {
				b.WriteString(fmt.Sprintf("Torpedo hit Klingon at [%d,%d]: %.0f units damage\r\n", e.Target.Row(), e.Target.Col(), e.Damage))
			}
		case engine.EventPhaserFired:
			b.WriteString(fmt.Sprintf("%s[PHASERS FIRED]%s Allocated %.0f units\r\n", ansiYellow, ansiReset, e.Energy))
		case engine.EventPhaserHit:
			if e.Destroyed {
				b.WriteString(fmt.Sprintf("%s*** KLINGON DESTROYED BY PHASER FIRE ***%s\r\n", ansiRed, ansiReset))
			} else {
				b.WriteString(fmt.Sprintf("Phaser beam hit target at [%d,%d]: %.0f units\r\n", e.Target.Row(), e.Target.Col(), e.Damage))
			}
		case engine.EventShieldTransfer:
			b.WriteString(fmt.Sprintf("Deflector Shields: %.0f  |  Total energy: %.0f\r\n", e.NewShields, e.NewEnergy))
		case engine.EventShipMoved:
			if e.FromQuad != e.ToQuad {
				b.WriteString(fmt.Sprintf("%s[WARP ENGINES ENGAGED]%s Arrived at quadrant [%d,%d] sector [%d,%d]\r\n", ansiGreen, ansiReset, e.ToQuad.Row(), e.ToQuad.Col(), e.ToSector.Row(), e.ToSector.Col()))
			} else {
				b.WriteString(fmt.Sprintf("%s[WARP ENGINES ENGAGED]%s Arrived at sector [%d,%d]\r\n", ansiGreen, ansiReset, e.ToSector.Row(), e.ToSector.Col()))
			}
		case engine.EventObstacleEncountered:
			b.WriteString(fmt.Sprintf("%s[NAVIGATION HAZARD]%s Blocked at sector [%d,%d]\r\n", ansiRed, ansiReset, e.Sector.Row(), e.Sector.Col()))
		case engine.EventDocked:
			b.WriteString(fmt.Sprintf("%s[STARBASE DOCKING]%s Docked with Starbase at [%d,%d]. Shields dropped, energy and torpedoes replenished.\r\n", ansiCyan, ansiReset, e.Starbase.Row(), e.Starbase.Col()))
		case engine.EventKlingonCounterAttack:
			b.WriteString(fmt.Sprintf("%s[RETURN FIRE]%s Klingon returns fire: %.0f units damage\r\n", ansiRed, ansiReset, e.Damage))
		case engine.EventSubsystemDamaged:
			name := "Subsystem"
			if e.Device >= 0 && int(e.Device) < len(deviceNames) {
				name = deviceNames[e.Device]
			}
			b.WriteString(fmt.Sprintf("%s[DAMAGE]%s %s damaged! Repair in %.1f stardates\r\n", ansiRed, ansiReset, name, e.RepairTime))
		case engine.EventSubsystemRepaired:
			name := "Subsystem"
			if e.Device >= 0 && int(e.Device) < len(deviceNames) {
				name = deviceNames[e.Device]
			}
			b.WriteString(fmt.Sprintf("%s[REPAIR]%s %s has been repaired.\r\n", ansiGreen, ansiReset, name))
		case engine.EventGameOver:
			if e.Reason == engine.GameOverWon {
				b.WriteString(fmt.Sprintf("%s*** FEDERATION MISSION ACCOMPLISHED ***%s (Score: %.0f)\r\n", ansiGreen, ansiReset, e.Score))
			} else {
				b.WriteString(fmt.Sprintf("%s*** MISSION TERMINATED ***%s (Score: %.0f)\r\n", ansiRed, ansiReset, e.Score))
			}
		}
	}
	return b.String()
}
