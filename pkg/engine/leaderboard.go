package engine

import (
	"encoding/json"
	"errors"
	"os"
	"path/filepath"
	"sort"
	"time"
)

// ScoreEntry records an individual captain's completed mission score and credentials.
type ScoreEntry struct {
	CaptainName string    `json:"captain_name"`
	Score       int       `json:"score"`
	Rank        string    `json:"rank"`
	Skill       string    `json:"skill,omitempty"`
	Difficulty  string    `json:"difficulty,omitempty"`
	Stardate    float64   `json:"stardate,omitempty"`
	Date        time.Time `json:"date"`
	GameWon     bool      `json:"game_won,omitempty"`
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

// LoadLeaderboard reads a leaderboard from disk. If the file does not exist,
// it returns DefaultLeaderboard(). If the file is corrupted JSON, it backs up
// the file to path + ".corrupt" and falls back to DefaultLeaderboard().
func LoadLeaderboard(path string) (*Leaderboard, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			return DefaultLeaderboard(), nil
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
