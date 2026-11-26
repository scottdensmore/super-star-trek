//! Starship refits (port of `pkg/engine/refits.go`).

use std::collections::BTreeMap;
use std::sync::LazyLock;

use crate::actions::EngineError;
use crate::campaign::TourState;
use crate::state::GameState;

/// Go `RefitID` (string type).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct RefitID(pub String);

impl RefitID {
    pub fn new(s: impl Into<String>) -> Self {
        RefitID(s.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn dilithium_core() -> Self {
        RefitID("dilithium_core".into())
    }
    pub fn deflector_grid() -> Self {
        RefitID("deflector_grid".into())
    }
    pub fn torpedo_casings() -> Self {
        RefitID("torpedo_casings".into())
    }
    pub fn torpedo_bays() -> Self {
        RefitID("torpedo_bays".into())
    }
    pub fn sensor_matrix() -> Self {
        RefitID("sensor_matrix".into())
    }
    pub fn damage_nanites() -> Self {
        RefitID("damage_nanites".into())
    }
}

impl std::fmt::Display for RefitID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

crate::go_json_string_newtype!(RefitID, "engine.RefitID");

/// Go `RefitDefinition`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RefitDefinition {
    pub id: RefitID,
    pub name: String,
    pub description: String,
    pub tier_costs: [i32; 3],
    pub tier_effects: [String; 3],
}

crate::go_json_struct!(RefitDefinition, "RefitDefinition", "engine.RefitDefinition" {
    id => "id",
    name => "name",
    description => "description",
    tier_costs => "tier_costs",
    tier_effects => "tier_effects",
});

fn def(
    id: RefitID,
    name: &str,
    desc: &str,
    costs: [i32; 3],
    effects: [&str; 3],
) -> RefitDefinition {
    RefitDefinition {
        id,
        name: name.into(),
        description: desc.into(),
        tier_costs: costs,
        tier_effects: effects.map(String::from),
    }
}

static CATALOG: LazyLock<Vec<RefitDefinition>> = LazyLock::new(|| {
    vec![
        def(
            RefitID::dilithium_core(),
            "Dilithium Core Tuning",
            "Increases primary warp core energy storage capacity.",
            [500, 1000, 1750],
            [
                "Max Energy: 3,500 (+500)",
                "Max Energy: 4,000 (+1,000)",
                "Max Energy: 4,500 (+1,500)",
            ],
        ),
        def(
            RefitID::deflector_grid(),
            "Reinforced Deflectors",
            "Enhances deflector shield absorption efficiency under attack.",
            [450, 900, 1500],
            [
                "Shield drain reduced by 15%",
                "Shield drain reduced by 30%",
                "Shield drain reduced by 45%",
            ],
        ),
        def(
            RefitID::torpedo_casings(),
            "High-Yield Torpedoes",
            "Upgrades photon warhead casing density and antimatter payload.",
            [400, 800, 1400],
            [
                "Torpedo damage +25%",
                "Torpedo damage +50%",
                "Torpedo damage +75%",
            ],
        ),
        def(
            RefitID::torpedo_bays(),
            "Auxiliary Torpedo Magazine",
            "Expands physical torpedo storage capacity in forward ordnance deck.",
            [350, 700, 1200],
            [
                "Max Torpedoes: 14 (+4)",
                "Max Torpedoes: 18 (+8)",
                "Max Torpedoes: 22 (+12)",
            ],
        ),
        def(
            RefitID::sensor_matrix(),
            "Subspace Sensor Matrix",
            "Extends long-range scan radius and reveals cloaked silhouettes.",
            [300, 600, 1000],
            [
                "LRS scan radius +1 quadrant",
                "Cloaked hostile silhouettes revealed",
                "Automatic free LRS on quadrant entry",
            ],
        ),
        def(
            RefitID::damage_nanites(),
            "Automated Repair Bots",
            "Deployable micro-drones accelerate damage control during warp transit.",
            [400, 850, 1450],
            [
                "Warp movement passive repair +0.5d",
                "Warp movement passive repair +1.0d",
                "Warp movement passive repair +1.5d",
            ],
        ),
    ]
});

/// Go `RefitCatalog`.
pub fn refit_catalog() -> &'static [RefitDefinition] {
    &CATALOG
}

/// Go `GetRefitDefinition`.
pub fn get_refit_definition(id: &RefitID) -> Option<&'static RefitDefinition> {
    CATALOG.iter().find(|d| &d.id == id)
}

/// Go `ApplyRefits` (`refits == None` is a nil map: no-op).
pub fn apply_refits(g: &mut GameState, refits: Option<&BTreeMap<RefitID, i32>>) {
    let Some(refits) = refits else { return };
    g.active_refits = Some(refits.clone());
    let tier = |id: RefitID| refits.get(&id).copied().unwrap_or(0);
    let t = tier(RefitID::dilithium_core());
    if t > 0 {
        g.enterprise.max_energy = 3000.0 + (t * 500) as f64;
    } else {
        g.enterprise.max_energy = 3000.0;
    }
    if g.enterprise.energy > g.enterprise.max_energy {
        g.enterprise.energy = g.enterprise.max_energy;
    }
    let t = tier(RefitID::torpedo_bays());
    if t > 0 {
        g.enterprise.max_torpedoes = 10 + t * 4;
    } else {
        g.enterprise.max_torpedoes = 10;
    }
    if g.enterprise.torpedoes > g.enterprise.max_torpedoes {
        g.enterprise.torpedoes = g.enterprise.max_torpedoes;
    }
}

/// Go `PurchaseRefit`.
pub fn purchase_refit(tour: Option<&mut TourState>, id: &RefitID) -> Result<(), EngineError> {
    let Some(tour) = tour else {
        return Err(EngineError::new("tour is nil"));
    };
    let Some(def) = get_refit_definition(id) else {
        return Err(EngineError(format!("unknown refit module: {}", id.0)));
    };
    let refits = tour.installed_refits.get_or_insert_with(BTreeMap::new);
    let current = refits.get(id).copied().unwrap_or(0);
    if current >= 3 {
        return Err(EngineError(format!(
            "{} is already at maximum tier 3",
            def.name
        )));
    }
    let next = current + 1;
    let cost = def.tier_costs[(next - 1) as usize];
    if tour.requisition_points < cost {
        return Err(EngineError(format!(
            "insufficient requisition: requires {}, have {}",
            cost, tour.requisition_points
        )));
    }
    tour.requisition_points -= cost;
    tour.installed_refits
        .as_mut()
        .unwrap()
        .insert(id.clone(), next);
    Ok(())
}

fn active_tier(g: Option<&GameState>, id: RefitID) -> Option<i32> {
    let g = g?;
    let refits = g.active_refits.as_ref()?;
    Some(refits.get(&id).copied().unwrap_or(0).clamp(0, 3))
}

/// Go `CalculateTorpedoDamage`.
pub fn calculate_torpedo_damage(g: Option<&GameState>, base: f64) -> f64 {
    match active_tier(g, RefitID::torpedo_casings()) {
        Some(t) if t > 0 => base * (1.0 + t as f64 * 0.25),
        _ => base,
    }
}

/// Go `CalculateShieldDamageAbsorption`.
pub fn calculate_shield_damage_absorption(g: Option<&GameState>, incoming: f64) -> f64 {
    match active_tier(g, RefitID::deflector_grid()) {
        Some(t) if t > 0 => incoming * (1.0 - t as f64 * 0.15),
        _ => incoming,
    }
}
