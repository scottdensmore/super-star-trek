package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func TestDifficultyPresets(t *testing.T) {
	casual := DefaultRulesForProfile(ProfileCasual)
	if casual.Surveillance != SurveillanceFull || casual.SensorDegradation || casual.RepairMultiplier != 0.75 || casual.KlingonCloak || casual.TimeMargin != 1.25 {
		t.Errorf("unexpected Casual preset: %+v", casual)
	}

	normal := DefaultRulesForProfile(ProfileNormal)
	if normal.Surveillance != SurveillanceClassic || !normal.SensorDegradation || normal.RepairMultiplier != 1.00 || normal.KlingonCloak || normal.TimeMargin != 1.00 {
		t.Errorf("unexpected Normal preset: %+v", normal)
	}

	hardcore := DefaultRulesForProfile(ProfileHardcore)
	if hardcore.Surveillance != SurveillanceLocal || !hardcore.SensorDegradation || hardcore.RepairMultiplier != 1.50 || !hardcore.KlingonCloak || hardcore.TimeMargin != 0.80 {
		t.Errorf("unexpected Hardcore preset: %+v", hardcore)
	}

	nightmare := DefaultRulesForProfile(ProfileNightmare)
	if nightmare.Surveillance != SurveillanceBlackout || !nightmare.SensorDegradation || nightmare.RepairMultiplier != 2.00 || !nightmare.KlingonCloak || nightmare.TimeMargin != 0.60 {
		t.Errorf("unexpected Nightmare preset: %+v", nightmare)
	}
}

func TestNewGameWithOptions_BlackoutHidesStarbases(t *testing.T) {
	rules := DefaultRulesForProfile(ProfileNightmare)
	g := NewGameWithOptions(12345, SkillGood, LengthMedium, rules)

	if g.Rules.Surveillance != SurveillanceBlackout {
		t.Fatalf("expected Blackout surveillance, got %v", g.Rules.Surveillance)
	}
	if g.TimeRemaining != 30.0*0.60 {
		t.Errorf("expected TimeRemaining 18.0, got %f", g.TimeRemaining)
	}

	knownBasesCount := 0
	for r := 1; r <= 8; r++ {
		for c := 1; c <= 8; c++ {
			if g.ChartKnownBases[r][c] {
				knownBasesCount++
			}
		}
	}
	if knownBasesCount != 0 {
		t.Errorf("expected 0 known bases in blackout mode at start, found %d", knownBasesCount)
	}
}

func TestNewGameWithOptions_NormalChartsStarbases(t *testing.T) {
	rules := DefaultRulesForProfile(ProfileNormal)
	g := NewGameWithOptions(12345, SkillGood, LengthMedium, rules)

	knownBasesCount := 0
	for r := 1; r <= 8; r++ {
		for c := 1; c <= 8; c++ {
			if g.ChartKnownBases[r][c] {
				knownBasesCount++
			}
		}
	}
	if knownBasesCount != g.RemainingStarbases {
		t.Errorf("expected %d known bases in normal mode at start, found %d", g.RemainingStarbases, knownBasesCount)
	}
}

func TestSaveLoad_GameRulesPersistenceAndBackwardCompatibility(t *testing.T) {
	tempDir := t.TempDir()
	savePath := filepath.Join(tempDir, "TESTSAVE.TRK")

	rules := GameRules{
		Profile:           ProfileCustom,
		Surveillance:      SurveillanceLocal,
		SensorDegradation: true,
		RepairMultiplier:  1.75,
		KlingonCloak:      true,
		TimeMargin:        0.9,
	}
	g := NewGameWithOptions(999, SkillExpert, LengthLong, rules)
	if err := g.Save(savePath); err != nil {
		t.Fatalf("Save failed: %v", err)
	}

	loaded, err := LoadGame(savePath)
	if err != nil {
		t.Fatalf("Load failed: %v", err)
	}
	if loaded.Rules != rules {
		t.Errorf("Rules mismatch after load: got %+v, want %+v", loaded.Rules, rules)
	}

	// Test backward compatibility: old save with no "rules" key
	rawOld := map[string]interface{}{
		"Skill":              SkillGood,
		"Length":             LengthMedium,
		"RemainingKlingons":  15,
		"RemainingStarbases": 3,
	}
	data, _ := json.Marshal(rawOld)
	oldPath := filepath.Join(tempDir, "OLDSAVE.TRK")
	_ = os.WriteFile(oldPath, data, 0644)

	loadedOld, err := LoadGame(oldPath)
	if err != nil {
		t.Fatalf("Load old save failed: %v", err)
	}
	if loadedOld.Rules.Profile != ProfileNormal || loadedOld.Rules.Surveillance != SurveillanceClassic {
		t.Errorf("expected default Normal rules for legacy save, got %+v", loadedOld.Rules)
	}
}
