//! Core enumerations and coordinates (port of the type declarations in
//! `pkg/engine/state.go`, `geometry.go` and `events.go`).
//!
//! Go's integer-backed enums become `i32` newtypes with associated
//! constants (`SkillLevel::Expert`), so every Go value (including the zero
//! value `SkillLevel(0)`) is representable. They encode to JSON as their
//! integer value, exactly like Go.

/// A 1-indexed `[row, col]` coordinate (1..=8) within a quadrant or the galaxy.
/// Go: `type Coord [2]int`.
pub type Coord = [i32; 2];

/// Go `Coord.Row()`.
#[inline]
pub fn row(c: Coord) -> i32 {
    c[0]
}

/// Go `Coord.Col()`.
#[inline]
pub fn col(c: Coord) -> i32 {
    c[1]
}

macro_rules! int_enum {
    ($(#[$meta:meta])* $name:ident { $($variant:ident = $val:expr),+ $(,)? }) => {
        $(#[$meta])*
        ///
        /// Represented as an `i32` newtype (like Go's named `int`) so that any
        /// value, including the Go zero value and out-of-range values decoded
        /// from JSON, is representable. The named values are associated consts.
        #[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
        pub struct $name(pub i32);

        #[allow(non_upper_case_globals)]
        impl $name {
            $(pub const $variant: $name = $name($val);)+

            /// Integer value (Go's underlying `int`).
            #[inline]
            pub fn as_i32(self) -> i32 {
                self.0
            }

            /// Converts from the underlying integer (any value is kept, like Go).
            #[inline]
            pub fn from_i32(v: i32) -> Self {
                $name(v)
            }

            /// Variant name, if the value is one of the named constants.
            pub fn name(self) -> Option<&'static str> {
                match self.0 {
                    $(x if x == $val => Some(stringify!($variant)),)+
                    _ => None,
                }
            }
        }

        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self.name() {
                    Some(n) => write!(f, "{}::{}", stringify!($name), n),
                    None => write!(f, "{}({})", stringify!($name), self.0),
                }
            }
        }
    };
}

int_enum! {
    /// Go `EntityType`.
    EntityType {
        Empty = 0,
        Enterprise = 1,
        Klingon = 2,
        Commander = 3,
        SuperCommander = 4,
        Starbase = 5,
        Star = 6,
        Planet = 7,
        BlackHole = 8,
        Wormhole = 9,
        Romulan = 10,
        Tholian = 11,
        PlasmaTorpedo = 12,
        TholianWeb = 13,
    }
}

int_enum! {
    /// Go `FactionType`.
    FactionType {
        Klingon = 0,
        Romulan = 1,
        Tholian = 2,
    }
}

int_enum! {
    /// Go `EnvironmentType`.
    EnvironmentType {
        Normal = 0,
        Nebula = 1,
        IonStorm = 2,
    }
}

int_enum! {
    /// Go `ConditionType`.
    ConditionType {
        Green = 0,
        Yellow = 1,
        Red = 2,
        Docked = 3,
    }
}

int_enum! {
    /// Go `DeviceID`.
    DeviceID {
        Warp = 0,
        SRSensors = 1,
        LRSensors = 2,
        Phasers = 3,
        PhotonTubes = 4,
        DamageControl = 5,
        Shields = 6,
        Computer = 7,
    }
}

/// Go `NumDevices`.
pub const NUM_DEVICES: usize = 8;

impl DeviceID {
    /// All devices in index order.
    pub const ALL: [DeviceID; NUM_DEVICES] = [
        DeviceID::Warp,
        DeviceID::SRSensors,
        DeviceID::LRSensors,
        DeviceID::Phasers,
        DeviceID::PhotonTubes,
        DeviceID::DamageControl,
        DeviceID::Shields,
        DeviceID::Computer,
    ];

    #[inline]
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

int_enum! {
    /// Go `SkillLevel` (starts at 1).
    SkillLevel {
        Novice = 1,
        Fair = 2,
        Good = 3,
        Expert = 4,
        Emeritus = 5,
    }
}

int_enum! {
    /// Go `GameLength` (starts at 1).
    GameLength {
        Short = 1,
        Medium = 2,
        Long = 3,
    }
}

int_enum! {
    /// Go `GameOverReason`.
    GameOverReason {
        Won = 0,
        Energy = 1,
        Destroyed = 2,
        Time = 3,
        Stranded = 4,
        Lost = 5,
    }
}

crate::go_json_int_enum!(EntityType, "engine.EntityType");
crate::go_json_int_enum!(FactionType, "engine.FactionType");
crate::go_json_int_enum!(EnvironmentType, "engine.EnvironmentType");
crate::go_json_int_enum!(ConditionType, "engine.ConditionType");
crate::go_json_int_enum!(DeviceID, "engine.DeviceID");
crate::go_json_int_enum!(SkillLevel, "engine.SkillLevel");
crate::go_json_int_enum!(GameLength, "engine.GameLength");
crate::go_json_int_enum!(GameOverReason, "engine.GameOverReason");
