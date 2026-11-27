//! Root TUI model (port of `pkg/tui/model.go`).
//!
//! Go's `Model` is a value type: `Update` works on a copy and returns it.
//! The Rust [`Model`] is `Clone`; pointer fields (`*GameState`, `*TourState`,
//! the audio player and dispatcher, the active animation and the clock) are
//! `Rc` handles, so clones share them exactly like Go copies share pointers.

use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use sst_audio::{Dispatcher, NullPlayer, PlayerRef};
use sst_engine::gotime::{Clock, GoTime, SystemClock};
use sst_engine::{
    default_rules_for_profile, persist, ConditionType, Coord, DifficultyProfile, GameRef,
    GameRules, TourRef, ANIM_SPEED_OFF,
};
use sst_tuikit::tea::{self, Cmd, Msg};

use crate::anim::AnimationRef;
use crate::components::{
    commandbar, commandpalette, damageschematic, drydockmodal, galacticchart, halloffame, manual,
    optionsmodal, savebrowser, scenariomodal, sectorgrid, statuspanel, targetlock,
};
use crate::config;
use crate::theme::{self, ColorMode, Theme};

/// Go `ModalType`: the currently active floating modal overlay.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ModalType(pub i32);

#[allow(non_upper_case_globals)]
impl ModalType {
    pub const None: ModalType = ModalType(0);
    pub const TargetLock: ModalType = ModalType(1);
    pub const CommandPalette: ModalType = ModalType(2);
    pub const SaveBrowser: ModalType = ModalType(3);
    pub const GalacticChart: ModalType = ModalType(4);
    pub const DamageSchematic: ModalType = ModalType(5);
    pub const HallOfFame: ModalType = ModalType(6);
    pub const Manual: ModalType = ModalType(7);
    pub const Scenario: ModalType = ModalType(8);
}

/// Go `ModalNone` and friends.
pub const MODAL_NONE: ModalType = ModalType::None;
pub const MODAL_TARGET_LOCK: ModalType = ModalType::TargetLock;
pub const MODAL_COMMAND_PALETTE: ModalType = ModalType::CommandPalette;
pub const MODAL_SAVE_BROWSER: ModalType = ModalType::SaveBrowser;
pub const MODAL_GALACTIC_CHART: ModalType = ModalType::GalacticChart;
pub const MODAL_DAMAGE_SCHEMATIC: ModalType = ModalType::DamageSchematic;
pub const MODAL_HALL_OF_FAME: ModalType = ModalType::HallOfFame;
pub const MODAL_MANUAL: ModalType = ModalType::Manual;
pub const MODAL_SCENARIO: ModalType = ModalType::Scenario;

/// The root Bubble Tea model (Go `tui.Model`).
#[derive(Clone)]
pub struct Model {
    pub game: Option<GameRef>,
    pub theme: Option<Theme>,
    pub width: i32,
    pub height: i32,
    pub grid: sectorgrid::Model,
    pub status: statuspanel::Model,
    pub command_bar: commandbar::Model,
    pub selected_sector: Coord,

    pub active_modal: ModalType,
    pub target_lock: targetlock::Model,
    pub command_palette: commandpalette::Model,
    pub save_browser: savebrowser::Model,
    pub galactic_chart: galacticchart::Model,
    pub damage_schematic: damageschematic::Model,
    pub hall_of_fame: halloffame::Model,
    pub manual: manual::Model,
    pub scenario_modal: scenariomodal::Model,
    pub options_modal: optionsmodal::Model,
    pub show_options: bool,
    pub tour: Option<TourRef>,
    pub drydock: drydockmodal::Model,
    pub player_callsign: String,
    pub audio_player: Option<PlayerRef>,
    pub audio_dispatcher: Option<Rc<Dispatcher>>,
    pub active_anim: Option<AnimationRef>,
    pub anim_id: i32,
    pub last_click_time: GoTime,
    pub last_click_coord: Coord,
    pub last_dark_bg: bool,
    pub red_alert_cycle: i32,
    pub red_alert_active: bool,
    /// Source of `time.Now()` (double-click timing, scenario seeds, record dates).
    pub clock: Rc<dyn Clock>,
    /// Raised by the audio player's terminal-bell fallback (see
    /// [`visual_bell_callback`]); the next update flashes the command bar.
    pub bell_pending: Arc<AtomicBool>,
}

impl Model {
    /// Go `NewModel`.
    pub fn new(g: Option<GameRef>, th: impl Into<Option<Theme>>) -> Model {
        new_model(g, th)
    }

    /// Go `SetSoundEnabled`: mute state across player, status telemetry and options modal.
    pub fn set_sound_enabled(&mut self, enabled: bool) {
        if let Some(p) = &self.audio_player {
            p.set_muted(!enabled);
        }
        self.status.set_sound_enabled(enabled);
        self.options_modal.set_audio_enabled(enabled);
    }

    /// Go `SoundEnabled`.
    pub fn sound_enabled(&self) -> bool {
        if let Some(p) = &self.audio_player {
            return !p.is_muted();
        }
        self.status.sound_enabled()
    }

    /// Go `syncChildComponents`.
    pub(crate) fn sync_child_components(&mut self) {
        self.status.set_red_alert_cycle(self.red_alert_cycle);
    }

    /// Injects the clock used for `time.Now()` (also used by the hall of fame).
    pub fn set_clock(&mut self, clock: Rc<dyn Clock>) {
        self.hall_of_fame.set_clock(clock.clone());
        self.clock = clock;
    }

    /// Go `Init` (value receiver: the focus mutation happens on a copy).
    pub fn init(&self) -> Option<Cmd> {
        let mut cb = self.command_bar.clone();
        let cmd = cb.focus();
        if let Some(g) = &self.game {
            let g = g.borrow();
            if g.enterprise.condition == ConditionType::Red && g.rules.anim_speed != ANIM_SPEED_OFF
            {
                return tea::batch(vec![cmd, red_alert_pulse_cmd()]);
            }
        }
        cmd
    }

    /// Go `UpdateModel`: returns the updated copy, leaving `self` untouched.
    pub fn update_model(&self, msg: Msg) -> (Model, Option<Cmd>) {
        let mut m = self.clone();
        let cmd = m.update_in_place(msg);
        (m, cmd)
    }
}

impl tea::Model for Model {
    fn init(&self) -> Option<Cmd> {
        Model::init(self)
    }

    fn update(&mut self, msg: Msg) -> Option<Cmd> {
        self.update_in_place(msg)
    }

    fn view(&self) -> String {
        Model::view(self)
    }
}

/// Go `RedAlertPulseMsg`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RedAlertPulseMsg;

/// Go `VisualBellExpireMsg`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VisualBellExpireMsg;

/// Go `redAlertPulseCmd` (600ms tick).
pub fn red_alert_pulse_cmd() -> Option<Cmd> {
    tea::tick(std::time::Duration::from_millis(600), |_| RedAlertPulseMsg)
}

/// Go `visualBellCmd` (150ms tick).
pub fn visual_bell_cmd() -> Option<Cmd> {
    tea::tick(std::time::Duration::from_millis(150), |_| {
        VisualBellExpireMsg
    })
}

/// The visual-bell callback handed to the audio player: it may run on the
/// audio worker thread, so it only raises `pending`, which the model turns
/// into a command-bar flash on its next update.
pub fn visual_bell_callback(pending: &Arc<AtomicBool>) -> sst_audio::VisualBell {
    let pending = pending.clone();
    Arc::new(move || pending.store(true, Ordering::SeqCst))
}

fn make_player(bell_pending: &Arc<AtomicBool>) -> PlayerRef {
    if sst_audio::testing() {
        return Rc::new(NullPlayer::new());
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        // When no OS audio player works, the terminal-bell fallback rings the
        // bell and calls this. (Go's callback captured a local copy of the
        // command bar, so the flash never reached the rendered bar.)
        Rc::new(sst_audio::NativeOSPlayer::new(
            sst_audio::BellWriter::Stdout,
            Some(visual_bell_callback(bell_pending)),
        ))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = bell_pending;
        Rc::new(NullPlayer::new())
    }
}

/// Go `NewModel(g, th)`: a nil theme uses `theme.DefaultTheme()`.
pub fn new_model(g: Option<GameRef>, th: impl Into<Option<Theme>>) -> Model {
    let th: Theme = th.into().unwrap_or_else(theme::default_theme);

    let mut cb = commandbar::new(Some(th));
    cb.add_message("Super Star Trek — Tactical Display Online");
    cb.add_message("Type 'help' for commands, 'quit' to exit, F2 to cycle themes");

    let rules: GameRules = match &g {
        Some(g) => g.borrow().rules.clone(),
        None => default_rules_for_profile(&DifficultyProfile::normal()),
    };

    let bell_pending = Arc::new(AtomicBool::new(false));
    let player = make_player(&bell_pending);
    player.set_volume(80);
    let cfg = config::load_config();
    if let Ok(c) = &cfg {
        if let Ok(path) = config::config_path() {
            if std::fs::metadata(&path).is_ok() {
                player.set_volume(c.volume);
                player.set_muted(c.muted);
            }
        }
    }
    let dispatcher = Rc::new(Dispatcher::new(Some(player.clone())));

    let mut opt_modal = optionsmodal::new(th, rules);
    opt_modal.set_volume(player.volume());
    opt_modal.set_audio_enabled(!player.is_muted());
    opt_modal.set_player(player.clone());
    if let Ok(c) = &cfg {
        if !c.audio_mode.as_str().is_empty() {
            opt_modal.set_audio_mode(c.audio_mode.clone());
        }
    }

    let mut st = statuspanel::new(Some(th), &[]);
    st.set_audio_telemetry(player.volume(), player.is_muted());

    let last_dark_bg = match th.color_mode() {
        ColorMode::Light => false,
        ColorMode::Dark => true,
        _ => theme::detect::detect_dark_background(),
    };

    let clock: Rc<dyn Clock> = Rc::new(SystemClock);
    let mut hall = halloffame::new(th, 66, 18, &persist::default_leaderboard_path());
    hall.set_clock(clock.clone());

    let mut m = Model {
        game: g,
        theme: Some(th),
        width: 80,
        height: 24,
        grid: sectorgrid::new(Some(th)),
        status: st,
        command_bar: cb,
        selected_sector: [0, 0],
        active_modal: MODAL_NONE,
        target_lock: targetlock::new(Some(th)),
        command_palette: commandpalette::new(th, 56, 16),
        save_browser: savebrowser::new(th, "."),
        galactic_chart: galacticchart::new(Some(th), 64, 18),
        damage_schematic: damageschematic::new(Some(th), 66),
        hall_of_fame: hall,
        manual: manual::new(th, 66, 18),
        scenario_modal: scenariomodal::new_model(th),
        options_modal: opt_modal,
        show_options: false,
        tour: None,
        drydock: drydockmodal::new(None, th),
        player_callsign: "Enterprise".into(),
        audio_player: Some(player),
        audio_dispatcher: Some(dispatcher),
        active_anim: None,
        anim_id: 0,
        last_click_time: GoTime::default(),
        last_click_coord: [0, 0],
        last_dark_bg,
        red_alert_cycle: 0,
        red_alert_active: false,
        clock,
        bell_pending,
    };
    m.sync_child_components();
    m
}

/// Go `NewModelWithTour(tour, th)`: a nil tour falls back to `NewModel(nil, th)`.
pub fn new_model_with_tour(tour: Option<TourRef>, th: impl Into<Option<Theme>>) -> Model {
    let th: Theme = th.into().unwrap_or_else(theme::default_theme);
    let Some(tour) = tour else {
        return new_model(None, th);
    };

    let existing = tour.borrow().current_game_state.clone();
    let game = match existing {
        Some(g) => Some(g),
        None => tour.borrow_mut().start_current_sector(None).ok(),
    };
    if let Some(g) = &game {
        let mut gs = g.borrow_mut();
        if gs.current_quad.enemies.is_empty() {
            let (q, s) = (gs.enterprise.quad, gs.enterprise.sector);
            gs.populate_quadrant(q, s);
        }
    }

    let mut m = new_model(game, th);
    m.tour = Some(tour.clone());
    m.drydock = drydockmodal::new(Some(tour), th);
    m.player_callsign = "Enterprise".into();
    m
}
