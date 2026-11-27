//! Patrol tour campaign (port of `pkg/engine/campaign.go`).
//!
//! `TourState.current_game_state` is a [`GameRef`] so the tour and the
//! TUI/session share the very same game, like Go's `*GameState`. The tour
//! itself is usually shared too ([`TourRef`]).

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use crate::actions::EngineError;
use crate::json::GoVec;
use crate::refits::{apply_refits, RefitID};
use crate::state::{
    difficulty_normal, game_ref, ptr, sync_quadrant_enemies, EnemyVessel, GameRef, GameState,
};
use crate::types::*;

/// Go `TourID`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct TourID(pub String);
crate::go_json_string_newtype!(TourID, "engine.TourID");

/// Go `SectorObjectiveType`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct SectorObjectiveType(pub String);
crate::go_json_string_newtype!(SectorObjectiveType, "engine.SectorObjectiveType");

impl SectorObjectiveType {
    pub fn border_patrol() -> Self {
        Self("border_patrol".into())
    }
    pub fn deep_surveillance() -> Self {
        Self("deep_surveillance".into())
    }
    pub fn convoy_escort() -> Self {
        Self("convoy_escort".into())
    }
    pub fn starbase_siege() -> Self {
        Self("starbase_siege".into())
    }
}

/// Go `TourSectorConfig`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TourSectorConfig {
    pub index: i32,
    pub name: String,
    pub objective: SectorObjectiveType,
    pub description: String,
    pub initial_days: f64,
    pub hostile_count: i32,
    pub starbase_count: i32,
    pub anomalies: bool,
    pub bonus_bounty: i32,
}

crate::go_json_struct!(TourSectorConfig, "TourSectorConfig", "engine.TourSectorConfig" {
    index => "index",
    name => "name",
    objective => "objective",
    description => "description",
    initial_days => "initial_days",
    hostile_count => "hostile_count",
    starbase_count => "starbase_count",
    anomalies => "anomalies",
    bonus_bounty => "bonus_bounty",
});

/// Go `TourState`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TourState {
    pub id: TourID,
    pub active: bool,
    pub current_sector_index: i32,
    pub sectors: GoVec<TourSectorConfig>,
    pub requisition_points: i32,
    pub installed_refits: Option<BTreeMap<RefitID, i32>>,
    pub total_tour_score: i32,
    pub sectors_completed: i32,
    pub hostiles_destroyed: i32,
    pub current_game_state: Option<GameRef>,
    pub in_drydock: bool,
    pub completed: bool,
    pub failed: bool,
    pub failure_reason: GameOverReason,
    pub medals: GoVec<String>,
    pub seed: i64,
}

crate::go_json_struct!(TourState, "TourState", "engine.TourState" {
    id => "id",
    active => "active",
    current_sector_index => "current_sector_index",
    sectors => "sectors",
    requisition_points => "requisition_points",
    installed_refits => "installed_refits",
    total_tour_score => "total_tour_score",
    sectors_completed => "sectors_completed",
    hostiles_destroyed => "hostiles_destroyed",
    current_game_state => "current_game_state" omitempty,
    in_drydock => "in_drydock",
    completed => "completed",
    failed => "failed",
    failure_reason => "failure_reason" omitempty,
    medals => "medals",
    seed => "seed",
});

/// Shared tour handle (Go `*TourState`).
pub type TourRef = Rc<RefCell<TourState>>;

#[allow(clippy::too_many_arguments)]
fn sector(
    index: i32,
    name: &str,
    objective: SectorObjectiveType,
    description: &str,
    initial_days: f64,
    hostile_count: i32,
    starbase_count: i32,
    anomalies: bool,
    bonus_bounty: i32,
) -> TourSectorConfig {
    TourSectorConfig {
        index,
        name: name.into(),
        objective,
        description: description.into(),
        initial_days,
        hostile_count,
        starbase_count,
        anomalies,
        bonus_bounty,
    }
}

/// Go `defaultSectors`.
pub fn default_sectors() -> GoVec<TourSectorConfig> {
    GoVec::from_vec(vec![
        sector(
            1,
            "Neutral Zone Patrol",
            SectorObjectiveType::border_patrol(),
            "Eliminate cloaked Romulan incursions infiltrating Federation border space.",
            30.0,
            2,
            1,
            false,
            250,
        ),
        sector(
            2,
            "Border Outpost Defense",
            SectorObjectiveType::deep_surveillance(),
            "Neutralize Tholian web spinners constructing energy barriers around border outposts.",
            32.0,
            2,
            1,
            true,
            350,
        ),
        sector(
            3,
            "Commander Decapitation",
            SectorObjectiveType::convoy_escort(),
            "Eliminate the Klingon commander flagship and its escort wolf-pack.",
            35.0,
            3,
            2,
            false,
            500,
        ),
        sector(
            4,
            "Invasion Fleet Interception",
            SectorObjectiveType::starbase_siege(),
            "Defend against a multi-faction armada of Klingon, Romulan, and Tholian warships.",
            40.0,
            4,
            1,
            true,
            750,
        ),
    ])
}

impl TourState {
    /// Go `NewTour(seed)` with the `time.Now().UnixNano()` value for a zero
    /// seed supplied by the caller (see [`crate::persist::new_tour`]).
    pub fn new_at(seed: i64, now_unix_nano: i64) -> TourState {
        let seed = if seed == 0 { now_unix_nano } else { seed };
        TourState {
            id: TourID(format!("tour-{}", seed)),
            active: true,
            current_sector_index: 0,
            sectors: default_sectors(),
            requisition_points: 0,
            installed_refits: Some(BTreeMap::new()),
            total_tour_score: 0,
            sectors_completed: 0,
            in_drydock: false,
            completed: false,
            failed: false,
            medals: GoVec::empty(),
            seed,
            ..Default::default()
        }
    }

    /// Go `TourState.CurrentSector` (a copy; mutate via `sectors` directly).
    pub fn current_sector(&self) -> Option<&TourSectorConfig> {
        if self.current_sector_index < 0 || self.current_sector_index as usize >= self.sectors.len()
        {
            return None;
        }
        Some(&self.sectors[self.current_sector_index as usize])
    }

    /// Go `TourState.StartCurrentSector(indices...)`.
    pub fn start_current_sector(&mut self, index: Option<i32>) -> Result<GameRef, EngineError> {
        if let Some(i) = index {
            self.current_sector_index = i;
        }
        let Some(sec) = self.current_sector().cloned() else {
            return Err(EngineError(format!(
                "invalid sector index: {}",
                self.current_sector_index
            )));
        };
        let index = self.current_sector_index;
        let sector_seed = self.seed.wrapping_add((index as i64).wrapping_mul(1000));
        let mut g = GameState::new_game_with_seed(sector_seed);
        g.options.difficulty = difficulty_normal();
        g.days_remaining = sec.initial_days;
        g.time_remaining = sec.initial_days;
        g.klingons_remaining = sec.hostile_count;
        g.remaining_klingons = sec.hostile_count;

        apply_refits(&mut g, self.installed_refits.as_ref());

        g.enterprise.energy = g.enterprise.max_energy;
        g.enterprise.torpedoes = g.enterprise.max_torpedoes;
        g.enterprise.sector = [4, 4];
        g.current_quad.grid[4][4] = EntityType::Enterprise;

        let ev = |id, faction, sector: Coord, energy, shields| EnemyVessel {
            id,
            faction,
            sector,
            energy,
            shields,
            ..Default::default()
        };
        let grid = &mut g.current_quad.grid;
        match index {
            0 => {
                g.current_quad.enemies = GoVec::from_vec(vec![
                    ptr(EnemyVessel {
                        max_energy: 1000.0,
                        is_cloaked: true,
                        cloak_turns: 2,
                        ..ev(1, FactionType::Romulan, [2, 3], 1000.0, 400.0)
                    }),
                    ptr(EnemyVessel {
                        max_energy: 1000.0,
                        is_cloaked: true,
                        cloak_turns: 3,
                        ..ev(2, FactionType::Romulan, [6, 7], 1000.0, 400.0)
                    }),
                ]);
                grid[2][3] = EntityType::Empty;
                grid[6][7] = EntityType::Empty;
            }
            1 => {
                g.current_quad.enemies = GoVec::from_vec(vec![
                    ptr(EnemyVessel {
                        special_state: 0,
                        ..ev(11, FactionType::Tholian, [1, 1], 800.0, 300.0)
                    }),
                    ptr(EnemyVessel {
                        special_state: 1,
                        ..ev(12, FactionType::Tholian, [8, 8], 800.0, 300.0)
                    }),
                ]);
                grid[1][1] = EntityType::Tholian;
                grid[8][8] = EntityType::Tholian;
            }
            2 => {
                g.current_quad.enemies = GoVec::from_vec(vec![
                    ptr(EnemyVessel {
                        is_commander: true,
                        ..ev(21, FactionType::Klingon, [4, 7], 1200.0, 600.0)
                    }),
                    ptr(ev(22, FactionType::Klingon, [3, 5], 600.0, 250.0)),
                    ptr(ev(23, FactionType::Klingon, [5, 5], 600.0, 250.0)),
                ]);
                grid[4][7] = EntityType::Commander;
                grid[3][5] = EntityType::Klingon;
                grid[5][5] = EntityType::Klingon;
            }
            3 => {
                g.current_quad.enemies = GoVec::from_vec(vec![
                    ptr(EnemyVessel {
                        is_commander: true,
                        ..ev(31, FactionType::Klingon, [4, 8], 1800.0, 800.0)
                    }),
                    ptr(EnemyVessel {
                        is_cloaked: true,
                        cloak_turns: 2,
                        ..ev(32, FactionType::Romulan, [2, 6], 1200.0, 500.0)
                    }),
                    ptr(EnemyVessel {
                        special_state: 0,
                        ..ev(33, FactionType::Tholian, [1, 2], 800.0, 300.0)
                    }),
                    ptr(EnemyVessel {
                        special_state: 1,
                        ..ev(34, FactionType::Tholian, [8, 2], 800.0, 300.0)
                    }),
                ]);
                grid[4][8] = EntityType::Commander;
                grid[2][6] = EntityType::Empty;
                grid[1][2] = EntityType::Tholian;
                grid[8][2] = EntityType::Tholian;
            }
            _ => {}
        }
        sync_quadrant_enemies(&mut g.current_quad);

        if !g.current_quad.enemies.is_empty() {
            g.klingons_remaining = g.current_quad.enemies.len() as i32;
            g.remaining_klingons = g.current_quad.enemies.len() as i32;
            g.enterprise.condition = ConditionType::Red;
        }

        let gr = game_ref(g);
        self.current_game_state = Some(gr.clone());
        self.in_drydock = false;
        Ok(gr)
    }

    /// Go `TourState.EvaluateSector`: (cleared, failed, bounty).
    pub fn evaluate_sector(&mut self) -> (bool, bool, i32) {
        let Some(gr) = self.current_game_state.clone() else {
            return (false, false, 0);
        };
        let g = gr.borrow();
        if g.game_over && g.game_over_reason != GameOverReason::Won {
            self.active = false;
            self.failed = true;
            self.failure_reason = g.game_over_reason;
            return (false, true, 0);
        }
        let Some(sec) = self.current_sector() else {
            return (false, false, 0);
        };
        // A won game clears the sector even if the hostile counters were not
        // driven to zero (Go only looked at the counters).
        let won = g.game_won || (g.game_over && g.game_over_reason == GameOverReason::Won);
        if won
            || g.klingons_remaining <= 0
            || g.remaining_klingons <= 0
            || (g.current_quad.enemies.is_empty() && g.current_quad.klingons.is_empty())
        {
            let base = 1000;
            let stardate_bonus = (g.days_remaining * 50.0) as i32;
            let has_damage = g.enterprise.damage.iter().any(|d| *d > 0.0)
                || g.enterprise.devices.iter().any(|d| *d > 0.0);
            let flawless = if has_damage { 0 } else { 250 };
            let bounty = base + stardate_bonus + flawless + sec.bonus_bounty;
            return (true, false, bounty);
        }
        (false, false, 0)
    }

    /// Go `TourState.AdvanceToDrydock`.
    pub fn advance_to_drydock(&mut self, bounty: i32) {
        self.in_drydock = true;
        self.requisition_points += bounty;
        self.sectors_completed += 1;
        if let Some(g) = &self.current_game_state {
            self.total_tour_score += g.borrow().score;
        }
    }

    /// Go `TourState.DisembarkToNextSector`: `Ok(None)` when the tour is complete.
    pub fn disembark_to_next_sector(&mut self) -> Result<Option<GameRef>, EngineError> {
        if !self.in_drydock {
            return Err(EngineError::new(
                "cannot disembark: ship is not docked in drydock",
            ));
        }
        if self.current_sector_index + 1 >= self.sectors.len() as i32 {
            self.completed = true;
            self.active = false;
            self.in_drydock = false;
            return Ok(None);
        }
        self.current_sector_index += 1;
        self.start_current_sector(None).map(Some)
    }
}
