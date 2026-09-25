# Starfleet Career & Campaign Mode (Patrol Tour) Design Specification

**Status:** Approved  
**Author:** Google Antigravity & Scott Densmore  
**Date:** 2026-09-24  
**Target Milestone:** v2.1.0  
**Related Sub-Projects:**  
1. **Sub-Project 1:** Starfleet Career & Campaign Mode (Current)  
2. **Sub-Project 2:** Expanded Adversaries & Faction Tactics (Follow-up)  
3. **Sub-Project 3:** Audio & Presentation Polish (Follow-up)  

---

## 1. Overview & Goals

The **Starfleet Career & Campaign Mode (Patrol Tour)** transforms Super Star Trek from a series of isolated, single-galaxy skirmishes into an overarching multi-sector tour of duty. Players command the USS *Enterprise* through a connected sequence of 4 distinct operational patrol sectors with varying tactical objectives, earning Starfleet Requisition Points to spend at the **Starbase Drydock** for modular starship refits between missions.

### Key Goals
1. **Multi-Sector Tour Progression:** A structured 4-sector tour where victory conditions, enemy compositions, and environmental conditions vary by sector.
2. **Modular Starship Refits:** 6 distinct ship upgrades across 3 upgrade tiers (Dilithium Tuning, Reinforced Deflector Coils, High-Yield Torpedoes, Auxiliary Torpedo Bays, Subspace Sensor Matrices, and Automated Damage Nanites).
3. **Interactive Starbase Drydock Modal:** A rich, authentic Bubbletea modal between sectors featuring an ASCII starship schematic, visual upgrade indicators, and interactive purchasing.
4. **Roguelite Iron Man Stakes:** Permanent loss of the *Enterprise* ends the tour, recording a comprehensive service record, medals, and commission rank in the Starfleet Hall of Fame.
5. **Full Backward Compatibility & Decoupling:** Campaign logic cleanly wraps existing single-game `GameState` instances without disrupting classic play or standalone scenarios.

### Non-Goals
* Multi-branching procedural galaxy travel networks (kept focused to sequential 4-sector tours per YAGNI).
* Complex trade economies or planetary resource harvesting.
* Real-time tactical combat (the core simulation remains faithful turn-based strategy).

---

## 2. Core Architecture & Data Models

### 2.1 Package Organization
* `pkg/engine/campaign.go`: Tour state machine, sector definitions, objective evaluators, and reward calculations.
* `pkg/engine/refits.go`: Refit definitions, tier requirements, costs, and combat/system stat modifiers.
* `pkg/engine/save.go`: Extended serialization supporting combined tour + active sector state.
* `pkg/engine/leaderboard.go`: Extended career scoring, tour records, and medals.
* `pkg/tui/components/drydockmodal/`: Interactive Bubbletea modal for starbase drydock refits.
* `pkg/tui/`: Integration into global hotkeys, command palette, header status, and scenario selection.

### 2.2 Tour State Machine (`pkg/engine/campaign.go`)

```go
package engine

type TourID string

type SectorObjectiveType string

const (
	ObjectiveBorderPatrol     SectorObjectiveType = "border_patrol"     // Neutralize vanguard battlecruisers
	ObjectiveDeepSurveillance SectorObjectiveType = "deep_surveillance" // Chart anomaly sectors & eliminate scouts
	ObjectiveConvoyEscort     SectorObjectiveType = "convoy_escort"     // Protect Federation transport ship
	ObjectiveStarbaseSiege    SectorObjectiveType = "starbase_siege"    // Defend Starbase 01 against waves
)

// TourSectorConfig defines parameters for an individual sector in the tour.
type TourSectorConfig struct {
	Index         int                 `json:"index"`          // 1 to 4
	Name          string              `json:"name"`           // e.g. "Vanguard Expanse"
	Objective     SectorObjectiveType `json:"objective"`
	Description   string              `json:"description"`
	InitialDays   float64             `json:"initial_days"`
	HostileCount  int                 `json:"hostile_count"`
	StarbaseCount int                 `json:"starbase_count"`
	Anomalies     bool                `json:"anomalies"`
	BonusBounty   int                 `json:"bonus_bounty"`
}

// TourState tracks overarching campaign progression across sectors.
type TourState struct {
	ID                 TourID             `json:"id"`
	Active             bool               `json:"active"`
	CurrentSectorIndex int                `json:"current_sector_index"` // 0 to len(Sectors)-1
	Sectors            []TourSectorConfig `json:"sectors"`
	RequisitionPoints  int                `json:"requisition_points"`
	InstalledRefits    map[RefitID]int    `json:"installed_refits"` // RefitID -> Tier (1..3)
	TotalTourScore     int                `json:"total_tour_score"`
	SectorsCompleted   int                `json:"sectors_completed"`
	HostilesDestroyed  int                `json:"hostiles_destroyed"`
	CurrentGameState   *GameState         `json:"current_game_state,omitempty"`
	InDrydock          bool               `json:"in_drydock"`
	Completed          bool               `json:"completed"`
	Failed             bool               `json:"failed"`
	FailureReason      GameOverReason     `json:"failure_reason,omitempty"`
	Medals             []string           `json:"medals"`
	Seed               int64              `json:"seed"`
}
```

---

## 3. Sector Objectives & Campaign Flow

A Tour of Duty consists of 4 distinct operational sectors:

1. **Sector 1: Vanguard Border Incursion (`ObjectiveBorderPatrol`)**
   * *Briefing:* Klingon vanguard forces have infiltrated Sector 12. Eliminate all hostile battlecruisers before they establish an operational beachhead.
   * *Parameters:* 8×8 galaxy, 4 Klingons, 1 Starbase, 30.0 Stardates, standard environmental rules.
   * *Victory:* All Klingons destroyed.
2. **Sector 2: Mutara Deep Surveillance (`ObjectiveDeepSurveillance`)**
   * *Briefing:* Chart unmapped anomaly quadrants in the Mutara rift and eliminate stealth scout vessels.
   * *Parameters:* Active spatial anomalies (plasma storms, asteroid fields), 5 Klingons (with tactical cloaking enabled), 1 Starbase, 32.0 Stardates.
   * *Victory:* 4 anomaly sectors scanned and all hostiles eliminated.
3. **Sector 3: Federation Transport Escort (`ObjectiveConvoyEscort`)**
   * *Briefing:* Escort the medical transport SS *Columbia* through hostile territory to Starbase rendezvous.
   * *Parameters:* Convoy vessel present in starting quadrant, 6 Klingons actively hunting the transport, 2 Starbases, 35.0 Stardates.
   * *Victory:* Transport reaches designated Starbase quadrant safely without being destroyed.
4. **Sector 4: Starbase 01 Final Siege (`ObjectiveStarbaseSiege`)**
   * *Briefing:* Hostile fleet vanguard attempting to obliterate the Federation central command post. Hold the line at all costs.
   * *Parameters:* 8 hostiles, 1 Starbase, heavy enemy aggression, 40.0 Stardates.
   * *Victory:* Starbase remains intact while all attacking squadrons are eliminated.

### Requisition Economy & Payout Formula
Upon completing each sector, Requisition Points are awarded:
$$\text{Requisition} = \text{Base} (1000) + (\text{Hostiles} \times 150) + (\text{Remaining Stardates} \times 50) + \text{Flawless Bonus} (250)$$
* Base clear payout: `1,000` points.
* Hostiles destroyed: `150` pts (Cruiser), `300` pts (Commander), `500` pts (Super Commander).
* Stardate efficiency: `50` pts per unused stardate.
* Flawless hull bonus: `250` pts if no ship subsystems sustained damage by sector conclusion.

---

## 4. Modular Starship Refits Specification

Refits are managed via `pkg/engine/refits.go` and applied directly to the *Enterprise* during sector generation.

### 4.1 Refit Catalog

```go
type RefitID string

const (
	RefitDilithiumCore  RefitID = "dilithium_core"
	RefitDeflectorGrid  RefitID = "deflector_grid"
	RefitTorpedoCasings RefitID = "torpedo_casings"
	RefitTorpedoBays    RefitID = "torpedo_bays"
	RefitSensorMatrix   RefitID = "sensor_matrix"
	RefitDamageNanites  RefitID = "damage_nanites"
)

type RefitDefinition struct {
	ID          RefitID   `json:"id"`
	Name        string    `json:"name"`
	Description string    `json:"description"`
	TierCosts   [3]int    `json:"tier_costs"`
	TierEffects [3]string `json:"tier_effects"`
}
```

| Refit ID | Subsystem Name | Cost (T1 / T2 / T3) | Tactical Formula & Mechanics |
|---|---|---|---|
| `dilithium_core` | **Dilithium Core Tuning** | 500 / 1,000 / 1,750 | `MaxEnergy` increases from `3,000` $\to$ `3,500` $\to$ `4,000` $\to$ `4,500`. Starting sector energy scales proportionally. |
| `deflector_grid` | **Reinforced Deflector Grid** | 450 / 900 / 1,500 | Shield absorption efficiency: Incoming enemy damage to shields is reduced by `15%` / `30%` / `45%`. |
| `torpedo_casings` | **High-Yield Torpedoes** | 400 / 800 / 1,400 | Torpedo kinetic yield scaled by `+25%` / `+50%` / `+75%` (`damage = base * (1 + 0.25*tier)`). |
| `torpedo_bays` | **Auxiliary Torpedo Magazine** | 350 / 700 / 1,200 | `MaxTorpedoes` expands from `10` $\to$ `14` $\to$ `18` $\to$ `22`. Sector starbase docking replenishes up to new maximum. |
| `sensor_matrix` | **Subspace Sensor Matrix** | 300 / 600 / 1,000 | T1: LRS scan range extended (+1 quadrant radius).<br>T2: Cloaked hostile silhouette indicated on sector grid.<br>T3: Automatic free LRS scan upon entering new quadrant. |
| `damage_nanites` | **Automated Repair Bots** | 400 / 850 / 1,450 | Each warp movement passively restores damaged subsystems by `+0.5` / `+1.0` / `+1.5` stardates in addition to normal time passage. |

---

## 5. User Interface: Starbase Drydock Modal

### 5.1 Modal Presentation & Layout (`pkg/tui/components/drydockmodal/`)
When a sector objective is completed, the main dashboard pauses and renders the interactive full-screen **Drydock Modal**:

```
┌───────────────────────── STARBASE 01 DRYDOCK & REFIT FACILITY ─────────────────────────┐
│ Tour: Vanguard Patrol — Sector 1 of 4 Completed                       Requisition: 1,850 │
├─────────────────────────────────────┬────────────────────────────────────────────────────┤
│         STARSHIP SCHEMATIC          │               AVAILABLE SHIP REFITS                │
│                                     │                                                    │
│           /================\        │ > [*] Dilithium Core Tuning       Tier 1 [Upgrade] │
│          /                  \       │       Max Energy: 3,500 (+500)      Cost: 1,000    │
│      ===|     NCC - 1701     |===   │                                                    │
│      #  \                    /  #   │   [ ] Reinforced Deflectors       Tier 0 [Buy]     │
│      #   \==================/   #   │       Shield Absorption +15%        Cost: 450      │
│      #            ||            #   │                                                    │
│      #+-------+   ||   +-------+#   │   [*] High-Yield Torpedoes        Tier 1 [Upgrade] │
│       | WARP  |=======| WARP  |     │       Explosion Damage +25%         Cost: 800      │
│       +-------+       +-------+     │                                                    │
│                                     │   [ ] Auxiliary Torpedo Bay       Tier 0 [Buy]     │
│  Equipped Modules: 2/6              │       Max Torpedoes: 14 (+4)        Cost: 350      │
│  Hull Integrity: 100%               │                                                    │
│  Systems: Nominal                   │   [ ] Automated Repair Bots       Tier 0 [Buy]     │
│                                     │       Warp Auto-Repair +0.5d        Cost: 400      │
├─────────────────────────────────────┴────────────────────────────────────────────────────┤
│ [↑/↓ / j/k]: Navigate   [Enter]: Purchase Upgrade   [Space / D]: Disembark to Next Sector│
└──────────────────────────────────────────────────────────────────────────────────────────┘
```

### 5.2 Keybindings & Controls
* `↑` / `↓`, `k` / `j`: Move selection highlight through the refit catalog.
* `Enter` / Click: Purchase highlighted refit upgrade (verifies requisition balance, deducts points, increments tier, plays audio chime).
* `Space` / `d` / `D`: Disembark to the next sector, loading the new galaxy state with all upgrades active.
* `F1`–`F5`: Standard global shortcuts remain accessible for manual and telemetry review.

### 5.3 Themes & Visual Identity
* **Modern:** Crisp cyan borders (`#00D7D7`), gold header accents, green tier indicators.
* **LCARS:** Federation tan/orange pill blocks (`#FFAA00`, `#CC6699`), lavender labels.
* **CRT:** Glowing monochrome amber (`#FFB000`) with dim inactive indicators.

---

## 6. Persistence & Leaderboards

### 6.1 Save Game Format (`pkg/engine/save.go`)
The standard JSON save format incorporates an optional `tour` block:
```json
{
  "version": 2,
  "game_state": { ... },
  "tour_state": {
    "id": "tour-20260924-1936",
    "active": true,
    "current_sector_index": 1,
    "requisition_points": 1250,
    "installed_refits": {
      "dilithium_core": 1,
      "torpedo_casings": 1
    },
    "sectors_completed": 1,
    "in_drydock": false,
    "total_tour_score": 6800
  }
}
```
* **Iron Man Enforcement:** In accordance with roguelite permadeath, when a tour concludes (either through victory in Sector 4 or destruction of the *Enterprise*), the save file is automatically archived to prevent post-mortem reload exploits.

### 6.2 Hall of Fame Records (`pkg/engine/leaderboard.go`)
* Leaderboard entry type `TourOfDuty` records:
  * Player Call-Sign / Name
  * Final Starfleet Rank (e.g. *Captain*, *Fleet Captain*, *Commodore*, *Rear Admiral*, *Admiral of the Fleet*)
  * Sectors Cleared (e.g. `4/4 COMPLETED`, `2/4 KIA`)
  * Total Refits Installed
  * Medals Awarded (*Starfleet Citation*, *Legion of Honor*, *Klingon Campaign Ribbon*)
  * Total Accumulated Score

---

## 7. Verification & Testing Strategy

### 7.1 Unit Tests
* `pkg/engine/campaign_test.go`:
  * Tour initialization: ensures 4 sectors configured with correct parameters and seeds.
  * State transitions: tests victory detection, stardate calculations, and requisition point awards.
  * Permadeath: verifies ship destruction flags `Failed = true` and calculates tour score.
* `pkg/engine/refits_test.go`:
  * Stat application: validates max energy, max torpedoes, shield absorption, torpedo yield, and sensor radius modifications.
  * Repair nanites: validates passive subsystem countdown reduction on warp engagement.
* `pkg/engine/save_test.go`:
  * Round-trip JSON serialization and deserialization of active tours and drydock states.

### 7.2 Component & TUI Tests
* `pkg/tui/components/drydockmodal/drydock_test.go`:
  * Navigation and key handling (`j`/`k`, `Enter`, `Space`).
  * Balance checks: purchasing with adequate points vs blocked when balance is insufficient.
  * Theme rendering across Modern, LCARS, and CRT presets.

### 7.3 Automated User Journey Test
* `pkg/tui/tour_journey_test.go`:
  * End-to-end headless simulation: starts a tour, completes sector 1, opens drydock, purchases refit, disembarks into sector 2, and verifies state persistence.

---

## 8. Rollout Plan
* Target release: **v2.1.0**.
* Fully verified through Go race detector (`go test -race ./...`), linter (`golangci-lint run ./...`), and snapshot verification before merging.
