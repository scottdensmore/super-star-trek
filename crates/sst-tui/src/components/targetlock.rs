//! Tactical Target Lock HUD (port of
//! `pkg/tui/components/targetlock/targetlock.go`).
//!
//! Targets are sorted with a port of Go's `sort.Slice` (unstable pdqsort)
//! so that ties order exactly like Go.

mod pdqsort;

use sst_engine::gofmt::{fmt_f, parse_float};
use sst_engine::gomath;
use sst_engine::{bearing, distance, Coord, EntityType, FactionType, GameState, Klingon, Ptr};
use sst_tuikit::lipgloss::{self, CENTER};
use sst_tuikit::tea::{
    self, Cmd, KeyMsg, Msg, KEY_BACKSPACE, KEY_ENTER, KEY_ESC, KEY_LEFT, KEY_RIGHT, KEY_SHIFT_TAB,
    KEY_TAB,
};

use crate::theme::{self, Theme};

#[cfg(test)]
mod parity_tests;
#[cfg(test)]
mod tests;

/// Tactical telemetry and ballistics for one target (Go `TargetInfo`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TargetInfo {
    pub klingon_id: i32,
    pub coord: Coord,
    pub distance: f64,
    pub bearing: f64,
    pub hit_probability: f64,
    pub power: f64,
    pub name: String,
    pub typ: EntityType,
}

/// Emitted when firing a torpedo at the locked target.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FireTorpedoMsg {
    pub target: Coord,
    pub bearing: f64,
}

/// Emitted when firing phasers with the entered energy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FirePhasersMsg {
    pub energy: f64,
}

/// Emitted when dismissing the HUD.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CloseHUDMsg;

/// The Target Lock HUD component (Go `targetlock.Model`).
#[derive(Clone, Debug, Default)]
pub struct Model {
    theme: Option<Theme>,
    targets: Vec<TargetInfo>,
    target_idx: i32,
    ent_sector: Coord,
    ent_energy: f64,
    torpedo_count: i32,
    inputting_phaser: bool,
    phaser_input: String,
    warning_message: String,
}

/// `New`: a HUD using `th` (nil: default theme) with 10 torpedoes.
pub fn new(th: Option<Theme>) -> Model {
    Model {
        theme: Some(th.unwrap_or_else(theme::default_theme)),
        torpedo_count: 10,
        ..Default::default()
    }
}

/// SST bearing (1..9) from `from` to `to`, distance and hit probability.
fn ballistics(from: Coord, to: Coord) -> (f64, f64, f64) {
    let dist = distance(from, to);
    let rad = bearing(from, to);
    let mut b = 1.0 + rad * 4.0 / gomath::PI;
    if b >= 9.0 {
        b -= 8.0;
    }
    if b < 1.0 {
        b += 8.0;
    }
    let hit_prob = gomath::max(0.10, gomath::min(0.95, 1.0 - dist / 15.0));
    (dist, b, hit_prob)
}

fn sort_targets(targets: &mut [TargetInfo]) {
    pdqsort::sort_slice(targets, |a, b| {
        if (a.distance - b.distance).abs() > 1e-6 {
            return a.distance < b.distance;
        }
        a.klingon_id < b.klingon_id
    });
}

impl Model {
    /// `SetTheme` (nil falls back to the default theme).
    pub fn set_theme(&mut self, th: Option<Theme>) {
        self.theme = Some(th.unwrap_or_else(theme::default_theme));
    }

    /// `Theme`.
    pub fn theme(&self) -> Theme {
        self.theme.unwrap_or_else(theme::default_theme)
    }

    fn reset_for(&mut self, ent_sector: Coord, ent_energy: f64, torpedoes: i32) {
        self.ent_sector = ent_sector;
        self.ent_energy = ent_energy;
        self.torpedo_count = torpedoes;
        self.inputting_phaser = false;
        self.phaser_input.clear();
        self.warning_message.clear();
        self.targets.clear();
    }

    fn select_initial(&mut self, initial_target: Coord) {
        sort_targets(&mut self.targets);
        self.target_idx = 0;
        if initial_target != [0, 0] {
            if let Some(i) = self.targets.iter().position(|t| t.coord == initial_target) {
                self.target_idx = i as i32;
            }
        }
    }

    /// `SetState`: ship status plus telemetry for `klingons`, sorted by
    /// distance, selecting `initial_target` when given (non-zero).
    pub fn set_state(
        &mut self,
        ent_sector: Coord,
        ent_energy: f64,
        torpedoes: i32,
        klingons: &[Ptr<Klingon>],
        initial_target: Coord,
    ) {
        self.reset_for(ent_sector, ent_energy, torpedoes);
        for k in klingons.iter().flatten() {
            let k = k.borrow();
            let (dist, b, hit_prob) = ballistics(ent_sector, k.sector);
            self.targets.push(TargetInfo {
                klingon_id: k.id,
                coord: k.sector,
                distance: dist,
                bearing: b,
                hit_probability: hit_prob,
                power: k.energy,
                name: String::new(),
                typ: EntityType::Empty,
            });
        }
        self.select_initial(initial_target);
    }

    /// `SetStateFromGame`: targets are adversary vessels (or legacy
    /// Klingons), in-flight plasma torpedoes and web segments. A nil game
    /// leaves the model unchanged.
    pub fn set_state_from_game(&mut self, g: Option<&GameState>, initial_target: Coord) {
        let Some(g) = g else { return };
        self.reset_for(
            g.enterprise.sector,
            g.enterprise.energy,
            g.enterprise.torpedoes,
        );
        let from = self.ent_sector;
        let push = |targets: &mut Vec<TargetInfo>,
                    id: i32,
                    coord: Coord,
                    power: f64,
                    name: String,
                    typ: EntityType| {
            let (dist, b, hit_prob) = ballistics(from, coord);
            targets.push(TargetInfo {
                klingon_id: id,
                coord,
                distance: dist,
                bearing: b,
                hit_probability: hit_prob,
                power,
                name,
                typ,
            });
        };

        let q = &g.current_quad;
        if !q.enemies.is_empty() {
            for e in q.enemies.iter().flatten() {
                let e = e.borrow();
                if e.energy <= 0.0 || e.is_cloaked {
                    continue;
                }
                let (name, typ) = match e.faction {
                    FactionType::Romulan => {
                        (format!("ROMULAN RAIDER #{}", e.id), EntityType::Romulan)
                    }
                    FactionType::Tholian => {
                        (format!("THOLIAN SPINNER #{}", e.id), EntityType::Tholian)
                    }
                    _ if e.is_commander => (
                        format!("KLINGON COMMANDER #{}", e.id),
                        EntityType::Commander,
                    ),
                    _ => (
                        format!("KLINGON BATTLECRUISER #{}", e.id),
                        EntityType::Klingon,
                    ),
                };
                push(&mut self.targets, e.id, e.sector, e.energy, name, typ);
            }
        } else {
            for k in q.klingons.iter().flatten() {
                let k = k.borrow();
                if k.energy <= 0.0 || k.is_cloaked {
                    continue;
                }
                let (name, typ) = if k.is_commander {
                    (
                        format!("KLINGON COMMANDER #{}", k.id),
                        EntityType::Commander,
                    )
                } else {
                    (
                        format!("KLINGON BATTLECRUISER #{}", k.id),
                        EntityType::Klingon,
                    )
                };
                push(&mut self.targets, k.id, k.sector, k.energy, name, typ);
            }
        }

        for pt in q.plasma_torpedoes.iter().flatten() {
            let pt = pt.borrow();
            if pt.energy <= 0.0 {
                continue;
            }
            let name = format!("INCOMING PLASMA TORPEDO #{}", pt.id);
            push(
                &mut self.targets,
                pt.id,
                pt.sector,
                pt.energy,
                name,
                EntityType::PlasmaTorpedo,
            );
        }

        for (i, ws) in q.web_segments.iter().enumerate() {
            let Some(ws) = ws else { continue };
            let ws = ws.borrow();
            if ws.strength <= 0.0 {
                continue;
            }
            let name = format!("THOLIAN WEB FILAMENT [{},{}]", ws.coord[0], ws.coord[1]);
            push(
                &mut self.targets,
                10000 + i as i32,
                ws.coord,
                ws.strength,
                name,
                EntityType::TholianWeb,
            );
        }

        self.select_initial(initial_target);
    }

    /// `CurrentTarget`: the active target, or `None`.
    pub fn current_target(&self) -> Option<&TargetInfo> {
        if self.target_idx < 0 {
            return None;
        }
        self.targets.get(self.target_idx as usize)
    }

    /// `Targets`: a copy of all targets (Go returns nil when none were
    /// ever loaded; that is an empty vector here).
    pub fn targets(&self) -> Vec<TargetInfo> {
        self.targets.clone()
    }

    /// `WarningMessage`.
    pub fn warning_message(&self) -> &str {
        &self.warning_message
    }

    /// `InputtingPhaser`.
    pub fn inputting_phaser(&self) -> bool {
        self.inputting_phaser
    }

    /// `PhaserInput`.
    pub fn phaser_input(&self) -> &str {
        &self.phaser_input
    }

    /// `TargetIndex`.
    pub fn target_index(&self) -> i32 {
        self.target_idx
    }

    /// `Update`: fire torpedoes, phaser prompt input, target cycling, close.
    pub fn update(&mut self, msg: &Msg) -> Option<Cmd> {
        let key = msg.downcast_ref::<KeyMsg>()?;
        let key_str = key.string();

        if key.typ == KEY_ESC || key_str == "esc" {
            return tea::cmd(|| CloseHUDMsg);
        }

        if key_str == "p" || key_str == "P" {
            self.inputting_phaser = !self.inputting_phaser;
            self.phaser_input.clear();
            self.warning_message.clear();
            return None;
        }

        if self.inputting_phaser {
            if key.typ == KEY_ENTER || key_str == "enter" {
                let val = self
                    .phaser_input
                    .trim_matches(sst_tuikit::unicode::go_is_space);
                if let Ok(energy) = parse_float(val) {
                    if energy > 0.0 {
                        self.inputting_phaser = false;
                        self.phaser_input.clear();
                        self.warning_message.clear();
                        return tea::cmd(move || FirePhasersMsg { energy });
                    }
                }
                self.warning_message = "*** INVALID ENERGY AMOUNT ***".into();
                return None;
            }
            if key.typ == KEY_BACKSPACE || key_str == "backspace" {
                self.warning_message.clear();
                // Go slices off the last byte; the buffer only ever holds
                // ASCII digits and dots.
                self.phaser_input.pop();
                return None;
            }
            let b = key_str.as_bytes();
            if (b.len() == 1 && b[0].is_ascii_digit()) || key_str == "." {
                self.warning_message.clear();
                self.phaser_input.push_str(&key_str);
            }
            return None;
        }

        let n = self.targets.len() as i32;
        if key.typ == KEY_TAB || key.typ == KEY_RIGHT || key_str == "tab" || key_str == "right" {
            if n > 0 {
                self.target_idx = (self.target_idx + 1) % n;
                self.warning_message.clear();
            }
            return None;
        }
        if key.typ == KEY_SHIFT_TAB
            || key.typ == KEY_LEFT
            || key_str == "shift+tab"
            || key_str == "left"
        {
            if n > 0 {
                self.target_idx = (self.target_idx - 1 + n) % n;
                self.warning_message.clear();
            }
            return None;
        }
        if key.typ == KEY_ENTER || key_str == "enter" {
            let target = self.current_target()?.clone();
            if self.torpedo_count <= 0 {
                self.warning_message = "*** NO TORPEDOES REMAINING ***".into();
                return None;
            }
            return tea::cmd(move || FireTorpedoMsg {
                target: target.coord,
                bearing: target.bearing,
            });
        }
        None
    }

    /// `View`: the 52x12 tactical targeting panel.
    pub fn view(&self) -> String {
        let styles = self.theme().styles();

        const TARGET_WIDTH: i32 = 52;
        const TARGET_HEIGHT: i32 = 12;

        let panel_style = styles.panel.clone().border(styles.border.clone(), &[true]);
        let mut border_h = panel_style.get_horizontal_border_size();
        if border_h == 0 {
            border_h = 2;
        }
        let padding_h = panel_style.get_horizontal_padding();
        let mut border_v = panel_style.get_vertical_border_size();
        if border_v == 0 {
            border_v = 2;
        }
        let inner_width = (TARGET_WIDTH - border_h - padding_h).max(10);
        let width_no_borders = TARGET_WIDTH - border_h;
        let height_no_borders = TARGET_HEIGHT - border_v;

        let target = self.current_target();

        let title = styles.panel_title.render("TACTICAL TARGET LOCK");
        let line1 = lipgloss::new_style()
            .width(inner_width)
            .align(&[CENTER])
            .render(&title);

        let line2 = match target {
            Some(t) if !t.name.is_empty() => styles.klingon.render(&format!("Target: {}", t.name)),
            Some(t) => styles
                .klingon
                .render(&format!("Target: KLINGON BATTLECRUISER #{}", t.klingon_id)),
            None => styles
                .gauge_label
                .render("Target: NO HOSTILE VESSELS DETECTED"),
        };

        let line3 = match target {
            Some(t) => styles.gauge_label.render(&format!(
                "Position: Sector [{}, {}]",
                t.coord[0], t.coord[1]
            )),
            None => styles.gauge_label.render("Position: Sector [--, --]"),
        };

        let line4 = match target {
            Some(t) => styles.gauge_label.render(&format!(
                "Range: {} sectors  Bearing: {} ({})",
                fmt_f(t.distance, 2),
                fmt_f(t.bearing, 2),
                direction_label(t.bearing)
            )),
            None => styles
                .gauge_label
                .render("Range: --             Bearing: --"),
        };

        let line5 = match target {
            Some(t) => styles.gauge_label.render(&format!(
                "Hit Prob: {}%     Target Shielding: ~{} units",
                fmt_f(t.hit_probability * 100.0, 0),
                fmt_f(t.power, 0)
            )),
            None => styles
                .gauge_label
                .render("Hit Prob: --        Target Shielding: --"),
        };

        let line6 = styles.grid_header.render(&"─".repeat(inner_width as usize));

        let line7 = styles.gauge_value.render(&format!(
            "Enterprise Weapons: [TORP: {}/10] [ENERGY: {}]",
            self.torpedo_count,
            fmt_f(self.ent_energy, 0)
        ));

        let line8 = if !self.warning_message.is_empty() {
            styles.condition_red.render(&self.warning_message)
        } else if self.inputting_phaser {
            styles.prompt.render("PHASER ENERGY> ")
                + &styles
                    .command_text
                    .render(&format!("{}█", self.phaser_input))
        } else {
            String::new()
        };

        let line9 = styles
            .gauge_label
            .render("[Enter] Fire Torpedo [P] Phaser [Tab] Next [Esc]");

        let content = [
            line1,
            line2,
            line3,
            line4,
            line5,
            line6,
            line7,
            line8,
            line9,
            String::new(),
        ]
        .join("\n");

        panel_style
            .width(width_no_borders)
            .height(height_no_borders)
            .render(&content)
    }
}

/// `directionLabel`: compass abbreviation for an SST bearing (1..9).
fn direction_label(bearing: f64) -> &'static str {
    let mut b = gomath::mod_(bearing - 0.5, 8.0);
    if b < 0.0 {
        b += 8.0;
    }
    match gomath::go_int(b) {
        1 => "NE",
        2 => "N",
        3 => "NW",
        4 => "W",
        5 => "SW",
        6 => "S",
        7 => "SE",
        _ => "E",
    }
}
