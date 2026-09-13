package sectorgrid

import (
	"strings"
	"testing"

	"github.com/scottdensmore/super-star-trek/pkg/engine"
	"github.com/scottdensmore/super-star-trek/pkg/tui/theme"
)

func TestGridRendering(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th)

	var quad engine.QuadrantState
	quad.Grid[2][3] = engine.EntityEnterprise
	quad.Grid[4][5] = engine.EntityKlingon
	quad.Grid[1][1] = engine.EntityStarbase
	quad.Grid[8][8] = engine.EntityStar

	view := m.View(&quad, engine.Coord{2, 3}, engine.Coord{})

	// Check header contains columns 1..8
	if !strings.Contains(view, "1") || !strings.Contains(view, "8") {
		t.Fatalf("grid view missing column headers:\n%s", view)
	}

	// Check entity glyphs
	if !strings.Contains(view, "<E>") {
		t.Fatalf("grid view missing Enterprise glyph <E>:\n%s", view)
	}
	if !strings.Contains(view, "+K+") {
		t.Fatalf("grid view missing Klingon glyph +K+:\n%s", view)
	}
	if !strings.Contains(view, ">B<") {
		t.Fatalf("grid view missing Starbase glyph >B<:\n%s", view)
	}
	if !strings.Contains(view, " * ") {
		t.Fatalf("grid view missing Star glyph *:\n%s", view)
	}
}

func TestAllEntityGlyphs(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th)

	var quad engine.QuadrantState
	quad.Grid[1][1] = engine.EntityEnterprise
	quad.Grid[1][2] = engine.EntityKlingon
	quad.Grid[1][3] = engine.EntityCommander
	quad.Grid[1][4] = engine.EntitySuperCommander
	quad.Grid[1][5] = engine.EntityStarbase
	quad.Grid[1][6] = engine.EntityStar
	quad.Grid[1][7] = engine.EntityPlanet
	quad.Grid[1][8] = engine.EntityBlackHole
	// row 2 is empty

	view := m.View(&quad, engine.Coord{1, 1}, engine.Coord{})

	expectedGlyphs := []struct {
		name  string
		glyph string
	}{
		{"Enterprise", "<E>"},
		{"Klingon", "+K+"},
		{"Starbase", ">B<"},
		{"Star", " * "},
		{"Planet", " O "},
		{"BlackHole", " @ "},
		{"Empty", " . "},
	}

	for _, eg := range expectedGlyphs {
		if !strings.Contains(view, eg.glyph) {
			t.Errorf("grid view missing %s glyph %q:\n%s", eg.name, eg.glyph, view)
		}
	}
}

func TestGridAxes(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th)

	var quad engine.QuadrantState
	view := m.View(&quad, engine.Coord{1, 1}, engine.Coord{})

	for i := 1; i <= 8; i++ {
		digit := string(rune('0' + i))
		if !strings.Contains(view, digit) {
			t.Errorf("grid view missing axis digit %s:\n%s", digit, view)
		}
	}
}

func TestSetTheme(t *testing.T) {
	m := New(theme.ModernTheme{})
	if m.Theme().Name() != "modern" {
		t.Fatalf("expected modern theme, got %s", m.Theme().Name())
	}

	var quad engine.QuadrantState
	quad.Grid[1][1] = engine.EntityEnterprise

	viewModern := m.View(&quad, engine.Coord{1, 1}, engine.Coord{})

	m.SetTheme(theme.LcarsTheme{})
	if m.Theme().Name() != "lcars" {
		t.Fatalf("expected lcars theme, got %s", m.Theme().Name())
	}
	viewLcars := m.View(&quad, engine.Coord{1, 1}, engine.Coord{})

	if viewModern == "" || viewLcars == "" {
		t.Fatalf("expected non-empty rendered views")
	}
	if viewModern == viewLcars {
		t.Errorf("expected different styled/bordered output between Modern and LCARS themes")
	}

	// Setting nil theme should fallback to default without panicking
	m.SetTheme(nil)
	if m.Theme().Name() != "modern" {
		t.Fatalf("expected fallback to modern theme, got %s", m.Theme().Name())
	}
	viewFallback := m.View(&quad, engine.Coord{1, 1}, engine.Coord{})
	if viewFallback != viewModern {
		t.Errorf("expected fallback to default modern theme view")
	}
}

func TestNilQuadrantAndDefaults(t *testing.T) {
	m := New(nil) // should fallback to default theme
	if m.Theme().Name() != "modern" {
		t.Fatalf("expected default modern theme for nil argument")
	}

	// Nil quadrant should not panic, but render grid with Enterprise at entSector
	view := m.View(nil, engine.Coord{3, 4}, engine.Coord{})
	if !strings.Contains(view, "<E>") {
		t.Fatalf("expected Enterprise glyph even with nil quadrant:\n%s", view)
	}
	if !strings.Contains(view, " . ") {
		t.Fatalf("expected empty cell glyphs with nil quadrant:\n%s", view)
	}

	// Out-of-bounds entSector
	viewNoEnt := m.View(nil, engine.Coord{0, 0}, engine.Coord{})
	if strings.Contains(viewNoEnt, "<E>") {
		t.Fatalf("did not expect Enterprise glyph with out-of-bounds entSector:\n%s", viewNoEnt)
	}
}

func TestHitTest_ValidCells(t *testing.T) {
	m := New(nil)

	// Every sector cell from (1,1) to (8,8)
	for r := 1; r <= 8; r++ {
		for c := 1; c <= 8; c++ {
			// Cell c occupies relX: 2 + 4*(c-1), 2 + 4*(c-1) + 1, 2 + 4*(c-1) + 2
			startX := 2 + 4*(c-1)
			for dx := 0; dx < 3; dx++ {
				relX := startX + dx
				relY := r
				coord, ok := m.HitTest(relX, relY)
				if !ok {
					t.Fatalf("expected HitTest(%d, %d) to return ok=true for cell [%d, %d]", relX, relY, r, c)
				}
				expected := engine.Coord{r, c}
				if coord != expected {
					t.Fatalf("HitTest(%d, %d) = %v, expected %v", relX, relY, coord, expected)
				}
			}
		}
	}
}

func TestHitTest_OutOfBoundsAndBorders(t *testing.T) {
	m := New(nil)

	cases := []struct {
		name string
		relX int
		relY int
	}{
		// Vertical bounds
		{"header line 0", 2, 0},
		{"negative Y", 2, -1},
		{"below grid row 9", 2, 9},
		{"below grid row 20", 5, 20},

		// Horizontal bounds (left)
		{"row header col 0", 0, 1},
		{"row header col 1", 1, 1},
		{"negative X", -1, 1},

		// Separator spaces between cells: relX = 5, 9, 13, 17, 21, 25, 29
		{"separator between c1 and c2", 5, 1},
		{"separator between c2 and c3", 9, 2},
		{"separator between c3 and c4", 13, 3},
		{"separator between c4 and c5", 17, 4},
		{"separator between c5 and c6", 21, 5},
		{"separator between c6 and c7", 25, 6},
		{"separator between c7 and c8", 29, 7},

		// Horizontal bounds (right)
		{"trailing space after c8", 33, 8},
		{"beyond grid col 34", 34, 1},
		{"far right col 50", 50, 4},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			coord, ok := m.HitTest(tc.relX, tc.relY)
			if ok {
				t.Fatalf("expected HitTest(%d, %d) to return false, got true with coord %v", tc.relX, tc.relY, coord)
			}
			if coord != (engine.Coord{}) {
				t.Fatalf("expected zero Coord for out of bounds hit, got %v", coord)
			}
		})
	}
}

func TestViewWithReticle(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th)

	var quad engine.QuadrantState
	quad.Grid[2][3] = engine.EntityEnterprise
	quad.Grid[4][5] = engine.EntityKlingon
	quad.Grid[1][1] = engine.EntityStarbase
	quad.Grid[6][7] = engine.EntityStar
	quad.Grid[7][8] = engine.EntityPlanet
	quad.Grid[8][1] = engine.EntityBlackHole

	// 1. When selected is [3, 4] (an empty cell), it should render as "[.]"
	view := m.View(&quad, engine.Coord{2, 3}, engine.Coord{3, 4})
	if !strings.Contains(view, "[.]") {
		t.Fatalf("expected reticle [.] at empty cell [3, 4], view:\n%s", view)
	}

	// 2. When selected is [2, 3] (Enterprise), it should render as "[E]"
	viewEnt := m.View(&quad, engine.Coord{2, 3}, engine.Coord{2, 3})
	if !strings.Contains(viewEnt, "[E]") {
		t.Fatalf("expected reticle [E] at Enterprise cell [2, 3], view:\n%s", viewEnt)
	}
	if strings.Contains(viewEnt, "<E>") {
		t.Fatalf("did not expect unbracketed <E> when Enterprise is selected:\n%s", viewEnt)
	}

	// 3. When selected is [4, 5] (Klingon), it should render as "[K]"
	viewKlingon := m.View(&quad, engine.Coord{2, 3}, engine.Coord{4, 5})
	if !strings.Contains(viewKlingon, "[K]") {
		t.Fatalf("expected reticle [K] at Klingon cell [4, 5], view:\n%s", viewKlingon)
	}
	if strings.Contains(viewKlingon, "+K+") {
		t.Fatalf("did not expect unbracketed +K+ when Klingon is selected:\n%s", viewKlingon)
	}

	// 4. When selected is [1, 1] (Starbase), it should render as "[B]"
	viewBase := m.View(&quad, engine.Coord{2, 3}, engine.Coord{1, 1})
	if !strings.Contains(viewBase, "[B]") {
		t.Fatalf("expected reticle [B] at Starbase cell [1, 1], view:\n%s", viewBase)
	}

	// 5. Star, Planet, BlackHole reticle brackets
	viewStar := m.View(&quad, engine.Coord{2, 3}, engine.Coord{6, 7})
	if !strings.Contains(viewStar, "[*]") {
		t.Fatalf("expected reticle [*] at Star cell [6, 7], view:\n%s", viewStar)
	}
	viewPlanet := m.View(&quad, engine.Coord{2, 3}, engine.Coord{7, 8})
	if !strings.Contains(viewPlanet, "[O]") {
		t.Fatalf("expected reticle [O] at Planet cell [7, 8], view:\n%s", viewPlanet)
	}
	viewHole := m.View(&quad, engine.Coord{2, 3}, engine.Coord{8, 1})
	if !strings.Contains(viewHole, "[@]") {
		t.Fatalf("expected reticle [@] at BlackHole cell [8, 1], view:\n%s", viewHole)
	}

	// 6. When selected is [0, 0] (no selection), no brackets should appear
	viewNoSel := m.View(&quad, engine.Coord{2, 3}, engine.Coord{0, 0})
	for _, b := range []string{"[.]", "[E]", "[K]", "[B]", "[*]", "[O]", "[@]"} {
		if strings.Contains(viewNoSel, b) {
			t.Fatalf("did not expect reticle %s when selected is [0, 0]:\n%s", b, viewNoSel)
		}
	}
}

func TestCloakedKlingonRendering_SensorAnomaly(t *testing.T) {
	th := theme.DefaultTheme()
	m := New(th)

	var quad engine.QuadrantState
	quad.Grid[4][5] = engine.EntityCommander
	cloakedKlingon := &engine.Klingon{
		ID:          1,
		Sector:      engine.Coord{4, 5},
		IsCommander: true,
		IsCloaked:   true,
	}
	quad.Klingons = []*engine.Klingon{cloakedKlingon}

	// 1. Unselected cloaked Klingon should render as " ? " sensor ghost, NOT "+K+"
	viewUnselected := m.View(&quad, engine.Coord{1, 1}, engine.Coord{0, 0})
	if !strings.Contains(viewUnselected, " ? ") {
		t.Fatalf("expected cloaked Klingon to render as sensor anomaly echo ' ? ', got view:\n%s", viewUnselected)
	}
	if strings.Contains(viewUnselected, "+K+") {
		t.Fatalf("did not expect '+K+' to render for cloaked Klingon, got view:\n%s", viewUnselected)
	}

	// 2. Selected cloaked Klingon should render as "[?]", NOT "[K]"
	viewSelected := m.View(&quad, engine.Coord{1, 1}, engine.Coord{4, 5})
	if !strings.Contains(viewSelected, "[?]") {
		t.Fatalf("expected selected cloaked Klingon to render as '[?]', got view:\n%s", viewSelected)
	}
	if strings.Contains(viewSelected, "[K]") {
		t.Fatalf("did not expect '[K]' to render for selected cloaked Klingon, got view:\n%s", viewSelected)
	}

	// 3. When decloaked, should render as standard Klingon "+K+" / "[K]"
	cloakedKlingon.IsCloaked = false
	viewDecloaked := m.View(&quad, engine.Coord{1, 1}, engine.Coord{0, 0})
	if !strings.Contains(viewDecloaked, "+K+") {
		t.Fatalf("expected decloaked Klingon to render as '+K+', got view:\n%s", viewDecloaked)
	}
	if strings.Contains(viewDecloaked, " ? ") {
		t.Fatalf("did not expect ' ? ' for decloaked Klingon, got view:\n%s", viewDecloaked)
	}

	viewDecloakedSel := m.View(&quad, engine.Coord{1, 1}, engine.Coord{4, 5})
	if !strings.Contains(viewDecloakedSel, "[K]") {
		t.Fatalf("expected selected decloaked Klingon to render as '[K]', got view:\n%s", viewDecloakedSel)
	}

	// 4. Quadrant with 1 cloaked commander and 1 uncloaked Klingon
	cloakedKlingon.IsCloaked = true
	uncloakedKlingon := &engine.Klingon{
		ID:          2,
		Sector:      engine.Coord{6, 6},
		IsCommander: false,
		IsCloaked:   false,
	}
	quad.Grid[6][6] = engine.EntityKlingon
	quad.Klingons = []*engine.Klingon{cloakedKlingon, uncloakedKlingon}

	viewBoth := m.View(&quad, engine.Coord{1, 1}, engine.Coord{0, 0})
	if !strings.Contains(viewBoth, " ? ") {
		t.Fatalf("expected view to contain ' ? ' for cloaked commander:\n%s", viewBoth)
	}
	if !strings.Contains(viewBoth, "+K+") {
		t.Fatalf("expected view to contain '+K+' for uncloaked Klingon:\n%s", viewBoth)
	}
}
