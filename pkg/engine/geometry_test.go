package engine

import (
	"math"
	"path/filepath"
	"testing"
)

func TestCoordinateGeometry(t *testing.T) {
	c1 := Coord{1, 1}
	c2 := Coord{4, 5}
	expectedDist := 5.0
	dist := Distance(c1, c2)
	if math.Abs(dist-expectedDist) > 0.0001 {
		t.Fatalf("expected distance %f, got %f", expectedDist, dist)
	}

	game := NewGame(12345, SkillGood, LengthMedium)
	if game.Enterprise.Energy <= 0 || game.Enterprise.Torpedoes != 10 {
		t.Fatalf("invalid enterprise initial state: %+v", game.Enterprise)
	}
}

func TestDistanceCalculations(t *testing.T) {
	tests := []struct {
		name     string
		c1, c2   Coord
		expected float64
	}{
		{"identical coords", Coord{3, 3}, Coord{3, 3}, 0.0},
		{"horizontal distance", Coord{2, 1}, Coord{2, 6}, 5.0},
		{"vertical distance", Coord{1, 4}, Coord{7, 4}, 6.0},
		{"diagonal 3-4-5", Coord{1, 1}, Coord{4, 5}, 5.0},
		{"full board diagonal", Coord{1, 1}, Coord{8, 8}, math.Hypot(7, 7)},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			got := Distance(tc.c1, tc.c2)
			if math.Abs(got-tc.expected) > 0.0001 {
				t.Errorf("Distance(%v, %v) = %f; expected %f", tc.c1, tc.c2, got, tc.expected)
			}
		})
	}
}

func TestBearingCalculations(t *testing.T) {
	tests := []struct {
		name     string
		from, to Coord
		expected float64
	}{
		{"due east (right)", Coord{4, 1}, Coord{4, 7}, 0.0},
		{"due north (up)", Coord{4, 1}, Coord{1, 1}, math.Pi / 2.0},
		{"due west (left)", Coord{4, 7}, Coord{4, 1}, math.Pi},
		{"due south (down)", Coord{1, 1}, Coord{4, 1}, 3.0 * math.Pi / 2.0},
		{"northeast (45 deg)", Coord{4, 4}, Coord{1, 7}, math.Pi / 4.0},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			got := Bearing(tc.from, tc.to)
			if math.Abs(got-tc.expected) > 0.0001 {
				t.Errorf("Bearing(%v, %v) = %f; expected %f", tc.from, tc.to, got, tc.expected)
			}
		})
	}
}

func TestNewGameInitialization(t *testing.T) {
	seed := int64(42)
	game := NewGame(seed, SkillExpert, LengthLong)

	if game.Skill != SkillExpert {
		t.Errorf("expected skill %v, got %v", SkillExpert, game.Skill)
	}
	if game.Length != LengthLong {
		t.Errorf("expected length %v, got %v", LengthLong, game.Length)
	}
	if game.Enterprise.Energy != 5000 {
		t.Errorf("expected enterprise energy 5000, got %f", game.Enterprise.Energy)
	}
	if game.Enterprise.Shields != 0 {
		t.Errorf("expected enterprise shields 0, got %f", game.Enterprise.Shields)
	}
	if game.Enterprise.Torpedoes != 10 {
		t.Errorf("expected enterprise torpedoes 10, got %d", game.Enterprise.Torpedoes)
	}
	if game.Enterprise.Condition != ConditionGreen {
		t.Errorf("expected condition green, got %v", game.Enterprise.Condition)
	}
	if game.Stardate < 2000 || game.Stardate >= 3000 {
		t.Errorf("stardate out of range [2000, 3000): %f", game.Stardate)
	}
	if game.InitialStardate != game.Stardate {
		t.Errorf("expected InitialStardate %f == Stardate %f", game.InitialStardate, game.Stardate)
	}
	if game.TimeRemaining != 30.0 {
		t.Errorf("expected TimeRemaining 30.0, got %f", game.TimeRemaining)
	}
	if game.RemainingKlingons != 15 {
		t.Errorf("expected RemainingKlingons 15, got %d", game.RemainingKlingons)
	}
	if game.RemainingStarbases != 3 {
		t.Errorf("expected RemainingStarbases 3, got %d", game.RemainingStarbases)
	}

	// Coordinates within 1..8
	if game.Enterprise.Quad[0] < 1 || game.Enterprise.Quad[0] > 8 ||
		game.Enterprise.Quad[1] < 1 || game.Enterprise.Quad[1] > 8 {
		t.Errorf("quad coordinates out of bounds: %v", game.Enterprise.Quad)
	}
	if game.Enterprise.Sector[0] < 1 || game.Enterprise.Sector[0] > 8 ||
		game.Enterprise.Sector[1] < 1 || game.Enterprise.Sector[1] > 8 {
		t.Errorf("sector coordinates out of bounds: %v", game.Enterprise.Sector)
	}
}

func TestEnumsAndConstants(t *testing.T) {
	if EntityEmpty != 0 || EntityBlackHole != 8 {
		t.Errorf("unexpected EntityType enum values: Empty=%d, BlackHole=%d", EntityEmpty, EntityBlackHole)
	}
	if ConditionGreen != 0 || ConditionDocked != 3 {
		t.Errorf("unexpected ConditionType enum values: Green=%d, Docked=%d", ConditionGreen, ConditionDocked)
	}
	if DeviceWarp != 0 || NumDevices != 8 {
		t.Errorf("unexpected DeviceID enum values: Warp=%d, NumDevices=%d", DeviceWarp, NumDevices)
	}
	if SkillNovice != 1 || SkillEmeritus != 5 {
		t.Errorf("unexpected SkillLevel enum values: Novice=%d, Emeritus=%d", SkillNovice, SkillEmeritus)
	}
	if LengthShort != 1 || LengthLong != 3 {
		t.Errorf("unexpected GameLength enum values: Short=%d, Long=%d", LengthShort, LengthLong)
	}
}

func TestProceduralGalaxyGeneration(t *testing.T) {
	seed := int64(12345)
	g := NewGame(seed, SkillGood, LengthMedium)

	totalStars := 0
	totalStarbases := 0
	totalKlingons := 0

	for r := 1; r <= 8; r++ {
		for c := 1; c <= 8; c++ {
			val := g.GalaxyChart[r][c]
			k := val / 100
			b := (val % 100) / 10
			s := val % 10

			if s < 1 || s > 9 {
				t.Errorf("quadrant [%d,%d] invalid star count: %d", r, c, s)
			}
			totalStars += s
			totalStarbases += b
			totalKlingons += k

			if b > 0 && !g.ChartKnownBases[r][c] {
				t.Errorf("quadrant [%d,%d] has starbase but ChartKnownBases is false", r, c)
			}
		}
	}

	if totalStarbases != g.RemainingStarbases {
		t.Errorf("expected total starbases %d, got %d", g.RemainingStarbases, totalStarbases)
	}
	if totalKlingons != g.RemainingKlingons {
		t.Errorf("expected total klingons %d, got %d", g.RemainingKlingons, totalKlingons)
	}
	if !g.ChartDiscovered[g.Enterprise.Quad[0]][g.Enterprise.Quad[1]] {
		t.Errorf("starting quadrant %v was not marked discovered", g.Enterprise.Quad)
	}
}

func TestSaveRoundtripDiscoveryAndBases(t *testing.T) {
	tempDir := t.TempDir()
	savePath := filepath.Join(tempDir, "TESTDISC.TRK")

	g := NewGame(12345, SkillGood, LengthMedium)
	g.ChartDiscovered[2][3] = true
	g.ChartKnownBases[4][5] = true

	if err := g.Save(savePath); err != nil {
		t.Fatalf("failed to save: %v", err)
	}

	loaded, err := LoadGame(savePath)
	if err != nil {
		t.Fatalf("failed to load: %v", err)
	}

	if loaded.GalaxyChart != g.GalaxyChart {
		t.Errorf("GalaxyChart mismatch after save/load")
	}
	if loaded.ChartDiscovered != g.ChartDiscovered {
		t.Errorf("ChartDiscovered mismatch after save/load")
	}
	if loaded.ChartKnownBases != g.ChartKnownBases {
		t.Errorf("ChartKnownBases mismatch after save/load")
	}
}
