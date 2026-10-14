<div align="center">

```text
╔══════════════════════════════════════════════════════════════════════════════╗
║  ★  S T A R F L E E T   T A C T I C A L   C O M M A N D   T E R M I N A L  ★ ║
╠══════════════════════════════════════════════════════════════════════════════╣
║                                                                              ║
║     ____  _   _ ____  _____ ____    ____ _____  _    ____                    ║
║    / ___|| | | |  _ \| ____|  _ \  / ___|_   _|/ \  |  _ \                   ║
║    \___ \| | | | |_) |  _| | |_) | \___ \ | | / _ \ | |_) |                  ║
║     ___) | |_| |  __/| |___|  _ <   ___) || |/ ___ \|  _ <                   ║
║    |____/ \___/|_|   |_____|_| \_\ |____/ |_/_/   \_\_| \_\                  ║
║                       _____ ____  _____ _  __                                ║
║                      |_   _|  _ \| ____| |/ /                                ║
║                        | | | |_) |  _| | ' /                                 ║
║                        | | |  _ <| |___| . \                                 ║
║                        |_| |_| \_\_____|_|\_\                                ║
║                                                                              ║
║                      ______________________                                  ║
║                     /=======/   ____---==='      .  *        .               ║
║                    '-----==='  /    \==================================.     ║
║               *               | (==) |   U.S.S. ENTERPRISE  NCC-1701   |     ║
║                         .      \____/=================================='     ║
║                                   \  \        .                *             ║
║                     .              \  \________________                      ║
║                                    |==================/                      ║
║                                                                              ║
╠══════════════════════════════════════════════════════════════════════════════╣
║  [SYS: ONLINE]   [STARDATE: 1978.2026]   [GRID: 80x24]   [STATUS: COND RED]  ║
╚══════════════════════════════════════════════════════════════════════════════╝
```

### **1978 Mainframe Strategy. Rebuilt for the Modern Terminal.**

[![Go 1.26+](https://img.shields.io/badge/Go-1.26+-00ADD8?style=flat-square&logo=go)](https://golang.org)
[![Charm TUI](https://img.shields.io/badge/TUI-Bubble_Tea_%26_Lipgloss-FF69B4?style=flat-square)](https://charm.sh)
[![C17 / Ncurses](https://img.shields.io/badge/Classic_Edition-C17_%2B_ncurses-A8B9CC?style=flat-square)](./c)
[![WebAssembly](https://img.shields.io/badge/Target-WASM_CRT_Browser-654FF0?style=flat-square)](./web)
[![License: MIT / Public Domain](https://img.shields.io/badge/License-Public_Domain_%2F_MIT-2EA44F?style=flat-square)](#-license--heritage)

**[Quick Start](#-60-second-flight-check-quick-start)** •
**[Bridge Tour](#-step-onto-the-bridge)** •
**[Tactical Features](#-tactical-systems--features)** •
**[Scenarios & Campaign](#-tactical-scenarios--patrol-tours)** •
**[Flight Controls](#-bridge-controls--command-reference)** •
**[Installation](#-installation)**

</div>

---

## `>> INCOMING SUBSPACE TRANSMISSION // PRIORITY ONE`

```text
FROM:     STARFLEET COMMAND — SECTOR 001
TO:       CAPTAIN, U.S.S. ENTERPRISE (NCC-1701)
SUBJECT:  GENERAL ORDER 1 — QUADRANT DEFENSE DIRECTIVE
──────────────────────────────────────────────────────────────────────────────
Klingon battlecruisers, cloaked Romulan warbirds, and Tholian web-spinners are
massing across an 8×8 galactic grid. Your mission: chart unknown quadrants,
manage finite dilithium energy reserves, defend Federation starbases, and
eliminate all hostile warships before time runs out.
──────────────────────────────────────────────────────────────────────────────
```

**Super Star Trek** is an authentic, supercharged reimagining of the legendary 1978 BASIC computer game popularized by David H. Ahl's *BASIC Computer Games*.

Whether you want a **state-of-the-art 80×24 interactive terminal dashboard** with real-time ASCII weapon animations and procedural retro audio, a **multi-sector Starfleet career campaign**, or the **pure 1978 teletype printing-terminal experience**, everything ships in a single zero-dependency binary.

---

## `>> STEP ONTO THE BRIDGE`

### 1. Live 80×24 Tactical Command Console
Every metric you need to survive—Short-Range Sector Grid, Dilithium Energy & Shield Gauges, Subsystem Health, Adjacent Quadrant Radar (`±1`), and an Interactive Command Prompt—at a single glance:

```text
 ★ SUPER STAR TREK ★  USS ENTERPRISE NCC-1701              [Theme: MODERN (F2)] 
┌─────────────────────────────────┐╭─────────────────────────────────────────╮  
│  1   2   3   4   5   6   7   8  ││ CONDITION GREEN   LOC: Q[3,4] S[2,6]    │  
│1  .  --- ---  .   .   .   .   . ││ Stardate: 2508.0   Time Remaining: 30.0 │  
│2  .   .   .   .   .  <E>  .   . ││ Energy:   [██████████] 5000/5000        │  
│3  .   .   .   .   .   .   .   . ││ Shields:  [░░░░░░░░░░]    0/2500        │  
│4  .   .   .   .   .   .   .   . ││ Torpedoes: [TORP: 10/10]   🔊 80%       │  
│5  .   .   .   .   .   .   .   . ││ SUBSYSTEM REPAIR STATUS:                │  
│6  .   .  +K+  .   .   .   .   . ││ Warp:      OK  Tubes:             OK    │  
│7  .   .   .   .   .   .   .   . ││ SRS:       OK  Damage Control:    OK    │  
│8  .   .   .   .   .   .   .   . ││ LRS:       OK  Shields:           OK    │  
└─────────────────────────────────┘│ Phasers:   OK  Computer:          OK    │  
                                   │ RADAR (±1) [K-B-S]:  3    4    5        │  
                                   │   2   008  004  006                     │  
                                   │   3   007 <006> 008                     │  
                                   │   4   005  008  004                     │  
                                   ╰─────────────────────────────────────────╯  
╭──────────────────────────────────────────────────────────────────────────────╮
│ > Phaser banks discharged: 412 units hit Klingon at [6,3]                    │
│ > Klingon shields buckled — vessel destroyed!                                │
│ Super Star Trek — Tactical Display Online                                    │
│ Type 'help' for commands, 'quit' to exit, F2 to cycle themes                 │
│ COMMAND> tor 6 3█                                                            │
╰──────────────────────────────────────────────────────────────────────────────╯
```

### 2. Interactive Subsystem Cutaways & Tactical Target Lock
Pop open full-screen overlay modals without losing bridge awareness. Inspect your ship's hull in the **Damage Control Schematic (`D`)**, lock weapons with the **Tactical Target Lock HUD (`T`)**, or plot warp vectors on the **Galactic Star Chart (`C`)**:

```text
┌─DAMAGE CONTROL SCHEMATIC───────────────────────────────────────┐
│ USS ENTERPRISE  NCC-1701                ALERT STATUS: GREEN    │
│                                                                │
│          .---[SRS: OK]---.              [LRS: OK]              │
│         /    [COMP: 2.1]  \             [PHAS: OK]             │
│        |   (=) Saucer      |===[TUB: 1.4]===.                  │
│         \     Bridge      /                 |                  │
│          '---[SHL: OK]---'                  |                  │
│                   \                         |                  │
│                    \===[DAM: OK]======[WARP: OK]               │
│                                                                │
│ SUBSYSTEM          STATUS    IN-FLIGHT  DOCKED  EFFECT         │
│ Photon Tubes       DAMAGED     1.4 SD    0.3 SD  Tubes Locked  │
│ Library Computer   OFFLINE     2.1 SD    0.5 SD  No Chart/Nav  │
│                                                                │
│ Dock at starbase for accelerated 4x damage repair.             │
│ [ESC / ENTER / Q / D] Dismiss Damage Schematic                 │
└────────────────────────────────────────────────────────────────┘
```

---

## `>> TACTICAL SYSTEMS & FEATURES`

```text
┌──────────────────────────────────────┬──────────────────────────────────────┐
│  [01] REAL-TIME ASCII FX ENGINE      │  [02] RETRO & LCARS VISUAL THEMES    │
│  • Torpedo flight paths (· → o → O)  │  • MODERN: High-contrast tactical    │
│  • 3-frame blast shockwaves (#*#)    │  • LCARS: 24th-century Starfleet UI  │
│  • Directional phaser raycasts       │  • CRT: Vintage amber phosphor glow  │
│  • Condition Red klaxon pulse        │  • Auto/Dark/Light luminance sensing │
│  • Instant zero-lag key-skip         │  • Switch live anytime with [F2]     │
├──────────────────────────────────────┼──────────────────────────────────────┤
│  [03] PROCEDURAL SYNTH AUDIO         │  [04] MOUSE + KEYBOARD PRECISION     │
│  • Built-in PCM wave synthesizer     │  • Click any sector to inspect/lock  │
│  • Retro phasers, torpedoes & alerts │  • Double-click empty space to warp  │
│  • Terminal bell & visual flash mode │  • Fuzzy Command Palette ([/] / ^P)  │
│  • Live mute [M] & volume controls   │  • Full Vi (HJKL) & arrow navigation │
└──────────────────────────────────────┴──────────────────────────────────────┘
```

### Deep Simulation & Expanded Threat Matrix
* **Multi-Faction Adversaries (`--adversaries` / `-a`):**
  * **Klingon Wolf-Packs:** Coordinated fleet maneuvers, flagship commanders, and optional tactical cloaking (`--klingon-cloak`).
  * **Romulan Warbirds:** Ambush predators that strike from behind a cloaking device and launch high-yield homing **Plasma Torpedoes**.
  * **Tholian Web Spinners:** Fast crystalline vessels that weave impassable **Energy Webs** across sector lanes to trap the *Enterprise*.
* **Volatile Spatial Anomalies (`--anomalies`):** Navigate ion storms, sensor-blinding nebulae, gravitic mines, and radiation hazards.
* **Two-Tier Sensor Degradation:** Partial SRS/LRS damage introduces realistic telemetry noise and signal dropouts before total subsystem failure.
* **Starfleet Hall of Fame (`H`):** Earn official Starfleet rank commissions, detailed post-mission efficiency breakdowns, and persistent high-score leaderboards.
* **Starfleet Technical Manual (`F1` / `?`):** Built-in 8-chapter interactive codex covering flight math, weapon decay formulas, subsystem mechanics, and tactical cheatsheets.

---

## `>> TACTICAL SCENARIOS & PATROL TOURS`

Want more than a standard skirmish? Launch curated Starfleet Academy simulations or embark on a multi-sector career tour:

### Starfleet Career & Patrol Tour (`sst --tour`)
Progress through **4 escalating campaign sectors** (*Neutral Zone Patrol* $\to$ *Border Outpost Defense* $\to$ *Commander Decapitation* $\to$ *Invasion Fleet Interception*). Between sectors, dock at **Starbase 01 Drydock** to spend requisition bounties on **3-tier starship refits**:
* **Dilithium Core Tuning:** Boost max reactor capacity up to `4,500` units.
* **Reinforced Deflectors:** Cut shield absorption drain by up to `45%`.
* **High-Yield Torpedoes:** Increase antimatter warhead damage by up to `+75%`.
* **Auxiliary Torpedo Magazine:** Expand forward ordnance bays up to `22` torpedoes.
* **Subspace Sensor Matrix:** Reveal cloaked silhouettes and auto-scan upon quadrant entry.
* **Automated Repair Bots:** Deploy micro-drones for passive hull repair during warp transit.

### Curated Tactical Scenarios (`sst --scenario <id>`)
| Scenario ID | Simulation Name | Difficulty | Tactical Briefing |
| :--- | :--- | :---: | :--- |
| `kobayashi-maru` | **Kobayashi Maru** | `EXTREME` | *The Unwinnable Simulation.* Rescue a crippled neutronic fuel carrier in the Klingon Neutral Zone against overwhelming waves. |
| `mutara-nebula` | **Mutara Nebula** | `HARD` | *Blind Tactical Duel.* Hunt a rogue Klingon Super-Commander inside an ionized gas cloud with shields disabled and sensors blinded. |
| `starbase-siege` | **Starbase Under Siege** | `CHALLENGE` | *Defend Starbase 12.* Intercept a coordinated Klingon strike fleet bombarding Starbase 12 before its orbital shields collapse. |

---

## `>> 60-SECOND FLIGHT CHECK (QUICK START)`

```bash
# 1. Launch the modern interactive TUI bridge console
go run ./cmd/sst

# 2. Launch with CRT amber phosphor theme, all enemy factions, and anomalies
go run ./cmd/sst --theme crt --adversaries --anomalies

# 3. Embark on the multi-sector Starfleet Patrol Tour campaign
go run ./cmd/sst --tour

# 4. Face the infamous Kobayashi Maru no-win simulation
go run ./cmd/sst --scenario kobayashi-maru

# 5. Experience pure 1978 teletype terminal mode
go run ./cmd/sst --classic
```

---

## `>> INSTALLATION`

### Go Toolchain (macOS / Linux / Windows)
Requires [Go 1.26+](https://golang.org):
```bash
go install github.com/scottdensmore/super-star-trek/cmd/sst@latest
```

### Debian / Ubuntu (`.deb`)
Download the `.deb` package from the [Releases page](https://github.com/scottdensmore/super-star-trek/releases):
```bash
sudo dpkg -i super-star-trek_2.0.0_linux_amd64.deb
```

### Fedora / RHEL (`.rpm`)
Download the `.rpm` package from the [Releases page](https://github.com/scottdensmore/super-star-trek/releases):
```bash
sudo rpm -i super-star-trek_2.0.0_linux_amd64.rpm
```

### Standalone Pre-Compiled Binaries
Zero-dependency binaries for **Linux** (`amd64`, `arm64`), **macOS** (Universal Apple Silicon & Intel), and **Windows** (`amd64`) are available on the [Releases page](https://github.com/scottdensmore/super-star-trek/releases).

### WebAssembly CRT Browser Edition
Play inside a retro CRT browser monitor! Download `super-star-trek_2.0.0_wasm.tar.gz` from the [Releases page](https://github.com/scottdensmore/super-star-trek/releases) (or build locally with `./scripts/build-wasm.sh`) and serve static files:
```bash
python3 -m http.server 8080 --directory web
```

---

## `>> BRIDGE CONTROLS & COMMAND REFERENCE`

### Global Bridge Hotkeys
*(Active when the command prompt is empty, or everywhere for Function/Ctrl keys)*

| Keybinding | Bridge System / Action |
| :--- | :--- |
| `F1` or `?` | **Starfleet Technical Manual & Codex** (8-chapter interactive guide) |
| `F2` | **Cycle Visual Theme** (`MODERN` $\to$ `LCARS` $\to$ `CRT`) |
| `Shift+F2` or `Ctrl+T` | **Cycle Luminance Mode** (`Auto` $\to$ `Dark` $\to$ `Light`) |
| `T` | **Tactical Target Lock HUD** (Auto-compute bearings, hit probability & fire) |
| `C` or `Ctrl+G` | **Interactive Galactic Star Chart** (8×8 map, fog-of-war & one-key warp) |
| `D` or `Ctrl+D` | **Damage Control Schematic** (ASCII starship cutaway & repair ETAs) |
| `H` or `Ctrl+H` | **Starfleet Hall of Fame** (Live score breakdown & persistent leaderboard) |
| `O` | **Tactical Options Modal** (Difficulty, audio, animation speed & rules) |
| `/` or `Ctrl+P` | **Interactive Command Palette** (Search and trigger any ship command) |
| `Ctrl+O` | **Save / Thaw Mission Browser** (Inspect and load saved missions) |
| `M` or `Ctrl+S` | **Toggle Retro Audio Synthesizer** (Mute / unmute procedural sound FX) |
| `Mouse Click` | **Select Sector / Lock Hostile Target** (Double-click empty sector to move) |
| `Esc` / `Ctrl+C` | **Dismiss Modal / Clear Input / Emergency Exit** |

### Command Prompt Lexicon
Type classic or modern commands directly into the `COMMAND>` bar at the bottom of the screen:

| Command | Syntax & Examples | Tactical Action |
| :--- | :--- | :--- |
| **`nav`** | `nav <r> <c>` · `nav q <r> <c> [w]` · `nav <course> <warp>` | Engage impulse/warp drive to sector `[r,c]`, quadrant `[r,c]`, or along a vector. |
| **`tor`** | `tor <r> <c>` · `tor <angle>` · `tor c <1-9>` | Launch a photon torpedo at sector `[r,c]` or along a directional heading. |
| **`pha`** | `pha <energy>` · `pha <energy> <r> <c>` | Discharge phaser banks (auto-split across hostiles or focused on `[r,c]`). |
| **`she`** | `she <amount>` (e.g. `she 500`, `she -200`) | Transfer dilithium energy between main reactor and deflector shields. |
| **`doc`** | `doc` / `dock` | Dock with an adjacent Starbase (`>!<`) for full energy, torpedoes & 4× repair. |
| **`lrscan`** | `lrscan` | Execute a Long-Range Sensor scan of the 3×3 surrounding quadrants. |
| **`srscan`** | `srscan` | Refresh Short-Range Sensor telemetry of the current quadrant. |
| **`target`** | `target` | Open the Tactical Target Lock HUD for computer-assisted weapons firing. |
| **`chart`** | `chart` / `map` | Open the interactive Galactic Star Chart & Warp Planner. |
| **`dam`** | `dam` / `damage` | Open the ASCII Starship Damage Control Schematic. |
| **`scenarios`**| `scenarios` / `scen` | Open the Starfleet Tactical Simulator to launch a challenge scenario. |
| **`tour`** | `tour` / `orders` | Display current Starfleet Patrol Tour sector briefing, bounty & objectives. |
| **`saves`** | `saves` · `thaw <file>` · `freeze <file>` | Open the Save Browser or load/save mission states directly. |
| **`theme`** | `theme <modern\|lcars\|crt>` · `color <auto\|dark\|light>` | Switch visual console aesthetic or color luminance mode. |
| **`anim`** | `anim <off\|fast\|normal\|cinematic>` | Adjust real-time weapon and alert animation speed. |
| **`help`** | `help [nav\|tor\|pha\|she\|doc\|chart\|saves\|opts]` | Open context-sensitive Starfleet Technical Manual chapters. |

### Command-Line Launch Flags
```text
Usage: sst [flags]

  --classic               Run in authentic 1978 teletype plain-text mode
  --theme <name>          Initial visual theme: modern (default), lcars, crt
  --mode <mode>           Color luminance mode: auto (default), dark, light
  --difficulty <level>    Profile: casual, normal (default), hardcore, nightmare, expert, emeritus
  --tour / --campaign     Launch Starfleet Career & Campaign (Patrol Tour) mode
  --scenario, -s <id>     Launch tactical scenario (kobayashi-maru, mutara-nebula, starbase-siege)
  --list-scenarios        Display all registered tactical scenarios and exit
  --adversaries, -a       Enable Romulan and Tholian adversary factions in standard games
  --anomalies             Enable spatial anomalies & environmental hazards
  --klingon-cloak         Enable Klingon commander tactical cloaking
  --surveillance <mode>   Starbase network coverage: full, classic, local, blackout
  --sensor-degradation    Enable two-tier sensor damage degradation curve (default: true)
  --repair-multiplier <f> Subsystem repair duration multiplier (default: 1.0)
  --sound / --no-sound    Enable or disable procedural retro audio FX (default: enabled)
  --volume <0-100>        Set audio synthesizer playback volume
  --audio-mode <mode>     Audio backend: auto, native, bell, off
  --seed <int>            Fixed PRNG seed for reproducible galaxies (0 = random)
  --version, -v           Print version telemetry and exit
```

---

## `>> CLASSIC C17 & NCURSES HERITAGE EDITION`

For purists and retro-computing historians, the original C implementation and two-panel `ncurses` terminal interface are preserved in the [`c/`](./c) directory.

### Requirements
* C17 compiler (`gcc` or `clang`)
* CMake `3.21+`
* `libncurses-dev` (Linux) or system `ncurses` (macOS)

### Build & Run
```bash
cmake --preset debug           # or 'release'
cmake --build --preset debug

# Run classic scrolling teletype C edition
./build/debug/c/sst

# Run classic curses two-panel full-screen mode (requires 72×24+ terminal)
./build/debug/c/sst -t
```

### Run Verification Suite
```bash
ctest --preset debug
bash c/tests/golden.sh ./build/debug/c/sst
```

---

## `>> LICENSE & HERITAGE`

Released under **Public Domain / MIT**.

Inspired by the original 1971–1978 **Star Trek** / **Super Star Trek** mainframe games by Mike Mayfield, Bob Leedom, and David H. Ahl (*BASIC Computer Games*, 1978), and Tom Almy's C port. Re-engineered for modern terminals with Go and [Charmbracelet](https://charm.sh).

<div align="center">

```text
══════════════════════════════════════════════════════════════════════════════
       "Second star to the right, and straight on 'til morning."
                        [ END OF TRANSMISSION ]
══════════════════════════════════════════════════════════════════════════════
```

</div>
