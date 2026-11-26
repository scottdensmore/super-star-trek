//! Port of `pkg/tui/components/optionsmodal` (modal.go): the configuration
//! and rules overlay (difficulty presets, rule toggles, color mode, audio
//! settings and a sound-test row that auditions through the shared player).

use sst_audio::{
    PlayerRef, SoundID, SOUND_CLOAK, SOUND_COMM_CHIME, SOUND_COMPUTER_BEEP, SOUND_DAMAGE,
    SOUND_DECLOAK, SOUND_DEFEAT, SOUND_DOCK, SOUND_EXPLOSION, SOUND_PHASER, SOUND_PLASMA_IMPACT,
    SOUND_PLASMA_LAUNCH, SOUND_POINT_DEFENSE, SOUND_RED_ALERT, SOUND_SHIELDS, SOUND_THOLIAN_WEB,
    SOUND_TORPEDO_LAUNCH, SOUND_VICTORY, SOUND_WARP, SOUND_WEB_BREACHED,
};
use sst_engine::rules::{
    default_rules_for_profile, DifficultyProfile, GameRules, SurveillanceMode,
};
use sst_tuikit::lipgloss;
use sst_tuikit::tea::{self, Cmd, KeyMsg, Msg};

use crate::config::AudioMode;
use crate::theme::{default_theme, ColorMode, Theme};

/// Go `Row`: a configurable option or action row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Row(pub i32);

#[allow(non_upper_case_globals)]
impl Row {
    pub const Profile: Row = Row(0);
    pub const Surveillance: Row = Row(1);
    pub const SensorDegradation: Row = Row(2);
    pub const RepairMultiplier: Row = Row(3);
    pub const KlingonCloak: Row = Row(4);
    pub const TimeMargin: Row = Row(5);
    pub const AnimSpeed: Row = Row(6);
    pub const ColorMode: Row = Row(7);
    pub const Audio: Row = Row(8);
    pub const AudioMode: Row = Row(9);
    pub const Volume: Row = Row(10);
    pub const SoundTest: Row = Row(11);
    pub const Done: Row = Row(12);
}

/// Go `NumRows`.
pub const NUM_ROWS: Row = Row(13);

/// Go `PlaySoundMsg`: a sound test audition was triggered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaySoundMsg {
    pub sound: SoundID,
}

fn default_test_sounds() -> Vec<SoundID> {
    vec![
        SOUND_PHASER,
        SOUND_TORPEDO_LAUNCH,
        SOUND_EXPLOSION,
        SOUND_RED_ALERT,
        SOUND_DOCK,
        SOUND_WARP,
        SOUND_DAMAGE,
        SOUND_SHIELDS,
        SOUND_VICTORY,
        SOUND_DEFEAT,
        SOUND_CLOAK,
        SOUND_DECLOAK,
        SOUND_PLASMA_LAUNCH,
        SOUND_PLASMA_IMPACT,
        SOUND_THOLIAN_WEB,
        SOUND_WEB_BREACHED,
        SOUND_POINT_DEFENSE,
        SOUND_COMM_CHIME,
        SOUND_COMPUTER_BEEP,
    ]
}

/// Go `Model`.
#[derive(Clone)]
pub struct Model {
    /// Go `Theme`.
    pub theme: Theme,
    rules: GameRules,
    color_mode: ColorMode,
    audio_enabled: bool,
    audio_mode: AudioMode,
    volume: i32,
    test_sound_idx: i32,
    test_sounds: Vec<SoundID>,
    player: Option<PlayerRef>,
    /// Go `SelectedRow`.
    pub selected_row: Row,
    /// Go `Closed`: set when the player leaves the modal.
    pub closed: bool,
    active: bool,
}

/// Go `New(th, rules)`; `None` is Go's nil theme.
pub fn new(th: impl Into<Option<Theme>>, rules: GameRules) -> Model {
    let th = th.into().unwrap_or_else(default_theme);
    Model {
        theme: th,
        rules,
        color_mode: th.color_mode(),
        audio_enabled: true,
        audio_mode: AudioMode::auto(),
        volume: 80,
        test_sound_idx: 0,
        test_sounds: default_test_sounds(),
        player: None,
        selected_row: Row::Profile,
        closed: false,
        active: true,
    }
}

impl Model {
    /// `AudioEnabled`.
    pub fn audio_enabled(&self) -> bool {
        self.audio_enabled
    }
    /// `SetAudioEnabled`.
    pub fn set_audio_enabled(&mut self, enabled: bool) {
        self.audio_enabled = enabled;
    }
    /// `Volume` (0-100).
    pub fn volume(&self) -> i32 {
        self.volume
    }
    /// `SetVolume`: clamps to [0, 100] and forwards to the player.
    pub fn set_volume(&mut self, vol: i32) {
        self.volume = vol.clamp(0, 100);
        if let Some(p) = &self.player {
            p.set_volume(self.volume);
        }
    }
    /// `AudioMode`.
    pub fn audio_mode(&self) -> AudioMode {
        self.audio_mode.clone()
    }
    /// `SetAudioMode`: unknown modes become `auto`.
    pub fn set_audio_mode(&mut self, mode: AudioMode) {
        self.audio_mode = match mode.as_str() {
            "auto" | "native" | "bell" | "off" => mode,
            _ => AudioMode::auto(),
        };
    }
    /// `SelectedSound` (empty when there are no test sounds).
    pub fn selected_sound(&self) -> SoundID {
        if self.test_sounds.is_empty() {
            return SoundID::new("");
        }
        let n = self.test_sounds.len() as i32;
        let mut idx = self.test_sound_idx % n;
        if idx < 0 {
            idx += n;
        }
        self.test_sounds[idx as usize].clone()
    }
    /// `SetSelectedSound`.
    pub fn set_selected_sound(&mut self, s: &SoundID) {
        if let Some(i) = self.test_sounds.iter().position(|x| x == s) {
            self.test_sound_idx = i as i32;
        }
    }
    /// `SetPlayer`.
    pub fn set_player(&mut self, p: impl Into<Option<PlayerRef>>) {
        self.player = p.into();
    }
    /// `Player`.
    pub fn player(&self) -> Option<PlayerRef> {
        self.player.clone()
    }
    /// `Rules`.
    pub fn rules(&self) -> GameRules {
        self.rules.clone()
    }
    /// `SetRules`.
    pub fn set_rules(&mut self, r: GameRules) {
        self.rules = r;
    }
    /// `ColorMode`.
    pub fn color_mode(&self) -> ColorMode {
        self.color_mode
    }
    /// `SetColorMode`.
    pub fn set_color_mode(&mut self, mode: ColorMode) {
        self.color_mode = mode;
    }
    /// `SetTheme`: also adopts the theme's color mode.
    pub fn set_theme(&mut self, th: impl Into<Option<Theme>>) {
        self.theme = th.into().unwrap_or_else(default_theme);
        self.color_mode = self.theme.color_mode();
    }
    /// `Active`.
    pub fn active(&self) -> bool {
        !self.closed && self.active
    }
    /// `SetActive`.
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
        if active {
            self.closed = false;
        }
    }

    /// `Update`: navigation and option cycling. Sound auditions play on the
    /// player immediately and return a [`PlaySoundMsg`] command.
    pub fn update(&mut self, msg: &Msg) -> Option<Cmd> {
        let k = msg.downcast_ref::<KeyMsg>()?;
        match k.string().as_str() {
            "up" | "k" => {
                if self.selected_row > Row(0) {
                    self.selected_row.0 -= 1;
                } else {
                    self.selected_row = Row(NUM_ROWS.0 - 1);
                }
            }
            "down" | "j" => {
                if self.selected_row.0 < NUM_ROWS.0 - 1 {
                    self.selected_row.0 += 1;
                } else {
                    self.selected_row = Row(0);
                }
            }
            "left" | "h" => self.cycle_option(-1),
            "right" | "l" => self.cycle_option(1),
            " " | "space" => {
                if self.selected_row == Row::SoundTest {
                    return self.audition_sound();
                }
                self.cycle_option(1);
            }
            "enter" => match self.selected_row {
                Row::Done => {
                    self.closed = true;
                    self.active = false;
                }
                Row::SoundTest => return self.audition_sound(),
                _ => self.cycle_option(1),
            },
            "esc" | "q" => {
                self.closed = true;
                self.active = false;
            }
            _ => {}
        }
        None
    }

    fn audition_sound(&self) -> Option<Cmd> {
        let sound = self.selected_sound();
        if sound.as_str().is_empty() {
            return None;
        }
        if let Some(p) = &self.player {
            p.set_volume(self.volume);
            p.play(sound.clone());
        }
        tea::cmd(move || PlaySoundMsg { sound })
    }

    fn cycle_option(&mut self, dir: i32) {
        fn pick<T: PartialEq + Clone>(list: &[T], cur: &T, dir: i32) -> T {
            let idx = list.iter().position(|x| x == cur).unwrap_or(0) as i32;
            let n = list.len() as i32;
            list[((idx + dir + n) % n) as usize].clone()
        }
        let profiles = [
            DifficultyProfile::casual(),
            DifficultyProfile::normal(),
            DifficultyProfile::hardcore(),
            DifficultyProfile::nightmare(),
            DifficultyProfile::expert(),
            DifficultyProfile::emeritus(),
            DifficultyProfile::custom(),
        ];
        let surv = [
            SurveillanceMode::full(),
            SurveillanceMode::classic(),
            SurveillanceMode::local(),
            SurveillanceMode::blackout(),
        ];
        let rep_mults = [0.75, 1.00, 1.50, 2.00];
        let time_margins = [1.25, 1.00, 0.80, 0.60];

        match self.selected_row {
            Row::Profile => {
                let p = pick(&profiles, &self.rules.profile, dir);
                if p != DifficultyProfile::custom() {
                    self.rules = default_rules_for_profile(&p);
                } else {
                    self.rules.profile = DifficultyProfile::custom();
                }
            }
            Row::Surveillance => {
                self.rules.surveillance = pick(&surv, &self.rules.surveillance, dir);
                self.rules.profile = DifficultyProfile::custom();
            }
            Row::SensorDegradation => {
                self.rules.sensor_degradation = !self.rules.sensor_degradation;
                self.rules.profile = DifficultyProfile::custom();
            }
            Row::RepairMultiplier => {
                self.rules.repair_multiplier = pick(&rep_mults, &self.rules.repair_multiplier, dir);
                self.rules.profile = DifficultyProfile::custom();
            }
            Row::KlingonCloak => {
                self.rules.klingon_cloak = !self.rules.klingon_cloak;
                self.rules.profile = DifficultyProfile::custom();
            }
            Row::TimeMargin => {
                self.rules.time_margin = pick(&time_margins, &self.rules.time_margin, dir);
                self.rules.profile = DifficultyProfile::custom();
            }
            Row::AnimSpeed => {
                self.rules.anim_speed = (self.rules.anim_speed + dir + 4) % 4;
            }
            Row::ColorMode => {
                let modes = [ColorMode::Auto, ColorMode::Dark, ColorMode::Light];
                self.color_mode = pick(&modes, &self.color_mode, dir);
            }
            Row::Audio => self.audio_enabled = !self.audio_enabled,
            Row::AudioMode => {
                let modes = [
                    AudioMode::auto(),
                    AudioMode::native(),
                    AudioMode::bell(),
                    AudioMode::off(),
                ];
                let idx = modes
                    .iter()
                    .position(|m| *m == self.audio_mode)
                    .unwrap_or(0) as i32;
                let mut ni = (idx + dir) % modes.len() as i32;
                if ni < 0 {
                    ni += modes.len() as i32;
                }
                self.audio_mode = modes[ni as usize].clone();
            }
            Row::Volume => {
                let v = (self.volume + dir * 10).clamp(0, 100);
                self.volume = v;
                if let Some(p) = &self.player {
                    p.set_volume(v);
                }
            }
            Row::SoundTest if !self.test_sounds.is_empty() => {
                let n = self.test_sounds.len() as i32;
                let mut ni = (self.test_sound_idx + dir) % n;
                if ni < 0 {
                    ni += n;
                }
                self.test_sound_idx = ni;
            }
            _ => {}
        }
    }

    /// `View`.
    pub fn view(&self) -> String {
        let styles = self.theme.styles();
        let box_style = styles.panel.clone().padding(&[1, 2]).width(66);
        let title_style = styles
            .panel_title
            .clone()
            .bold(true)
            .align(&[lipgloss::CENTER]);

        let mut rows: Vec<String> = vec![
            title_style.render("⚙ STARFLEET CONFIGURATION & RULES"),
            String::new(),
        ];
        let render_row = |r: Row, label: &str, val: &str| -> String {
            let (prefix, style) = if self.selected_row == r {
                ("▶ ", &styles.gauge_value)
            } else {
                ("  ", &styles.log_text)
            };
            format!(
                "{}{}",
                prefix,
                style.render(&format!("{:<30} ◀ {} ▶", label, val))
            )
        };

        let r = &self.rules;
        rows.push(render_row(
            Row::Profile,
            "Difficulty Profile",
            &r.profile.as_str().to_uppercase(),
        ));
        rows.push(render_row(
            Row::Surveillance,
            "Starbase Surveillance",
            &r.surveillance.as_str().to_uppercase(),
        ));
        let en = |b: bool| if b { "ENABLED" } else { "DISABLED" };
        rows.push(render_row(
            Row::SensorDegradation,
            "Sensor Degradation",
            en(r.sensor_degradation),
        ));
        rows.push(render_row(
            Row::RepairMultiplier,
            "Repair Multiplier",
            &format!("{:.2}x", r.repair_multiplier),
        ));
        rows.push(render_row(
            Row::KlingonCloak,
            "Klingon Cloaking",
            en(r.klingon_cloak),
        ));
        rows.push(render_row(
            Row::TimeMargin,
            "Stardate Time Margin",
            &format!("{:.0}%", r.time_margin * 100.0),
        ));
        let anim = match r.anim_speed {
            0 => "OFF",
            1 => "FAST",
            3 => "CINEMATIC",
            _ => "NORMAL",
        };
        rows.push(render_row(Row::AnimSpeed, "Combat Animations", anim));
        rows.push(render_row(
            Row::ColorMode,
            "Color Mode",
            &self.color_mode.as_str().to_uppercase(),
        ));
        rows.push(render_row(
            Row::Audio,
            "Sound FX",
            if self.audio_enabled {
                "[ENABLED]"
            } else {
                "[DISABLED]"
            },
        ));
        let mode_str = match self.audio_mode.as_str() {
            "auto" => "AUTO".to_string(),
            "native" => "NATIVE OS".to_string(),
            "bell" => "TERMINAL BELL".to_string(),
            "off" => "OFF".to_string(),
            other => other.to_uppercase(),
        };
        rows.push(render_row(Row::AudioMode, "Audio Mode", &mode_str));
        let filled = self.volume / 10;
        let vol_bar = format!(
            "[{}{}] {}%",
            "■".repeat(filled.max(0) as usize),
            "·".repeat((10 - filled).max(0) as usize),
            self.volume
        );
        rows.push(render_row(Row::Volume, "Audio Volume", &vol_bar));
        let sound_name = self
            .selected_sound()
            .as_str()
            .replace('_', " ")
            .to_uppercase();
        rows.push(render_row(Row::SoundTest, "Sound Test", &sound_name));

        let (prefix, done_style) = if self.selected_row == Row::Done {
            ("▶ ", &styles.gauge_value)
        } else {
            ("  ", &styles.log_text)
        };
        rows.push(String::new());
        rows.push(format!(
            "{}{}",
            prefix,
            done_style.render("[ Done / Resume Mission ]")
        ));
        rows.push(String::new());
        rows.push(
            styles
                .log_text
                .clone()
                .italic(true)
                .render("↑/↓: Navigate • ←/→/Space: Change • Esc/q: Close"),
        );
        box_style.render(&rows.join("\n"))
    }
}

#[cfg(test)]
mod parity_tests;
#[cfg(test)]
mod tests;
