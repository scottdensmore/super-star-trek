//! Scenario registry (port of `pkg/engine/scenarios.go`). The Go global
//! registry (guarded by a `sync.RWMutex`) is a process-wide `RwLock` here;
//! scenarios are handed out as `Arc<Scenario>` (Go `*Scenario`).

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, RwLock};

use crate::json::GoVec;
use crate::score::ScoreBreakdown;
use crate::state::GameState;
use crate::types::GameOverReason;

/// Go `ScenarioID` (string type; `""` = none).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct ScenarioID(pub String);
crate::go_json_string_newtype!(ScenarioID, "engine.ScenarioID");

impl ScenarioID {
    pub fn new(s: impl Into<String>) -> Self {
        ScenarioID(s.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    /// Go `ScenarioNone`.
    pub fn none() -> Self {
        ScenarioID(String::new())
    }
    /// Go `ScenarioKobayashiMaru`.
    pub fn kobayashi_maru() -> Self {
        ScenarioID("kobayashi-maru".into())
    }
    /// Go `ScenarioMutaraNebula`.
    pub fn mutara_nebula() -> Self {
        ScenarioID("mutara-nebula".into())
    }
    /// Go `ScenarioStarbaseSiege`.
    pub fn starbase_siege() -> Self {
        ScenarioID("starbase-siege".into())
    }
}

impl std::fmt::Display for ScenarioID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Go `Scenario.Build`.
pub type BuildFn = fn(i64) -> GameState;
/// Go `Scenario.Evaluate`: (done, won, reason).
pub type EvaluateFn = fn(&mut GameState) -> (bool, bool, GameOverReason);
/// Go `Scenario.ComputeScore`.
pub type ComputeScoreFn = fn(&GameState, bool) -> ScoreBreakdown;

/// Go `Scenario`.
#[derive(Debug, Clone, Default)]
pub struct Scenario {
    pub id: ScenarioID,
    pub name: String,
    pub subtitle: String,
    pub difficulty: String,
    pub description: String,
    pub briefing: GoVec<String>,
    pub aliases: GoVec<String>,
    pub build: Option<BuildFn>,
    pub evaluate: Option<EvaluateFn>,
    pub compute_score: Option<ComputeScoreFn>,
}

crate::go_json_struct!(Scenario, "Scenario", "engine.Scenario" {
    id => "id",
    name => "name",
    subtitle => "subtitle",
    difficulty => "difficulty",
    description => "description",
    briefing => "briefing",
    aliases => "aliases" omitempty,
});

impl Scenario {
    /// Calls `Build` (panics like Go if it is nil).
    pub fn build(&self, seed: i64) -> GameState {
        (self.build.expect("nil Build func"))(seed)
    }
}

#[derive(Default)]
struct Registry {
    map: HashMap<ScenarioID, Arc<Scenario>>,
    alias: HashMap<String, ScenarioID>,
    order: Vec<ScenarioID>,
}

static REGISTRY: LazyLock<RwLock<Registry>> = LazyLock::new(|| {
    let lock = RwLock::new(Registry::default());
    {
        let mut r = lock.write().unwrap();
        for s in builtin_scenarios() {
            register_into(&mut r, s);
        }
    }
    lock
});

fn register_into(r: &mut Registry, s: Scenario) {
    if s.id == ScenarioID::none() {
        return;
    }
    let norm = s.id.0.trim().to_lowercase();
    if !r.order.contains(&s.id) {
        r.order.push(s.id.clone());
    }
    let id = s.id.clone();
    let aliases: Vec<String> = s.aliases.to_vec();
    r.map.insert(id.clone(), Arc::new(s));
    r.alias.insert(norm, id.clone());
    for a in aliases {
        let na = a.trim().to_lowercase();
        if !na.is_empty() {
            r.alias.insert(na, id.clone());
        }
    }
}

/// Go `RegisterScenario`.
pub fn register_scenario(s: Scenario) {
    let mut r = REGISTRY.write().unwrap();
    register_into(&mut r, s);
}

/// Go `GetScenario`: lookup by ID or alias (case-insensitive).
pub fn get_scenario(id: &ScenarioID) -> Option<Arc<Scenario>> {
    let r = REGISTRY.read().unwrap();
    let norm = id.0.trim().to_lowercase();
    if norm.is_empty() {
        return None;
    }
    let target = r.alias.get(&norm)?;
    r.map.get(target).cloned()
}

/// Go `ListScenarios` (registration order).
pub fn list_scenarios() -> Vec<Arc<Scenario>> {
    let r = REGISTRY.read().unwrap();
    r.order
        .iter()
        .filter_map(|id| r.map.get(id).cloned())
        .collect()
}

/// Snapshot of the registry (for tests that register temporary scenarios).
pub struct RegistrySnapshot {
    map: HashMap<ScenarioID, Arc<Scenario>>,
    alias: HashMap<String, ScenarioID>,
    order: Vec<ScenarioID>,
}

/// Saves the registry state.
pub fn snapshot_registry() -> RegistrySnapshot {
    let r = REGISTRY.read().unwrap();
    RegistrySnapshot {
        map: r.map.clone(),
        alias: r.alias.clone(),
        order: r.order.clone(),
    }
}

/// Restores a saved registry state.
pub fn restore_registry(s: RegistrySnapshot) {
    let mut r = REGISTRY.write().unwrap();
    r.map = s.map;
    r.alias = s.alias;
    r.order = s.order;
}

fn strs(v: &[&str]) -> GoVec<String> {
    GoVec::from_vec(v.iter().map(|s| s.to_string()).collect())
}

fn builtin_scenarios() -> Vec<Scenario> {
    use crate::{scenario_kobayashi as km, scenario_mutara as mn, scenario_siege as sg};
    vec![
        Scenario {
            id: ScenarioID::kobayashi_maru(),
            name: "Kobayashi Maru".into(),
            subtitle: "The Unwinnable Simulation".into(),
            difficulty: "Extreme".into(),
            description: "Starfleet Academy's infamous no-win tactical scenario in the Klingon Neutral Zone.".into(),
            briefing: strs(&[
                "STATION LOG: GAMMA HYDRA SECTOR 10 (KLINGON NEUTRAL ZONE)",
                "Third-class neutronic fuel carrier KOBAYASHI MARU has struck a gravitic mine.",
                "Hull breached, 81 crew aboard, 300 passengers. Power systems failing.",
                "You are entering the Neutral Zone in violation of the Organian Peace Treaty.",
            ]),
            aliases: strs(&["kobayashi", "km"]),
            build: Some(km::build_kobayashi_maru),
            evaluate: Some(km::evaluate_kobayashi_maru),
            compute_score: Some(km::compute_score_kobayashi_maru),
        },
        Scenario {
            id: ScenarioID::mutara_nebula(),
            name: "Mutara Nebula".into(),
            subtitle: "Blind Tactical Duel".into(),
            difficulty: "Hard".into(),
            description: "A tactical duel against a cloaked Klingon commander inside an ionized gas cloud with shields disabled.".into(),
            briefing: strs(&[
                "TACTICAL BRIEFING: MUTARA SECTOR (IONIZED GAS CLOUD)",
                "A rogue Klingon Super-Commander has lured the Enterprise into the Mutara Nebula.",
                "High electrostatic discharge has disabled defensive shields and blinded long-range sensors.",
                "Locate and destroy the enemy commander before your ship is compromised.",
                "Sensors cannot lock on to the cloaked commander. Fire photon torpedoes along its last known bearing to force it out of cloak.",
            ]),
            aliases: strs(&["mutara", "nebula"]),
            build: Some(mn::build_mutara_nebula),
            evaluate: Some(mn::evaluate_mutara_nebula),
            compute_score: Some(mn::compute_score_mutara_nebula),
        },
        Scenario {
            id: ScenarioID::starbase_siege(),
            name: "Starbase Under Siege".into(),
            subtitle: "Defend Starbase 12".into(),
            difficulty: "Challenge".into(),
            description: "Klingon strike fleet is assaulting Starbase 12. Intercept and eliminate the attackers before the base falls.".into(),
            briefing: strs(&[
                "RED ALERT: STARBASE 12 UNDER ATTACK",
                "A coordinated Klingon strike fleet is bombarding Starbase 12 in Sector [4, 4].",
                "The starbase shields are degrading under heavy orbital bombardment.",
                "Rush to their defense and eliminate all hostile warships before the base is destroyed.",
            ]),
            aliases: strs(&["siege", "starbase"]),
            build: Some(sg::build_starbase_siege),
            evaluate: Some(sg::evaluate_starbase_siege),
            compute_score: Some(sg::compute_score_starbase_siege),
        },
    ]
}
