package engine

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestSpockFilenameValidation(t *testing.T) {
	// Valid filenames
	valid := []string{"GAME", "save1", "TrekGame", "A", "GAME.TRK", "test.trk"}
	for _, fn := range valid {
		if err := ValidateSaveFilename(fn); err != nil {
			t.Errorf("expected valid filename %q, got error: %v", fn, err)
		}
	}

	// Invalid filenames: starts with non-letter, >9 chars, empty, multi-byte non-alphabetic
	invalid := []string{"123game", "*save*", "toolongfilename", "", "   ", ".trk", "toolongname.trk", "🚀game", "★trek"}
	for _, fn := range invalid {
		if err := ValidateSaveFilename(fn); err == nil {
			t.Errorf("expected error for invalid filename %q, got nil", fn)
		}
	}

	// Verify Spock quote on non-letter start
	err := ValidateSaveFilename("1bad")
	if err == nil {
		t.Fatalf("expected error for '1bad', got nil")
	}
	expectedMsg := `spock- "Captain, file names must begin with an alphabetic letter (A-Z)."`
	if err.Error() != expectedMsg {
		t.Errorf("expected error %q, got %q", expectedMsg, err.Error())
	}

	// Verify length error message
	err = ValidateSaveFilename("toolongfilename")
	if err == nil || !strings.Contains(err.Error(), "exceed 9 characters") {
		t.Errorf("expected length error for 'toolongfilename', got: %v", err)
	}

	// Verify empty error message
	err = ValidateSaveFilename("")
	if err == nil || !strings.Contains(err.Error(), "cannot be empty") {
		t.Errorf("expected empty error for '', got: %v", err)
	}
}

func TestGameSaveAndLoadRoundtrip(t *testing.T) {
	tmpDir := t.TempDir()
	savePath := filepath.Join(tmpDir, "TESTSAVE.TRK")

	orig := NewGame(9999, SkillExpert, LengthLong)
	orig.Enterprise.Energy = 4242.0
	orig.Enterprise.Shields = 500.0
	orig.Enterprise.Torpedoes = 8
	orig.Stardate = 3141.5
	orig.RemainingKlingons = 12

	if err := orig.Save(savePath); err != nil {
		t.Fatalf("failed to save game: %v", err)
	}

	loaded, err := LoadGame(savePath)
	if err != nil {
		t.Fatalf("failed to load game: %v", err)
	}

	if loaded.Enterprise.Energy != 4242.0 {
		t.Errorf("mismatch loaded energy: expected 4242.0, got %f", loaded.Enterprise.Energy)
	}
	if loaded.Enterprise.Shields != 500.0 {
		t.Errorf("mismatch loaded shields: expected 500.0, got %f", loaded.Enterprise.Shields)
	}
	if loaded.Enterprise.Torpedoes != 8 {
		t.Errorf("mismatch loaded torpedoes: expected 8, got %d", loaded.Enterprise.Torpedoes)
	}
	if loaded.Stardate != 3141.5 {
		t.Errorf("mismatch loaded stardate: expected 3141.5, got %f", loaded.Stardate)
	}
	if loaded.RemainingKlingons != 12 {
		t.Errorf("mismatch loaded remaining klingons: expected 12, got %d", loaded.RemainingKlingons)
	}
	if loaded.Skill != SkillExpert {
		t.Errorf("mismatch loaded skill: expected %v, got %v", SkillExpert, loaded.Skill)
	}
	if loaded.Length != LengthLong {
		t.Errorf("mismatch loaded length: expected %v, got %v", LengthLong, loaded.Length)
	}

	// Verify PRNG is initialized and deterministic based on Stardate
	if loaded.RNG == nil {
		t.Fatalf("expected non-nil RNG after LoadGame")
	}
	val := loaded.RNG.Intn(100)
	expectedPRNG := NewPRNG(int64(loaded.Stardate))
	expectedVal := expectedPRNG.Intn(100)
	if val != expectedVal {
		t.Errorf("expected RNG value %d, got %d", expectedVal, val)
	}
}

func TestScenarioSaveAndLoadRoundtrip(t *testing.T) {
	tmpDir := t.TempDir()

	// 1. Scenario roundtrip with ScenarioMutaraNebula
	savePath := filepath.Join(tmpDir, "SCENMUT.TRK")
	orig := NewGame(1234, SkillExpert, LengthShort)
	orig.Scenario = ScenarioMutaraNebula

	if err := orig.Save(savePath); err != nil {
		t.Fatalf("failed to save game with scenario: %v", err)
	}

	loaded, err := LoadGame(savePath)
	if err != nil {
		t.Fatalf("failed to load game with scenario: %v", err)
	}

	if loaded.Scenario != ScenarioMutaraNebula {
		t.Errorf("expected loaded scenario %q, got %q", ScenarioMutaraNebula, loaded.Scenario)
	}

	// 2. ScenarioNone omitempty verification
	nonePath := filepath.Join(tmpDir, "SCENNONE.TRK")
	origNone := NewGame(5678, SkillGood, LengthMedium)
	origNone.Scenario = ScenarioNone

	if err := origNone.Save(nonePath); err != nil {
		t.Fatalf("failed to save game with ScenarioNone: %v", err)
	}

	data, err := os.ReadFile(nonePath)
	if err != nil {
		t.Fatalf("failed to read raw save file: %v", err)
	}
	if strings.Contains(string(data), `"scenario"`) {
		t.Errorf("expected 'scenario' field to be omitted for ScenarioNone in JSON, got: %s", string(data))
	}

	loadedNone, err := LoadGame(nonePath)
	if err != nil {
		t.Fatalf("failed to load game with ScenarioNone: %v", err)
	}
	if loadedNone.Scenario != ScenarioNone {
		t.Errorf("expected loaded scenario %q, got %q", ScenarioNone, loadedNone.Scenario)
	}
}

func TestGameSaveAutoAppendExtension(t *testing.T) {
	tmpDir := t.TempDir()
	savePathWithoutExt := filepath.Join(tmpDir, "AUTOSAVE")

	orig := NewGame(1234, SkillGood, LengthMedium)
	orig.Enterprise.Energy = 3333.0

	if err := orig.Save(savePathWithoutExt); err != nil {
		t.Fatalf("failed to save game: %v", err)
	}

	expectedPath := filepath.Join(tmpDir, "AUTOSAVE.TRK")
	if _, err := os.Stat(expectedPath); os.IsNotExist(err) {
		t.Fatalf("expected save file %q to exist", expectedPath)
	}

	loaded, err := LoadGame(expectedPath)
	if err != nil {
		t.Fatalf("failed to load game: %v", err)
	}
	if loaded.Enterprise.Energy != 3333.0 {
		t.Errorf("mismatch loaded energy: expected 3333.0, got %f", loaded.Enterprise.Energy)
	}
}

func TestGameSaveInvalidFilename(t *testing.T) {
	tmpDir := t.TempDir()
	savePath := filepath.Join(tmpDir, "123BADNAME.TRK")

	orig := NewGame(1234, SkillFair, LengthShort)
	err := orig.Save(savePath)
	if err == nil {
		t.Fatalf("expected error saving with invalid filename %q, got nil", savePath)
	}
}

func TestLoadGameNotFound(t *testing.T) {
	tmpDir := t.TempDir()
	_, err := LoadGame(filepath.Join(tmpDir, "NONEXISTENT.TRK"))
	if err == nil {
		t.Fatalf("expected error loading nonexistent file, got nil")
	}
}

func TestLoadGameCorrupted(t *testing.T) {
	tmpDir := t.TempDir()
	corruptPath := filepath.Join(tmpDir, "CORRUPT.TRK")
	if err := os.WriteFile(corruptPath, []byte("not valid json {"), 0644); err != nil {
		t.Fatalf("failed to write corrupt file: %v", err)
	}

	_, err := LoadGame(corruptPath)
	if err == nil {
		t.Fatalf("expected error loading corrupted file, got nil")
	}
}

func TestGameSaveTrailingWhitespace(t *testing.T) {
	tmpDir := t.TempDir()
	savePath := filepath.Join(tmpDir, "TESTSAVE   ")

	orig := NewGame(1234, SkillGood, LengthMedium)
	orig.Enterprise.Energy = 3333.0

	if err := orig.Save(savePath); err != nil {
		t.Fatalf("failed to save game with trailing whitespace: %v", err)
	}

	expectedPath := filepath.Join(tmpDir, "TESTSAVE.TRK")
	if _, err := os.Stat(expectedPath); os.IsNotExist(err) {
		t.Fatalf("expected save file %q to exist", expectedPath)
	}

	loaded, err := LoadGame(expectedPath)
	if err != nil {
		t.Fatalf("failed to load game: %v", err)
	}
	if loaded.Enterprise.Energy != 3333.0 {
		t.Errorf("mismatch loaded energy: expected 3333.0, got %f", loaded.Enterprise.Energy)
	}
}

func TestInspectSave(t *testing.T) {
	tempDir := t.TempDir()
	savePath := filepath.Join(tempDir, "TESTSAVE.TRK")

	g := NewGame(12345, SkillGood, LengthMedium)
	g.Stardate = 3450.5
	g.TimeRemaining = 24.5
	g.Enterprise.Condition = ConditionYellow
	g.RemainingKlingons = 7

	if err := g.Save(savePath); err != nil {
		t.Fatalf("failed to create test save: %v", err)
	}

	meta, err := InspectSave(savePath)
	if err != nil {
		t.Fatalf("InspectSave returned unexpected error: %v", err)
	}

	if meta.Filename != "TESTSAVE.TRK" {
		t.Errorf("expected Filename 'TESTSAVE.TRK', got %q", meta.Filename)
	}
	if meta.Path != savePath {
		t.Errorf("expected Path %q, got %q", savePath, meta.Path)
	}
	if meta.Skill != SkillGood {
		t.Errorf("expected Skill %v, got %v", SkillGood, meta.Skill)
	}
	if meta.Stardate != 3450.5 {
		t.Errorf("expected Stardate 3450.5, got %f", meta.Stardate)
	}
	if meta.TimeRemaining != 24.5 {
		t.Errorf("expected TimeRemaining 24.5, got %f", meta.TimeRemaining)
	}
	if meta.Condition != ConditionYellow {
		t.Errorf("expected Condition %v, got %v", ConditionYellow, meta.Condition)
	}
	if meta.KlingonsLeft != 7 {
		t.Errorf("expected KlingonsLeft 7, got %d", meta.KlingonsLeft)
	}
	if meta.ModTime.IsZero() {
		t.Errorf("expected non-zero ModTime")
	}

	// Test non-existent file
	if _, err := InspectSave(filepath.Join(tempDir, "NONEXIST.TRK")); err == nil {
		t.Errorf("expected error for non-existent file, got nil")
	}

	// Test corrupted file
	corruptPath := filepath.Join(tempDir, "CORRUPT.TRK")
	if err := os.WriteFile(corruptPath, []byte("NOT_JSON"), 0644); err != nil {
		t.Fatalf("failed to write corrupt file: %v", err)
	}
	if _, err := InspectSave(corruptPath); err == nil {
		t.Errorf("expected error for corrupt JSON save, got nil")
	}
}

func TestSaveLoadWithAnomalies(t *testing.T) {
	tempDir := t.TempDir()
	savePath := filepath.Join(tempDir, "ANOMALY.TRK")

	rules := DefaultRulesForProfile(ProfileHardcore)
	if !rules.SpatialAnomalies {
		t.Fatalf("expected Hardcore profile to have SpatialAnomalies enabled")
	}

	orig := NewGameWithOptions(777, SkillExpert, LengthMedium, rules)

	// Verify anomalies actually exist in orig
	hasAnomaly := false
	for r := 1; r <= 8; r++ {
		for c := 1; c <= 8; c++ {
			if orig.QuadrantEnv[r][c] != EnvNormal {
				hasAnomaly = true
				break
			}
		}
	}
	if !hasAnomaly {
		t.Fatalf("expected seeded anomalies in Hardcore game")
	}

	if err := orig.Save(savePath); err != nil {
		t.Fatalf("failed to save game: %v", err)
	}

	loaded, err := LoadGame(savePath)
	if err != nil {
		t.Fatalf("failed to load game: %v", err)
	}

	if !loaded.Rules.SpatialAnomalies {
		t.Errorf("expected loaded.Rules.SpatialAnomalies to be true")
	}

	for r := 1; r <= 8; r++ {
		for c := 1; c <= 8; c++ {
			if loaded.QuadrantEnv[r][c] != orig.QuadrantEnv[r][c] {
				t.Errorf("quadrant [%d,%d] env mismatch: want %v, got %v", r, c, orig.QuadrantEnv[r][c], loaded.QuadrantEnv[r][c])
			}
		}
	}
}

func TestSaveAndLoadTourGame(t *testing.T) {
	tmpDir := t.TempDir()
	savePath := filepath.Join(tmpDir, "test_tour.json")

	tour := NewTour(777)
	g := tour.StartCurrentSector()
	tour.RequisitionPoints = 1450
	tour.InstalledRefits[RefitDilithiumCore] = 1

	err := SaveTourGame(savePath, g, tour)
	if err != nil {
		t.Fatalf("failed to save tour game: %v", err)
	}

	loadedGame, loadedTour, err := LoadTourGame(savePath)
	if err != nil {
		t.Fatalf("failed to load tour game: %v", err)
	}
	if loadedTour == nil {
		t.Fatal("expected non-nil loadedTour")
	}
	if loadedTour.RequisitionPoints != 1450 {
		t.Errorf("expected 1450 RequisitionPoints, got %d", loadedTour.RequisitionPoints)
	}
	if loadedTour.InstalledRefits[RefitDilithiumCore] != 1 {
		t.Errorf("expected tier 1 Dilithium Core, got %d", loadedTour.InstalledRefits[RefitDilithiumCore])
	}
	if loadedGame.Enterprise.MaxEnergy != 3500.0 {
		t.Errorf("expected loadedGame MaxEnergy 3500, got %f", loadedGame.Enterprise.MaxEnergy)
	}
	if loadedTour.CurrentGameState != loadedGame {
		t.Errorf("expected loadedTour.CurrentGameState to point to loadedGame instance (%p), got %p", loadedGame, loadedTour.CurrentGameState)
	}
	if loadedTour.CurrentGameState.RNG == nil {
		t.Errorf("expected initialized PRNG on loadedTour.CurrentGameState")
	}
}

func TestSaveAndLoadTourGame_PreservesDamagedEnergy(t *testing.T) {
	tmpDir := t.TempDir()
	savePath := filepath.Join(tmpDir, "damaged_tour.json")

	tour := NewTour(777)
	g := tour.StartCurrentSector()
	g.Enterprise.Energy = 1234.0
	g.Enterprise.Torpedoes = 4
	tour.RequisitionPoints = 1000

	err := SaveTourGame(savePath, g, tour)
	if err != nil {
		t.Fatalf("failed to save tour game: %v", err)
	}

	loadedGame, _, err := LoadTourGame(savePath)
	if err != nil {
		t.Fatalf("failed to load tour game: %v", err)
	}
	if loadedGame.Enterprise.Energy != 1234.0 {
		t.Errorf("expected loaded energy to be 1234.0, got %f", loadedGame.Enterprise.Energy)
	}
	if loadedGame.Enterprise.Torpedoes != 4 {
		t.Errorf("expected loaded torpedoes to be 4, got %d", loadedGame.Enterprise.Torpedoes)
	}
}

func TestSaveAndLoadTourGame_InDrydock(t *testing.T) {
	tmpDir := t.TempDir()
	savePath := filepath.Join(tmpDir, "drydock_tour.json")

	tour := NewTour(888)
	tour.InDrydock = true
	tour.RequisitionPoints = 2000
	tour.InstalledRefits[RefitDeflectorGrid] = 2

	err := SaveTourGame(savePath, nil, tour)
	if err != nil {
		t.Fatalf("failed to save drydock tour game: %v", err)
	}

	loadedGame, loadedTour, err := LoadTourGame(savePath)
	if err != nil {
		t.Fatalf("failed to load drydock tour game: %v", err)
	}
	if loadedGame != nil {
		t.Errorf("expected nil loadedGame for drydock save, got %v", loadedGame)
	}
	if loadedTour == nil {
		t.Fatal("expected non-nil loadedTour")
	}
	if !loadedTour.InDrydock {
		t.Errorf("expected InDrydock true, got false")
	}
	if loadedTour.InstalledRefits[RefitDeflectorGrid] != 2 {
		t.Errorf("expected tier 2 deflector grid, got %d", loadedTour.InstalledRefits[RefitDeflectorGrid])
	}
}

func TestSaveAndLoadTourGame_BackwardCompatibility(t *testing.T) {
	tmpDir := t.TempDir()

	// 1. Single game saved with g.Save(), loaded via LoadTourGame()
	legacySavePath := filepath.Join(tmpDir, "LEGACY.TRK")
	orig := NewGame(1234, SkillGood, LengthMedium)
	orig.Enterprise.Energy = 2800.0
	orig.Stardate = 3200.0

	if err := orig.Save(legacySavePath); err != nil {
		t.Fatalf("failed to save legacy game: %v", err)
	}

	loadedGame, loadedTour, err := LoadTourGame(legacySavePath)
	if err != nil {
		t.Fatalf("failed to load legacy game via LoadTourGame: %v", err)
	}
	if loadedGame == nil {
		t.Fatal("expected non-nil loadedGame from legacy save")
	}
	if loadedTour != nil {
		t.Errorf("expected nil loadedTour from legacy save, got %v", loadedTour)
	}
	if loadedGame.Enterprise.Energy != 2800.0 {
		t.Errorf("expected loaded energy 2800.0, got %f", loadedGame.Enterprise.Energy)
	}
	if loadedGame.RNG == nil {
		t.Errorf("expected initialized PRNG on loadedGame")
	}

	// 2. Tour game saved with SaveTourGame(), loaded via LoadGame() and InspectSave()
	tourSavePath := filepath.Join(tmpDir, "TOURSAVE.TRK")
	tour := NewTour(999)
	g := tour.StartCurrentSector()
	g.Stardate = 3500.0
	g.TimeRemaining = 25.0
	g.Enterprise.Condition = ConditionRed
	g.RemainingKlingons = 5
	tour.InstalledRefits[RefitDilithiumCore] = 2

	if err := SaveTourGame(tourSavePath, g, tour); err != nil {
		t.Fatalf("failed to save tour game: %v", err)
	}

	// LoadGame interoperability
	lg, err := LoadGame(tourSavePath)
	if err != nil {
		t.Fatalf("LoadGame failed on tour save: %v", err)
	}
	if lg.Enterprise.MaxEnergy != 4000.0 {
		t.Errorf("expected MaxEnergy 4000.0 with tier 2 core, got %f", lg.Enterprise.MaxEnergy)
	}
	if lg.RNG == nil {
		t.Errorf("expected initialized PRNG from LoadGame on tour save")
	}

	// InspectSave interoperability
	meta, err := InspectSave(tourSavePath)
	if err != nil {
		t.Fatalf("InspectSave failed on tour save: %v", err)
	}
	if meta.Stardate != 3500.0 {
		t.Errorf("expected Stardate 3500.0, got %f", meta.Stardate)
	}
	if meta.Condition != ConditionRed {
		t.Errorf("expected ConditionRed, got %v", meta.Condition)
	}
	if meta.KlingonsLeft != 5 {
		t.Errorf("expected KlingonsLeft 5, got %d", meta.KlingonsLeft)
	}
}

func TestSaveAndLoadTourGame_Errors(t *testing.T) {
	tmpDir := t.TempDir()

	// Nonexistent file
	_, _, err := LoadTourGame(filepath.Join(tmpDir, "MISSING.TRK"))
	if err == nil {
		t.Errorf("expected error loading missing file, got nil")
	}

	// Corrupted file
	corruptPath := filepath.Join(tmpDir, "CORRUPT.TRK")
	if err := os.WriteFile(corruptPath, []byte("NOT_VALID_JSON{"), 0644); err != nil {
		t.Fatalf("failed to write corrupt file: %v", err)
	}
	_, _, err = LoadTourGame(corruptPath)
	if err == nil {
		t.Errorf("expected error loading corrupt file, got nil")
	}

	// Empty JSON schema without game_state or tour_state
	emptyPath := filepath.Join(tmpDir, "EMPTY.json")
	if err := os.WriteFile(emptyPath, []byte("{}"), 0644); err != nil {
		t.Fatalf("failed to write empty file: %v", err)
	}
	_, _, err = LoadTourGame(emptyPath)
	if err == nil {
		t.Errorf("expected error loading empty JSON, got nil")
	}

	// Unrecognized JSON schema
	unrecPath := filepath.Join(tmpDir, "UNRECOGNIZED.json")
	if err := os.WriteFile(unrecPath, []byte(`{"some_other_app": true}`), 0644); err != nil {
		t.Fatalf("failed to write unrecognized JSON file: %v", err)
	}
	_, _, err = LoadTourGame(unrecPath)
	if err == nil {
		t.Errorf("expected error loading unrecognized JSON schema, got nil")
	}
}

