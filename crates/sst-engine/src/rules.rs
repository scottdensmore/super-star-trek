//! Difficulty profiles and realism rules (port of `pkg/engine/rules.go`).
//!
//! Go's string-backed enums (`DifficultyProfile`, `SurveillanceMode`) can
//! hold arbitrary strings (e.g. an unknown `--surveillance` value is stored
//! verbatim and saved to JSON), so they are string newtypes here.

macro_rules! string_enum {
    ($(#[$meta:meta])* $name:ident { $($konst:ident = $val:expr),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
        pub struct $name(pub String);

        impl $name {
            $(pub fn $konst() -> Self { $name($val.to_string()) })+

            pub fn new(s: impl Into<String>) -> Self {
                $name(s.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl PartialEq<&str> for $name {
            fn eq(&self, other: &&str) -> bool {
                self.0 == *other
            }
        }
    };
}

string_enum! {
    /// Go `DifficultyProfile`.
    DifficultyProfile {
        casual = "casual",
        normal = "normal",
        hardcore = "hardcore",
        nightmare = "nightmare",
        expert = "expert",
        emeritus = "emeritus",
        custom = "custom",
    }
}

string_enum! {
    /// Go `SurveillanceMode`.
    SurveillanceMode {
        full = "full",
        classic = "classic",
        local = "local",
        blackout = "blackout",
    }
}

/// Animation speed constants for combat visual FX.
pub const ANIM_SPEED_OFF: i32 = 0;
pub const ANIM_SPEED_FAST: i32 = 1;
pub const ANIM_SPEED_NORMAL: i32 = 2;
pub const ANIM_SPEED_CINEMATIC: i32 = 3;

/// Go `GameRules`. JSON field names: profile, surveillance,
/// sensor_degradation, repair_multiplier, klingon_cloak, spatial_anomalies,
/// adversaries, time_margin, anim_speed.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GameRules {
    pub profile: DifficultyProfile,
    pub surveillance: SurveillanceMode,
    pub sensor_degradation: bool,
    pub repair_multiplier: f64,
    pub klingon_cloak: bool,
    pub spatial_anomalies: bool,
    pub adversaries: bool,
    pub time_margin: f64,
    pub anim_speed: i32,
}

/// Go `DefaultRulesForProfile`.
pub fn default_rules_for_profile(profile: &DifficultyProfile) -> GameRules {
    #[allow(clippy::too_many_arguments)]
    fn r(
        profile: DifficultyProfile,
        surveillance: SurveillanceMode,
        sensor_degradation: bool,
        repair_multiplier: f64,
        klingon_cloak: bool,
        spatial_anomalies: bool,
        adversaries: bool,
        time_margin: f64,
    ) -> GameRules {
        GameRules {
            profile,
            surveillance,
            sensor_degradation,
            repair_multiplier,
            klingon_cloak,
            spatial_anomalies,
            adversaries,
            time_margin,
            anim_speed: ANIM_SPEED_NORMAL,
        }
    }
    match profile.as_str() {
        "casual" => r(
            DifficultyProfile::casual(),
            SurveillanceMode::full(),
            false,
            0.75,
            false,
            false,
            false,
            1.25,
        ),
        "hardcore" => r(
            DifficultyProfile::hardcore(),
            SurveillanceMode::local(),
            true,
            1.50,
            true,
            true,
            false,
            0.80,
        ),
        "nightmare" => r(
            DifficultyProfile::nightmare(),
            SurveillanceMode::blackout(),
            true,
            2.00,
            true,
            true,
            false,
            0.60,
        ),
        "expert" => r(
            DifficultyProfile::expert(),
            SurveillanceMode::classic(),
            true,
            1.25,
            true,
            true,
            true,
            0.90,
        ),
        "emeritus" => r(
            DifficultyProfile::emeritus(),
            SurveillanceMode::blackout(),
            true,
            2.00,
            true,
            true,
            true,
            0.60,
        ),
        _ => r(
            DifficultyProfile::normal(),
            SurveillanceMode::classic(),
            true,
            1.00,
            false,
            false,
            false,
            1.00,
        ),
    }
}

crate::go_json_string_newtype!(DifficultyProfile, "engine.DifficultyProfile");
crate::go_json_string_newtype!(SurveillanceMode, "engine.SurveillanceMode");

crate::go_json_struct!(GameRules, "GameRules", "engine.GameRules" {
    profile => "profile",
    surveillance => "surveillance",
    sensor_degradation => "sensor_degradation",
    repair_multiplier => "repair_multiplier",
    klingon_cloak => "klingon_cloak",
    spatial_anomalies => "spatial_anomalies",
    adversaries => "adversaries",
    time_margin => "time_margin",
    anim_speed => "anim_speed",
});
