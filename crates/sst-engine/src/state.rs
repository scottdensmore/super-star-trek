//! Game state (port of `pkg/engine/state.go`).
//!
//! Pointer semantics: Go's `[]*Klingon`, `[]*EnemyVessel`, `[]*PlasmaTorpedo`
//! and `[]*TholianWebSegment` become `GoVec<Ptr<T>>` where
//! `Ptr<T> = Option<Rc<RefCell<T>>>` (`None` = Go nil element). Mutating an
//! element through one handle is visible through every other handle, exactly
//! like Go. `Klingons` and `Enemies` never share elements: Go's
//! `SyncQuadrantEnemies` rebuilds `Klingons` as fresh copies, so the two lists
//! can drift apart (e.g. `KlingonTurn` drains Klingon energy without touching
//! `Enemies`); the port keeps that behaviour.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use crate::json::{Decoder, Encoder, GoJson, GoVec, Value};
use crate::prng::PRNG;
use crate::refits::RefitID;
use crate::rules::{default_rules_for_profile, DifficultyProfile, GameRules, SurveillanceMode};
use crate::scenarios::ScenarioID;
use crate::types::*;

/// Shared, nullable Go pointer.
pub type Ptr<T> = Option<Rc<RefCell<T>>>;
/// `*Klingon`.
pub type KlingonRef = Rc<RefCell<Klingon>>;
/// `*EnemyVessel`.
pub type EnemyRef = Rc<RefCell<EnemyVessel>>;

/// Wraps a value into a new non-nil shared pointer (Go `&T{...}`).
pub fn ptr<T>(v: T) -> Ptr<T> {
    Some(Rc::new(RefCell::new(v)))
}

/// Go `GalaxySize`.
pub const GALAXY_SIZE: usize = 8;

/// Go `Enterprise` (and its alias `EnterpriseState`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Enterprise {
    pub quad: Coord,
    pub sector: Coord,
    pub energy: f64,
    pub shields: f64,
    pub torpedoes: i32,
    pub condition: ConditionType,
    /// 0 = operational, >0 = turns until repaired.
    pub devices: [f64; NUM_DEVICES],
    /// Subsystem damage indicators.
    pub damage: [f64; NUM_DEVICES],
    pub max_energy: f64,
    pub max_torpedoes: i32,
    pub life_support: f64,
}

/// Go `EnterpriseState = Enterprise`.
pub type EnterpriseState = Enterprise;

crate::go_json_struct!(Enterprise, "Enterprise", "engine.Enterprise" {
    quad => "Quad",
    sector => "Sector",
    energy => "Energy",
    shields => "Shields",
    torpedoes => "Torpedoes",
    condition => "Condition",
    devices => "Devices",
    damage => "Damage",
    max_energy => "MaxEnergy",
    max_torpedoes => "MaxTorpedoes",
    life_support => "LifeSupport",
});

/// Go `Klingon` (legacy per-quadrant Klingon list entry).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Klingon {
    pub id: i32,
    pub sector: Coord,
    pub energy: f64,
    pub shields: f64,
    pub is_commander: bool,
    pub is_cloaked: bool,
}

crate::go_json_struct!(Klingon, "Klingon", "engine.Klingon" {
    id => "ID",
    sector => "Sector",
    energy => "Energy",
    shields => "Shields",
    is_commander => "IsCommander",
    is_cloaked => "IsCloaked",
});

/// Go `EnemyVessel`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EnemyVessel {
    pub id: i32,
    pub faction: FactionType,
    pub sector: Coord,
    pub energy: f64,
    pub shields: f64,
    pub max_energy: f64,
    pub is_commander: bool,
    pub is_cloaked: bool,
    pub cloak_turns: i32,
    pub special_state: i32,
}

crate::go_json_struct!(EnemyVessel, "EnemyVessel", "engine.EnemyVessel" {
    id => "id",
    faction => "faction",
    sector => "sector",
    energy => "energy",
    shields => "shields",
    max_energy => "max_energy",
    is_commander => "is_commander",
    is_cloaked => "is_cloaked",
    cloak_turns => "cloak_turns",
    special_state => "special_state",
});

/// Go `PlasmaTorpedo`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlasmaTorpedo {
    pub id: i32,
    pub source_id: i32,
    pub sector: Coord,
    pub energy: f64,
    pub target_sector: Coord,
    pub turns_in_flight: i32,
}

crate::go_json_struct!(PlasmaTorpedo, "PlasmaTorpedo", "engine.PlasmaTorpedo" {
    id => "id",
    source_id => "source_id",
    sector => "sector",
    energy => "energy",
    target_sector => "target",
    turns_in_flight => "turns",
});

/// Go `TholianWebSegment`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TholianWebSegment {
    pub coord: Coord,
    pub strength: f64,
}

crate::go_json_struct!(TholianWebSegment, "TholianWebSegment", "engine.TholianWebSegment" {
    coord => "coord",
    strength => "strength",
});

/// Go `QuadrantState`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct QuadrantState {
    pub grid: [[EntityType; 9]; 9],
    pub enemies: GoVec<Ptr<EnemyVessel>>,
    pub plasma_torpedoes: GoVec<Ptr<PlasmaTorpedo>>,
    pub web_segments: GoVec<Ptr<TholianWebSegment>>,
    pub starbase: Option<Coord>,
    pub stars: GoVec<Coord>,
    /// `json:"-"`.
    pub klingons: GoVec<Ptr<Klingon>>,
}

crate::go_json_struct!(QuadrantState, "QuadrantState", "engine.QuadrantState" {
    grid => "grid",
    enemies => "enemies",
    plasma_torpedoes => "plasma_torpedoes" omitempty,
    web_segments => "web_segments" omitempty,
    starbase => "starbase" omitempty,
    stars => "stars" omitempty,
});

/// Go `SyncQuadrantEnemies`: rebuilds the legacy Klingons slice from Enemies.
pub fn sync_quadrant_enemies(quad: &mut QuadrantState) {
    let mut legacy: GoVec<Ptr<Klingon>> = GoVec::nil();
    for e in quad.enemies.iter() {
        let Some(e) = e else { continue };
        let e = e.borrow();
        if e.faction == FactionType::Klingon {
            legacy.push(ptr(Klingon {
                id: e.id,
                sector: e.sector,
                energy: e.energy,
                shields: e.shields,
                is_commander: e.is_commander,
                is_cloaked: e.is_cloaked,
            }));
        }
    }
    quad.klingons = legacy;
}

/// Writes the legacy Klingon list back into `Enemies` (the reverse of
/// [`sync_quadrant_enemies`]).
///
/// Phasers, torpedoes, `KlingonTurn` and cloaking act on the legacy
/// `Klingons` list, while the adversary turn acts on `Enemies` and then
/// rebuilds `Klingons` from it. Go never copied the legacy list back, so with
/// adversaries active the rebuild restored damaged Klingons to full energy.
/// This copies each Klingon's energy, shields, position and cloak into its
/// `Enemies` entry (matched by ID); entries without a legacy counterpart are
/// left alone (kills remove theirs through [`remove_enemy_klingon`]).
pub fn sync_enemies_from_klingons(quad: &mut QuadrantState) {
    for e in quad.enemies.iter().flatten() {
        let (faction, id) = {
            let eb = e.borrow();
            (eb.faction, eb.id)
        };
        if faction != FactionType::Klingon {
            continue;
        }
        if let Some(k) = quad.klingons.iter().flatten().find(|k| k.borrow().id == id) {
            let k = k.borrow();
            let mut eb = e.borrow_mut();
            eb.energy = k.energy;
            eb.shields = k.shields;
            eb.sector = k.sector;
            eb.is_cloaked = k.is_cloaked;
            eb.is_commander = k.is_commander;
        }
    }
}

/// Removes the Klingon with `id` from `Enemies` when it is destroyed on the
/// legacy list. Go left it there, so the next adversary turn (which rebuilds
/// the legacy list from `Enemies`) brought the destroyed Klingon back.
pub fn remove_enemy_klingon(quad: &mut QuadrantState, id: i32) {
    let pos = quad.enemies.iter().position(|e| {
        e.as_ref().is_some_and(|e| {
            let e = e.borrow();
            e.faction == FactionType::Klingon && e.id == id
        })
    });
    if let Some(i) = pos {
        quad.enemies.remove(i);
    }
}

/// Go `GameMetrics`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GameMetrics {
    pub klingons_killed: i32,
    pub commanders_killed: i32,
    pub super_commanders_killed: i32,
    pub romulans_killed: i32,
    pub romulans_surrendered: i32,
    pub starbases_destroyed: i32,
    pub stars_destroyed: i32,
    pub planets_destroyed: i32,
    pub casualties: i32,
    pub help_calls: i32,
    pub starships_lost: i32,
}

crate::go_json_struct!(GameMetrics, "GameMetrics", "engine.GameMetrics" {
    klingons_killed => "klingons_killed",
    commanders_killed => "commanders_killed",
    super_commanders_killed => "super_commanders_killed",
    romulans_killed => "romulans_killed",
    romulans_surrendered => "romulans_surrendered",
    starbases_destroyed => "starbases_destroyed",
    stars_destroyed => "stars_destroyed",
    planets_destroyed => "planets_destroyed",
    casualties => "casualties",
    help_calls => "help_calls",
    starships_lost => "starships_lost",
});

/// Go `QuadrantInfo`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct QuadrantInfo {
    pub klingons: i32,
    pub commanders: i32,
    pub super_commanders: i32,
}

/// Go `GameOptions`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GameOptions {
    pub difficulty: DifficultyProfile,
}

crate::go_json_struct!(GameOptions, "GameOptions", "engine.GameOptions" {
    difficulty => "difficulty",
});

/// Go `DifficultyNormal = ProfileNormal`.
pub fn difficulty_normal() -> DifficultyProfile {
    DifficultyProfile::normal()
}

impl GoJson for PRNG {
    fn encode_json(&self, e: &mut Encoder) {
        e.raw("{}");
    }
    fn decode_json(&mut self, v: &Value, d: &mut Decoder) {
        match v {
            Value::Null | Value::Object(_) => {}
            other => d.type_error(other.kind_name(), "engine.PRNG"),
        }
    }
    fn go_type_name() -> String {
        "engine.PRNG".into()
    }
}

/// Go `GameState`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GameState {
    /// `*PRNG` (JSON `"RNG": {}`; decoding yields a PRNG with a nil source).
    pub rng: Option<PRNG>,
    pub scenario: ScenarioID,
    pub rules: GameRules,
    pub skill: SkillLevel,
    pub length: GameLength,
    pub enterprise: Enterprise,
    pub current_quad: QuadrantState,
    /// Klingons*100 + Starbases*10 + Stars.
    pub galaxy_chart: [[i32; 9]; 9],
    pub chart_discovered: [[bool; 9]; 9],
    pub chart_known_bases: [[bool; 9]; 9],
    pub quadrant_env: [[EnvironmentType; 9]; 9],
    pub remaining_klingons: i32,
    pub remaining_starbases: i32,
    pub stardate: f64,
    pub initial_stardate: f64,
    pub time_remaining: f64,
    pub metrics: GameMetrics,
    pub game_won: bool,
    pub game_over: bool,
    pub game_over_reason: GameOverReason,
    pub days_remaining: f64,
    pub klingons_remaining: i32,
    pub score: i32,
    pub options: GameOptions,
    pub active_refits: Option<BTreeMap<RefitID, i32>>,
    /// `json:"-"`.
    pub galaxy: [[Option<QuadrantInfo>; GALAXY_SIZE]; GALAXY_SIZE],
}

crate::go_json_struct!(GameState, "GameState", "engine.GameState" {
    rng => "RNG",
    scenario => "scenario" omitempty,
    rules => "rules",
    skill => "Skill",
    length => "Length",
    enterprise => "Enterprise",
    current_quad => "CurrentQuad",
    galaxy_chart => "GalaxyChart",
    chart_discovered => "ChartDiscovered",
    chart_known_bases => "ChartKnownBases",
    quadrant_env => "quadrant_env",
    remaining_klingons => "RemainingKlingons",
    remaining_starbases => "RemainingStarbases",
    stardate => "Stardate",
    initial_stardate => "InitialStardate",
    time_remaining => "TimeRemaining",
    metrics => "metrics",
    game_won => "game_won",
    game_over => "game_over",
    game_over_reason => "game_over_reason" omitempty,
    days_remaining => "days_remaining",
    klingons_remaining => "klingons_remaining",
    score => "score",
    options => "options",
    active_refits => "active_refits" omitempty,
});

/// Shared game state handle (Go `*GameState` aliased by the TUI model,
/// `TourState.CurrentGameState`, save envelopes, ...).
pub type GameRef = Rc<RefCell<GameState>>;

/// Wraps a state into a new shared handle.
pub fn game_ref(g: GameState) -> GameRef {
    Rc::new(RefCell::new(g))
}

impl GameState {
    /// The PRNG; panics like Go on a nil `RNG` or a nil source.
    pub fn rng(&mut self) -> &mut PRNG {
        self.rng
            .as_mut()
            .expect("runtime error: invalid memory address or nil pointer dereference")
    }

    /// Go `NewGame`.
    pub fn new_game(seed: i64, skill: SkillLevel, length: GameLength) -> GameState {
        Self::new_game_with_options(
            seed,
            skill,
            length,
            default_rules_for_profile(&DifficultyProfile::normal()),
        )
    }

    /// Go `NewGameWithSeed`.
    pub fn new_game_with_seed(seed: i64) -> GameState {
        let mut g = Self::new_game(seed, SkillLevel::Good, GameLength::Medium);
        g.enterprise.max_energy = 3000.0;
        g.enterprise.max_torpedoes = 10;
        g
    }

    /// Go `NewGameWithOptions`.
    pub fn new_game_with_options(
        seed: i64,
        skill: SkillLevel,
        length: GameLength,
        rules: GameRules,
    ) -> GameState {
        let mut rng = PRNG::new(seed);
        // Go evaluates composite-literal fields in source order.
        let stardate = (2000 + rng.intn(1000)) as f64;
        let time_remaining = 30.0 * rules.time_margin;
        let days_remaining = 30.0 * rules.time_margin;
        let quad = [rng.intn(8) + 1, rng.intn(8) + 1];
        let sector = [rng.intn(8) + 1, rng.intn(8) + 1];
        let mut g = GameState {
            rng: Some(rng),
            rules,
            skill,
            length,
            stardate,
            time_remaining,
            days_remaining,
            remaining_klingons: 15,
            klingons_remaining: 15,
            remaining_starbases: 3,
            enterprise: Enterprise {
                quad,
                sector,
                energy: 5000.0,
                shields: 0.0,
                torpedoes: 10,
                max_energy: 5000.0,
                max_torpedoes: 10,
                condition: ConditionType::Green,
                ..Default::default()
            },
            ..Default::default()
        };
        g.initial_stardate = g.stardate;
        for row in g.galaxy.iter_mut() {
            for q in row.iter_mut() {
                *q = Some(QuadrantInfo::default());
            }
        }

        // 1. Stars
        for r in 1..=8 {
            for c in 1..=8 {
                g.galaxy_chart[r][c] = g.rng().intn(9) + 1;
            }
        }

        // 2. Starbases
        let mut placed_bases = 0;
        while placed_bases < g.remaining_starbases {
            let r = (g.rng().intn(8) + 1) as usize;
            let c = (g.rng().intn(8) + 1) as usize;
            if (g.galaxy_chart[r][c] % 100) / 10 == 0 {
                g.galaxy_chart[r][c] += 10;
                if g.rules.surveillance != SurveillanceMode::blackout() {
                    g.chart_known_bases[r][c] = true;
                }
                placed_bases += 1;
            }
        }

        // 3. Klingons in clusters of 1..3
        let mut klingons_to_place = g.remaining_klingons;
        while klingons_to_place > 0 {
            let r = (g.rng().intn(8) + 1) as usize;
            let c = (g.rng().intn(8) + 1) as usize;
            let current_k = g.galaxy_chart[r][c] / 100;
            if current_k < 9 {
                let mut cluster = g.rng().intn(3) + 1;
                if cluster > klingons_to_place {
                    cluster = klingons_to_place;
                }
                if current_k + cluster > 9 {
                    cluster = 9 - current_k;
                }
                g.galaxy_chart[r][c] += cluster * 100;
                klingons_to_place -= cluster;
            }
        }

        // 4. Starting quadrant discovered
        let q = g.enterprise.quad;
        g.chart_discovered[q[0] as usize][q[1] as usize] = true;

        // 5. Anomalies
        crate::anomalies::seed_anomalies(&mut g);
        g
    }

    /// Go `GameState.PopulateQuadrant`.
    pub fn populate_quadrant(&mut self, quad: Coord, ent_sector: Coord) {
        if quad[0] < 1 || quad[0] > 8 || quad[1] < 1 || quad[1] > 8 {
            return;
        }
        let mut ent_sector = ent_sector;
        if ent_sector[0] < 1 || ent_sector[0] > 8 || ent_sector[1] < 1 || ent_sector[1] > 8 {
            ent_sector = [4, 4];
        }
        if self.rng.is_none() {
            self.rng = Some(PRNG::new(12345));
        }

        self.current_quad = QuadrantState::default();
        self.current_quad.grid[ent_sector[0] as usize][ent_sector[1] as usize] =
            EntityType::Enterprise;
        self.enterprise.quad = quad;
        self.enterprise.sector = ent_sector;

        let val = self.galaxy_chart[quad[0] as usize][quad[1] as usize];
        let num_k = val / 100;
        let num_b = (val % 100) / 10;
        let num_s = val % 10;

        fn find_empty_sector(g: &mut GameState) -> Coord {
            loop {
                let r = g.rng().intn(8) + 1;
                let c = g.rng().intn(8) + 1;
                if g.current_quad.grid[r as usize][c as usize] == EntityType::Empty {
                    return [r, c];
                }
            }
        }

        if num_b > 0 {
            let sb = if self.scenario == ScenarioID::starbase_siege()
                && quad == [4, 4]
                && ent_sector != [4, 4]
            {
                [4, 4]
            } else {
                find_empty_sector(self)
            };
            self.current_quad.starbase = Some(sb);
            self.current_quad.grid[sb[0] as usize][sb[1] as usize] = EntityType::Starbase;
        }

        if num_k > 0 {
            self.current_quad.klingons = GoVec::empty();
            for i in 0..num_k {
                let k_coord = find_empty_sector(self);
                let is_commander = self.rules.klingon_cloak && i == 0;
                let ent_type = if is_commander {
                    EntityType::Commander
                } else {
                    EntityType::Klingon
                };
                self.current_quad.grid[k_coord[0] as usize][k_coord[1] as usize] = ent_type;
                let skill = self.skill.as_i32() as f64;
                let mut energy = 300.0 + self.rng().float64() * 150.0 + 25.0 * skill;
                if is_commander {
                    energy = 950.0 + 400.0 * self.rng().float64() + 50.0 * skill;
                }
                self.current_quad.klingons.push(ptr(Klingon {
                    id: i + 1,
                    sector: k_coord,
                    energy,
                    is_commander,
                    is_cloaked: false,
                    shields: 0.0,
                }));
            }

            if self.rules.adversaries && !self.current_quad.klingons.is_empty() {
                let convert_idx = self.current_quad.klingons.len() - 1;
                let target_k = self.current_quad.klingons[convert_idx]
                    .as_ref()
                    .unwrap()
                    .borrow()
                    .clone();
                let is_border = quad[0] == 1 || quad[0] == 8 || quad[1] == 1 || quad[1] == 8;
                let has_starbase = num_b > 0;
                let ts = target_k.sector;
                let adv = if has_starbase || is_border {
                    self.current_quad.grid[ts[0] as usize][ts[1] as usize] = EntityType::Tholian;
                    EnemyVessel {
                        id: target_k.id,
                        faction: FactionType::Tholian,
                        sector: ts,
                        energy: 800.0,
                        shields: 300.0,
                        max_energy: 800.0,
                        special_state: 0,
                        ..Default::default()
                    }
                } else {
                    self.current_quad.grid[ts[0] as usize][ts[1] as usize] = EntityType::Empty;
                    EnemyVessel {
                        id: target_k.id,
                        faction: FactionType::Romulan,
                        sector: ts,
                        energy: 1000.0,
                        shields: 400.0,
                        max_energy: 1000.0,
                        is_cloaked: true,
                        cloak_turns: 2,
                        ..Default::default()
                    }
                };
                let mut adv = Some(adv);
                let mut enemies = GoVec::empty();
                for (idx, k) in self.current_quad.klingons.iter().enumerate() {
                    if idx == convert_idx {
                        enemies.push(ptr(adv.take().unwrap()));
                    } else {
                        let k = k.as_ref().unwrap().borrow();
                        enemies.push(ptr(EnemyVessel {
                            id: k.id,
                            faction: FactionType::Klingon,
                            sector: k.sector,
                            energy: k.energy,
                            shields: k.shields,
                            max_energy: k.energy,
                            is_commander: k.is_commander,
                            is_cloaked: k.is_cloaked,
                            ..Default::default()
                        }));
                    }
                }
                self.current_quad.enemies = enemies;
                sync_quadrant_enemies(&mut self.current_quad);
            }
        }

        if num_s > 0 {
            self.current_quad.stars = GoVec::empty();
            for _ in 0..num_s {
                let s = find_empty_sector(self);
                self.current_quad.stars.push(s);
                self.current_quad.grid[s[0] as usize][s[1] as usize] = EntityType::Star;
            }
        }

        self.chart_discovered[quad[0] as usize][quad[1] as usize] = true;

        self.enterprise.condition = if num_k > 0 {
            ConditionType::Red
        } else {
            ConditionType::Green
        };
    }
}

/// Grid cell accessor with Go-like indexing (panics when out of range).
#[inline]
pub(crate) fn cell(grid: &[[EntityType; 9]; 9], c: Coord) -> EntityType {
    grid[c[0] as usize][c[1] as usize]
}

/// Mutable grid cell accessor.
#[inline]
pub(crate) fn cell_mut(grid: &mut [[EntityType; 9]; 9], c: Coord) -> &mut EntityType {
    &mut grid[c[0] as usize][c[1] as usize]
}
