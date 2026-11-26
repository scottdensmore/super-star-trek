//! Telemetry and status panel (port of
//! `pkg/tui/components/statuspanel/statuspanel.go`).
//!
//! Go's value-receiver `Render`/`View` mutate a copy of the model; here they
//! take `&self` and work on a clone, so the panel's own state is untouched
//! exactly like in Go.

use sst_engine::gofmt::fmt_f;
use sst_engine::gomath;
use sst_engine::{
    calculate_web_containment, default_rules_for_profile, detect_crossfire_bracket, ConditionType,
    DeviceID, DifficultyProfile, Enterprise, GameRules, GameState, ANIM_SPEED_OFF,
};
use sst_tuikit::lipgloss::Style;

use crate::anim;
use crate::theme::{self, Styles, Theme};

#[cfg(test)]
mod parity_tests;
#[cfg(test)]
mod tests;

/// Telemetry and configuration needed to render the panel (Go `PanelData`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PanelData {
    pub enterprise: Enterprise,
    pub devices: [f64; 8],
    pub time_remaining: f64,
    pub is_docked: bool,
    pub galaxy_chart: [[i32; 9]; 9],
    pub stardate: f64,
    pub rules: GameRules,
    pub sound_enabled: bool,
    pub audio_volume: i32,
    pub audio_muted: bool,
    pub hazard_alerts: Vec<String>,
}

/// `SamplePanelData`: default data for tests and previews.
pub fn sample_panel_data() -> PanelData {
    PanelData {
        enterprise: Enterprise {
            quad: [4, 4],
            sector: [2, 3],
            energy: 5000.0,
            shields: 1000.0,
            torpedoes: 10,
            condition: ConditionType::Green,
            ..Default::default()
        },
        devices: [0.0; 8],
        time_remaining: 30.0,
        is_docked: false,
        galaxy_chart: [[0; 9]; 9],
        stardate: 2800.0,
        rules: default_rules_for_profile(&DifficultyProfile::normal()),
        sound_enabled: true,
        audio_volume: 80,
        audio_muted: false,
        hazard_alerts: Vec::new(),
    }
}

/// The status panel component (Go `statuspanel.Model`).
#[derive(Clone, Default)]
pub struct Model {
    theme: Option<Theme>,
    // Stored by `New` but never read (as in Go).
    #[allow(dead_code)]
    width: i32,
    #[allow(dead_code)]
    height: i32,
    enterprise: Enterprise,
    time_remaining: f64,
    is_docked: bool,
    galaxy_chart: [[i32; 9]; 9],
    stardate: f64,
    rules: GameRules,
    has_state: bool,
    red_alert_cycle: i32,
    sound_enabled: bool,
    audio_volume: i32,
    audio_muted: bool,
    hazard_alerts: Vec<String>,
}

struct PanelStyles {
    base: Styles,
    text_warn: Style,
    text_muted: Style,
    normal: Style,
}

fn get_panel_styles(th: Option<Theme>) -> PanelStyles {
    let base = th.unwrap_or_else(theme::default_theme).styles();
    PanelStyles {
        text_warn: base.subsystem_damaged.clone(),
        text_muted: base.empty.clone(),
        normal: base.gauge_label.clone(),
        base,
    }
}

/// `New(th, dims...)`: `dims` are the optional width and height.
pub fn new(th: Option<Theme>, dims: &[i32]) -> Model {
    let sample = sample_panel_data();
    Model {
        theme: Some(th.unwrap_or_else(theme::default_theme)),
        width: dims.first().copied().unwrap_or(0),
        height: dims.get(1).copied().unwrap_or(0),
        enterprise: sample.enterprise,
        time_remaining: sample.time_remaining,
        is_docked: sample.is_docked,
        galaxy_chart: sample.galaxy_chart,
        stardate: sample.stardate,
        rules: sample.rules,
        has_state: true,
        red_alert_cycle: 0,
        sound_enabled: true,
        audio_volume: 80,
        audio_muted: false,
        hazard_alerts: Vec::new(),
    }
}

const DEVICES_COL1: [(DeviceID, &str); 4] = [
    (DeviceID::Warp, "Warp"),
    (DeviceID::SRSensors, "SRS"),
    (DeviceID::LRSensors, "LRS"),
    (DeviceID::Phasers, "Phasers"),
];

const DEVICES_COL2: [(DeviceID, &str); 4] = [
    (DeviceID::PhotonTubes, "Tubes"),
    (DeviceID::DamageControl, "Damage Control"),
    (DeviceID::Shields, "Shields"),
    (DeviceID::Computer, "Computer"),
];

impl Model {
    /// `SetAudioTelemetry`.
    pub fn set_audio_telemetry(&mut self, vol: i32, muted: bool) {
        self.audio_volume = vol;
        self.audio_muted = muted;
        self.sound_enabled = !muted && vol > 0;
    }

    /// `AudioVolume`.
    pub fn audio_volume(&self) -> i32 {
        self.audio_volume
    }

    /// `AudioMuted`.
    pub fn audio_muted(&self) -> bool {
        self.audio_muted
    }

    /// `SoundEnabled`.
    pub fn sound_enabled(&self) -> bool {
        self.sound_enabled
    }

    /// `SetSoundEnabled`.
    pub fn set_sound_enabled(&mut self, enabled: bool) {
        self.sound_enabled = enabled;
        self.audio_muted = !enabled;
    }

    /// `SetRedAlertCycle`.
    pub fn set_red_alert_cycle(&mut self, cycle: i32) {
        self.red_alert_cycle = cycle;
    }

    /// `SetHazardAlerts`.
    pub fn set_hazard_alerts(&mut self, alerts: Vec<String>) {
        self.hazard_alerts = alerts;
    }

    /// `SetState(ent, timeRemaining, klingonsLeft, starbasesLeft, isDocked,
    /// chart, rules...)`; `rules` is Go's optional variadic argument.
    #[allow(clippy::too_many_arguments)]
    pub fn set_state(
        &mut self,
        ent: Enterprise,
        time_remaining: f64,
        _klingons_left: i32,
        _starbases_left: i32,
        is_docked: bool,
        chart: [[i32; 9]; 9],
        rules: Option<GameRules>,
    ) {
        self.enterprise = ent;
        self.time_remaining = time_remaining;
        self.is_docked = is_docked;
        self.galaxy_chart = chart;
        if let Some(r) = rules {
            self.rules = r;
        } else if self.rules.profile.as_str().is_empty() {
            self.rules = default_rules_for_profile(&DifficultyProfile::normal());
        }
        self.has_state = true;
    }

    /// `SetTheme` (nil falls back to the default theme).
    pub fn set_theme(&mut self, th: Option<Theme>) {
        self.theme = Some(th.unwrap_or_else(theme::default_theme));
    }

    /// `Theme`.
    pub fn theme(&self) -> Theme {
        self.theme.unwrap_or_else(theme::default_theme)
    }

    fn render_radar(&self, styles: &PanelStyles) -> (String, [String; 3]) {
        let qr = self.enterprise.quad[0];
        let qc = self.enterprise.quad[1];

        let lrs_sensor = self.enterprise.devices[DeviceID::LRSensors.as_i32() as usize];
        let mut sensor_degradation = self.rules.sensor_degradation;
        if self.rules.profile.as_str().is_empty() && !self.rules.sensor_degradation {
            sensor_degradation = true;
        }

        let mut is_offline = false;
        let mut is_degraded = false;
        if !self.is_docked {
            if lrs_sensor >= 2.0 {
                is_offline = true;
            } else if sensor_degradation && lrs_sensor > 0.0 {
                is_degraded = true;
            }
        }

        let mut radar_header = if is_offline {
            styles.text_warn.render("RADAR (±1) [LRS OFFLINE]:")
        } else if is_degraded {
            styles.text_warn.render("RADAR (±1) [LRS DEGRADED]:")
        } else {
            styles.text_muted.render("RADAR (±1) [K-B-S]:")
        };
        if !is_offline && !is_degraded {
            radar_header.push_str(&format!(
                "  {:<4} {:<4} {:<4}",
                radar_header_label(qc - 1),
                radar_header_label(qc),
                radar_header_label(qc + 1)
            ));
        }

        let mut radar_rows: [String; 3] = Default::default();
        for dr in -1..=1 {
            let r = qr + dr;
            let mut cells: [String; 3] = Default::default();
            for dc in -1..=1 {
                let c = qc + dc;
                let idx = (dc + 1) as usize;
                cells[idx] = if !(1..=8).contains(&r) || !(1..=8).contains(&c) {
                    styles.text_muted.render(" *** ")
                } else {
                    let val = self.galaxy_chart[r as usize][c as usize];
                    if dr == 0 && dc == 0 {
                        styles.base.enterprise.render(&format!("<{val:03}>"))
                    } else if is_offline {
                        styles.text_warn.render(" ??? ")
                    } else if is_degraded {
                        styles.text_warn.render("  ?  ")
                    } else {
                        styles.normal.render(&format!(" {val:03} "))
                    }
                };
            }
            radar_rows[(dr + 1) as usize] = format!(
                "  {}  {}{}{}",
                radar_header_label(r),
                cells[0],
                cells[1],
                cells[2]
            );
        }
        (radar_header, radar_rows)
    }

    /// `Render(data)`: renders the panel for `data` (on a copy of the model).
    pub fn render(&self, data: &PanelData) -> String {
        let mut devices = data.devices;
        if devices.iter().all(|&v| v == 0.0) {
            devices = data.enterprise.devices;
        }
        let mut ent = data.enterprise.clone();
        ent.devices = devices;

        let mut m = self.clone();
        m.enterprise = ent;
        m.time_remaining = data.time_remaining;
        m.is_docked = data.is_docked;
        m.galaxy_chart = data.galaxy_chart;
        m.stardate = data.stardate;
        m.rules = data.rules.clone();
        m.sound_enabled = data.sound_enabled;
        if data.audio_volume >= 0 || data.audio_muted {
            m.audio_volume = data.audio_volume;
            m.audio_muted = data.audio_muted;
        }
        m.hazard_alerts = data.hazard_alerts.clone();
        if m.rules.profile.as_str().is_empty() && !m.rules.sensor_degradation {
            m.rules = default_rules_for_profile(&DifficultyProfile::normal());
        }
        m.has_state = true;
        m.render_internal()
    }

    /// `View(gs...)`: with a game, renders its telemetry plus hazard alerts;
    /// otherwise renders the stored state.
    pub fn view(&self, g: Option<&GameState>) -> String {
        if let Some(g) = g {
            let th = self.theme();
            let mut alerts = Vec::new();
            if !g.current_quad.plasma_torpedoes.is_empty() {
                alerts.push(th.alert_red().render("⚠️ INCOMING PLASMA TORPEDO TRACKING"));
            }
            if !g.current_quad.web_segments.is_empty() {
                let containment = calculate_web_containment(Some(g));
                alerts.push(th.alert_yellow().render(&format!(
                    "⚠️ THOLIAN WEB ENCLOSURE: {}%",
                    fmt_f(containment, 0)
                )));
            }
            if detect_crossfire_bracket(Some(g)) {
                alerts.push(
                    th.alert_red()
                        .render("⚠️ CROSSFIRE BRACKET ACTIVE (+35% DMG)"),
                );
            }
            return self.render(&PanelData {
                enterprise: g.enterprise.clone(),
                devices: g.enterprise.devices,
                time_remaining: g.time_remaining,
                is_docked: g.enterprise.condition == ConditionType::Docked,
                galaxy_chart: g.galaxy_chart,
                stardate: g.stardate,
                rules: g.rules.clone(),
                sound_enabled: self.sound_enabled,
                audio_volume: self.audio_volume,
                audio_muted: self.audio_muted,
                hazard_alerts: alerts,
            });
        }
        if !self.has_state {
            let styles = get_panel_styles(self.theme);
            return styles
                .base
                .panel
                .render(&styles.base.gauge_label.render("NO TELEMETRY AVAILABLE"));
        }
        self.render_internal()
    }

    fn render_internal(&self) -> String {
        let styles = get_panel_styles(self.theme);
        let s = &styles.base;

        let (cond_str, cond_style) = match self.enterprise.condition {
            ConditionType::Yellow => ("CONDITION YELLOW", s.condition_yellow.clone()),
            ConditionType::Red => (
                "CONDITION RED",
                if self.rules.anim_speed != ANIM_SPEED_OFF {
                    anim::red_alert_badge_style(self.red_alert_cycle)
                } else {
                    s.condition_red.clone()
                },
            ),
            ConditionType::Docked => ("CONDITION DOCKED", s.condition_docked.clone()),
            _ => ("CONDITION GREEN", s.condition_green.clone()),
        };
        let cond_loc_line = format!(
            "{}   {}{}",
            cond_style.render(cond_str),
            s.gauge_label.render("LOC: "),
            s.prompt.render(&format!(
                "Q[{},{}] S[{},{}]",
                self.enterprise.quad[0],
                self.enterprise.quad[1],
                self.enterprise.sector[0],
                self.enterprise.sector[1]
            ))
        );

        let stardate_time_line = format!(
            "{}{}   {}{}",
            s.gauge_label.render("Stardate: "),
            s.gauge_value.render(&fmt_f(self.stardate, 1)),
            s.gauge_label.render("Time Remaining: "),
            s.gauge_value.render(&fmt_f(self.time_remaining, 1)),
        );

        let energy_line = render_progress_bar("Energy", self.enterprise.energy, 5000.0, s);
        let shields_line = render_progress_bar("Shields", self.enterprise.shields, 2500.0, s);

        let is_muted = self.audio_muted || !self.sound_enabled || self.audio_volume == 0;
        let snd_badge = self.theme().format_audio_badge(self.audio_volume, is_muted);
        let torp_line = format!(
            "{}{}   {}",
            s.gauge_label.render("Torpedoes: "),
            s.gauge_value
                .render(&format!("[TORP: {}/10]", self.enterprise.torpedoes)),
            snd_badge
        );

        let dev_header = s.panel_title.render("SUBSYSTEM REPAIR STATUS:");
        let status = |id: DeviceID| {
            let v = self.enterprise.devices[id.as_i32() as usize];
            if v > 0.0 {
                s.subsystem_damaged.render(&format!("{:>4}", fmt_f(v, 1)))
            } else {
                s.subsystem_normal.render("  OK")
            }
        };
        let dev_rows: Vec<String> = (0..4)
            .map(|i| {
                let (id1, n1) = DEVICES_COL1[i];
                let (id2, n2) = DEVICES_COL2[i];
                format!(
                    "{:<8} {}  {:<16} {}",
                    format!("{n1}:"),
                    status(id1),
                    format!("{n2}:"),
                    status(id2)
                )
            })
            .collect();

        let (radar_header, radar_rows) = self.render_radar(&styles);

        let mut b = String::with_capacity(512);
        b.push_str(&cond_loc_line);
        b.push('\n');
        b.push_str(&stardate_time_line);
        b.push('\n');
        b.push_str(&energy_line);
        b.push('\n');
        b.push_str(&shields_line);
        b.push('\n');
        b.push_str(&torp_line);
        b.push('\n');
        b.push_str(&dev_header);
        for row in &dev_rows {
            b.push('\n');
            b.push_str(row);
        }
        b.push('\n');
        b.push_str(&radar_header);
        for row in &radar_rows {
            b.push('\n');
            b.push_str(row);
        }
        for alert in &self.hazard_alerts {
            b.push('\n');
            b.push_str(alert);
        }

        s.panel.render(&b)
    }
}

/// `renderProgressBar`: label, segmented bar and numeric readout.
fn render_progress_bar(label: &str, current: f64, max_val: f64, styles: &Styles) -> String {
    let mut ratio = 0.0;
    if max_val > 0.0 {
        ratio = current / max_val;
    }
    // Like Go's `if ratio < 0 {..} else if ratio > 1 {..}`, NaN passes through.
    ratio = ratio.clamp(0.0, 1.0);

    const BAR_WIDTH: i32 = 10;
    // Go `int(NaN)` is MinInt64 on amd64; the clamp then yields 0.
    let filled =
        gomath::go_int(gomath::round(ratio * BAR_WIDTH as f64)).clamp(0, BAR_WIDTH as i64) as i32;
    let empty = BAR_WIDTH - filled;

    let filled_str = styles
        .progress_bar_filled
        .render(&"█".repeat(filled as usize));
    let empty_str = styles
        .progress_bar_empty
        .render(&"░".repeat(empty as usize));
    let bar = format!(
        "{}{}{}{}",
        styles.gauge_label.render("["),
        filled_str,
        empty_str,
        styles.gauge_label.render("]")
    );

    let lbl = styles
        .gauge_label
        .render(&format!("{:<9}", format!("{label}:")));
    let val = styles
        .gauge_value
        .render(&format!("{:>4}/{}", fmt_f(current, 0), fmt_f(max_val, 0)));
    format!("{lbl} {bar} {val}")
}

/// `radarColHeader` / `radarRowHeader`.
fn radar_header_label(v: i32) -> String {
    if !(1..=8).contains(&v) {
        " ".into()
    } else {
        v.to_string()
    }
}
