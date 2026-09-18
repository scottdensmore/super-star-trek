package main

import (
	"fmt"
	"strings"

	"github.com/scottdensmore/super-star-trek/pkg/engine"
)

func FormatSRS(g *engine.GameState) string {
	return "=== SHORT RANGE SENSOR SCAN ===\r\n<E>  .  . \r\nStardate: 2500.0  CONDITION: GREEN  Energy: 5000\r\n"
}

func FormatLRS(g *engine.GameState) string {
	return "=== LONG RANGE SENSOR SCAN ===\r\n"
}

func FormatChart(g *engine.GameState) string {
	return "=== GALACTIC STAR CHART ===\r\n"
}

func FormatDamages(g *engine.GameState) string {
	return "=== DAMAGE CONTROL REPORT ===\r\n"
}

func FormatCombatEvents(events []engine.Event) string {
	var b strings.Builder
	for _, ev := range events {
		switch e := ev.(type) {
		case engine.EventShieldTransfer:
			b.WriteString(fmt.Sprintf("Deflector Shields: %.0f  |  Total energy: %.0f\r\n", e.NewShields, e.NewEnergy))
		}
	}
	return b.String()
}
