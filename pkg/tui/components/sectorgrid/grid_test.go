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

	view := m.View(&quad, engine.Coord{2, 3})

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

	view := m.View(&quad, engine.Coord{1, 1})

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
	view := m.View(&quad, engine.Coord{1, 1})

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

	viewModern := m.View(&quad, engine.Coord{1, 1})

	m.SetTheme(theme.LcarsTheme{})
	if m.Theme().Name() != "lcars" {
		t.Fatalf("expected lcars theme, got %s", m.Theme().Name())
	}
	viewLcars := m.View(&quad, engine.Coord{1, 1})

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
	viewFallback := m.View(&quad, engine.Coord{1, 1})
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
	view := m.View(nil, engine.Coord{3, 4})
	if !strings.Contains(view, "<E>") {
		t.Fatalf("expected Enterprise glyph even with nil quadrant:\n%s", view)
	}
	if !strings.Contains(view, " . ") {
		t.Fatalf("expected empty cell glyphs with nil quadrant:\n%s", view)
	}

	// Out-of-bounds entSector
	viewNoEnt := m.View(nil, engine.Coord{0, 0})
	if strings.Contains(viewNoEnt, "<E>") {
		t.Fatalf("did not expect Enterprise glyph with out-of-bounds entSector:\n%s", viewNoEnt)
	}
}
