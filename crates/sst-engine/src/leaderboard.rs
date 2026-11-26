//! Hall of fame (pure parts of `pkg/engine/leaderboard.go`; file I/O lives
//! in [`crate::persist`]).

use crate::campaign::TourState;
use crate::gotime::GoTime;
use crate::json::GoVec;
use crate::scenarios::{get_scenario, ScenarioID};

/// Go `ScoreEntry`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ScoreEntry {
    pub captain_name: String,
    pub score: i32,
    pub rank: String,
    pub skill: String,
    pub difficulty: String,
    pub stardate: f64,
    pub date: GoTime,
    pub game_won: bool,
    pub scenario: String,
    pub commendation: String,
}

crate::go_json_struct!(ScoreEntry, "ScoreEntry", "engine.ScoreEntry" {
    captain_name => "captain_name",
    score => "score",
    rank => "rank",
    skill => "skill" omitempty,
    difficulty => "difficulty" omitempty,
    stardate => "stardate" omitempty,
    date => "date",
    game_won => "game_won" omitempty,
    scenario => "scenario" omitempty,
    commendation => "commendation" omitempty,
});

/// Go `TourRecord`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TourRecord {
    pub date: String,
    pub callsign: String,
    pub rank: String,
    pub score: i32,
    pub sectors_cleared: i32,
    pub total_sectors: i32,
    pub refits_count: i32,
    pub medals: GoVec<String>,
    pub completed: bool,
}

crate::go_json_struct!(TourRecord, "TourRecord", "engine.TourRecord" {
    date => "date",
    callsign => "callsign",
    rank => "rank",
    score => "score",
    sectors_cleared => "sectors_cleared",
    total_sectors => "total_sectors",
    refits_count => "refits_count",
    medals => "medals",
    completed => "completed",
});

/// Go `Leaderboard`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Leaderboard {
    pub entries: GoVec<ScoreEntry>,
    pub tour_records: GoVec<TourRecord>,
}

crate::go_json_struct!(Leaderboard, "Leaderboard", "engine.Leaderboard" {
    entries => "entries",
    tour_records => "tour_records" omitempty,
});

fn legend_date() -> GoTime {
    GoTime::date(2265, 1, 1, 0, 0, 0, 0, 0)
}

fn entry(name: &str, score: i32, rank: &str, skill: &str, stardate: f64) -> ScoreEntry {
    ScoreEntry {
        captain_name: name.into(),
        score,
        rank: rank.into(),
        skill: skill.into(),
        difficulty: skill.into(),
        stardate,
        date: legend_date(),
        game_won: true,
        ..Default::default()
    }
}

impl Leaderboard {
    /// Go `NewLeaderboard`.
    pub fn new() -> Leaderboard {
        Leaderboard {
            entries: GoVec::empty(),
            tour_records: GoVec::empty(),
        }
    }

    /// Go `DefaultLeaderboard` (classic Starfleet legends).
    pub fn default_board() -> Leaderboard {
        Leaderboard {
            entries: GoVec::from_vec(vec![
                entry("James T. Kirk", 1180, "[FADM]", "Emeritus", 3421.5),
                entry("Spock", 980, "[RADM]", "Expert", 3210.4),
                entry("Christopher Pike", 840, "[COMM]", "Expert", 3105.2),
                entry("Hikaru Sulu", 720, "[COMM]", "Good", 2980.1),
                entry("Nyota Uhura", 560, "[CAPT]", "Good", 2840.7),
                entry("Montgomery Scott", 480, "[CDR]", "Good", 2710.3),
                entry("Leonard McCoy", 420, "[CDR]", "Fair", 2650.9),
                entry("Pavel Chekov", 310, "[LT]", "Fair", 2510.6),
                entry("Christine Chapel", 180, "[LT]", "Novice", 2390.2),
                entry("Janice Rand", 90, "[CADET]", "Novice", 2210.8),
            ]),
            tour_records: GoVec::empty(),
        }
    }

    /// Go `Leaderboard.Qualifies`.
    pub fn qualifies(&self, score: i32) -> bool {
        if self.entries.len() < 10 {
            return true;
        }
        score > self.entries[self.entries.len() - 1].score
    }

    /// Go `Leaderboard.Add`: insert, stable sort descending, keep top 10.
    pub fn add(&mut self, e: ScoreEntry) {
        self.entries.push(e);
        sort_and_cap(&mut self.entries);
    }

    /// Go `Leaderboard.RecordTour` with an explicit "now" (Go uses `time.Now()`).
    pub fn record_tour_at(&mut self, tour: &TourState, callsign: &str, now: GoTime) -> TourRecord {
        let callsign = if callsign.is_empty() {
            "Enterprise"
        } else {
            callsign
        };
        let (rank, medals) = calculate_tour_commission(Some(tour));
        let rec = TourRecord {
            date: now.format_date_minute(),
            callsign: callsign.into(),
            rank: rank.into(),
            score: tour.total_tour_score,
            sectors_cleared: tour.sectors_completed,
            total_sectors: tour.sectors.len() as i32,
            refits_count: count_installed_refits(tour),
            medals,
            completed: tour.completed,
        };
        self.tour_records.push(rec.clone());
        rec
    }

    /// Post-load normalisation done by Go `LoadLeaderboard` (nil -> empty, sort, cap).
    pub fn normalize_loaded(&mut self, tour_records_too: bool) {
        if self.entries.is_nil() {
            self.entries = GoVec::empty();
        }
        if tour_records_too && self.tour_records.is_nil() {
            self.tour_records = GoVec::empty();
        }
        sort_and_cap(&mut self.entries);
    }
}

fn sort_and_cap(entries: &mut GoVec<ScoreEntry>) {
    entries
        .vec_mut()
        .sort_by_key(|e| std::cmp::Reverse(e.score));
    if entries.len() > 10 {
        entries.truncate(10);
    }
}

fn count_installed_refits(tour: &TourState) -> i32 {
    tour.installed_refits
        .as_ref()
        .map_or(0, |m| m.values().sum())
}

/// Go `CalculateTourCommission`: (rank, medals).
pub fn calculate_tour_commission(tour: Option<&TourState>) -> (&'static str, GoVec<String>) {
    let mut medals: GoVec<String> = GoVec::empty();
    let Some(tour) = tour else {
        return ("Cadet", medals);
    };
    let rank = match tour.sectors_completed {
        4 => {
            for m in [
                "Starfleet Legion of Honor",
                "Klingon Campaign Ribbon",
                "Vanguard Star",
            ] {
                medals.push(m.into());
            }
            "Admiral of the Fleet"
        }
        3 => {
            for m in ["Starfleet Merit Citation", "Klingon Campaign Ribbon"] {
                medals.push(m.into());
            }
            "Commodore"
        }
        2 => {
            medals.push("Frontier Service Medal".into());
            "Fleet Captain"
        }
        1 => {
            medals.push("Patrol Ribbon".into());
            "Captain"
        }
        _ => "Commander (KIA)",
    };
    if count_installed_refits(tour) >= 6 {
        medals.push("Master Starship Architect".into());
    }
    (rank, medals)
}

/// Resolves an ID or alias to the canonical scenario ID (Go `normID` logic).
pub fn normalize_scenario_id(id: &ScenarioID) -> ScenarioID {
    match get_scenario(id) {
        Some(s) => s.id.clone(),
        None => id.clone(),
    }
}

/// Go `DefaultScenarioLeaderboard`.
pub fn default_scenario_leaderboard(id: &ScenarioID) -> Leaderboard {
    let norm = normalize_scenario_id(id);
    let d = legend_date();
    let se = |name: &str, score, rank: &str, comm: &str, won| ScoreEntry {
        captain_name: name.into(),
        score,
        rank: rank.into(),
        commendation: comm.into(),
        scenario: norm.0.clone(),
        date: d,
        game_won: won,
        ..Default::default()
    };
    if norm == ScenarioID::kobayashi_maru() {
        Leaderboard {
            entries: GoVec::from_vec(vec![
                se(
                    "Cadet James T. Kirk",
                    2400,
                    "[KIRK-AWARD]",
                    "Commendation for Original Thinking",
                    false,
                ),
                se(
                    "Cadet Spock",
                    950,
                    "[COMM-3]",
                    "Starfleet Cross of Honor",
                    false,
                ),
                se(
                    "Lieutenant Saavik",
                    620,
                    "[COMM-2]",
                    "Tactical Excellence",
                    false,
                ),
            ]),
            tour_records: GoVec::nil(),
        }
    } else if norm == ScenarioID::mutara_nebula() {
        Leaderboard {
            entries: GoVec::from_vec(vec![
                se("Admiral James T. Kirk", 1500, "[ADM]", "", true),
                se("Captain Clark Terrell", 820, "[CAPT]", "", false),
            ]),
            tour_records: GoVec::nil(),
        }
    } else if norm == ScenarioID::starbase_siege() {
        Leaderboard {
            entries: GoVec::from_vec(vec![
                se("Captain Hikaru Sulu", 1350, "[CAPT]", "", true),
                se("Commander Montgomery Scott", 1100, "[COMM]", "", true),
            ]),
            tour_records: GoVec::nil(),
        }
    } else {
        Leaderboard {
            entries: GoVec::empty(),
            tour_records: GoVec::nil(),
        }
    }
}

/// The in-memory part of Go `AddScenarioScore`: fills date/scenario, checks
/// qualification, inserts, and returns `(rank, added)`.
pub fn add_scenario_score_to(
    lb: &mut Leaderboard,
    norm: &ScenarioID,
    mut entry: ScoreEntry,
    now: GoTime,
) -> (i32, bool) {
    if entry.date.is_zero() {
        entry.date = now;
    }
    if entry.scenario.is_empty() {
        entry.scenario = norm.0.clone();
    }
    if !lb.qualifies(entry.score) {
        return (0, false);
    }
    lb.add(entry.clone());
    let mut rank = 0;
    for i in (0..lb.entries.len()).rev() {
        let e = &lb.entries[i];
        if e.captain_name == entry.captain_name
            && e.score == entry.score
            && e.date.equal(&entry.date)
        {
            rank = i as i32 + 1;
            break;
        }
    }
    if rank == 0 {
        for i in (0..lb.entries.len()).rev() {
            let e = &lb.entries[i];
            if e.captain_name == entry.captain_name && e.score == entry.score {
                rank = i as i32 + 1;
                break;
            }
        }
    }
    if rank == 0 {
        rank = 1;
    }
    (rank, true)
}
