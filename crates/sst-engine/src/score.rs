//! Scoring (port of `pkg/engine/score.go`).

use crate::gomath;
use crate::state::GameState;

/// Go `ScoreBreakdown`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ScoreBreakdown {
    pub klingons_killed: i32,
    pub klingon_points: i32,
    pub commanders_killed: i32,
    pub commander_points: i32,
    pub super_commanders_killed: i32,
    pub super_commander_points: i32,
    pub romulans_killed: i32,
    pub romulan_points: i32,
    pub romulans_surrendered: i32,
    pub surrendered_points: i32,
    pub elapsed_stardates: f64,
    pub kill_rate: f64,
    pub kill_rate_points: i32,
    pub game_won: bool,
    pub win_bonus: i32,
    pub starbases_lost: i32,
    pub starbase_penalty: i32,
    pub starships_lost: i32,
    pub starship_penalty: i32,
    pub help_calls: i32,
    pub help_penalty: i32,
    pub planets_destroyed: i32,
    pub planet_penalty: i32,
    pub stars_destroyed: i32,
    pub star_penalty: i32,
    pub casualties: i32,
    pub casualty_penalty: i32,
    pub total_score: i32,
    pub rank_title: String,
    pub rank_badge: String,
}

crate::go_json_struct!(ScoreBreakdown, "ScoreBreakdown", "engine.ScoreBreakdown" {
    klingons_killed => "KlingonsKilled",
    klingon_points => "KlingonPoints",
    commanders_killed => "CommandersKilled",
    commander_points => "CommanderPoints",
    super_commanders_killed => "SuperCommandersKilled",
    super_commander_points => "SuperCommanderPoints",
    romulans_killed => "RomulansKilled",
    romulan_points => "RomulanPoints",
    romulans_surrendered => "RomulansSurrendered",
    surrendered_points => "SurrenderedPoints",
    elapsed_stardates => "ElapsedStardates",
    kill_rate => "KillRate",
    kill_rate_points => "KillRatePoints",
    game_won => "GameWon",
    win_bonus => "WinBonus",
    starbases_lost => "StarbasesLost",
    starbase_penalty => "StarbasePenalty",
    starships_lost => "StarshipsLost",
    starship_penalty => "StarshipPenalty",
    help_calls => "HelpCalls",
    help_penalty => "HelpPenalty",
    planets_destroyed => "PlanetsDestroyed",
    planet_penalty => "PlanetPenalty",
    stars_destroyed => "StarsDestroyed",
    star_penalty => "StarPenalty",
    casualties => "Casualties",
    casualty_penalty => "CasualtyPenalty",
    total_score => "TotalScore",
    rank_title => "RankTitle",
    rank_badge => "RankBadge",
});

/// Go `ComputeScore` (classic rules.c scoring).
pub fn compute_score(g: Option<&GameState>, won: bool) -> ScoreBreakdown {
    let Some(g) = g else {
        return ScoreBreakdown::default();
    };
    let m = &g.metrics;
    let mut elapsed = g.stardate - g.initial_stardate;
    if (!won || elapsed <= 0.0) && elapsed < 5.0 {
        elapsed = 5.0;
    }
    let klingon_pts = m.klingons_killed * 10;
    let commander_pts = m.commanders_killed * 50;
    let super_pts = m.super_commanders_killed * 200;
    let romulan_pts = m.romulans_killed * 20;
    let surrendered_pts = m.romulans_surrendered;
    let total_kills = m.klingons_killed + m.commanders_killed + m.super_commanders_killed;
    let kill_rate = if elapsed > 0.0 {
        total_kills as f64 / elapsed
    } else {
        0.0
    };
    let kill_rate_pts = gomath::round(500.0 * kill_rate) as i32;
    let win_bonus = if won { 100 * g.skill.as_i32() } else { 0 };
    let starbase_penalty = m.starbases_destroyed * 100;
    let starship_penalty = m.starships_lost * 100;
    let help_penalty = m.help_calls * 45;
    let planet_penalty = m.planets_destroyed * 10;
    let star_penalty = m.stars_destroyed * 5;
    let casualty_penalty = m.casualties;
    let total = klingon_pts
        + commander_pts
        + super_pts
        + romulan_pts
        + surrendered_pts
        + kill_rate_pts
        + win_bonus
        - starbase_penalty
        - starship_penalty
        - help_penalty
        - planet_penalty
        - star_penalty
        - casualty_penalty;
    let (title, badge) = rank_for_score(total);
    ScoreBreakdown {
        klingons_killed: m.klingons_killed,
        klingon_points: klingon_pts,
        commanders_killed: m.commanders_killed,
        commander_points: commander_pts,
        super_commanders_killed: m.super_commanders_killed,
        super_commander_points: super_pts,
        romulans_killed: m.romulans_killed,
        romulan_points: romulan_pts,
        romulans_surrendered: m.romulans_surrendered,
        surrendered_points: surrendered_pts,
        elapsed_stardates: elapsed,
        kill_rate,
        kill_rate_points: kill_rate_pts,
        game_won: won,
        win_bonus,
        starbases_lost: m.starbases_destroyed,
        starbase_penalty,
        starships_lost: m.starships_lost,
        starship_penalty,
        help_calls: m.help_calls,
        help_penalty,
        planets_destroyed: m.planets_destroyed,
        planet_penalty,
        stars_destroyed: m.stars_destroyed,
        star_penalty,
        casualties: m.casualties,
        casualty_penalty,
        total_score: total,
        rank_title: title.into(),
        rank_badge: badge.into(),
    }
}

/// Go `RankForScore`: (title, badge).
pub fn rank_for_score(score: i32) -> (&'static str, &'static str) {
    match score {
        s if s < 0 => ("Dishonorable Discharge", "[DISHONOR]"),
        s if s < 100 => ("Starfleet Cadet", "[CADET]"),
        s if s < 200 => ("Lieutenant", "[LT]"),
        s if s < 350 => ("Commander", "[CDR]"),
        s if s < 500 => ("Captain", "[CAPT]"),
        s if s < 750 => ("Commodore", "[COMM]"),
        s if s < 1000 => ("Rear Admiral", "[RADM]"),
        _ => ("Fleet Admiral", "[FADM]"),
    }
}
