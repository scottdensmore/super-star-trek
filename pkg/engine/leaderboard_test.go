package engine

import (
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
