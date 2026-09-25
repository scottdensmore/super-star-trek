package engine

import (
	"fmt"
	"os"
	"path/filepath"
	"testing"
	"time"
)

func TestLeaderboard_DefaultSeedAndQualifies(t *testing.T) {
	lb := DefaultLeaderboard()
	if len(lb.Entries) == 0 {
		t.Fatalf("expected non-empty default leaderboard")
	}
	if len(lb.Entries) > 10 {
		t.Fatalf("expected at most 10 default entries, got %d", len(lb.Entries))
	}

	lowestScore := lb.Entries[len(lb.Entries)-1].Score
	if !lb.Qualifies(lowestScore + 10) {
		t.Errorf("expected score %d to qualify against lowest %d", lowestScore+10, lowestScore)
	}
	if lb.Qualifies(lowestScore-50) && len(lb.Entries) >= 10 {
		t.Errorf("expected score %d not to qualify when table is full", lowestScore-50)
	}
}

func TestLeaderboard_SaveAndLoadAtomic(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "highscores.json")

	lb := &Leaderboard{
		Entries: []ScoreEntry{
			{CaptainName: "Kirk", Score: 1180, Rank: "[FADM]", Date: time.Now()},
			{CaptainName: "Spock", Score: 980, Rank: "[RADM]", Date: time.Now()},
		},
	}

	if err := lb.Save(path); err != nil {
		t.Fatalf("failed to save leaderboard: %v", err)
	}

	loaded, err := LoadLeaderboard(path)
	if err != nil {
		t.Fatalf("failed to load leaderboard: %v", err)
	}

	if len(loaded.Entries) != 2 {
		t.Fatalf("expected 2 entries, got %d", len(loaded.Entries))
	}
	if loaded.Entries[0].CaptainName != "Kirk" || loaded.Entries[0].Score != 1180 {
		t.Errorf("unexpected top entry: %+v", loaded.Entries[0])
	}
}

func TestLeaderboard_AddAndTruncateTop10(t *testing.T) {
	lb := DefaultLeaderboard()

	entry := ScoreEntry{
		CaptainName: "Picard",
		Score:       2000,
		Rank:        "[FADM]",
		Date:        time.Now(),
	}

	lb.Add(entry)

	if len(lb.Entries) > 10 {
		t.Errorf("expected max 10 entries after add, got %d", len(lb.Entries))
	}
	if lb.Entries[0].CaptainName != "Picard" || lb.Entries[0].Score != 2000 {
		t.Errorf("expected Picard to be #1, got %+v", lb.Entries[0])
	}
}

func TestLeaderboard_CorruptedFileRecovery(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "corrupt.json")

	if err := os.WriteFile(path, []byte("{ invalid json "), 0644); err != nil {
		t.Fatalf("failed to write corrupt file: %v", err)
	}

	lb, err := LoadLeaderboard(path)
	if err != nil {
		t.Fatalf("expected clean fallback on corrupt file, got err: %v", err)
	}
	if len(lb.Entries) == 0 {
		t.Errorf("expected default entries on corrupt fallback")
	}

	// Corrupted file should be backed up
	if _, err := os.Stat(path + ".corrupt"); err != nil {
		t.Errorf("expected .corrupt backup file to exist")
	}
}

func TestLeaderboard_DefaultLeaderboardPath(t *testing.T) {
	tmpDir := t.TempDir()
	t.Setenv("XDG_CONFIG_HOME", tmpDir)

	path := DefaultLeaderboardPath()
	expected := filepath.Join(tmpDir, "super-star-trek", "highscores.json")
	if path != expected {
		t.Errorf("expected path %q, got %q", expected, path)
	}

	// Verify path purity: DefaultLeaderboardPath must NOT create directories on disk
	appDir := filepath.Join(tmpDir, "super-star-trek")
	if _, err := os.Stat(appDir); !os.IsNotExist(err) {
		t.Errorf("expected directory %q not to exist, but DefaultLeaderboardPath created it", appDir)
	}

	// Unset XDG_CONFIG_HOME to test fallback to HOME/.config
	t.Setenv("XDG_CONFIG_HOME", "")
	homeDir := t.TempDir()
	t.Setenv("HOME", homeDir)

	fallbackPath := DefaultLeaderboardPath()
	expectedFallback := filepath.Join(homeDir, ".config", "super-star-trek", "highscores.json")
	if fallbackPath != expectedFallback {
		t.Errorf("expected fallback path %q, got %q", expectedFallback, fallbackPath)
	}

	// When neither XDG_CONFIG_HOME nor HOME is set, fallback to ./.sst-scores.json
	t.Setenv("XDG_CONFIG_HOME", "")
	t.Setenv("HOME", "")
	uncreatablePath := DefaultLeaderboardPath()
	if uncreatablePath != "./.sst-scores.json" {
		t.Errorf("expected local fallback './.sst-scores.json', got %q", uncreatablePath)
	}
}

func TestLeaderboard_LoadNonExistent(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "does_not_exist.json")

	lb, err := LoadLeaderboard(path)
	if err != nil {
		t.Fatalf("expected nil error on non-existent file, got %v", err)
	}
	if len(lb.Entries) == 0 {
		t.Errorf("expected default entries when file does not exist")
	}
}

func TestLeaderboard_QualifiesAndAddEdgeCases(t *testing.T) {
	var nilLB *Leaderboard
	if nilLB.Qualifies(1000) {
		t.Errorf("nil leaderboard should not qualify any score")
	}
	nilLB.Add(ScoreEntry{CaptainName: "Test", Score: 500}) // should not panic

	// Empty leaderboard
	emptyLB := &Leaderboard{}
	if !emptyLB.Qualifies(10) {
		t.Errorf("empty leaderboard should qualify any score")
	}
	emptyLB.Add(ScoreEntry{CaptainName: "First", Score: 100})
	if len(emptyLB.Entries) != 1 {
		t.Fatalf("expected 1 entry, got %d", len(emptyLB.Entries))
	}

	// Equal score qualification (must beat lowest score)
	fullLB := &Leaderboard{
		Entries: []ScoreEntry{
			{CaptainName: "Kirk", Score: 1000},
			{CaptainName: "Spock", Score: 900},
			{CaptainName: "Pike", Score: 800},
			{CaptainName: "Sulu", Score: 700},
			{CaptainName: "Uhura", Score: 600},
			{CaptainName: "Scotty", Score: 500},
			{CaptainName: "McCoy", Score: 400},
			{CaptainName: "Chekov", Score: 300},
			{CaptainName: "Chapel", Score: 200},
			{CaptainName: "Rand", Score: 100},
		},
	}
	if fullLB.Qualifies(100) {
		t.Errorf("score equal to lowest score should not qualify")
	}
	if !fullLB.Qualifies(101) {
		t.Errorf("score greater than lowest score should qualify")
	}

	// Add entry in middle
	fullLB.Add(ScoreEntry{CaptainName: "Bones", Score: 450})
	if len(fullLB.Entries) != 10 {
		t.Errorf("expected 10 entries after adding Bones, got %d", len(fullLB.Entries))
	}
	if fullLB.Entries[6].CaptainName != "Bones" || fullLB.Entries[6].Score != 450 {
		t.Errorf("expected Bones at index 6, got %+v", fullLB.Entries[6])
	}
	// Rand (100) should have been dropped
	for _, e := range fullLB.Entries {
		if e.CaptainName == "Rand" {
			t.Errorf("expected Rand to be dropped from top 10")
		}
	}
}

func TestLeaderboard_DefaultLegendsPresence(t *testing.T) {
	lb := DefaultLeaderboard()
	if len(lb.Entries) != 10 {
		t.Fatalf("expected exactly 10 default legends, got %d", len(lb.Entries))
	}

	expectedLegends := []string{
		"James T. Kirk",
		"Spock",
		"Christopher Pike",
		"Hikaru Sulu",
		"Nyota Uhura",
		"Montgomery Scott",
		"Leonard McCoy",
		"Pavel Chekov",
		"Christine Chapel",
		"Janice Rand",
	}

	for i, expected := range expectedLegends {
		if lb.Entries[i].CaptainName != expected {
			t.Errorf("entry %d: expected name %q, got %q", i, expected, lb.Entries[i].CaptainName)
		}
	}

	// Check that scores are strictly descending
	for i := 0; i < len(lb.Entries)-1; i++ {
		if lb.Entries[i].Score <= lb.Entries[i+1].Score {
			t.Errorf("entry %d (%d) is not greater than entry %d (%d)",
				i, lb.Entries[i].Score, i+1, lb.Entries[i+1].Score)
		}
	}

	// Kirk must be James T. Kirk with score 1180
	if lb.Entries[0].CaptainName != "James T. Kirk" || lb.Entries[0].Score != 1180 {
		t.Errorf("expected James T. Kirk with 1180 pts, got %+v", lb.Entries[0])
	}
}

func TestScenarioLeaderboards_Isolation(t *testing.T) {
	tempDir := t.TempDir()
	t.Setenv("HOME", tempDir)
	t.Setenv("XDG_CONFIG_HOME", "")

	// Save score for Kobayashi Maru
	entry := ScoreEntry{
		CaptainName:  "Test Cadet",
		Score:        1200,
		Rank:         "[COMM-3]",
		Commendation: "Starfleet Cross of Honor",
		Date:         time.Now(),
	}
	rank, added, err := AddScenarioScore(ScenarioKobayashiMaru, entry)
	if err != nil || !added {
		t.Fatalf("failed to add scenario score: %v, added=%v", err, added)
	}
	if rank < 1 {
		t.Errorf("expected positive rank, got %d", rank)
	}

	// Verify standard leaderboard was NOT touched
	stdLB, err := LoadLeaderboard()
	if err != nil {
		t.Fatalf("unexpected error loading standard leaderboard: %v", err)
	}
	for _, e := range stdLB.Entries {
		if e.CaptainName == "Test Cadet" {
			t.Errorf("standard leaderboard should not contain scenario entry")
		}
	}

	// Verify scenario leaderboard contains entry
	scenLB, err := LoadScenarioLeaderboard(ScenarioKobayashiMaru)
	if err != nil {
		t.Fatalf("failed to load scenario leaderboard: %v", err)
	}
	found := false
	for _, e := range scenLB.Entries {
		if e.CaptainName == "Test Cadet" && e.Commendation == "Starfleet Cross of Honor" {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("entry with commendation not found in scenario leaderboard")
	}
}

func TestScenarioLeaderboards_Defaults(t *testing.T) {
	// Kobayashi Maru defaults
	kmLB := DefaultScenarioLeaderboard(ScenarioKobayashiMaru)
	if len(kmLB.Entries) != 3 {
		t.Fatalf("expected 3 default entries for Kobayashi Maru, got %d", len(kmLB.Entries))
	}
	if kmLB.Entries[0].CaptainName != "Cadet James T. Kirk" || kmLB.Entries[0].Rank != "[KIRK-AWARD]" || kmLB.Entries[0].Commendation != "Commendation for Original Thinking" {
		t.Errorf("unexpected Kirk entry: %+v", kmLB.Entries[0])
	}
	if kmLB.Entries[1].CaptainName != "Cadet Spock" || kmLB.Entries[1].Rank != "[COMM-3]" || kmLB.Entries[1].Commendation != "Starfleet Cross of Honor" {
		t.Errorf("unexpected Spock entry: %+v", kmLB.Entries[1])
	}
	if kmLB.Entries[2].CaptainName != "Lieutenant Saavik" || kmLB.Entries[2].Rank != "[COMM-2]" || kmLB.Entries[2].Commendation != "Tactical Excellence" {
		t.Errorf("unexpected Saavik entry: %+v", kmLB.Entries[2])
	}

	// Mutara Nebula defaults
	mnLB := DefaultScenarioLeaderboard(ScenarioMutaraNebula)
	if len(mnLB.Entries) != 2 {
		t.Fatalf("expected 2 default entries for Mutara Nebula, got %d", len(mnLB.Entries))
	}
	if mnLB.Entries[0].CaptainName != "Admiral James T. Kirk" || mnLB.Entries[0].Score != 1500 || !mnLB.Entries[0].GameWon {
		t.Errorf("unexpected Kirk Mutara entry: %+v", mnLB.Entries[0])
	}
	if mnLB.Entries[1].CaptainName != "Captain Clark Terrell" || mnLB.Entries[1].Score != 820 || mnLB.Entries[1].GameWon {
		t.Errorf("unexpected Terrell Mutara entry: %+v", mnLB.Entries[1])
	}

	// Starbase Under Siege defaults
	ssLB := DefaultScenarioLeaderboard(ScenarioStarbaseSiege)
	if len(ssLB.Entries) != 2 {
		t.Fatalf("expected 2 default entries for Starbase Siege, got %d", len(ssLB.Entries))
	}
	if ssLB.Entries[0].CaptainName != "Captain Hikaru Sulu" || ssLB.Entries[0].Score != 1350 || !ssLB.Entries[0].GameWon {
		t.Errorf("unexpected Sulu Siege entry: %+v", ssLB.Entries[0])
	}
	if ssLB.Entries[1].CaptainName != "Commander Montgomery Scott" || ssLB.Entries[1].Score != 1100 || !ssLB.Entries[1].GameWon {
		t.Errorf("unexpected Scott Siege entry: %+v", ssLB.Entries[1])
	}

	// Unknown scenario
	unkLB := DefaultScenarioLeaderboard("unknown-scenario")
	if len(unkLB.Entries) != 0 {
		t.Errorf("expected 0 entries for unknown scenario, got %d", len(unkLB.Entries))
	}
}

func TestScenarioLeaderboards_CorruptedFileRecovery(t *testing.T) {
	tempDir := t.TempDir()
	t.Setenv("HOME", tempDir)
	t.Setenv("XDG_CONFIG_HOME", "")

	path := ScenarioLeaderboardPath(ScenarioKobayashiMaru)
	if err := os.MkdirAll(filepath.Dir(path), 0755); err != nil {
		t.Fatalf("failed to create directory: %v", err)
	}
	if err := os.WriteFile(path, []byte("{ corrupt json"), 0644); err != nil {
		t.Fatalf("failed to write corrupt file: %v", err)
	}

	lb, err := LoadScenarioLeaderboard(ScenarioKobayashiMaru)
	if err != nil {
		t.Fatalf("expected fallback to default, got error: %v", err)
	}
	if len(lb.Entries) != 3 {
		t.Errorf("expected 3 default entries on corrupt fallback, got %d", len(lb.Entries))
	}

	// Corrupt file should be backed up
	if _, err := os.Stat(path + ".corrupt"); err != nil {
		t.Errorf("expected corrupt backup file %s.corrupt to exist", path)
	}
}

func TestScenarioLeaderboards_AddAndRankCases(t *testing.T) {
	tempDir := t.TempDir()
	t.Setenv("HOME", tempDir)
	t.Setenv("XDG_CONFIG_HOME", "")

	// 1. Add top score (beats Kirk's 2400)
	topEntry := ScoreEntry{
		CaptainName:  "Cheating Cadet",
		Score:        3000,
		Rank:         "[KIRK-AWARD]",
		Commendation: "Reprogrammed Simulator",
	}
	rank, added, err := AddScenarioScore(ScenarioKobayashiMaru, topEntry)
	if err != nil || !added || rank != 1 {
		t.Fatalf("expected rank 1 and added=true, got rank=%d, added=%v, err=%v", rank, added, err)
	}

	// 2. Fill leaderboard to 10 entries
	for i := 0; i < 6; i++ {
		e := ScoreEntry{
			CaptainName: fmt.Sprintf("Cadet %d", i),
			Score:       500 - (i * 10),
		}
		_, added, err := AddScenarioScore(ScenarioKobayashiMaru, e)
		if err != nil || !added {
			t.Fatalf("failed to fill scenario score %d: %v", i, err)
		}
	}

	lb, err := LoadScenarioLeaderboard(ScenarioKobayashiMaru)
	if err != nil {
		t.Fatalf("failed to load leaderboard: %v", err)
	}
	if len(lb.Entries) != 10 {
		t.Fatalf("expected 10 entries, got %d", len(lb.Entries))
	}

	// 3. Add score that does NOT qualify (score <= lowest score)
	lowest := lb.Entries[len(lb.Entries)-1].Score
	lowEntry := ScoreEntry{
		CaptainName: "Failed Cadet",
		Score:       lowest - 50,
	}
	rank, added, err = AddScenarioScore(ScenarioKobayashiMaru, lowEntry)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if added || rank != 0 {
		t.Errorf("expected added=false, rank=0 for non-qualifying score, got rank=%d, added=%v", rank, added)
	}
}

func TestScenarioLeaderboards_AliasAndPaths(t *testing.T) {
	tempDir := t.TempDir()
	t.Setenv("HOME", tempDir)
	t.Setenv("XDG_CONFIG_HOME", "")

	// Verify alias resolves to the same path
	path1 := ScenarioLeaderboardPath(ScenarioKobayashiMaru)
	path2 := ScenarioLeaderboardPath("km")
	path3 := ScenarioLeaderboardPath("KOBAYASHI")
	if path1 != path2 || path1 != path3 {
		t.Errorf("paths do not match across aliases: %s, %s, %s", path1, path2, path3)
	}

	expectedName := "leaderboard_kobayashi-maru.json"
	if filepath.Base(path1) != expectedName {
		t.Errorf("expected filename %s, got %s", expectedName, filepath.Base(path1))
	}

	// Test nil save error
	var nilLB *Leaderboard
	if err := SaveScenarioLeaderboard(ScenarioKobayashiMaru, nilLB); err == nil {
		t.Errorf("expected error saving nil leaderboard, got nil")
	}
}

func TestLeaderboard_RecordTourAndRankCommission(t *testing.T) {
	lb := NewLeaderboard()
	tour := NewTour(888)
	tour.SectorsCompleted = 4
	tour.Completed = true
	tour.TotalTourScore = 18500
	tour.InstalledRefits[RefitDilithiumCore] = 3

	rec := lb.RecordTour(tour, "Kirk")
	if rec == nil {
		t.Fatal("expected non-nil TourRecord")
	}
	if rec.Rank != "Admiral of the Fleet" {
		t.Errorf("expected Admiral of the Fleet for 4/4 clear, got %s", rec.Rank)
	}
	if len(rec.Medals) == 0 {
		t.Errorf("expected medals awarded for full tour clear")
	}
	if rec.SectorsCleared != 4 {
		t.Errorf("expected 4 sectors cleared, got %d", rec.SectorsCleared)
	}
}

func TestCalculateTourCommission_RanksAndMedals(t *testing.T) {
	tests := []struct {
		name             string
		sectorsCompleted int
		installedRefits  map[RefitID]int
		expectedRank     string
		expectedMedals   []string
	}{
		{
			name:             "4 sectors full clear with architect refits",
			sectorsCompleted: 4,
			installedRefits: map[RefitID]int{
				RefitDilithiumCore: 3,
				RefitTorpedoBays:   3,
			},
			expectedRank: "Admiral of the Fleet",
			expectedMedals: []string{
				"Starfleet Legion of Honor",
				"Klingon Campaign Ribbon",
				"Vanguard Star",
				"Master Starship Architect",
			},
		},
		{
			name:             "3 sectors clear",
			sectorsCompleted: 3,
			installedRefits:  map[RefitID]int{},
			expectedRank:     "Commodore",
			expectedMedals: []string{
				"Starfleet Merit Citation",
				"Klingon Campaign Ribbon",
			},
		},
		{
			name:             "2 sectors clear",
			sectorsCompleted: 2,
			installedRefits:  map[RefitID]int{},
			expectedRank:     "Fleet Captain",
			expectedMedals: []string{
				"Frontier Service Medal",
			},
		},
		{
			name:             "1 sector clear",
			sectorsCompleted: 1,
			installedRefits:  map[RefitID]int{},
			expectedRank:     "Captain",
			expectedMedals: []string{
				"Patrol Ribbon",
			},
		},
		{
			name:             "0 sectors clear (KIA)",
			sectorsCompleted: 0,
			installedRefits:  map[RefitID]int{},
			expectedRank:     "Commander (KIA)",
			expectedMedals:   []string{},
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			tour := NewTour(123)
			tour.SectorsCompleted = tc.sectorsCompleted
			tour.InstalledRefits = tc.installedRefits

			rank, medals := CalculateTourCommission(tour)
			if rank != tc.expectedRank {
				t.Errorf("expected rank %q, got %q", tc.expectedRank, rank)
			}
			if len(medals) != len(tc.expectedMedals) {
				t.Fatalf("expected %d medals, got %d: %v", len(tc.expectedMedals), len(medals), medals)
			}
			for i, m := range tc.expectedMedals {
				if medals[i] != m {
					t.Errorf("expected medal %d to be %q, got %q", i, m, medals[i])
				}
			}
		})
	}

	// Nil tour
	rank, medals := CalculateTourCommission(nil)
	if rank != "Cadet" {
		t.Errorf("expected Cadet for nil tour, got %s", rank)
	}
	if len(medals) != 0 {
		t.Errorf("expected 0 medals for nil tour, got %v", medals)
	}
}

func TestLeaderboard_RecordTourEdgeCases(t *testing.T) {
	lb := NewLeaderboard()

	// Default callsign
	tour := NewTour(101)
	tour.SectorsCompleted = 1
	rec := lb.RecordTour(tour, "")
	if rec == nil {
		t.Fatal("expected non-nil record")
	}
	if rec.Callsign != "Enterprise" {
		t.Errorf("expected default callsign Enterprise, got %s", rec.Callsign)
	}

	// Nil tour
	nilRec := lb.RecordTour(nil, "Enterprise")
	if nilRec != nil {
		t.Errorf("expected nil record for nil tour, got %+v", nilRec)
	}

	// Nil leaderboard
	var nilLB *Leaderboard
	if nilLB.RecordTour(tour, "Enterprise") != nil {
		t.Errorf("expected nil record when recording on nil leaderboard")
	}
}

func TestLeaderboard_SaveAndLoadTourRecords(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "highscores.json")

	lb := NewLeaderboard()
	tour := NewTour(777)
	tour.SectorsCompleted = 4
	tour.Completed = true
	tour.TotalTourScore = 20000
	tour.InstalledRefits[RefitDilithiumCore] = 3
	tour.InstalledRefits[RefitDeflectorGrid] = 3

	rec := lb.RecordTour(tour, "Excelsior")
	if rec == nil {
		t.Fatal("expected non-nil record")
	}

	if err := lb.Save(path); err != nil {
		t.Fatalf("failed to save leaderboard: %v", err)
	}

	loaded, err := LoadLeaderboard(path)
	if err != nil {
		t.Fatalf("failed to load leaderboard: %v", err)
	}

	if len(loaded.TourRecords) != 1 {
		t.Fatalf("expected 1 tour record, got %d", len(loaded.TourRecords))
	}

	loadedRec := loaded.TourRecords[0]
	if loadedRec.Callsign != "Excelsior" {
		t.Errorf("expected callsign Excelsior, got %s", loadedRec.Callsign)
	}
	if loadedRec.Rank != "Admiral of the Fleet" {
		t.Errorf("expected Admiral of the Fleet, got %s", loadedRec.Rank)
	}
	if loadedRec.Score != 20000 {
		t.Errorf("expected score 20000, got %d", loadedRec.Score)
	}
	if loadedRec.SectorsCleared != 4 {
		t.Errorf("expected 4 sectors cleared, got %d", loadedRec.SectorsCleared)
	}
	if loadedRec.RefitsCount != 6 {
		t.Errorf("expected 6 refits count, got %d", loadedRec.RefitsCount)
	}
	if !loadedRec.Completed {
		t.Errorf("expected completed true")
	}
}

