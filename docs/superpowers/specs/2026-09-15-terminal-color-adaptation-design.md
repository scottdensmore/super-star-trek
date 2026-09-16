# Super Star Trek: Terminal Color & System Luminance Adaptation Design Spec

## Overview & Motivation

Super Star Trek's modern Charmbracelet TUI defaults to high-contrast TrueColor styling assuming a dark terminal background (e.g., `#0B0F19` for Starfleet Modern, `#000000` for LCARS, `#050B05` for CRT Phosphor). When executed inside terminals with light backgrounds (such as macOS Terminal in default Light theme, Solarized Light, or GitHub Light), these hardcoded dark panels and bright neon glyphs produce jarring contrast mismatches, unreadable borders, or washed-out text.

This specification introduces automatic terminal background detection and first-class adaptive light/dark color palettes across all three themes (Modern, LCARS, and CRT), paired with user controls to switch between **Auto**, **Force Dark**, and **Force Light** modes.

---

## 1. ColorMode Architecture & Theme Interface (`pkg/tui/theme`)

### 1.1 ColorMode Type

We introduce `ColorMode` in `pkg/tui/theme`:

```go
// ColorMode represents the luminance mode for TUI themes.
type ColorMode string

const (
	// ColorModeAuto dynamically detects terminal background darkness.
	ColorModeAuto ColorMode = "auto"
	// ColorModeDark forces dark mode palettes regardless of terminal query.
	ColorModeDark ColorMode = "dark"
	// ColorModeLight forces light mode palettes regardless of terminal query.
	ColorModeLight ColorMode = "light"
)

// Resolve returns true if the active mode resolves to dark, false for light.
func (m ColorMode) Resolve(terminalHasDarkBg bool) bool {
	switch m {
	case ColorModeDark:
		return true
	case ColorModeLight:
		return false
	case ColorModeAuto:
		return terminalHasDarkBg
	default:
		return terminalHasDarkBg
	}
}

// Next cycles through Auto -> Dark -> Light -> Auto.
func (m ColorMode) Next() ColorMode {
	switch m {
	case ColorModeAuto:
		return ColorModeDark
	case ColorModeDark:
		return ColorModeLight
	case ColorModeLight:
		return ColorModeAuto
	default:
		return ColorModeAuto
	}
}

// ParseColorMode converts a string to a valid ColorMode, defaulting to ColorModeAuto.
func ParseColorMode(val string) ColorMode {
	switch strings.ToLower(strings.TrimSpace(val)) {
	case "dark":
		return ColorModeDark
	case "light":
		return ColorModeLight
	case "auto":
		return ColorModeAuto
	default:
		return ColorModeAuto
	}
}
```

### 1.2 Extended Theme Interface

The `Theme` interface is extended to support color mode inspection and cloning while preserving backward-compatible `Styles()` behavior:

```go
type Theme interface {
	Name() string
	Styles() Styles                      // Returns styles for active ColorMode
	PaletteStyles(isDark bool) Styles    // Returns concrete light or dark styles
	ColorMode() ColorMode                // Returns the configured ColorMode
	WithColorMode(mode ColorMode) Theme  // Returns a new Theme instance configured with mode
	Next() Theme                         // Cycles aesthetic (Modern -> LCARS -> CRT)
}
```

Each concrete theme struct (`ModernTheme`, `LcarsTheme`, `CrtTheme`) stores `mode ColorMode` (defaulting to `ColorModeAuto`). When `Styles()` is called, it resolves `isDark := t.mode.Resolve(lipgloss.HasDarkBackground())` and delegates to `t.PaletteStyles(isDark)`.

---

## 2. Paired Theme Palettes

Each theme provides distinct, hand-crafted palettes for both dark and light terminal backgrounds:

### 2.1 Starfleet Modern

| Element | Dark Palette (Existing) | Light Palette (New) |
|---|---|---|
| **Panel / Title Background** | `#0B0F19` (Dark space navy) | `#F8FAFC` (Slate-50) |
| **Panel Border** | `#1E293B` (Slate-800) | `#CBD5E1` (Slate-300) |
| **Title / Panel Title** | `#00E5FF` (Electric cyan) | `#0284C7` (Sky blue 600) |
| **Grid Borders / Separators** | `#1E293B` | `#CBD5E1` |
| **Enterprise `<E>`** | `#00E5FF` (Bold Cyan) | `#0284C7` (Bold Sky Blue) |
| **Klingon `+K+`** | `#FF3344` (Alert Red) | `#DC2626` (Crimson Red 600) |
| **Starbase `>B<`** | `#FFD700` (Gold) | `#D97706` (Deep Amber 600) |
| **Star ` * `** | `#FFFFFF` (White) | `#475569` (Slate-600) |
| **Planet `@`** | `#00E676` (Emerald Green) | `#16A34A` (Forest Green 600) |
| **Black Hole ` # `** | `#A855F7` (Purple) | `#7C3AED` (Deep Violet 600) |
| **Empty Sector ` · `** | `#334155` (Slate-700) | `#94A3B8` (Slate-400) |
| **Condition Green** | `#00E676` | `#16A34A` |
| **Condition Yellow** | `#FFD700` | `#D97706` |
| **Condition Red** | `#FF3344` | `#DC2626` |
| **Progress Bar Filled** | `#00E5FF` | `#0284C7` |
| **Progress Bar Empty** | `#1E293B` | `#E2E8F0` |
| **Gauge Label / Value** | `#E2E8F0` / `#00E5FF` | `#334155` / `#0F172A` |
| **Log Text / Prompt** | `#E2E8F0` / `#00E5FF` | `#0F172A` / `#0284C7` |

### 2.2 LCARS Theme

| Element | Dark Palette (Existing) | Light Palette (New) |
|---|---|---|
| **Panel Background** | `#000000` (Black) | `#FEF9EF` (Parchment Cream) |
| **Title Background / Text** | `#FF9900` / `#000000` | `#C2410C` / `#FEF9EF` |
| **Panel Border** | `#FF9900` (Amber) | `#C2410C` (Dark LCARS Amber) |
| **Panel Title / Headers** | `#FFCC66` (Peach) | `#9A3412` (Deep Peach/Brown) |
| **Enterprise `<E>`** | `#FFCC66` (Bold Peach) | `#C2410C` (Bold Amber) |
| **Klingon `+K+`** | `#FF3300` (Red-Orange) | `#B91C1C` (Ruby Red) |
| **Starbase `>B<`** | `#3399CC` (LCARS Teal) | `#0E7490` (Deep Teal) |
| **Star ` * `** | `#FFFFFF` (White) | `#57534E` (Stone-600) |
| **Progress Bar Filled / Empty** | `#FF9900` / `#333333` | `#C2410C` / `#E7E5E4` |
| **Telemetry Text / Gauges** | `#FFCC66` / `#3399CC` | `#1C1917` / `#0E7490` |

### 2.3 CRT Phosphor Theme

| Element | Dark Palette (Existing) | Light Palette (New) |
|---|---|---|
| **Panel Background** | `#050B05` (Dark Green Black) | `#F0FDF4` (Pale Mint Paper) |
| **Title Background / Text** | `#33FF33` / `#050B05` | `#14532D` / `#F0FDF4` |
| **Panel Border** | `#33FF33` (Phosphor Green) | `#166534` (Deep Green) |
| **Enterprise `<E>`** | `#33FF33` (Reversed) | `#14532D` (Reversed) |
| **Klingon `+K+` / Starbase `>B<`** | `#33FF33` | `#14532D` |
| **Star ` * `** | `#22CC22` | `#15803D` |
| **Telemetry Text / Gauges** | `#33FF33` | `#14532D` |
| **Empty Sector ` · `** | `#116611` | `#86EFAC` |

---

## 3. Runtime Controls & TUI Integration

### 3.1 Model State
`pkg/tui.Model` retains `Theme theme.Theme`. `applyTheme(th theme.Theme)` updates the model and propagates the theme to all sub-components (`sectorgrid`, `statuspanel`, `commandbar`, `targetlock`, `commandpalette`, `savebrowser`, `galacticchart`, `damageschematic`, `halloffame`, `manual`, and `optionsmodal`).

### 3.2 Keybindings
- **`F2`**: Cycles aesthetic themes (`Modern` $\to$ `LCARS` $\to$ `CRT`), preserving the current `ColorMode`.
- **`Shift+F2` / `Ctrl+T`**: Cycles color mode (`Auto` $\to$ `Dark` $\to$ `Light` $\to$ `Auto`) on the active theme:
  ```go
  case msg.Type == tea.KeyCtrlT || msg.String() == "ctrl+t" || msg.String() == "shift+f2":
      newMode := m.Theme.ColorMode().Next()
      m = m.applyTheme(m.Theme.WithColorMode(newMode))
      m.CommandBar.AddMessage(fmt.Sprintf("Color mode set to %s (%s)", newMode, m.Theme.Name()))
      return m, nil
  ```

### 3.3 Dynamic Background Change Handling
When `m.Theme.ColorMode() == ColorModeAuto`, in `Update()` upon receiving `tea.WindowSizeMsg`:
- Query `hasDark := lipgloss.HasDarkBackground()`.
- Compare against cached background state. If the terminal background changed (e.g. system mode switch), trigger `m = m.applyTheme(m.Theme)` so all styles instantly adapt.

---

## 4. Options Modal & Command Palette Integration

### 4.1 Options Modal (`pkg/tui/components/optionsmodal`)
- Add `RowColorMode` to `Row` enum.
- Display row: `Color Mode:  ◄ [ AUTO ] ►` (or `DARK` / `LIGHT`).
- Left/Right arrows and Spacebar cycle the mode.
- Selecting and saving changes updates the active model theme mode.

### 4.2 Spock Command Palette (`pkg/tui/components/commandpalette`)
Add commands:
- `THEME MODE AUTO` — Set theme adaptation to automatic terminal background detection.
- `THEME MODE DARK` — Force dark theme palette.
- `THEME MODE LIGHT` — Force light theme palette.

---

## 5. CLI Flags (`cmd/sst/main.go`)

Add the `-mode` flag:
```go
modeFlag := fs.String("mode", "auto", "theme color mode (auto, dark, light)")
```

Validation ensures values are constrained to `auto`, `dark`, or `light`. If invalid, return descriptive error message.

---

## 6. Verification & Testing Strategy

### 6.1 Unit Tests
- **`pkg/tui/theme/theme_test.go`**:
  - Test `ColorMode.Resolve(true)` and `ColorMode.Resolve(false)` across all 3 modes.
  - Test `ColorMode.Next()` cycle.
  - Test `ParseColorMode()` case-insensitivity and defaults.
  - Test `PaletteStyles(true)` vs `PaletteStyles(false)` for `ModernTheme`, `LcarsTheme`, and `CrtTheme`.
  - Test `WithColorMode()` preserves aesthetic while updating mode.
- **`pkg/tui/components/optionsmodal/modal_test.go`**:
  - Test `RowColorMode` cycling.
- **`cmd/sst/main_test.go`**:
  - Test `-mode` flag parsing (`light`, `dark`, `auto`, and error handling for invalid modes).

### 6.2 Golden Snapshot Tests (`tests/tui_golden_test.go`)
- **Preserve Existing Snapshots**: Existing golden snapshot tests explicitly use `WithColorMode(theme.ColorModeDark)` so baselines remain 100% deterministic and unaffected by the host runner's terminal background.
- **Add Light Mode Snapshots**:
  - `TestTUIGolden_ModernDashboard80x24_Light` (`modern_dashboard_80x24_light.golden`)
  - `TestTUIGolden_LcarsDashboard80x24_Light` (`lcars_dashboard_80x24_light.golden`)
  - `TestTUIGolden_CrtDashboard80x24_Light` (`crt_dashboard_80x24_light.golden`)
  All snapshots assert exact 24-line height budget and width $\le 80$.

### 6.3 Concurrency & Linting
- `go test -v -race ./...` (0 race conditions).
- `go vet ./...`.
