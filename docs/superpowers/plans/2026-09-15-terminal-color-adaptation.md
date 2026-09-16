# Terminal Color & System Luminance Adaptation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement automatic terminal background darkness detection and adaptive Light/Dark palettes across all TUI themes (Modern, LCARS, CRT), with CLI flag and in-game controls (Options modal, `Shift+F2`/`Ctrl+T`, and Spock command palette).

**Architecture:** Introduce `ColorMode` (`auto`, `dark`, `light`) into `pkg/tui/theme`. Update `ModernTheme`, `LcarsTheme`, and `CrtTheme` to provide paired light/dark palettes via `PaletteStyles(isDark bool)`. Integrate dynamic background detection and user mode toggling into `pkg/tui.Model`, `optionsmodal`, `commandpalette`, and `cmd/sst/main.go`. Protect existing dark golden snapshot baselines while adding new light mode golden snapshot fixtures.

**Tech Stack:** Go 1.26+, Charmbracelet Bubble Tea (`v1.3.4`), Lip Gloss (`v1.0.0`), Termenv.

**Spec:** [`docs/superpowers/specs/2026-09-15-terminal-color-adaptation-design.md`](file:///Users/scottdensmore/Developer/super-star-trek/docs/superpowers/specs/2026-09-15-terminal-color-adaptation-design.md)

## Global Constraints

- 100% adherence to original gameplay and combat formulas.
- Backward compatibility: `Theme.Styles()` must continue to return valid styles for all existing call sites.
- Golden snapshots: Existing dark snapshots must remain bit-for-bit identical; light snapshots must strictly conform to 80×24 terminal budget.
- Concurrency: Zero data races under `go test -v -race ./...`.

---

### Task 1: Core ColorMode and Extended Theme Interface

**Files:**
- Modify: `pkg/tui/theme/theme.go`
- Test: `pkg/tui/theme/theme_test.go`

**Interfaces:**
- Produces:
  - `type ColorMode string` (`ColorModeAuto`, `ColorModeDark`, `ColorModeLight`)
  - `func (m ColorMode) Resolve(hasDarkBg bool) bool`
  - `func (m ColorMode) Next() ColorMode`
  - `func ParseColorMode(val string) (ColorMode, error)`
  - Extended `Theme` interface: `ColorMode() ColorMode`, `WithColorMode(mode ColorMode) Theme`, `PaletteStyles(isDark bool) Styles`

- [ ] **Step 1: Write failing unit tests for ColorMode in `pkg/tui/theme/theme_test.go`**

```go
func TestColorModeResolution(t *testing.T) {
	tests := []struct {
		mode       ColorMode
		terminalDark bool
		expectedDark bool
	}{
		{ColorModeAuto, true, true},
		{ColorModeAuto, false, false},
		{ColorModeDark, true, true},
		{ColorModeDark, false, true},
		{ColorModeLight, true, false},
		{ColorModeLight, false, false},
	}

	for _, tt := range tests {
		got := tt.mode.Resolve(tt.terminalDark)
		if got != tt.expectedDark {
			t.Errorf("%s.Resolve(%v) = %v, expected %v", tt.mode, tt.terminalDark, got, tt.expectedDark)
		}
	}
}

func TestColorModeNext(t *testing.T) {
	if ColorModeAuto.Next() != ColorModeDark {
		t.Errorf("expected Auto.Next() == Dark, got %s", ColorModeAuto.Next())
	}
	if ColorModeDark.Next() != ColorModeLight {
		t.Errorf("expected Dark.Next() == Light, got %s", ColorModeDark.Next())
	}
	if ColorModeLight.Next() != ColorModeAuto {
		t.Errorf("expected Light.Next() == Auto, got %s", ColorModeLight.Next())
	}
}

func TestParseColorMode(t *testing.T) {
	tests := []struct {
		input       string
		expected    ColorMode
		expectError bool
	}{
		{"auto", ColorModeAuto, false},
		{"AUTO", ColorModeAuto, false},
		{"dark", ColorModeDark, false},
		{"Dark", ColorModeDark, false},
		{"light", ColorModeLight, false},
		{"LIGHT", ColorModeLight, false},
		{"invalid", ColorModeAuto, true},
	}

	for _, tt := range tests {
		got, err := ParseColorMode(tt.input)
		if tt.expectError && err == nil {
			t.Errorf("expected error for input %q, got nil", tt.input)
		}
		if !tt.expectError && err != nil {
			t.Errorf("unexpected error for input %q: %v", tt.input, err)
		}
		if got != tt.expected {
			t.Errorf("ParseColorMode(%q) = %v, expected %v", tt.input, got, tt.expected)
		}
	}
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v ./pkg/tui/theme -run TestColorMode`
Expected: Compilation failure or undefined `ColorMode`, `ParseColorMode`.

- [ ] **Step 3: Implement ColorMode and update Theme interface in `pkg/tui/theme/theme.go`**

Add `ColorMode` constants, methods, and update `Theme` interface in `pkg/tui/theme/theme.go`. Ensure `DefaultTheme()` returns `ModernTheme` with `ColorModeAuto`.

- [ ] **Step 4: Run test to verify it passes**

Run: `go test -v ./pkg/tui/theme -run TestColorMode`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add pkg/tui/theme/theme.go pkg/tui/theme/theme_test.go
git commit -m "feat(theme): add ColorMode enum and extend Theme interface"
```

---

### Task 2: Paired Light & Dark Palettes for Modern, LCARS, and CRT Themes

**Files:**
- Modify: `pkg/tui/theme/modern.go`
- Modify: `pkg/tui/theme/lcars.go`
- Modify: `pkg/tui/theme/crt.go`
- Test: `pkg/tui/theme/theme_test.go`

**Interfaces:**
- Consumes: `ColorMode`, `Theme` interface from Task 1
- Produces: Complete light and dark styles for `ModernTheme`, `LcarsTheme`, and `CrtTheme`

- [ ] **Step 1: Write failing unit tests for theme light/dark palettes in `pkg/tui/theme/theme_test.go`**

```go
func TestThemePaletteStyles(t *testing.T) {
	themes := []Theme{
		ModernTheme{},
		LcarsTheme{},
		CrtTheme{},
	}

	for _, th := range themes {
		darkStyles := th.PaletteStyles(true)
		lightStyles := th.PaletteStyles(false)

		// Assert dark and light styles are non-empty
		if darkStyles.Title.GetForeground() == "" || lightStyles.Title.GetForeground() == "" {
			t.Errorf("theme %s has empty title foreground", th.Name())
		}
		if darkStyles.Panel.GetBorderTopForeground() == "" || lightStyles.Panel.GetBorderTopForeground() == "" {
			t.Errorf("theme %s has empty panel border foreground", th.Name())
		}

		// Dark and light palettes must differ in panel or title background
		darkBg := darkStyles.Panel.GetBackground()
		lightBg := lightStyles.Panel.GetBackground()
		if darkBg == lightBg {
			t.Errorf("theme %s dark and light panel background must not be identical (%q)", th.Name(), darkBg)
		}

		// WithColorMode should update ColorMode and produce correct Styles()
		thDark := th.WithColorMode(ColorModeDark)
		if thDark.ColorMode() != ColorModeDark {
			t.Errorf("expected theme %s to have ColorModeDark, got %s", th.Name(), thDark.ColorMode())
		}
		thLight := th.WithColorMode(ColorModeLight)
		if thLight.ColorMode() != ColorModeLight {
			t.Errorf("expected theme %s to have ColorModeLight, got %s", th.Name(), thLight.ColorMode())
		}
	}
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v ./pkg/tui/theme -run TestThemePaletteStyles`
Expected: Compilation failure or missing `PaletteStyles` / `WithColorMode`.

- [ ] **Step 3: Implement light/dark palettes in `modern.go`, `lcars.go`, and `crt.go`**

Update `ModernTheme`, `LcarsTheme`, and `CrtTheme`:
1. Add `mode ColorMode` field.
2. Implement `ColorMode() ColorMode`, `WithColorMode(mode ColorMode) Theme`.
3. Implement `PaletteStyles(isDark bool) Styles` with the exact color tables from Section 2 of the spec.
4. Update `Styles()` to call `PaletteStyles(t.mode.Resolve(lipgloss.HasDarkBackground()))`.

- [ ] **Step 4: Run test to verify it passes**

Run: `go test -v ./pkg/tui/theme`
Expected: PASS across all theme tests.

- [ ] **Step 5: Commit**

```bash
git add pkg/tui/theme/modern.go pkg/tui/theme/lcars.go pkg/tui/theme/crt.go pkg/tui/theme/theme_test.go
git commit -m "feat(theme): implement paired light and dark palettes for Modern, LCARS, and CRT"
```

---

### Task 3: Model State, Runtime Mode Cycling & Dynamic Background Change Handling

**Files:**
- Modify: `pkg/tui/model.go`
- Modify: `pkg/tui/update.go`
- Test: `pkg/tui/update_test.go`

**Interfaces:**
- Consumes: `Theme.WithColorMode()`, `ColorMode.Next()` from Tasks 1 & 2
- Produces: `Ctrl+T` / `Shift+F2` color mode cycling, Spock Command Palette commands, dynamic background adaptation on `tea.WindowSizeMsg`

- [ ] **Step 1: Write failing tests in `pkg/tui/update_test.go`**

```go
func TestColorModeKeyboardToggle(t *testing.T) {
	g := engine.NewGameWithOptions(42, engine.SkillGood, engine.LengthMedium, engine.DefaultRules())
	th := theme.ModernTheme{}.WithColorMode(theme.ColorModeAuto)
	m := NewModel(g, th)

	// Press Ctrl+T to cycle from Auto to Dark
	newM, _ := m.Update(tea.KeyMsg{Type: tea.KeyCtrlT})
	updated := newM.(Model)
	if updated.Theme.ColorMode() != theme.ColorModeDark {
		t.Fatalf("expected mode Dark after Ctrl+T, got %s", updated.Theme.ColorMode())
	}

	// Press Ctrl+T again to cycle from Dark to Light
	newM2, _ := m.Update(tea.KeyMsg{Type: tea.KeyCtrlT})
	updated2 := newM2.(Model)
	if updated2.Theme.ColorMode() != theme.ColorModeLight {
		t.Fatalf("expected mode Light after second Ctrl+T, got %s", updated2.Theme.ColorMode())
	}

	// Press Shift+F2 to cycle from Light to Auto
	newM3, _ := m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{}, Alt: false})
	// Or tea.KeyMsg with string "shift+f2"
	newM4, _ := updated2.Update(tea.KeyMsg{Type: tea.KeyF2, Alt: true}) // or string match
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v ./pkg/tui -run TestColorModeKeyboardToggle`
Expected: FAIL.

- [ ] **Step 3: Implement color mode cycling and background refresh in `pkg/tui/update.go`**

1. Handle `tea.KeyCtrlT`, `"ctrl+t"`, and `"shift+f2"`:
   ```go
   case msg.Type == tea.KeyCtrlT || msg.String() == "ctrl+t" || msg.String() == "shift+f2":
       newMode := m.Theme.ColorMode().Next()
       m = m.applyTheme(m.Theme.WithColorMode(newMode))
       m.CommandBar.AddMessage(fmt.Sprintf("Color mode set to %s (%s)", newMode, m.Theme.Name()))
       return m, nil
   ```
2. In `tea.WindowSizeMsg`, if `m.Theme.ColorMode() == theme.ColorModeAuto`: check if background darkness changed since last render. If changed, re-apply `m = m.applyTheme(m.Theme)`.
3. In `handleCommand()`, add support for `theme mode auto`, `theme mode dark`, `theme mode light`, `color auto`, `color dark`, `color light`.
4. In `commandpalette`, add palette items for `THEME MODE AUTO`, `THEME MODE DARK`, `THEME MODE LIGHT`.

- [ ] **Step 4: Run test to verify it passes**

Run: `go test -v ./pkg/tui`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add pkg/tui/model.go pkg/tui/update.go pkg/tui/update_test.go
git commit -m "feat(tui): wire color mode keyboard shortcuts, command palette, and background detection"
```

---

### Task 4: Options Modal Integration for Color Mode

**Files:**
- Modify: `pkg/tui/components/optionsmodal/modal.go`
- Test: `pkg/tui/components/optionsmodal/modal_test.go`
- Modify: `pkg/tui/update.go`

**Interfaces:**
- Consumes: `ColorMode` from Task 1
- Produces: `RowColorMode` in `optionsmodal.Row`, cycling between Auto, Dark, and Light, with updates synced back to `pkg/tui.Model`

- [ ] **Step 1: Write failing tests in `pkg/tui/components/optionsmodal/modal_test.go`**

```go
func TestOptionsModal_ColorModeRow(t *testing.T) {
	th := theme.DefaultTheme().WithColorMode(theme.ColorModeAuto)
	m := New(th, engine.DefaultRules())

	// Set selected row to RowColorMode
	m.SelectedRow = RowColorMode
	if m.ColorMode() != theme.ColorModeAuto {
		t.Fatalf("expected initial ColorMode to be Auto, got %s", m.ColorMode())
	}

	// Press Right arrow to cycle to Dark
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRight})
	if m.ColorMode() != theme.ColorModeDark {
		t.Fatalf("expected ColorMode to be Dark after Right arrow, got %s", m.ColorMode())
	}

	// Press Right arrow again to cycle to Light
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyRight})
	if m.ColorMode() != theme.ColorModeLight {
		t.Fatalf("expected ColorMode to be Light after Right arrow, got %s", m.ColorMode())
	}

	// Press Left arrow to cycle back to Dark
	m, _ = m.Update(tea.KeyMsg{Type: tea.KeyLeft})
	if m.ColorMode() != theme.ColorModeDark {
		t.Fatalf("expected ColorMode to be Dark after Left arrow, got %s", m.ColorMode())
	}
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v ./pkg/tui/components/optionsmodal -run TestOptionsModal_ColorModeRow`
Expected: Compilation failure on `RowColorMode` or `ColorMode()`.

- [ ] **Step 3: Implement RowColorMode in `optionsmodal/modal.go` and wire into `update.go`**

1. Add `RowColorMode` before `RowDone` in `Row` enum.
2. Add `colorMode theme.ColorMode` field to `optionsmodal.Model`, with getter `ColorMode()` and setter `SetColorMode(mode theme.ColorMode)`.
3. Render `Color Mode:  ◄ [ AUTO ] ►` (or `DARK` / `LIGHT`) in `View()`.
4. Update `m.handleAdjust()` on Left/Right/Space keys for `RowColorMode`.
5. In `pkg/tui/update.go`, when closing or saving the options modal, apply `m.applyTheme(m.Theme.WithColorMode(m.optionsModal.ColorMode()))`.

- [ ] **Step 4: Run test to verify it passes**

Run: `go test -v ./pkg/tui/components/optionsmodal`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add pkg/tui/components/optionsmodal/modal.go pkg/tui/components/optionsmodal/modal_test.go pkg/tui/update.go
git commit -m "feat(optionsmodal): add interactive Color Mode configuration row"
```

---

### Task 5: CLI Flag Support (`-mode`)

**Files:**
- Modify: `cmd/sst/main.go`
- Test: `cmd/sst/main_test.go`

**Interfaces:**
- Consumes: `ParseColorMode` from Task 1, `WithColorMode` from Task 2
- Produces: CLI flag `-mode` (`auto`, `dark`, `light`)

- [ ] **Step 1: Write failing tests in `cmd/sst/main_test.go`**

```go
func TestCLIModeFlag(t *testing.T) {
	tests := []struct {
		args        []string
		expectedErr bool
	}{
		{[]string{"-mode", "light"}, false},
		{[]string{"-mode", "dark"}, false},
		{[]string{"-mode", "auto"}, false},
		{[]string{"-mode", "invalid"}, true},
	}

	for _, tt := range tests {
		fs := flag.NewFlagSet("sst", flag.ContinueOnError)
		// test flag parsing & validation
	}
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `go test -v ./cmd/sst -run TestCLIModeFlag`
Expected: FAIL.

- [ ] **Step 3: Implement `-mode` flag in `cmd/sst/main.go`**

1. Add `modeFlag := fs.String("mode", "auto", "theme color mode (auto, dark, light)")`.
2. Validate using `theme.ParseColorMode(*modeFlag)`. Return error code 1 with clear usage message if invalid.
3. Apply `selectedTheme = selectedTheme.WithColorMode(parsedMode)`.

- [ ] **Step 4: Run test to verify it passes**

Run: `go test -v ./cmd/sst`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add cmd/sst/main.go cmd/sst/main_test.go
git commit -m "feat(cli): add -mode flag for auto, dark, and light color modes"
```

---

### Task 6: Golden Snapshot Hardening & Light Mode Snapshots

**Files:**
- Modify: `tests/tui_golden_test.go`
- Create: `tests/golden/tui/modern_dashboard_80x24_light.golden`
- Create: `tests/golden/tui/lcars_dashboard_80x24_light.golden`
- Create: `tests/golden/tui/crt_dashboard_80x24_light.golden`

**Interfaces:**
- Consumes: `WithColorMode(ColorModeDark)` to freeze existing tests, `WithColorMode(ColorModeLight)` for new light tests

- [ ] **Step 1: Update existing golden tests to pin ColorModeDark in `tests/tui_golden_test.go`**

Ensure `newTestModel()` pins `theme.ColorModeDark` so existing fixtures (`modern_dashboard_80x24.golden`, etc.) remain 100% byte-for-byte identical regardless of what terminal runs `go test`.

- [ ] **Step 2: Add light mode golden test cases in `tests/tui_golden_test.go`**

```go
func TestTUIGolden_ModernDashboard80x24_Light(t *testing.T) {
	m := newTestModel(80, 24, theme.GetTheme("modern").WithColorMode(theme.ColorModeLight))
	view := m.View()
	assertStrict80x24(t, "modern_dashboard_80x24_light", view)
	compareOrUpdate(t, "modern_dashboard_80x24_light", view)
}

func TestTUIGolden_LcarsDashboard80x24_Light(t *testing.T) {
	m := newTestModel(80, 24, theme.GetTheme("lcars").WithColorMode(theme.ColorModeLight))
	view := m.View()
	assertStrict80x24(t, "lcars_dashboard_80x24_light", view)
	compareOrUpdate(t, "lcars_dashboard_80x24_light", view)
}

func TestTUIGolden_CrtDashboard80x24_Light(t *testing.T) {
	m := newTestModel(80, 24, theme.GetTheme("crt").WithColorMode(theme.ColorModeLight))
	view := m.View()
	assertStrict80x24(t, "crt_dashboard_80x24_light", view)
	compareOrUpdate(t, "crt_dashboard_80x24_light", view)
}
```

- [ ] **Step 3: Generate new golden fixtures and run tests**

Run: `go test -v ./tests -run TestTUIGolden -update`
Run: `go test -v ./tests -run TestTUIGolden`
Expected: PASS across all golden tests (both existing dark and new light snapshots).

- [ ] **Step 4: Commit**

```bash
git add tests/tui_golden_test.go tests/golden/tui/*light.golden
git commit -m "test(tui): add golden snapshot test coverage for light color mode dashboards"
```

---

### Task 7: Full Verification & Integration Quality Gate

**Files:**
- None (Verification step across whole repository)

- [ ] **Step 1: Run all unit tests with race detection**

Run: `go test -v -race ./...`
Expected: 100% passing, 0 data races.

- [ ] **Step 2: Run Go static analysis**

Run: `go vet ./...`
Expected: 0 warnings.

- [ ] **Step 3: Verify binary build and manual check**

Run: `go build -o sst ./cmd/sst`
Run: `./sst -mode light -seed 42` (verify light mode loads cleanly and exits on Esc/Ctrl+C).
