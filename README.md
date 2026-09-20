# Super Star Trek

Super Star Trek is an authentic, modernized reimagining of the classic 1978 turn-based space strategy simulation originally written in BASIC and popularized on early mainframe and microcomputer systems.

As Captain of the USS *Enterprise* (NCC-1701), your mission is to explore Federation quadrants, manage your ship's finite energy reserves, eliminate invading Klingon and Romulan battlecruisers, and rendezvous with Starbases for repairs and replenishment.

This repository features both a **state-of-the-art terminal dashboard** built with Go and the [Charmbracelet](https://charm.sh) stack (`bubbletea`, `lipgloss`), and the **classic 1978 teletype/C edition**.

---

## Features at a Glance

- **Rich 80×24 TUI Dashboard:** Live telemetry gauges, sector grid, surrounding quadrant radar, and non-blocking input handling.
- **Dynamic Retro & Modern Themes:**
  - **Modern:** Contemporary high-contrast terminal palette.
  - **LCARS:** Authentic 24th-century Federation visual interface styling.
  - **CRT:** Vintage amber monochrome phosphorous monitor aesthetic.
  - **Luminance Adaptation:** Automatically senses terminal dark vs. light backgrounds or switches on demand (`Auto`, `Dark`, `Light`).
- **Real-Time Visual FX & Animation Engine:**
  - Torpedo flight tracking (` · ` $\to$ ` o ` $\to$ ` O `) and 3-frame explosion shockwaves (` * ` $\to$ `***` $\to$ `#*#`).
  - Directional phaser raycasts (`---`, `\\\`, `///`, `|||`) and multi-target beams with shield impact brackets.
  - Periodic Condition Red visual klaxon beacon.
  - Instant, non-blocking key-skip (keypresses immediately interrupt animations without input lag).
- **Interactive Modals:**
  - `F1` / `?`: **Starfleet Technical Manual & Codex** — 8 comprehensive curriculum chapters covering systems, flight math, tactical combat formulas, and cheatsheets.
  - `F2`: **Theme & Mode Switcher** — Live theme switching and color mode cycling.
  - `F3`: **Hall of Fame & Scoring Engine** — Authentic Starfleet rank commissions, performance ratings, and persistent leaderboards.
  - `F4`: **Damage Control Schematic** — ASCII starship cutaway highlighting subsystem operational states and repair countdowns.
  - `F5`: **Galactic Star Chart** — 8×8 quadrant map with exploration fog-of-war, starbase surveillance networks, and one-click impulse/warp targeting.
  - `o`: **Tactical Options** — Animation speed toggle (`Off`, `Fast`, `Normal`, `Cinematic`), themes, and gameplay settings.
- **Mouse & Keyboard Controls:** Full mouse click-to-select, targeting reticles, drag motion, and intuitive keyboard navigation.
- **Gameplay Depth:** Configurable difficulty profiles (`casual`, `normal`, `hardcore`, `nightmare`), two-tier sensor damage curves, and optional tactical Klingon cloaking.
- **Cross-Platform:** Native support across Linux, macOS, and Windows.

---

## Installation

### Homebrew (macOS & Linux)
```bash
brew install scottdensmore/tap/super-star-trek
```

### Debian / Ubuntu (`.deb`)
Download the `.deb` package from the [Releases page](https://github.com/scottdensmore/super-star-trek/releases) and install:
```bash
sudo dpkg -i super-star-trek_2.0.0_linux_amd64.deb
```

### Fedora / RHEL (`.rpm`)
Download the `.rpm` package from the [Releases page](https://github.com/scottdensmore/super-star-trek/releases) and install:
```bash
sudo rpm -i super-star-trek_2.0.0_linux_amd64.rpm
```

### Direct Binary Download
Pre-compiled standalone binaries for Linux (`amd64`, `arm64`), macOS (Universal Apple Silicon & Intel), and Windows (`amd64`) are available on the [Releases page](https://github.com/scottdensmore/super-star-trek/releases).

### Go Toolchain
```bash
go install github.com/scottdensmore/super-star-trek/cmd/sst@latest
```

### WebAssembly Browser Edition
Play instantly in your web browser with retro sound synthesis:
- **Live Online:** [https://scottdensmore.github.io/super-star-trek/](https://scottdensmore.github.io/super-star-trek/)
- **Self-Hosted:** Download `super-star-trek_2.0.0_wasm.tar.gz` from Releases and serve with any static web server (`python3 -m http.server 8080`).

---

## Quick Start (Go Edition)

### Requirements
- [Go 1.26+](https://golang.org)

### Run Directly
```bash
# Launch modern Charm TUI dashboard
go run ./cmd/sst

# Launch in classic 1978 teletype mode
go run ./cmd/sst --classic
```

### Command-Line Options
```bash
sst [flags]

Flags:
  --classic             Run in classic teletype terminal mode
  --theme <name>        Initial theme: modern (default), lcars, crt
  --mode <mode>         Theme color mode: auto (default), dark, light
  --difficulty <level>  Difficulty profile: casual, normal (default), hardcore, nightmare
  --seed <int>          PRNG seed for reproducible galaxies (default: random)
  --surveillance <mode> Starbase surveillance: full, classic, local, blackout
  --sensor-degradation  Enable two-tier sensor damage degradation (default: true)
  --klingon-cloak       Enable Klingon commander tactical cloaking (default: false)
  --repair-multiplier   Subsystem repair duration multiplier (default: 1.0)
```

---

## Controls & Keyboard Shortcuts

### Global Hotkeys
| Key | Function |
|---|---|
| `F1` or `?` | Toggle Starfleet Technical Manual & Codex |
| `F2` | Cycle Theme (`Modern` $\to$ `LCARS` $\to$ `CRT`) |
| `Shift + F2` | Cycle Color Mode (`Auto` $\to$ `Dark` $\to$ `Light`) |
| `F3` | Toggle Starfleet Hall of Fame & Scoring |
| `F4` | Toggle Damage Control Schematic |
| `F5` | Toggle Galactic Star Chart |
| `o` | Open Tactical Options Modal |
| `Tab` / `Shift+Tab` | Toggle modal focus / cycle active elements |
| `Esc` / `q` | Dismiss active modal or clear selection |
| `Ctrl+C` | Emergency exit |

### Core In-Game Commands
Type commands directly into the prompt bar at the bottom:
| Command | Short | Action |
|---|---|---|
| `nav <course> <warp>` | `nav` | Engage warp engines on course (1.0–9.0) for distance |
| `srs` | `sr` | Short-range sensor scan of current quadrant |
| `lrs` | `lr` | Long-range sensor scan of adjacent quadrants |
| `pha <energy>` | `pha` | Fire phaser banks allocated by energy units |
| `tor <course>` | `tor` | Launch photon torpedo on heading |
| `she <units>` | `she` | Transfer energy between main reserves and deflector shields |
| `dam` | `dam` | Damage report & repair ETA |
| `chart` | `cha` | Galactic star chart readout |
| `com <type>` | `com` | Starship computer operations (trajectory, status, score) |
| `anim <speed>` | `anim` | Set animation speed (`off`, `fast`, `normal`, `cinematic`) |
| `theme <name>` | `theme` | Set visual theme (`modern`, `lcars`, `crt`) |
| `save <name>` | `save` | Save game state to persistent storage |
| `load <name>` | `load` | Load saved game |

---

## Classic C Edition

The original C implementation and classic curses full-screen interface are preserved in the `c/` directory.

### Requirements
- C17 compiler (`gcc` or `clang`)
- CMake 3.21+
- `libncurses-dev` (Linux) or ncurses (macOS)

### Building
```bash
cmake --preset debug           # or release
cmake --build --preset debug
./build/debug/c/sst
```

### Running C Tests
```bash
ctest --preset debug
bash c/tests/golden.sh ./build/debug/c/sst
```

### C Full-Screen Mode (`sst -t`)
Run `sst -t` for the curses two-panel display. It requires a terminal of at least 72×24 columns and can be combined with `-f` (`sst -f -t`). Without `-t`, the classic scrolling display is used.

---

## License

Public Domain / MIT. Authentic reproduction based on David H. Ahl's *BASIC Computer Games* (1978).
