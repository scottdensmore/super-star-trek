//! Typed engine events (port of `pkg/engine/events.go`, plus the faction
//! events from `romulan.go`, `tholian.go` and `klingon_pack.go`).
//!
//! Every Go event struct maps to a Rust struct of the same name; the
//! [`Event`] enum replaces Go's `Event` interface and type switches.

use crate::rules::SurveillanceMode;
use crate::types::{ConditionType, Coord, DeviceID, EntityType, EnvironmentType, GameOverReason};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EventShieldTransfer {
    pub new_shields: f64,
    pub new_energy: f64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EventTorpedoFired {
    pub origin: Coord,
    pub angle: f64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EventTorpedoHit {
    pub target: Coord,
    pub entity: EntityType,
    pub damage: f64,
    pub destroyed: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EventPhaserFired {
    pub energy: f64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EventPhaserHit {
    pub target: Coord,
    pub klingon_id: i32,
    pub damage: f64,
    pub destroyed: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EventDocked {
    pub starbase: Coord,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EventShipMoved {
    pub from_quad: Coord,
    pub to_quad: Coord,
    pub from_sector: Coord,
    pub to_sector: Coord,
    pub warp: f64,
    pub energy_used: f64,
    pub time_used: f64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EventObstacleEncountered {
    pub sector: Coord,
    pub entity: EntityType,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EventConditionChanged {
    pub from: ConditionType,
    pub to: ConditionType,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EventGameOver {
    pub reason: GameOverReason,
    pub score: f64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EventKlingonCounterAttack {
    pub enemy_id: i32,
    pub damage: f64,
    pub shield_damage: f64,
    pub hull_damage: f64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EventSubsystemDamaged {
    pub device: DeviceID,
    pub repair_time: f64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EventSubsystemRepaired {
    pub device: DeviceID,
}

/// `ReadingsMap` is a Go `map[Coord]int`; a `BTreeMap` keeps iteration
/// deterministic (Go's iteration order is random, so no Go output may
/// depend on it).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EventLRScanCompleted {
    pub center_quad: Coord,
    pub scanned_quads: Vec<Coord>,
    pub relayed_by_base: bool,
    pub degraded: bool,
    pub readings: [[i32; 3]; 3],
    pub readings_map: BTreeMap<Coord, i32>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EventStarbaseSurveillance {
    pub starbase_coord: Coord,
    pub updated_quads: i32,
    pub mode: SurveillanceMode,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EventKlingonCloakState {
    pub klingon_id: i32,
    pub cloaked: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EventHelpCalled {}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EventHazardTriggered {
    pub hazard_type: String,
    pub description: String,
    pub energy_drain: f64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EventWormholeJump {
    pub from_quad: Coord,
    pub from_sector: Coord,
    pub to_quad: Coord,
    pub to_sector: Coord,
    pub time_delta: f64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EventSingularityAbsorption {
    pub sector: Coord,
    pub target: EntityType,
    pub weapon: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EventAnomalyDiscovered {
    pub quad: Coord,
    pub env: EnvironmentType,
}

// --- klingon_pack.go ---

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EventKlingonScreening {
    pub raider_id: i32,
    pub commander_id: i32,
    pub interposed: Coord,
}

// --- romulan.go ---

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EventRomulanDecloak {
    pub romulan_id: i32,
    pub sector: Coord,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EventPlasmaLaunched {
    pub torpedo_id: i32,
    pub source_id: i32,
    pub sector: Coord,
    pub energy: f64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EventPlasmaMoved {
    pub torpedo_id: i32,
    pub from: Coord,
    pub to: Coord,
    pub yield_: f64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EventPlasmaImpact {
    pub torpedo_id: i32,
    pub shield_damage: f64,
    pub hull_damage: f64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EventPlasmaDissipated {
    pub torpedo_id: i32,
    pub sector: Coord,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EventPlasmaIntercepted {
    pub torpedo_id: i32,
    pub sector: Coord,
    pub weapon: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EventEntityMove {
    pub target: EntityType,
    pub from: Coord,
    pub to: Coord,
}

// --- tholian.go ---

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EventWebSegmentLaid {
    pub spinner_id: i32,
    pub coord: Coord,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EventWebBreached {
    pub coord: Coord,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EventWebCollapsed {
    pub segments_count: i32,
}

/// Replaces Go's `Event` interface. Variant names drop the `Event` prefix.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    ShieldTransfer(EventShieldTransfer),
    TorpedoFired(EventTorpedoFired),
    TorpedoHit(EventTorpedoHit),
    PhaserFired(EventPhaserFired),
    PhaserHit(EventPhaserHit),
    Docked(EventDocked),
    ShipMoved(EventShipMoved),
    ObstacleEncountered(EventObstacleEncountered),
    ConditionChanged(EventConditionChanged),
    GameOver(EventGameOver),
    KlingonCounterAttack(EventKlingonCounterAttack),
    SubsystemDamaged(EventSubsystemDamaged),
    SubsystemRepaired(EventSubsystemRepaired),
    LRScanCompleted(EventLRScanCompleted),
    StarbaseSurveillance(EventStarbaseSurveillance),
    KlingonCloakState(EventKlingonCloakState),
    HelpCalled(EventHelpCalled),
    HazardTriggered(EventHazardTriggered),
    WormholeJump(EventWormholeJump),
    SingularityAbsorption(EventSingularityAbsorption),
    AnomalyDiscovered(EventAnomalyDiscovered),
    KlingonScreening(EventKlingonScreening),
    RomulanDecloak(EventRomulanDecloak),
    PlasmaLaunched(EventPlasmaLaunched),
    PlasmaMoved(EventPlasmaMoved),
    PlasmaImpact(EventPlasmaImpact),
    PlasmaDissipated(EventPlasmaDissipated),
    PlasmaIntercepted(EventPlasmaIntercepted),
    EntityMove(EventEntityMove),
    WebSegmentLaid(EventWebSegmentLaid),
    WebBreached(EventWebBreached),
    WebCollapsed(EventWebCollapsed),
}

impl Event {
    /// Go's `EventType()` string.
    pub fn event_type(&self) -> &'static str {
        match self {
            Event::ShieldTransfer(_) => "ShieldTransfer",
            Event::TorpedoFired(_) => "TorpedoFired",
            Event::TorpedoHit(_) => "TorpedoHit",
            Event::PhaserFired(_) => "PhaserFired",
            Event::PhaserHit(_) => "PhaserHit",
            Event::Docked(_) => "Docked",
            Event::ShipMoved(_) => "ShipMoved",
            Event::ObstacleEncountered(_) => "ObstacleEncountered",
            Event::ConditionChanged(_) => "ConditionChanged",
            Event::GameOver(_) => "GameOver",
            Event::KlingonCounterAttack(_) => "KlingonCounterAttack",
            Event::SubsystemDamaged(_) => "SubsystemDamaged",
            Event::SubsystemRepaired(_) => "SubsystemRepaired",
            Event::LRScanCompleted(_) => "LRScanCompleted",
            Event::StarbaseSurveillance(_) => "StarbaseSurveillance",
            Event::KlingonCloakState(_) => "KlingonCloakState",
            Event::HelpCalled(_) => "HelpCalled",
            Event::HazardTriggered(_) => "HazardTriggered",
            Event::WormholeJump(_) => "WormholeJump",
            Event::SingularityAbsorption(_) => "SingularityAbsorption",
            Event::AnomalyDiscovered(_) => "AnomalyDiscovered",
            Event::KlingonScreening(_) => "KlingonScreening",
            Event::RomulanDecloak(_) => "RomulanDecloak",
            Event::PlasmaLaunched(_) => "PlasmaLaunched",
            Event::PlasmaMoved(_) => "PlasmaMoved",
            Event::PlasmaImpact(_) => "PlasmaImpact",
            Event::PlasmaDissipated(_) => "PlasmaDissipated",
            Event::PlasmaIntercepted(_) => "PlasmaIntercepted",
            Event::EntityMove(_) => "EntityMove",
            Event::WebSegmentLaid(_) => "WebSegmentLaid",
            Event::WebBreached(_) => "WebBreached",
            Event::WebCollapsed(_) => "WebCollapsed",
        }
    }
}

// --- Go encoding/json encodings (Go field names; events carry no tags) ---

use crate::json::{Decoder, Encoder, GoJson, Value};

crate::go_json_struct!(EventShieldTransfer, "EventShieldTransfer", "engine.EventShieldTransfer" { new_shields => "NewShields", new_energy => "NewEnergy" });
crate::go_json_struct!(EventTorpedoFired, "EventTorpedoFired", "engine.EventTorpedoFired" { origin => "Origin", angle => "Angle" });
crate::go_json_struct!(EventTorpedoHit, "EventTorpedoHit", "engine.EventTorpedoHit" { target => "Target", entity => "Entity", damage => "Damage", destroyed => "Destroyed" });
crate::go_json_struct!(EventPhaserFired, "EventPhaserFired", "engine.EventPhaserFired" { energy => "Energy" });
crate::go_json_struct!(EventPhaserHit, "EventPhaserHit", "engine.EventPhaserHit" { target => "Target", klingon_id => "KlingonID", damage => "Damage", destroyed => "Destroyed" });
crate::go_json_struct!(EventDocked, "EventDocked", "engine.EventDocked" { starbase => "Starbase" });
crate::go_json_struct!(EventShipMoved, "EventShipMoved", "engine.EventShipMoved" { from_quad => "FromQuad", to_quad => "ToQuad", from_sector => "FromSector", to_sector => "ToSector", warp => "Warp", energy_used => "EnergyUsed", time_used => "TimeUsed" });
crate::go_json_struct!(EventObstacleEncountered, "EventObstacleEncountered", "engine.EventObstacleEncountered" { sector => "Sector", entity => "Entity" });
crate::go_json_struct!(EventConditionChanged, "EventConditionChanged", "engine.EventConditionChanged" { from => "From", to => "To" });
crate::go_json_struct!(EventGameOver, "EventGameOver", "engine.EventGameOver" { reason => "Reason", score => "Score" });
crate::go_json_struct!(EventKlingonCounterAttack, "EventKlingonCounterAttack", "engine.EventKlingonCounterAttack" { enemy_id => "EnemyID", damage => "Damage", shield_damage => "ShieldDamage", hull_damage => "HullDamage" });
crate::go_json_struct!(EventSubsystemDamaged, "EventSubsystemDamaged", "engine.EventSubsystemDamaged" { device => "Device", repair_time => "RepairTime" });
crate::go_json_struct!(EventSubsystemRepaired, "EventSubsystemRepaired", "engine.EventSubsystemRepaired" { device => "Device" });
crate::go_json_struct!(EventStarbaseSurveillance, "EventStarbaseSurveillance", "engine.EventStarbaseSurveillance" { starbase_coord => "StarbaseCoord", updated_quads => "UpdatedQuads", mode => "Mode" });
crate::go_json_struct!(EventKlingonCloakState, "EventKlingonCloakState", "engine.EventKlingonCloakState" { klingon_id => "KlingonID", cloaked => "Cloaked" });
crate::go_json_struct!(EventHelpCalled, "EventHelpCalled", "engine.EventHelpCalled" {});
crate::go_json_struct!(EventHazardTriggered, "EventHazardTriggered", "engine.EventHazardTriggered" { hazard_type => "HazardType", description => "Description", energy_drain => "EnergyDrain" });
crate::go_json_struct!(EventWormholeJump, "EventWormholeJump", "engine.EventWormholeJump" { from_quad => "FromQuad", from_sector => "FromSector", to_quad => "ToQuad", to_sector => "ToSector", time_delta => "TimeDelta" });
crate::go_json_struct!(EventSingularityAbsorption, "EventSingularityAbsorption", "engine.EventSingularityAbsorption" { sector => "Sector", target => "Target", weapon => "Weapon" });
crate::go_json_struct!(EventAnomalyDiscovered, "EventAnomalyDiscovered", "engine.EventAnomalyDiscovered" { quad => "Quad", env => "Env" });
crate::go_json_struct!(EventKlingonScreening, "EventKlingonScreening", "engine.EventKlingonScreening" { raider_id => "RaiderID", commander_id => "CommanderID", interposed => "Interposed" });
crate::go_json_struct!(EventRomulanDecloak, "EventRomulanDecloak", "engine.EventRomulanDecloak" { romulan_id => "RomulanID", sector => "Sector" });
crate::go_json_struct!(EventPlasmaLaunched, "EventPlasmaLaunched", "engine.EventPlasmaLaunched" { torpedo_id => "TorpedoID", source_id => "SourceID", sector => "Sector", energy => "Energy" });
crate::go_json_struct!(EventPlasmaMoved, "EventPlasmaMoved", "engine.EventPlasmaMoved" { torpedo_id => "TorpedoID", from => "From", to => "To", yield_ => "Yield" });
crate::go_json_struct!(EventPlasmaImpact, "EventPlasmaImpact", "engine.EventPlasmaImpact" { torpedo_id => "TorpedoID", shield_damage => "ShieldDamage", hull_damage => "HullDamage" });
crate::go_json_struct!(EventPlasmaDissipated, "EventPlasmaDissipated", "engine.EventPlasmaDissipated" { torpedo_id => "TorpedoID", sector => "Sector" });
crate::go_json_struct!(EventPlasmaIntercepted, "EventPlasmaIntercepted", "engine.EventPlasmaIntercepted" { torpedo_id => "TorpedoID", sector => "Sector", weapon => "Weapon" });
crate::go_json_struct!(EventEntityMove, "EventEntityMove", "engine.EventEntityMove" { target => "Target", from => "From", to => "To" });
crate::go_json_struct!(EventWebSegmentLaid, "EventWebSegmentLaid", "engine.EventWebSegmentLaid" { spinner_id => "SpinnerID", coord => "Coord" });
crate::go_json_struct!(EventWebBreached, "EventWebBreached", "engine.EventWebBreached" { coord => "Coord" });
crate::go_json_struct!(EventWebCollapsed, "EventWebCollapsed", "engine.EventWebCollapsed" { segments_count => "SegmentsCount" });

/// Go cannot marshal `ReadingsMap` (`map[Coord]int`); it is encoded as a
/// list `[{"Coord":[r,c],"Value":v}, ...]` sorted by coordinate. Decoding
/// is not supported (no-op).
impl GoJson for EventLRScanCompleted {
    fn encode_json(&self, e: &mut Encoder) {
        let mut st = e.begin_object();
        e.field(&mut st, "CenterQuad");
        self.center_quad.encode_json(e);
        e.field(&mut st, "ScannedQuads");
        if self.scanned_quads.is_empty() {
            e.null();
        } else {
            crate::json::GoVec::from_vec(self.scanned_quads.clone()).encode_json(e);
        }
        e.field(&mut st, "RelayedByBase");
        e.bool(self.relayed_by_base);
        e.field(&mut st, "Degraded");
        e.bool(self.degraded);
        e.field(&mut st, "Readings");
        self.readings.encode_json(e);
        e.field(&mut st, "ReadingsMap");
        if self.readings_map.is_empty() {
            e.null();
        } else {
            e.raw("[");
            for (i, (c, v)) in self.readings_map.iter().enumerate() {
                if i > 0 {
                    e.raw(",");
                }
                let mut s2 = e.begin_object();
                e.field(&mut s2, "Coord");
                c.encode_json(e);
                e.field(&mut s2, "Value");
                e.int(*v as i64);
                e.end_object();
            }
            e.raw("]");
        }
        e.end_object();
    }
    fn decode_json(&mut self, _v: &Value, _d: &mut Decoder) {}
    fn go_type_name() -> String {
        "engine.EventLRScanCompleted".into()
    }
}

impl Event {
    /// `{"type": <EventType()>, "data": <event>}` as compact Go JSON (the
    /// format used by the parity transcripts).
    pub fn to_json(&self) -> String {
        let mut e = Encoder::default();
        let mut st = e.begin_object();
        e.field(&mut st, "type");
        e.string(self.event_type());
        e.field(&mut st, "data");
        match self {
            Event::ShieldTransfer(x) => x.encode_json(&mut e),
            Event::TorpedoFired(x) => x.encode_json(&mut e),
            Event::TorpedoHit(x) => x.encode_json(&mut e),
            Event::PhaserFired(x) => x.encode_json(&mut e),
            Event::PhaserHit(x) => x.encode_json(&mut e),
            Event::Docked(x) => x.encode_json(&mut e),
            Event::ShipMoved(x) => x.encode_json(&mut e),
            Event::ObstacleEncountered(x) => x.encode_json(&mut e),
            Event::ConditionChanged(x) => x.encode_json(&mut e),
            Event::GameOver(x) => x.encode_json(&mut e),
            Event::KlingonCounterAttack(x) => x.encode_json(&mut e),
            Event::SubsystemDamaged(x) => x.encode_json(&mut e),
            Event::SubsystemRepaired(x) => x.encode_json(&mut e),
            Event::LRScanCompleted(x) => x.encode_json(&mut e),
            Event::StarbaseSurveillance(x) => x.encode_json(&mut e),
            Event::KlingonCloakState(x) => x.encode_json(&mut e),
            Event::HelpCalled(x) => x.encode_json(&mut e),
            Event::HazardTriggered(x) => x.encode_json(&mut e),
            Event::WormholeJump(x) => x.encode_json(&mut e),
            Event::SingularityAbsorption(x) => x.encode_json(&mut e),
            Event::AnomalyDiscovered(x) => x.encode_json(&mut e),
            Event::KlingonScreening(x) => x.encode_json(&mut e),
            Event::RomulanDecloak(x) => x.encode_json(&mut e),
            Event::PlasmaLaunched(x) => x.encode_json(&mut e),
            Event::PlasmaMoved(x) => x.encode_json(&mut e),
            Event::PlasmaImpact(x) => x.encode_json(&mut e),
            Event::PlasmaDissipated(x) => x.encode_json(&mut e),
            Event::PlasmaIntercepted(x) => x.encode_json(&mut e),
            Event::EntityMove(x) => x.encode_json(&mut e),
            Event::WebSegmentLaid(x) => x.encode_json(&mut e),
            Event::WebBreached(x) => x.encode_json(&mut e),
            Event::WebCollapsed(x) => x.encode_json(&mut e),
        }
        e.end_object();
        match e.err {
            Some(err) => format!("ERR {}", err),
            None => String::from_utf8(e.buf).expect("utf-8"),
        }
    }
}

/// JSON array of [`Event::to_json`] values.
pub fn events_to_json(events: &[Event]) -> String {
    let parts: Vec<String> = events.iter().map(|e| e.to_json()).collect();
    format!("[{}]", parts.join(","))
}
