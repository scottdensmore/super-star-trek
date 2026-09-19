package engine

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"time"
)

// ScoreEntry records an individual captain's completed mission score and credentials.
type ScoreEntry struct {
	CaptainName  string    `json:"captain_name"`
	Score        int       `json:"score"`
	Rank         string    `json:"rank"`
	Skill        string    `json:"skill,omitempty"`
	Difficulty   string    `json:"difficulty,omitempty"`
	Stardate     float64   `json:"stardate,omitempty"`
	Date         time.Time `json:"date"`
	GameWon      bool      `json:"game_won,omitempty"`
	Scenario     string    `json:"scenario,omitempty"`
	Commendation string    `json:"commendation,omitempty"`
}

// Leaderboard stores a list of top scores, capped at the top 10 rankings.
type Leaderboard struct {
	Entries []ScoreEntry `json:"entries"`
}

// DefaultLeaderboard returns a leaderboard seeded with classic Starfleet legends.
func DefaultLeaderboard() *Leaderboard {
	defaultDate := time.Date(2265, time.January, 1, 0, 0, 0, 0, time.UTC)
	return &Leaderboard{
		Entries: []ScoreEntry{
			{
				CaptainName: "James T. Kirk",
				Score:       1180,
				Rank:        "[FADM]",
				Skill:       "Emeritus",
				Difficulty:  "Emeritus",
				Stardate:    3421.5,
				Date:        defaultDate,
				GameWon:     true,
			},
			{
				CaptainName: "Spock",
				Score:       980,
				Rank:        "[RADM]",
				Skill:       "Expert",
				Difficulty:  "Expert",
				Stardate:    3210.4,
				Date:        defaultDate,
				GameWon:     true,
			},
			{
				CaptainName: "Christopher Pike",
				Score:       840,
				Rank:        "[COMM]",
				Skill:       "Expert",
				Difficulty:  "Expert",
				Stardate:    3105.2,
				Date:        defaultDate,
				GameWon:     true,
			},
			{
				CaptainName: "Hikaru Sulu",
				Score:       720,
				Rank:        "[COMM]",
				Skill:       "Good",
				Difficulty:  "Good",
				Stardate:    2980.1,
				Date:        defaultDate,
				GameWon:     true,
			},
			{
				CaptainName: "Nyota Uhura",
				Score:       560,
				Rank:        "[CAPT]",
				Skill:       "Good",
				Difficulty:  "Good",
				Stardate:    2840.7,
				Date:        defaultDate,
				GameWon:     true,
			},
			{
				CaptainName: "Montgomery Scott",
				Score:       480,
				Rank:        "[CDR]",
				Skill:       "Good",
				Difficulty:  "Good",
				Stardate:    2710.3,
				Date:        defaultDate,
				GameWon:     true,
			},
			{
				CaptainName: "Leonard McCoy",
				Score:       420,
				Rank:        "[CDR]",
				Skill:       "Fair",
				Difficulty:  "Fair",
				Stardate:    2650.9,
				Date:        defaultDate,
				GameWon:     true,
			},
			{
				CaptainName: "Pavel Chekov",
				Score:       310,
				Rank:        "[LT]",
				Skill:       "Fair",
				Difficulty:  "Fair",
				Stardate:    2510.6,
				Date:        defaultDate,
				GameWon:     true,
			},
			{
				CaptainName: "Christine Chapel",
				Score:       180,
				Rank:        "[LT]",
				Skill:       "Novice",
				Difficulty:  "Novice",
				Stardate:    2390.2,
				Date:        defaultDate,
				GameWon:     true,
			},
			{
				CaptainName: "Janice Rand",
				Score:       90,
				Rank:        "[CADET]",
				Skill:       "Novice",
				Difficulty:  "Novice",
				Stardate:    2210.8,
				Date:        defaultDate,
				GameWon:     true,
			},
		},
	}
}

// DefaultLeaderboardPath resolves the storage path for the high scores file.
// It checks $XDG_CONFIG_HOME/super-star-trek/highscores.json,
// falls back to ~/.config/super-star-trek/highscores.json or os.UserConfigDir(),
// and if neither is set, falls back to ./.sst-scores.json.
func DefaultLeaderboardPath() string {
	configDir := os.Getenv("XDG_CONFIG_HOME")
	if configDir == "" {
		if home := os.Getenv("HOME"); home != "" {
			configDir = filepath.Join(home, ".config")
		} else if ucd, err := os.UserConfigDir(); err == nil {
			configDir = ucd
		}
	}
	if configDir != "" {
		return filepath.Join(configDir, "super-star-trek", "highscores.json")
	}
	return "./.sst-scores.json"
}

// Qualifies returns whether the given score qualifies for inclusion on the leaderboard.
// A score qualifies if the leaderboard has fewer than 10 entries or if it beats
// the lowest recorded score.
func (lb *Leaderboard) Qualifies(score int) bool {
	if lb == nil {
		return false
	}
	if len(lb.Entries) < 10 {
		return true
	}
	return score > lb.Entries[len(lb.Entries)-1].Score
}

// Add inserts a score entry, sorts the leaderboard descending by score,
// and truncates the list to the top 10.
func (lb *Leaderboard) Add(entry ScoreEntry) {
	if lb == nil {
		return
	}
	lb.Entries = append(lb.Entries, entry)
	sort.SliceStable(lb.Entries, func(i, j int) bool {
		return lb.Entries[i].Score > lb.Entries[j].Score
	})
	if len(lb.Entries) > 10 {
		lb.Entries = lb.Entries[:10]
	}
}

// LoadLeaderboard reads a leaderboard from disk. If no path is specified, it uses DefaultLeaderboardPath().
// If the file does not exist, it returns DefaultLeaderboard(). If the file is corrupted JSON,
// it backs up the file to path + ".corrupt" and falls back to DefaultLeaderboard().
func LoadLeaderboard(path ...string) (*Leaderboard, error) {
	targetPath := DefaultLeaderboardPath()
	if len(path) > 0 && path[0] != "" {
		targetPath = path[0]
	}
	data, err := os.ReadFile(targetPath)
	if err != nil {
		if os.IsNotExist(err) {
			return DefaultLeaderboard(), nil
		}
		return nil, err
	}

	var lb Leaderboard
	if err := json.Unmarshal(data, &lb); err != nil {
		// Corrupted file recovery: backup corrupted file and fallback to default
		if renameErr := os.Rename(targetPath, targetPath+".corrupt"); renameErr != nil {
			_ = os.WriteFile(targetPath+".corrupt", data, 0644)
			_ = os.Remove(targetPath)
		}
		return DefaultLeaderboard(), nil
	}

	if lb.Entries == nil {
		lb.Entries = make([]ScoreEntry, 0)
	}

	sort.SliceStable(lb.Entries, func(i, j int) bool {
		return lb.Entries[i].Score > lb.Entries[j].Score
	})
	if len(lb.Entries) > 10 {
		lb.Entries = lb.Entries[:10]
	}

	return &lb, nil
}

// Save persists the leaderboard to disk atomically using a temporary file
// and os.Rename to prevent corruption.
func (lb *Leaderboard) Save(path string) error {
	if lb == nil {
		return errors.New("cannot save nil leaderboard")
	}

	dir := filepath.Dir(path)
	if err := os.MkdirAll(dir, 0755); err != nil {
		return err
	}

	data, err := json.MarshalIndent(lb, "", "  ")
	if err != nil {
		return err
	}

	tmpFile, err := os.CreateTemp(dir, "highscores-*.tmp")
	if err != nil {
		return err
	}
	tmpPath := tmpFile.Name()
	defer func() {
		if tmpPath != "" {
			_ = os.Remove(tmpPath)
		}
	}()

	if _, err := tmpFile.Write(data); err != nil {
		_ = tmpFile.Close()
		return err
	}
	if err := tmpFile.Sync(); err != nil {
		_ = tmpFile.Close()
		return err
	}
	if err := tmpFile.Close(); err != nil {
		return err
	}

	if err := os.Rename(tmpPath, path); err != nil {
		return err
	}
	tmpPath = ""
	return nil
}

// ScenarioLeaderboardPath resolves the file path for a scenario's dedicated leaderboard.
func ScenarioLeaderboardPath(id ScenarioID) string {
	normID := id
	if s, ok := GetScenario(id); ok {
		normID = s.ID
	}
	dir := filepath.Dir(DefaultLeaderboardPath())
	return filepath.Join(dir, fmt.Sprintf("leaderboard_%s.json", normID))
}

// DefaultScenarioLeaderboard returns the initial historical hall of fame for a specific scenario.
func DefaultScenarioLeaderboard(id ScenarioID) *Leaderboard {
	normID := id
	if s, ok := GetScenario(id); ok {
		normID = s.ID
	}
	defaultDate := time.Date(2265, time.January, 1, 0, 0, 0, 0, time.UTC)
	switch normID {
	case ScenarioKobayashiMaru:
		return &Leaderboard{
			Entries: []ScoreEntry{
				{
					CaptainName:  "Cadet James T. Kirk",
					Score:        2400,
					Rank:         "[KIRK-AWARD]",
					Commendation: "Commendation for Original Thinking",
					Scenario:     string(ScenarioKobayashiMaru),
					Date:         defaultDate,
				},
				{
					CaptainName:  "Cadet Spock",
					Score:        950,
					Rank:         "[COMM-3]",
					Commendation: "Starfleet Cross of Honor",
					Scenario:     string(ScenarioKobayashiMaru),
					Date:         defaultDate,
				},
				{
					CaptainName:  "Lieutenant Saavik",
					Score:        620,
					Rank:         "[COMM-2]",
					Commendation: "Tactical Excellence",
					Scenario:     string(ScenarioKobayashiMaru),
					Date:         defaultDate,
				},
			},
		}
	case ScenarioMutaraNebula:
		return &Leaderboard{
			Entries: []ScoreEntry{
				{
					CaptainName: "Admiral James T. Kirk",
					Score:       1500,
					Rank:        "[ADM]",
					Scenario:    string(ScenarioMutaraNebula),
					Date:        defaultDate,
					GameWon:     true,
				},
				{
					CaptainName: "Captain Clark Terrell",
					Score:       820,
					Rank:        "[CAPT]",
					Scenario:    string(ScenarioMutaraNebula),
					Date:        defaultDate,
					GameWon:     false,
				},
			},
		}
	case ScenarioStarbaseSiege:
		return &Leaderboard{
			Entries: []ScoreEntry{
				{
					CaptainName: "Captain Hikaru Sulu",
					Score:       1350,
					Rank:        "[CAPT]",
					Scenario:    string(ScenarioStarbaseSiege),
					Date:        defaultDate,
					GameWon:     true,
				},
				{
					CaptainName: "Commander Montgomery Scott",
					Score:       1100,
					Rank:        "[COMM]",
					Scenario:    string(ScenarioStarbaseSiege),
					Date:        defaultDate,
					GameWon:     true,
				},
			},
		}
	default:
		return &Leaderboard{
			Entries: []ScoreEntry{},
		}
	}
}

// LoadScenarioLeaderboard reads a scenario leaderboard from its dedicated file.
// If the file does not exist, it returns DefaultScenarioLeaderboard(id).
// If the file is corrupted JSON, it backs up the file and falls back to default.
func LoadScenarioLeaderboard(id ScenarioID) (*Leaderboard, error) {
	path := ScenarioLeaderboardPath(id)
	data, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			return DefaultScenarioLeaderboard(id), nil
		}
		return nil, err
	}

	var lb Leaderboard
	if err := json.Unmarshal(data, &lb); err != nil {
		// Corrupted file recovery: backup corrupted file and fallback to default
		if renameErr := os.Rename(path, path+".corrupt"); renameErr != nil {
			_ = os.WriteFile(path+".corrupt", data, 0644)
			_ = os.Remove(path)
		}
		return DefaultScenarioLeaderboard(id), nil
	}

	if lb.Entries == nil {
		lb.Entries = make([]ScoreEntry, 0)
	}

	sort.SliceStable(lb.Entries, func(i, j int) bool {
		return lb.Entries[i].Score > lb.Entries[j].Score
	})
	if len(lb.Entries) > 10 {
		lb.Entries = lb.Entries[:10]
	}

	return &lb, nil
}

// SaveScenarioLeaderboard persists the scenario leaderboard to disk atomically.
func SaveScenarioLeaderboard(id ScenarioID, lb *Leaderboard) error {
	path := ScenarioLeaderboardPath(id)
	return lb.Save(path)
}

// AddScenarioScore records a completed scenario run, saves it to the scenario's
// isolated leaderboard, and returns the 1-based rank and whether the entry qualified and was added.
func AddScenarioScore(id ScenarioID, entry ScoreEntry) (int, bool, error) {
	normID := id
	if s, ok := GetScenario(id); ok {
		normID = s.ID
	}
	lb, err := LoadScenarioLeaderboard(normID)
	if err != nil {
		return 0, false, err
	}

	if entry.Date.IsZero() {
		entry.Date = time.Now()
	}
	if entry.Scenario == "" {
		entry.Scenario = string(normID)
	}

	if !lb.Qualifies(entry.Score) {
		return 0, false, nil
	}

	lb.Add(entry)

	rank := 0
	for i := len(lb.Entries) - 1; i >= 0; i-- {
		e := lb.Entries[i]
		if e.CaptainName == entry.CaptainName && e.Score == entry.Score && e.Date.Equal(entry.Date) {
			rank = i + 1
			break
		}
	}
	if rank == 0 {
		for i := len(lb.Entries) - 1; i >= 0; i-- {
			e := lb.Entries[i]
			if e.CaptainName == entry.CaptainName && e.Score == entry.Score {
				rank = i + 1
				break
			}
		}
	}
	if rank == 0 {
		rank = 1
	}

	if err := SaveScenarioLeaderboard(normID, lb); err != nil {
		return 0, false, err
	}

	return rank, true, nil
}
