//! Root model message handling (port of `pkg/tui/update.go`).
//!
//! Go's `Update` has a value receiver and returns the modified copy; here
//! [`Model::update_in_place`] mutates `self` (the runtime discards the old
//! value anyway) and [`Model::update_model`] reproduces the copy semantics.

use std::cell::RefCell;
use std::rc::Rc;

use sst_engine::gofmt::{fmt_f, quote};
use sst_engine::parser::{fields, trim_space};
use sst_engine::{
    compute_score, execute_klingon_turn_if_active, get_scenario, gomath, gopath, persist, Action,
    ActionDock, ActionFirePhasers, ActionFireTorpedo, ActionLRScan, ActionMove, ActionPhaserDirect,
    ConditionType, Coord, DeviceID, EntityType, Event, EventGameOver, FactionType, GameOverReason,
    Leaderboard, ANIM_SPEED_CINEMATIC, ANIM_SPEED_FAST, ANIM_SPEED_OFF,
};
use sst_tuikit::tea::{
    self, Cmd, KeyMsg, MouseMsg, Msg, WindowSizeMsg, KEY_CTRL_C, KEY_CTRL_D, KEY_CTRL_G,
    KEY_CTRL_H, KEY_CTRL_O, KEY_CTRL_P, KEY_CTRL_S, KEY_CTRL_T, KEY_ESC, KEY_F1, KEY_F2,
    MOUSE_ACTION_MOTION, MOUSE_ACTION_RELEASE, MOUSE_BUTTON_LEFT,
};

use crate::anim::{self, AnimationRef};
use crate::components::{
    commandbar, commandpalette, damageschematic, drydockmodal, galacticchart, halloffame, manual,
    savebrowser, scenariomodal, targetlock,
};
use crate::config;
use crate::model::*;
use crate::theme::{self, ColorMode, Theme};

/// Go `parser.ParsedCommand` alias (pkg/tui/parser.go).
pub use sst_engine::parser::ParsedCommand;

/// Go `tui.ParseCommand` (thin wrapper around the parser package).
pub fn parse_command(input: &str) -> ParsedCommand {
    sst_engine::parser::parse_command(input)
}

const DOUBLE_CLICK_NANOS: i64 = 400_000_000;

fn is_key(k: &KeyMsg, typ: tea::KeyType, s: &str) -> bool {
    k.typ == typ || k.string() == s
}

fn is_ctrl_t(k: &KeyMsg) -> bool {
    k.typ == KEY_CTRL_T || k.string() == "ctrl+t" || k.string() == "shift+f2"
}

fn first_game_over(events: &[Event]) -> Option<EventGameOver> {
    events.iter().find_map(|e| match e {
        Event::GameOver(g) => Some(g.clone()),
        _ => None,
    })
}

impl Model {
    fn theme_or_default(&self) -> Theme {
        self.theme.unwrap_or_else(theme::default_theme)
    }

    fn focus_bar(&mut self) -> Option<Cmd> {
        self.command_bar.focus()
    }

    /// Go `checkVisualBell`.
    pub fn check_visual_bell(&mut self, events: &[Event]) -> Option<Cmd> {
        let mut triggered = false;
        for ev in events {
            match ev {
                Event::TorpedoHit(_)
                | Event::PhaserHit(_)
                | Event::KlingonCounterAttack(_)
                | Event::SubsystemDamaged(_) => triggered = true,
                Event::ConditionChanged(e) => {
                    if e.to == ConditionType::Red {
                        triggered = true;
                    }
                }
                Event::HazardTriggered(_) => triggered = true,
                Event::GameOver(e) if e.reason != GameOverReason::Won => triggered = true,
                _ => {}
            }
            if triggered {
                break;
            }
        }
        if triggered {
            self.command_bar.set_visual_bell(true);
            return visual_bell_cmd();
        }
        None
    }

    /// Go `checkRedAlertCmd`.
    pub fn check_red_alert_cmd(&mut self, prev_cond: ConditionType) -> Option<Cmd> {
        if let Some(g) = &self.game {
            let g = g.borrow();
            if g.enterprise.condition == ConditionType::Red
                && g.rules.anim_speed != ANIM_SPEED_OFF
                && (prev_cond != ConditionType::Red || !self.red_alert_active)
            {
                drop(g);
                self.red_alert_active = true;
                return red_alert_pulse_cmd();
            }
        }
        None
    }

    /// Go `Update`: `update` followed by `evaluateTourSector`.
    pub fn update_in_place(&mut self, msg: Msg) -> Option<Cmd> {
        let cmd = self.update_inner(msg);
        self.evaluate_tour_sector();
        cmd
    }

    fn condition(&self) -> ConditionType {
        self.game
            .as_ref()
            .map(|g| g.borrow().enterprise.condition)
            .unwrap_or_default()
    }

    fn anim_speed(&self) -> i32 {
        self.game
            .as_ref()
            .map(|g| g.borrow().rules.anim_speed)
            .unwrap_or_default()
    }

    /// Dispatches `action` on the game (`m.Game.Dispatch`).
    fn dispatch(&mut self, action: &Action) -> Result<Vec<Event>, sst_engine::EngineError> {
        let g = self.game.clone().expect("dispatch without game");
        let r = g.borrow_mut().dispatch(action);
        r
    }

    /// Go `executeKlingonTurnIfActive`.
    pub fn execute_klingon_turn_if_active(&mut self, action: &Action, events: &mut Vec<Event>) {
        if let Some(g) = &self.game {
            execute_klingon_turn_if_active(&mut g.borrow_mut(), action, events);
        }
    }

    /// Shared tail of the fire-torpedo / fire-phasers handlers.
    fn fire_action(&mut self, action: Action) -> Option<Cmd> {
        let mut anim_cmd = None;
        let mut pulse_cmd = None;
        if self.game.is_some() {
            let prev_cond = self.condition();
            match self.dispatch(&action) {
                Err(e) => self.command_bar.add_message(&e.to_string()),
                Ok(mut events) => {
                    self.execute_klingon_turn_if_active(&action, &mut events);
                    let bell_cmd = self.log_events(&events);
                    if let Some(go) = first_game_over(&events) {
                        return self.handle_game_over(go);
                    }
                    if self.anim_speed() > 0 {
                        if let Some(a) = self.create_combat_animation(&action, &events) {
                            anim_cmd = self.start_combat_animation(a);
                        }
                    }
                    pulse_cmd = self.check_red_alert_cmd(prev_cond);
                    self.active_modal = MODAL_NONE;
                    let cmd = self.focus_bar();
                    return tea::batch(vec![cmd, anim_cmd, pulse_cmd, bell_cmd]);
                }
            }
        }
        self.active_modal = MODAL_NONE;
        let cmd = self.focus_bar();
        tea::batch(vec![cmd, anim_cmd, pulse_cmd])
    }

    fn close_modal_focus(&mut self) -> Option<Cmd> {
        self.active_modal = MODAL_NONE;
        self.focus_bar()
    }

    fn update_inner(&mut self, msg: Msg) -> Option<Cmd> {
        if let Some(ws) = msg.downcast_ref::<WindowSizeMsg>() {
            self.width = ws.width;
            self.height = ws.height;
            self.command_bar.set_width(ws.width);
            if let Some(th) = self.theme {
                if th.color_mode() == ColorMode::Auto {
                    let has_dark = theme::detect::detect_dark_background();
                    if has_dark != self.last_dark_bg {
                        self.last_dark_bg = has_dark;
                        self.apply_theme(Some(th));
                    }
                }
            }
            return None;
        }

        if let Some(m) = msg.downcast_ref::<targetlock::FireTorpedoMsg>() {
            let action = Action::FireTorpedo(ActionFireTorpedo {
                target: m.target,
                direction: m.bearing,
                ..Default::default()
            });
            return self.fire_action(action);
        }

        if let Some(m) = msg.downcast_ref::<targetlock::FirePhasersMsg>() {
            if self.game.is_some() {
                let mut action = Action::FirePhasers(ActionFirePhasers {
                    energy: m.energy,
                    ..Default::default()
                });
                if let Some(t) = self.target_lock.current_target() {
                    if t.typ == EntityType::PlasmaTorpedo || t.typ == EntityType::TholianWeb {
                        action = Action::PhaserDirect(ActionPhaserDirect {
                            energy: m.energy,
                            target_sector: t.coord,
                        });
                    }
                }
                return self.fire_action(action);
            }
            return self.close_modal_focus();
        }

        if msg.downcast_ref::<targetlock::CloseHUDMsg>().is_some() {
            return self.close_modal_focus();
        }

        if let Some(m) = msg.downcast_ref::<commandpalette::CommandSelectedMsg>() {
            self.active_modal = MODAL_NONE;
            if m.parameterized {
                self.command_bar.set_value(&m.command_prefix);
                return self.focus_bar();
            }
            let cmd = self.focus_bar();
            let res = self.handle_command(&m.command_prefix);
            return tea::batch(vec![cmd, res]);
        }

        if msg
            .downcast_ref::<commandpalette::ClosePaletteMsg>()
            .is_some()
        {
            return self.close_modal_focus();
        }

        if let Some(m) = msg.downcast_ref::<savebrowser::LoadGameMsg>() {
            match persist::load_game(&m.path) {
                Err(e) => self.command_bar.add_message(&format!(
                    "Failed to thaw {}: {}",
                    gopath::base(&m.path),
                    e
                )),
                Ok(loaded) => {
                    let sd = loaded.borrow().stardate;
                    self.game = Some(loaded);
                    self.selected_sector = [0, 0];
                    self.command_bar.add_message(&format!(
                        "Mission thawed: {} (Stardate {})",
                        gopath::base(&m.path),
                        fmt_f(sd, 1)
                    ));
                }
            }
            self.active_modal = MODAL_NONE;
            let cmd = self.focus_bar();
            let pulse = self.check_red_alert_cmd(ConditionType::Green);
            return tea::batch(vec![cmd, pulse]);
        }

        if msg.downcast_ref::<savebrowser::CloseBrowserMsg>().is_some() {
            return self.close_modal_focus();
        }

        if let Some(m) = msg.downcast_ref::<galacticchart::WarpToQuadrantMsg>() {
            self.active_modal = MODAL_NONE;
            let pulse_cmd = None;
            if self.game.is_some() {
                let prev_cond = self.condition();
                let action = Action::Move(ActionMove {
                    dest_quad: m.dest_quad,
                    warp: m.warp,
                    ..Default::default()
                });
                match self.dispatch(&action) {
                    Err(e) => self.command_bar.add_message(&e.to_string()),
                    Ok(events) => {
                        self.selected_sector = [0, 0];
                        let bell_cmd = self.log_events(&events);
                        if let Some(go) = first_game_over(&events) {
                            return self.handle_game_over(go);
                        }
                        let pulse_cmd = self.check_red_alert_cmd(prev_cond);
                        let cmd = self.focus_bar();
                        return tea::batch(vec![cmd, pulse_cmd, bell_cmd]);
                    }
                }
            }
            let cmd = self.focus_bar();
            return tea::batch(vec![cmd, pulse_cmd]);
        }

        if let Some(m) = msg.downcast_ref::<galacticchart::WarpBlockedMsg>() {
            self.active_modal = MODAL_NONE;
            let cmd = self.focus_bar();
            self.command_bar.add_message(&m.reason);
            return cmd;
        }

        if msg.downcast_ref::<galacticchart::CloseChartMsg>().is_some()
            || msg
                .downcast_ref::<damageschematic::CloseModalMsg>()
                .is_some()
            || msg.downcast_ref::<halloffame::CloseModalMsg>().is_some()
            || msg.downcast_ref::<manual::CloseModalMsg>().is_some()
        {
            return self.close_modal_focus();
        }

        if let Some(m) = msg.downcast_ref::<scenariomodal::MsgLaunchScenario>() {
            self.active_modal = MODAL_NONE;
            if let Some(sc) = get_scenario(&m.scenario_id) {
                if let Some(build) = sc.build {
                    let seed = self.clock.now().unix_nano();
                    self.game = Some(sst_engine::game_ref(build(seed)));
                    self.selected_sector = [0, 0];
                    self.command_bar
                        .add_message(&format!("Tactical Scenario Launched: {}", sc.name));
                    let cmd = self.focus_bar();
                    let pulse = self.check_red_alert_cmd(ConditionType::Green);
                    return tea::batch(vec![cmd, pulse]);
                }
            }
            return self.focus_bar();
        }

        if msg
            .downcast_ref::<scenariomodal::MsgCloseScenarioModal>()
            .is_some()
        {
            return self.close_modal_focus();
        }

        if let Some(m) = msg.downcast_ref::<drydockmodal::RefitPurchasedMsg>() {
            self.command_bar.add_message(&format!(
                "Installed refit: {} (Tier {})",
                m.refit_id.0, m.tier
            ));
            return None;
        }

        if msg.downcast_ref::<drydockmodal::DisembarkMsg>().is_some() {
            self.active_modal = MODAL_NONE;
            self.handle_disembark();
            return self.focus_bar();
        }

        if let Some(m) = msg.downcast_ref::<halloffame::ScoreRecordedMsg>() {
            self.command_bar.add_message(&format!(
                "Score recorded for Captain {}: {} points ({})",
                m.entry.captain_name, m.entry.score, m.entry.rank
            ));
            return None;
        }

        if let Some(ev) = msg.downcast_ref::<EventGameOver>() {
            return self.handle_game_over(ev.clone());
        }

        if msg.downcast_ref::<RedAlertPulseMsg>().is_some() {
            let red = self.game.as_ref().is_some_and(|g| {
                let g = g.borrow();
                g.enterprise.condition == ConditionType::Red && g.rules.anim_speed != ANIM_SPEED_OFF
            });
            if red {
                self.red_alert_cycle += 1;
                self.sync_child_components();
                self.red_alert_active = true;
                return red_alert_pulse_cmd();
            }
            self.red_alert_active = false;
            return None;
        }

        if msg.downcast_ref::<VisualBellExpireMsg>().is_some() {
            self.command_bar.set_visual_bell(false);
            return None;
        }

        if let Some(t) = msg.downcast_ref::<anim::TickMsg>() {
            if t.anim_id == self.anim_id {
                if let Some(a) = self.active_anim.clone() {
                    if a.borrow().is_finished() {
                        self.active_anim = None;
                        self.grid.clear_anim_overrides();
                        return None;
                    }
                    let f = a.borrow_mut().step();
                    self.grid.set_anim_overrides(f.overrides);
                    return anim::tick_cmd(self.anim_id, t.step + 1, f.duration);
                }
            }
            return None;
        }

        if let Some(k) = msg.downcast_ref::<KeyMsg>() {
            return self.handle_key(k, &msg);
        }

        if let Some(mm) = msg.downcast_ref::<MouseMsg>() {
            return self.handle_mouse(mm);
        }

        if let Some(m) = msg.downcast_ref::<commandbar::CommandSubmittedMsg>() {
            let text = m.text.clone();
            return self.handle_command(&text);
        }

        self.command_bar.update(&msg)
    }

    fn handle_disembark(&mut self) {
        let Some(tour) = self.tour.clone() else {
            return;
        };
        if !tour.borrow().in_drydock {
            return;
        }
        let res = tour.borrow_mut().disembark_to_next_sector();
        match res {
            Err(e) => self
                .command_bar
                .add_message(&format!("Disembark error: {}", e)),
            Ok(Some(new_game)) => {
                {
                    let mut g = new_game.borrow_mut();
                    if g.current_quad.enemies.is_empty() {
                        let (q, s) = (g.enterprise.quad, g.enterprise.sector);
                        g.populate_quadrant(q, s);
                    }
                }
                self.game = Some(new_game);
                self.selected_sector = [0, 0];
                let sec = tour.borrow().current_sector().cloned();
                if let Some(sec) = sec {
                    self.command_bar.add_message(&format!(
                        "★ ENTERING SECTOR {}: {} ★",
                        sec.index,
                        sec.name.to_uppercase()
                    ));
                    self.command_bar.add_message(&sec.description);
                }
            }
            Ok(None) => {
                if tour.borrow().completed {
                    self.command_bar
                        .add_message("★ PATROL TOUR COMPLETED! ALL SECTORS CLEARED! ★");
                    self.record_tour_to_leaderboard(&tour);
                }
            }
        }
    }

    fn record_tour_to_leaderboard(&mut self, tour: &sst_engine::TourRef) {
        let path = persist::default_leaderboard_path();
        let mut lb =
            persist::load_leaderboard(Some(&path)).unwrap_or_else(|_| Leaderboard::default_board());
        let now = self.clock.now();
        lb.record_tour_at(&tour.borrow(), &self.player_callsign, now);
        let _ = persist::save_leaderboard(&lb, &path);
    }

    fn cycle_color_mode(&mut self) {
        let th = self.theme_or_default();
        let new_mode = th.color_mode().next();
        self.apply_theme(Some(th.with_color_mode(new_mode)));
        let name = self.theme_or_default().name();
        self.command_bar
            .add_message(&format!("Color mode set to {} ({})", new_mode, name));
    }

    fn next_theme(&mut self) {
        let th = self.theme_or_default();
        self.apply_theme(Some(th.next()));
    }

    fn handle_key(&mut self, k: &KeyMsg, msg: &Msg) -> Option<Cmd> {
        if let Some(a) = self.active_anim.take() {
            a.borrow_mut().skip();
            self.grid.clear_anim_overrides();
        }

        if self.show_options {
            if is_key(k, KEY_CTRL_C, "ctrl+c") {
                return tea::quit();
            }
            if is_key(k, KEY_F2, "f2") {
                self.next_theme();
                return None;
            }
            if is_ctrl_t(k) {
                self.cycle_color_mode();
                return None;
            }
            let cmd = self.options_modal.update(msg);
            if self.options_modal.closed {
                self.show_options = false;
                if let Some(g) = &self.game {
                    g.borrow_mut().rules = self.options_modal.rules();
                }
                if let Some(th) = self.theme {
                    self.apply_theme(Some(th.with_color_mode(self.options_modal.color_mode())));
                }
                let enabled = self.options_modal.audio_enabled();
                self.set_sound_enabled(enabled);
                if let Some(p) = &self.audio_player {
                    p.set_volume(self.options_modal.volume());
                }
                self.status.set_audio_telemetry(
                    self.options_modal.volume(),
                    !self.options_modal.audio_enabled(),
                );

                let mut cfg = config::load_config().unwrap_or_else(|_| config::default_config());
                cfg.volume = self.options_modal.volume();
                cfg.muted = !self.options_modal.audio_enabled();
                cfg.audio_mode = self.options_modal.audio_mode();
                if let Some(th) = self.theme {
                    cfg.theme = th.name().to_string();
                }
                cfg.anim_speed = match self.options_modal.rules().anim_speed {
                    ANIM_SPEED_FAST => "fast".into(),
                    ANIM_SPEED_CINEMATIC => "slow".into(),
                    _ => "normal".into(),
                };
                let _ = config::save_config(cfg);

                self.options_modal.set_active(false);
                self.options_modal.closed = false;
                return self.focus_bar();
            }
            return cmd;
        }

        if self.tour.as_ref().is_some_and(|t| t.borrow().in_drydock) {
            if is_key(k, KEY_CTRL_C, "ctrl+c") {
                return tea::quit();
            }
            if is_key(k, KEY_F2, "f2") {
                self.next_theme();
                return None;
            }
            if is_ctrl_t(k) {
                self.cycle_color_mode();
                return None;
            }
            return self.drydock.update(msg);
        }

        if self.active_modal != MODAL_NONE {
            if is_key(k, KEY_CTRL_C, "ctrl+c") {
                return tea::quit();
            }
            if is_key(k, KEY_F2, "f2") {
                self.next_theme();
                return None;
            }
            if is_ctrl_t(k) {
                self.cycle_color_mode();
                return None;
            }
            match self.active_modal {
                MODAL_HALL_OF_FAME => return self.hall_of_fame.update(msg),
                MODAL_DAMAGE_SCHEMATIC => return self.damage_schematic.update(msg),
                MODAL_MANUAL => return self.manual.update(msg),
                MODAL_SCENARIO => return self.scenario_modal.update(msg),
                _ => {}
            }
            if is_key(k, KEY_ESC, "esc") {
                return self.close_modal_focus();
            }
            return match self.active_modal {
                MODAL_TARGET_LOCK => self.target_lock.update(msg),
                MODAL_COMMAND_PALETTE => self.command_palette.update(msg),
                MODAL_SAVE_BROWSER => self.save_browser.update(msg),
                MODAL_GALACTIC_CHART => self.galactic_chart.update(msg),
                _ => None,
            };
        }

        let s = k.string();
        let empty = || trim_space(&self.command_bar.value()).is_empty();
        if k.typ == KEY_F1 || s == "f1" {
            return self.open_manual("");
        } else if k.typ == KEY_F2 || s == "f2" {
            self.next_theme();
            return None;
        } else if is_ctrl_t(k) {
            self.cycle_color_mode();
            return None;
        } else if k.typ == KEY_CTRL_C || s == "ctrl+c" {
            return tea::quit();
        } else if k.typ == KEY_CTRL_S || s == "ctrl+s" {
            return self.toggle_sound();
        } else if k.typ == KEY_ESC || s == "esc" {
            if empty() {
                return tea::quit();
            }
            self.command_bar.reset();
            return None;
        } else if k.typ == KEY_CTRL_P || s == "ctrl+p" {
            self.command_palette.reset();
            self.active_modal = MODAL_COMMAND_PALETTE;
            self.command_bar.blur();
            return None;
        } else if k.typ == KEY_CTRL_O || s == "ctrl+o" {
            let _ = self.save_browser.refresh();
            self.active_modal = MODAL_SAVE_BROWSER;
            self.command_bar.blur();
            return None;
        } else if k.typ == KEY_CTRL_G || s == "ctrl+g" {
            return self.open_galactic_chart();
        } else if k.typ == KEY_CTRL_D || s == "ctrl+d" {
            if empty() {
                return self.open_damage_schematic();
            }
        } else if k.typ == KEY_CTRL_H || s == "ctrl+h" {
            if empty() {
                return self.open_hall_of_fame(false);
            }
        } else if empty() {
            match s.as_str() {
                "t" | "T" => {
                    if self.no_targets() {
                        self.command_bar
                            .add_message("Sensors detect no hostile targets in sector.");
                        return None;
                    }
                    self.open_target_lock([0, 0]);
                    return None;
                }
                "/" => {
                    self.command_palette.reset();
                    self.active_modal = MODAL_COMMAND_PALETTE;
                    self.command_bar.blur();
                    return None;
                }
                "c" | "C" => return self.open_galactic_chart(),
                "m" | "M" => return self.toggle_sound(),
                "d" | "D" => return self.open_damage_schematic(),
                "h" | "H" => return self.open_hall_of_fame(false),
                "o" | "O" => return self.open_options_modal(),
                "?" => return self.open_manual(""),
                _ => {}
            }
        }

        self.command_bar.update(msg)
    }

    fn no_targets(&self) -> bool {
        match &self.game {
            None => true,
            Some(g) => {
                let g = g.borrow();
                let q = &g.current_quad;
                q.klingons.is_empty()
                    && q.enemies.is_empty()
                    && q.plasma_torpedoes.is_empty()
                    && q.web_segments.is_empty()
            }
        }
    }

    fn open_target_lock(&mut self, initial: Coord) {
        match &self.game {
            Some(g) => self
                .target_lock
                .set_state_from_game(Some(&g.borrow()), initial),
            None => self.target_lock.set_state_from_game(None, initial),
        }
        self.active_modal = MODAL_TARGET_LOCK;
        self.command_bar.blur();
    }

    fn is_double_click(&self, coord: Coord, now: &sst_engine::gotime::GoTime) -> bool {
        coord == self.last_click_coord
            && !self.last_click_time.is_zero()
            && now
                .unix_nano()
                .wrapping_sub(self.last_click_time.unix_nano())
                < DOUBLE_CLICK_NANOS
    }

    fn handle_mouse(&mut self, mm: &MouseMsg) -> Option<Cmd> {
        if self.show_options || self.tour.as_ref().is_some_and(|t| t.borrow().in_drydock) {
            return None;
        }
        let is_left = mm.button == MOUSE_BUTTON_LEFT;
        if !is_left || mm.action == MOUSE_ACTION_RELEASE || mm.action == MOUSE_ACTION_MOTION {
            return None;
        }

        if self.active_modal == MODAL_GALACTIC_CHART {
            if self.galactic_chart.showing_help() {
                self.galactic_chart.set_showing_help(false);
                return None;
            }
            let mut chart_w = self.galactic_chart.width();
            if chart_w <= 0 {
                chart_w = 64;
            }
            let mut chart_h = self.galactic_chart.height();
            if chart_h <= 0 {
                chart_h = 18;
            }
            let start_x = (self.width - chart_w) / 2;
            let start_y = (self.height - chart_h) / 2;
            let coord = self
                .galactic_chart
                .hit_test(mm.x - start_x, mm.y - start_y)?;

            let now = self.clock.now();
            if self.is_double_click(coord, &now) {
                self.last_click_time = Default::default();
                self.last_click_coord = coord;
                if self.galactic_chart.computer_damaged() {
                    self.command_bar.add_message("COMPUTER DAMAGED, USE A POCKET CALCULATOR. Manual navigation required (nav q <r> <c> [warp]).");
                    return None;
                }
                let dest = coord;
                let cur_quad = self
                    .game
                    .as_ref()
                    .map(|g| g.borrow().enterprise.quad)
                    .unwrap_or_default();
                let telem = galacticchart::calculate_telemetry(cur_quad, dest);
                self.active_modal = MODAL_NONE;
                if self.game.is_some() {
                    let prev_cond = self.condition();
                    let action = Action::Move(ActionMove {
                        dest_quad: dest,
                        warp: telem.recommended_warp,
                        ..Default::default()
                    });
                    match self.dispatch(&action) {
                        Err(e) => self.command_bar.add_message(&e.to_string()),
                        Ok(events) => {
                            self.selected_sector = [0, 0];
                            let bell_cmd = self.log_events(&events);
                            if let Some(go) = first_game_over(&events) {
                                return self.handle_game_over(go);
                            }
                            let pulse = self.check_red_alert_cmd(prev_cond);
                            return tea::batch(vec![pulse, bell_cmd]);
                        }
                    }
                }
                return None;
            }

            self.last_click_time = now;
            self.last_click_coord = coord;
            self.galactic_chart.set_cursor(coord);
            return None;
        }

        if self.active_modal != MODAL_NONE {
            return None;
        }

        let coord = self.grid.hit_test(mm.x, mm.y - 1)?;

        let now = self.clock.now();
        if self.is_double_click(coord, &now) {
            self.last_click_time = Default::default();
            self.last_click_coord = coord;

            let mut ent = EntityType::Empty;
            let mut is_base_coord = false;
            if let Some(g) = &self.game {
                let g = g.borrow();
                if (1..=8).contains(&coord[0]) && (1..=8).contains(&coord[1]) {
                    ent = g.current_quad.grid[coord[0] as usize][coord[1] as usize];
                }
                if g.enterprise.sector == coord {
                    ent = EntityType::Enterprise;
                }
                is_base_coord = g.current_quad.starbase == Some(coord);
            }

            if ent == EntityType::Empty {
                if self.game.is_some() {
                    let prev_cond = self.condition();
                    let action = Action::Move(ActionMove {
                        dest_sector: coord,
                        warp: 1.0,
                        ..Default::default()
                    });
                    match self.dispatch(&action) {
                        Err(e) => self.command_bar.add_message(&e.to_string()),
                        Ok(mut events) => {
                            self.execute_klingon_turn_if_active(&action, &mut events);
                            let bell_cmd = self.log_events(&events);
                            if let Some(go) = first_game_over(&events) {
                                return self.handle_game_over(go);
                            }
                            let pulse = self.check_red_alert_cmd(prev_cond);
                            return tea::batch(vec![pulse, bell_cmd]);
                        }
                    }
                }
                return None;
            }

            if ent == EntityType::Starbase || is_base_coord {
                if self.game.is_some() {
                    let prev_cond = self.condition();
                    match self.dispatch(&Action::Dock(ActionDock)) {
                        Err(e) => self.command_bar.add_message(&e.to_string()),
                        Ok(events) => {
                            let bell_cmd = self.log_events(&events);
                            if let Some(go) = first_game_over(&events) {
                                return self.handle_game_over(go);
                            }
                            let pulse = self.check_red_alert_cmd(prev_cond);
                            return tea::batch(vec![pulse, bell_cmd]);
                        }
                    }
                }
                return None;
            }
            return None;
        }

        // Single click
        self.selected_sector = coord;
        self.last_click_time = now;
        self.last_click_coord = coord;

        let mut targetable = false;
        if let Some(g) = &self.game {
            let g = g.borrow();
            let q = &g.current_quad;
            targetable = q.klingons.iter().flatten().any(|k| {
                let k = k.borrow();
                k.sector == coord && !k.is_cloaked
            });
            if !targetable {
                targetable = q.enemies.iter().flatten().any(|e| {
                    let e = e.borrow();
                    e.sector == coord && !e.is_cloaked
                });
            }
            if !targetable {
                targetable = q
                    .plasma_torpedoes
                    .iter()
                    .flatten()
                    .any(|p| p.borrow().sector == coord);
            }
            if !targetable {
                targetable = q
                    .web_segments
                    .iter()
                    .flatten()
                    .any(|w| w.borrow().coord == coord);
            }
            if !targetable && (1..=8).contains(&coord[0]) && (1..=8).contains(&coord[1]) {
                let ge = q.grid[coord[0] as usize][coord[1] as usize];
                targetable = matches!(
                    ge,
                    EntityType::Klingon
                        | EntityType::Commander
                        | EntityType::SuperCommander
                        | EntityType::Romulan
                        | EntityType::Tholian
                        | EntityType::PlasmaTorpedo
                        | EntityType::TholianWeb
                );
            }
        }

        if targetable {
            self.open_target_lock(coord);
            return None;
        }

        self.command_bar.add_message(&format!(
            "Target sector selected: [{}, {}]",
            coord[0], coord[1]
        ));
        None
    }

    /// Go `applyTheme`: propagates the theme (nil -> default) to every sub-component.
    pub fn apply_theme(&mut self, th: Option<Theme>) {
        let th = th.unwrap_or_else(theme::default_theme);
        self.theme = Some(th);
        self.last_dark_bg = theme::detect::detect_dark_background();
        self.grid.set_theme(Some(th));
        self.status.set_theme(Some(th));
        self.command_bar.set_theme(Some(th));
        self.target_lock.set_theme(Some(th));
        self.command_palette.set_theme(th);
        self.save_browser.set_theme(th);
        self.galactic_chart.set_theme(Some(th));
        self.damage_schematic.set_theme(Some(th));
        self.hall_of_fame.set_theme(th);
        self.manual.set_theme(th);
        self.scenario_modal.set_theme(th);
        self.options_modal.set_theme(th);
        self.drydock.set_theme(th);
        if let Some(t) = &self.tour {
            self.drydock.tour = Some(t.clone());
        }
        self.sync_child_components();
    }

    /// Go `openGalacticChart`.
    pub fn open_galactic_chart(&mut self) -> Option<Cmd> {
        let mut ent_quad: Coord = [1, 1];
        let mut chart = [[0i32; 9]; 9];
        let mut discovered = [[false; 9]; 9];
        let mut known = [[false; 9]; 9];
        let mut comp_damaged = false;
        if let Some(g) = &self.game {
            let g = g.borrow();
            ent_quad = g.enterprise.quad;
            chart = g.galaxy_chart;
            discovered = g.chart_discovered;
            known = g.chart_known_bases;
            comp_damaged = g.enterprise.devices[DeviceID::Computer.0 as usize] > 0.0;
        }
        self.galactic_chart
            .set_state(ent_quad, chart, discovered, known, comp_damaged);
        self.active_modal = MODAL_GALACTIC_CHART;
        self.command_bar.blur();
        None
    }

    /// Go `openDamageSchematic`.
    pub fn open_damage_schematic(&mut self) -> Option<Cmd> {
        let mut cond = "GREEN";
        let mut docked = false;
        let mut repair_mult = 1.0;
        let mut ent = sst_engine::EnterpriseState::default();
        if let Some(g) = &self.game {
            let g = g.borrow();
            ent = g.enterprise.clone();
            docked = g.enterprise.condition == ConditionType::Docked;
            cond = match g.enterprise.condition {
                ConditionType::Yellow => "YELLOW",
                ConditionType::Red => "RED",
                ConditionType::Docked => "DOCKED",
                _ => "GREEN",
            };
            if g.rules.repair_multiplier > 0.0 {
                repair_mult = g.rules.repair_multiplier;
            }
        }
        self.damage_schematic
            .set_state(&ent, cond, docked, repair_mult);
        self.active_modal = MODAL_DAMAGE_SCHEMATIC;
        self.command_bar.blur();
        None
    }

    fn load_default_leaderboard() -> Leaderboard {
        persist::load_leaderboard(Some(&persist::default_leaderboard_path()))
            .unwrap_or_else(|_| Leaderboard::default_board())
    }

    fn score(&self) -> sst_engine::ScoreBreakdown {
        match &self.game {
            Some(g) => {
                let g = g.borrow();
                compute_score(Some(&g), g.game_won)
            }
            None => compute_score(None, false),
        }
    }

    /// Go `openHallOfFame`.
    pub fn open_hall_of_fame(&mut self, prompt_name: bool) -> Option<Cmd> {
        let lb = Self::load_default_leaderboard();
        let score = self.score();
        self.hall_of_fame
            .set_state(score, Some(Rc::new(RefCell::new(lb))), prompt_name);
        self.active_modal = MODAL_HALL_OF_FAME;
        self.command_bar.blur();
        None
    }

    /// Go `openManual`.
    pub fn open_manual(&mut self, topic: &str) -> Option<Cmd> {
        if !topic.is_empty() {
            self.manual.select_chapter(topic);
            let f = fields(topic);
            if f.len() > 1 {
                self.manual.select_chapter(f[0]);
            }
        }
        self.active_modal = MODAL_MANUAL;
        self.command_bar.blur();
        None
    }

    /// Go `openOptionsModal`.
    pub fn open_options_modal(&mut self) -> Option<Cmd> {
        if let Some(g) = &self.game {
            self.options_modal.set_rules(g.borrow().rules.clone());
        }
        if let Some(th) = self.theme {
            self.options_modal.set_color_mode(th.color_mode());
        }
        let enabled = self.sound_enabled();
        self.options_modal.set_audio_enabled(enabled);
        if let Some(p) = self.audio_player.clone() {
            self.options_modal.set_volume(p.volume());
            self.options_modal.set_player(p);
        }
        if let Ok(cfg) = config::load_config() {
            if !cfg.audio_mode.as_str().is_empty() {
                self.options_modal.set_audio_mode(cfg.audio_mode);
            }
        }
        self.options_modal.set_active(true);
        self.options_modal.closed = false;
        self.show_options = true;
        self.command_bar.blur();
        None
    }

    /// Go `handleGameOver`.
    pub fn handle_game_over(&mut self, ev: EventGameOver) -> Option<Cmd> {
        let formatted = format_game_over(&ev);
        if !formatted.is_empty() {
            self.command_bar.add_message(&formatted);
        }
        if let Some(g) = &self.game {
            let mut g = g.borrow_mut();
            g.game_over = true;
            g.game_over_reason = ev.reason;
            if ev.reason == GameOverReason::Won {
                g.game_won = true;
            }
        }
        let score = self.score();
        let lb = Self::load_default_leaderboard();
        let qualifies = lb.qualifies(score.total_score);
        if self
            .tour
            .as_ref()
            .is_some_and(|t| !t.borrow().in_drydock && t.borrow().active)
        {
            self.evaluate_tour_sector();
            // A won sector goes straight to the drydock (Go never cleared a
            // sector on a win, so the Hall of Fame always opened here).
            if self.tour.as_ref().is_some_and(|t| t.borrow().in_drydock) {
                return None;
            }
        }
        self.open_hall_of_fame(qualifies)
    }

    /// Go `evaluateTourSector`.
    pub fn evaluate_tour_sector(&mut self) {
        let Some(tour) = self.tour.clone() else {
            return;
        };
        {
            let t = tour.borrow();
            if t.in_drydock || !t.active {
                return;
            }
        }
        if let Some(g) = &self.game {
            tour.borrow_mut().current_game_state = Some(g.clone());
        }
        let (cleared, failed, bounty) = tour.borrow_mut().evaluate_sector();
        if cleared {
            self.active_modal = MODAL_NONE;
            tour.borrow_mut().advance_to_drydock(bounty);
            self.drydock = drydockmodal::new(Some(tour.clone()), self.theme);
            let done = tour.borrow().sectors_completed;
            self.command_bar.add_message(&format!(
                "★ SECTOR {} CLEARED! REQUISITION BOUNTY: +{} PTS ★",
                done, bounty
            ));
            self.command_bar.add_message("Docked at Starbase 01 Drydock & Refit Facility. Select refits or press Space/D to disembark.");
            return;
        }
        if failed {
            self.record_tour_to_leaderboard(&tour);
            let reason = tour.borrow().failure_reason.0;
            self.command_bar.add_message(&format!(
                "Tour failed: {}. Commission recorded in Hall of Fame.",
                reason
            ));
        }
    }

    /// Go `displayTourOrders`.
    pub fn display_tour_orders(&mut self) {
        let Some(tour) = self.tour.clone() else {
            self.command_bar
                .add_message("No active patrol tour. Start one with --tour flag.");
            return;
        };
        let t = tour.borrow();
        let Some(sec) = t.current_sector() else {
            self.command_bar
                .add_message("Tour completed or no active sector.");
            return;
        };
        self.command_bar.add_message(&format!(
            "★ STARFLEET PATROL ORDERS — SECTOR {}/{}: {} ★",
            sec.index,
            t.sectors.len(),
            sec.name.to_uppercase()
        ));
        self.command_bar.add_message(&format!(
            "Objective: {} — {}",
            sec.objective.0, sec.description
        ));
        let (rem_k, rem_days) = match &self.game {
            Some(g) => {
                let g = g.borrow();
                (g.klingons_remaining, g.days_remaining)
            }
            None => (0, 0.0),
        };
        self.command_bar.add_message(&format!(
            "Status: {} hostiles remaining | Time left: {} stardates | Requisition: {} PTS | Bounty: {} PTS",
            rem_k,
            fmt_f(rem_days, 1),
            t.requisition_points,
            sec.bonus_bounty
        ));
        if t.in_drydock {
            self.command_bar
                .add_message("Ship currently docked in Drydock. Press Space or D to disembark.");
        }
    }

    fn set_color_mode_cmd(&mut self, mode_word: &str) {
        match mode_word {
            "auto" | "dark" | "light" => {
                let new_mode = theme::parse_color_mode(mode_word).unwrap_or_default();
                let th = self.theme_or_default();
                self.apply_theme(Some(th.with_color_mode(new_mode)));
                let name = self.theme_or_default().name();
                self.command_bar
                    .add_message(&format!("Color mode set to {} ({})", new_mode, name));
            }
            _ => self.command_bar.add_message(&format!(
                "Invalid color mode: {} (expected auto, dark, or light)",
                quote(mode_word)
            )),
        }
    }

    fn help_lines(&mut self, topic: &str) -> bool {
        let lines: &[&str] = match topic {
            "nav" => &[
                "NAV: Direct Quad: nav q <r c> [warp] (e.g. 'nav q 3 5')",
                "     Direct Sector: nav s <r c> (or double-click sector grid)",
                "     Vector: nav <course> <warp> (0.0=East, 1.57=North, 3.14=West, 4.71=South)",
                "     Warp 1.0 = 1 Quad. Dist = sqrt(ΔR²+ΔC²). [Ctrl+G] map tool. Shields UP = 2x energy.",
            ],
            "tor" => &[
                "TOR: Target Sector: tor <r c> (e.g. 'tor 4 7')",
                "     Bearing Angle: tor <angle> (0.0=East, 1.57=North, 3.14=West, 4.71=South)",
                "     Tactical HUD: Press [T] for Target Lock auto-aiming & telemetry",
                "     Damaged launcher cannot fire; torpedoes do not pass obstacles.",
            ],
            "pha" => &[
                "PHA: Fire phaser banks: pha <energy> (e.g. 'pha 300')",
                "     Energy is divided among all Klingons present in quadrant.",
                "     Damage drops with target distance. Damaged phasers cannot fire.",
            ],
            "she" => &[
                "SHE: Transfer shield energy: she <amount> (e.g. 'she 500', 'she -200')",
                "     Shields protect against incoming torpedo & phaser damage.",
                "     Shields UP doubles warp movement energy consumption!",
            ],
            "doc" => &[
                "DOC: Starbase docking: doc (must be in adjacent sector)",
                "     Replenishes full energy & photon torpedo supply.",
                "     Repairs all damaged ship systems and lowers shields.",
            ],
            "chart" => &[
                "CHART: Interactive Galactic Star Chart & Warp Planner (Ctrl+G or 'chart')",
                "       Inspect 8x8 quadrant grid, telemetry vectors, and distance calculations.",
                "       [Arrows/HJKL] Move cursor  [Enter] Warp to quadrant  [Esc] Close",
            ],
            "saves" => &[
                "SAVES: Open Save Browser: saves or bare thaw (hotkey Ctrl+O)",
                "       Inspects stardates, condition, and Klingons remaining.",
                "       Direct load: thaw <filename> | Freeze/save: freeze <filename>",
            ],
            "options" => &[
                "OPTIONS: Configure game difficulty & realism settings (hotkey [O] or 'options')",
                "         Adjust difficulty profile, surveillance mode, sensors, repair, and cloaking.",
            ],
            _ => return false,
        };
        for l in lines {
            self.command_bar.add_message(l);
        }
        true
    }

    fn no_help_msg(&mut self, name: &str) {
        self.command_bar.add_message(&format!(
            "No detailed help for {}. Available: help nav, help tor, help pha, help she, help doc, help chart, help saves, help options",
            quote(name)
        ));
    }

    /// Go `handleCommand`: tokenizes, parses and executes a player command.
    pub fn handle_command(&mut self, text: &str) -> Option<Cmd> {
        let trimmed = trim_space(text).to_lowercase();
        let f = fields(&trimmed);
        if f.len() == 3 && f[0] == "theme" && f[1] == "mode" {
            let w = f[2].to_string();
            self.set_color_mode_cmd(&w);
            return None;
        }
        if f.len() == 2 && f[0] == "color" {
            let w = f[1].to_string();
            self.set_color_mode_cmd(&w);
            return None;
        }
        if f.len() == 2 && f[0] == "theme" && f[1] == "mode" {
            self.command_bar
                .add_message("Usage: theme mode <auto|dark|light>");
            return None;
        }
        if f.len() == 1 && f[0] == "color" {
            self.command_bar
                .add_message("Usage: color <auto|dark|light>");
            return None;
        }

        const SPEED_NAMES: [&str; 4] = ["OFF", "FAST", "NORMAL", "CINEMATIC"];
        if !f.is_empty() && f[0] == "anim" {
            if f.len() == 1 {
                if let Some(g) = self.game.clone() {
                    let rules = {
                        let mut g = g.borrow_mut();
                        g.rules.anim_speed = (g.rules.anim_speed + 1) % 4;
                        g.rules.clone()
                    };
                    let speed = rules.anim_speed;
                    self.options_modal.set_rules(rules);
                    self.command_bar.add_message(&format!(
                        "Combat animation speed set to {}",
                        SPEED_NAMES[speed as usize]
                    ));
                } else {
                    self.command_bar
                        .add_message("Usage: anim <off|fast|normal|cinematic>");
                }
                return None;
            }
            if f.len() == 2 {
                let new_speed = match f[1] {
                    "off" | "0" => 0,
                    "fast" | "1" => 1,
                    "normal" | "2" => 2,
                    "cinematic" | "3" => 3,
                    other => {
                        self.command_bar.add_message(&format!(
                            "Invalid animation speed: {} (expected off, fast, normal, or cinematic)",
                            quote(other)
                        ));
                        return None;
                    }
                };
                if let Some(g) = self.game.clone() {
                    let rules = {
                        let mut g = g.borrow_mut();
                        g.rules.anim_speed = new_speed;
                        g.rules.clone()
                    };
                    self.options_modal.set_rules(rules);
                }
                self.command_bar.add_message(&format!(
                    "Combat animation speed set to {}",
                    SPEED_NAMES[new_speed as usize]
                ));
                return None;
            }
            self.command_bar
                .add_message("Usage: anim <off|fast|normal|cinematic>");
            return None;
        }

        match trimmed.as_str() {
            "target" => {
                if self.no_targets() {
                    self.command_bar
                        .add_message("Sensors detect no hostile targets in sector.");
                    return None;
                }
                self.open_target_lock([0, 0]);
                return None;
            }
            "srscan" => {
                self.command_bar.add_message("Short-range scan updated.");
                return None;
            }
            "lrscan" => {
                if self.game.is_none() {
                    self.command_bar.add_message("No active game.");
                    return None;
                }
                match self.dispatch(&Action::LRScan(ActionLRScan)) {
                    Err(e) => {
                        self.command_bar
                            .add_message(&format!("LONG-RANGE SENSORS DAMAGED. {}", e));
                    }
                    Ok(events) => {
                        for ev in &events {
                            if let Event::LRScanCompleted(s) = ev {
                                if s.relayed_by_base {
                                    self.command_bar.add_message(
                                        "Starbase relay: Long-range scan complete. Star chart updated.",
                                    );
                                } else {
                                    self.command_bar.add_message("Long-range scan complete. Star chart updated for 3x3 surrounding quadrants.");
                                }
                            }
                        }
                    }
                }
                return None;
            }
            "status" => {
                self.command_bar.add_message("Ship status nominal.");
                return None;
            }
            "dam" | "damage" | "damages" => return self.open_damage_schematic(),
            "chart" | "map" => return self.open_galactic_chart(),
            "saves" | "thaw" => {
                let _ = self.save_browser.refresh();
                self.active_modal = MODAL_SAVE_BROWSER;
                self.command_bar.blur();
                return None;
            }
            "score" | "scores" | "halloffame" | "hof" => return self.open_hall_of_fame(false),
            "opts" | "options" | "settings" => return self.open_options_modal(),
            "tour" | "orders" => {
                self.display_tour_orders();
                return None;
            }
            "help" | "man" | "manual" | "docs" | "codex" | "guide" => return self.open_manual(""),
            _ => {}
        }

        for p in ["help ", "man ", "manual ", "docs ", "codex ", "guide "] {
            if let Some(rest) = trimmed.strip_prefix(p) {
                let topic = trim_space(rest).to_string();
                let canonical = match topic.as_str() {
                    "nav" | "move" | "warp" => "nav",
                    "tor" | "torpedo" | "torpedoes" | "target" | "reticle" => "tor",
                    "pha" | "phaser" | "phasers" => "pha",
                    "she" | "shield" | "shields" | "def" => "she",
                    "doc" | "dock" => "doc",
                    "chart" | "map" => "chart",
                    "saves" | "thaw" | "freeze" => "saves",
                    "opts" | "options" | "settings" => "options",
                    _ => "",
                };
                if canonical.is_empty() {
                    if p.starts_with("help") {
                        self.no_help_msg(&topic);
                    }
                } else {
                    self.help_lines(canonical);
                }
                return self.open_manual(&topic);
            }
        }

        if trimmed.starts_with("thaw ") {
            let raw = trim_space(text);
            let path = trim_space(raw.get(5..).unwrap_or("")).to_string();
            match persist::load_game(&path) {
                Err(e) => self
                    .command_bar
                    .add_message(&format!("Failed to thaw {}: {}", path, e)),
                Ok(loaded) => {
                    let sd = loaded.borrow().stardate;
                    self.game = Some(loaded);
                    self.selected_sector = [0, 0];
                    self.command_bar.add_message(&format!(
                        "Mission thawed: {} (Stardate {})",
                        gopath::base(&path),
                        fmt_f(sd, 1)
                    ));
                }
            }
            return None;
        }

        let parsed = parse_command(text);
        if let Some(e) = &parsed.error {
            self.command_bar.add_message(&e.to_string());
            return None;
        }

        if !parsed.special.is_empty() {
            let sp = parsed.special.as_str();
            match sp {
                "quit" => return tea::quit(),
                "options" => return self.open_options_modal(),
                "scenarios" => {
                    self.scenario_modal.reset();
                    self.active_modal = MODAL_SCENARIO;
                    self.command_bar.blur();
                    return None;
                }
                "tour" | "orders" => {
                    self.display_tour_orders();
                    return None;
                }
                "help" => {
                    self.command_bar.add_message("COMMANDS: nav | tor | pha | she | doc | chart | saves | theme | options | tour | orders");
                    self.command_bar
                        .add_message("Type 'help <command>' (e.g. 'help nav') for detailed guide.");
                    self.command_bar.add_message("HOTKEYS: [Ctrl+P] Spock Palette | [Ctrl+G] Star Chart | [Ctrl+O] Saves | [O] Options | [T] Target Lock | [F2] Theme | [F1/?] Manual");
                    return self.open_manual("");
                }
                "help nav" | "help tor" | "help pha" | "help she" | "help doc" | "help chart"
                | "help saves" | "help options" => {
                    let topic = &sp[5..];
                    self.help_lines(topic);
                    return self.open_manual(topic);
                }
                "theme" => {
                    self.next_theme();
                    let name = self.theme_or_default().name();
                    self.command_bar
                        .add_message(&format!("Theme switched to {}", name));
                    return None;
                }
                _ => {}
            }
            if let Some(cmd_name) = sp.strip_prefix("help ") {
                let cmd_name = cmd_name.to_string();
                self.no_help_msg(&cmd_name);
                return self.open_manual(&cmd_name);
            }
            if let Some(name) = sp.strip_prefix("theme ") {
                let mut th = theme::get_theme(trim_space(name));
                if let Some(cur) = self.theme {
                    th = th.with_color_mode(cur.color_mode());
                }
                self.apply_theme(Some(th));
                let name = self.theme_or_default().name();
                self.command_bar
                    .add_message(&format!("Theme switched to {}", name));
                return None;
            }
        }

        if let Some(action) = parsed.action {
            if self.game.is_none() {
                self.command_bar.add_message("Error: no active game");
                return None;
            }
            let prev_cond = self.condition();
            let mut events = match self.dispatch(&action) {
                Err(e) => {
                    self.command_bar.add_message(&e.to_string());
                    return None;
                }
                Ok(ev) => ev,
            };
            self.execute_klingon_turn_if_active(&action, &mut events);
            let bell_cmd = self.log_events(&events);
            for ev in &events {
                if let Event::ShipMoved(mv) = ev {
                    if mv.from_quad != mv.to_quad {
                        self.selected_sector = [0, 0];
                    }
                }
                if let Event::GameOver(go) = ev {
                    return self.handle_game_over(go.clone());
                }
            }
            let mut anim_cmd = None;
            if self.anim_speed() > 0 {
                if let Some(a) = self.create_combat_animation(&action, &events) {
                    anim_cmd = self.start_combat_animation(a);
                }
            }
            let pulse = self.check_red_alert_cmd(prev_cond);
            return tea::batch(vec![anim_cmd, pulse, bell_cmd]);
        }
        None
    }

    /// Go `toggleSound`.
    pub fn toggle_sound(&mut self) -> Option<Cmd> {
        let enabled = !self.sound_enabled();
        self.set_sound_enabled(enabled);
        if enabled {
            self.command_bar.add_message("*** Audio: Enabled ***");
        } else {
            self.command_bar.add_message("*** Audio: Muted ***");
        }
        None
    }

    /// Go `logEvents`: logs events, plays their sounds and checks the visual bell.
    pub fn log_events(&mut self, events: &[Event]) -> Option<Cmd> {
        if let Some(d) = &self.audio_dispatcher {
            if !events.is_empty() {
                d.dispatch_events(events);
            }
        }
        let mut fired = false;
        let mut hit = false;
        for ev in events {
            match ev {
                Event::GameOver(_) => continue,
                Event::TorpedoFired(_) => fired = true,
                Event::TorpedoHit(_) | Event::SingularityAbsorption(_) => hit = true,
                _ => {}
            }
            let s = format_event(ev);
            if !s.is_empty() {
                self.command_bar.add_message(&s);
            }
        }
        if fired && !hit {
            self.command_bar.add_message("Torpedo missed.");
        }
        self.check_visual_bell(events)
    }

    /// Go `startCombatAnimation`.
    pub fn start_combat_animation(&mut self, a: AnimationRef) -> Option<Cmd> {
        self.anim_id += 1;
        self.active_anim = Some(a.clone());
        let f = a.borrow_mut().step();
        self.grid.set_anim_overrides(f.overrides);
        anim::tick_cmd(self.anim_id, 1, f.duration)
    }

    /// Go `createCombatAnimation`.
    pub fn create_combat_animation(
        &self,
        action: &Action,
        events: &[Event],
    ) -> Option<AnimationRef> {
        let g = self.game.as_ref()?.borrow();
        let speed = g.rules.anim_speed;
        match action {
            Action::FireTorpedo(act) => {
                let mut start = g.enterprise.sector;
                let mut hit_ev = None;
                let mut absorb_ev = None;
                let mut fired_angle = act.angle;
                let mut has_fired_angle = act.angle != 0.0;
                for ev in events {
                    match ev {
                        Event::TorpedoFired(e) => {
                            start = e.origin;
                            fired_angle = e.angle;
                            has_fired_angle = true;
                        }
                        Event::TorpedoHit(e) => hit_ev = Some(e.clone()),
                        Event::SingularityAbsorption(e) => absorb_ev = Some(e.clone()),
                        _ => {}
                    }
                }
                let hit = hit_ev.is_some();
                let mut hit_entity = EntityType::Klingon;
                let end = if let Some(h) = &hit_ev {
                    hit_entity = h.entity;
                    h.target
                } else if let Some(a) = &absorb_ev {
                    a.sector
                } else if act.target != [0, 0] {
                    act.target
                } else if has_fired_angle {
                    trace_torpedo_boundary(start, fired_angle)
                } else {
                    start
                };
                Some(Rc::new(RefCell::new(
                    anim::new_torpedo_animation_with_entity(start, end, hit, hit_entity, speed),
                )))
            }
            Action::FirePhasers(_) => {
                let start = g.enterprise.sector;
                let mut targets = Vec::new();
                let mut hits = Vec::new();
                for ev in events {
                    if let Event::PhaserHit(h) = ev {
                        targets.push(h.target);
                        hits.push(true);
                    }
                }
                if targets.is_empty() {
                    return None;
                }
                Some(Rc::new(RefCell::new(anim::new_multi_phaser_animation(
                    start, &targets, &hits, speed,
                ))))
            }
            Action::PhaserDirect(act) => {
                let start = g.enterprise.sector;
                Some(Rc::new(RefCell::new(anim::new_multi_phaser_animation(
                    start,
                    &[act.target_sector],
                    &[true],
                    speed,
                ))))
            }
            _ => None,
        }
    }
}

/// Go `traceTorpedoBoundary`.
pub fn trace_torpedo_boundary(start: Coord, angle: f64) -> Coord {
    let mut r = start[0] as f64;
    let mut c = start[1] as f64;
    let dr = -gomath::sin(angle) * 0.25;
    let dc = gomath::cos(angle) * 0.25;
    let mut last = start;
    for _ in 0..40 {
        r += dr;
        c += dc;
        let ir = gomath::round(r) as i32;
        let ic = gomath::round(c) as i32;
        if !(1..=8).contains(&ir) || !(1..=8).contains(&ic) {
            break;
        }
        last = [ir, ic];
    }
    last
}

fn format_game_over(e: &EventGameOver) -> String {
    match e.reason {
        GameOverReason::Won => {
            format!("*** MISSION ACCOMPLISHED! Score: {} ***", fmt_f(e.score, 0))
        }
        GameOverReason::Energy => "*** GAME OVER: Enterprise out of energy ***".into(),
        GameOverReason::Destroyed => "*** GAME OVER: Enterprise destroyed ***".into(),
        GameOverReason::Time => "*** GAME OVER: Federation conquered - time expired ***".into(),
        GameOverReason::Stranded => "*** GAME OVER: Enterprise stranded in space ***".into(),
        _ => "*** GAME OVER ***".into(),
    }
}

/// The ship name for a phaser hit on `f`.
fn faction_name(f: FactionType) -> &'static str {
    match f {
        FactionType::Romulan => "Romulan",
        FactionType::Tholian => "Tholian",
        _ => "Klingon",
    }
}

/// Go `formatEvent`: human-readable tactical log line ("" for silent events).
pub fn format_event(ev: &Event) -> String {
    match ev {
        Event::ShieldTransfer(e) => format!(
            "Shields: {}  Energy: {}",
            fmt_f(e.new_shields, 0),
            fmt_f(e.new_energy, 0)
        ),
        Event::TorpedoFired(e) => format!("Torpedo fired on course {}", fmt_f(e.angle, 2)),
        Event::TorpedoHit(e) => {
            if e.destroyed {
                format!(
                    "*** Target destroyed at [{},{}] ({} damage) ***",
                    e.target[0],
                    e.target[1],
                    fmt_f(e.damage, 0)
                )
            } else {
                let mut s = format!(
                    "Hit on [{},{}]: {} units damage",
                    e.target[0],
                    e.target[1],
                    fmt_f(e.damage, 0)
                );
                if e.shield_damage > 0.0 {
                    s += &format!(" (shields absorbed {})", fmt_f(e.shield_damage, 0));
                }
                s
            }
        }
        Event::PhaserFired(e) => format!("Phasers discharged with {} energy", fmt_f(e.energy, 0)),
        Event::PhaserHit(e) => {
            let ship = faction_name(e.faction);
            if e.destroyed {
                format!(
                    "*** {} #{} destroyed ({} damage) ***",
                    ship,
                    e.klingon_id,
                    fmt_f(e.damage, 0)
                )
            } else {
                format!(
                    "Hit on {} #{}: {} units damage",
                    ship,
                    e.klingon_id,
                    fmt_f(e.damage, 0)
                )
            }
        }
        Event::Docked(e) => format!(
            "Docked with Starbase at [{},{}]. Systems refueled.",
            e.starbase[0], e.starbase[1]
        ),
        Event::StarbaseSurveillance(e) => format!(
            "Starbase at [{},{}] downloaded surveillance. Updated {} quadrants.",
            e.starbase_coord[0], e.starbase_coord[1], e.updated_quads
        ),
        Event::LRScanCompleted(e) => {
            if e.relayed_by_base {
                "Starbase relay: Long-range scan complete. Star chart updated.".into()
            } else {
                "Long-range scan complete. Star chart updated for 3x3 surrounding quadrants.".into()
            }
        }
        Event::ShipMoved(e) => format!(
            "Ship arrived at Quadrant [{},{}], Sector [{},{}]",
            e.to_quad[0], e.to_quad[1], e.to_sector[0], e.to_sector[1]
        ),
        Event::ObstacleEncountered(e) => format!(
            "Maneuver stopped: obstacle at sector [{},{}]",
            e.sector[0], e.sector[1]
        ),
        Event::ConditionChanged(e) => {
            format!("Alert status changed to {}", condition_string(e.to))
        }
        Event::KlingonCounterAttack(e) => {
            if e.hull_damage > 0.0 && e.shield_damage > 0.0 {
                format!(
                    "Klingon #{} returned fire: {} damage (Shields absorbed {}, Hull hit: -{} Energy)",
                    e.enemy_id,
                    fmt_f(e.damage, 0),
                    fmt_f(e.shield_damage, 0),
                    fmt_f(e.hull_damage, 0)
                )
            } else if e.hull_damage > 0.0 {
                format!(
                    "Klingon #{} returned fire: {} damage (HULL HIT: -{} Energy)",
                    e.enemy_id,
                    fmt_f(e.damage, 0),
                    fmt_f(e.hull_damage, 0)
                )
            } else if e.shield_damage > 0.0 {
                format!(
                    "Klingon #{} returned fire: {} damage (Shields absorbed {})",
                    e.enemy_id,
                    fmt_f(e.damage, 0),
                    fmt_f(e.shield_damage, 0)
                )
            } else {
                format!(
                    "Klingon #{} returned fire: {} damage",
                    e.enemy_id,
                    fmt_f(e.damage, 0)
                )
            }
        }
        Event::KlingonCloakState(e) => {
            if e.cloaked {
                format!("Klingon #{} engaged cloaking device.", e.klingon_id)
            } else {
                format!("Klingon #{} decloaked!", e.klingon_id)
            }
        }
        Event::SubsystemDamaged(e) => format!(
            "{} damaged! Repair time: {} stardates",
            device_string(e.device),
            fmt_f(e.repair_time, 1)
        ),
        Event::SubsystemRepaired(e) => {
            format!("{} repaired and operational", device_string(e.device))
        }
        Event::GameOver(e) => format_game_over(e),
        _ => String::new(),
    }
}

/// Go `conditionString`.
pub fn condition_string(c: ConditionType) -> &'static str {
    match c {
        ConditionType::Green => "CONDITION GREEN",
        ConditionType::Yellow => "CONDITION YELLOW",
        ConditionType::Red => "CONDITION RED",
        ConditionType::Docked => "CONDITION DOCKED",
        _ => "CONDITION GREEN",
    }
}

/// Go `deviceString`.
pub fn device_string(d: DeviceID) -> &'static str {
    match d {
        DeviceID::Warp => "Warp Engines",
        DeviceID::SRSensors => "Short-Range Sensors",
        DeviceID::LRSensors => "Long-Range Sensors",
        DeviceID::Phasers => "Phasers",
        DeviceID::PhotonTubes => "Photon Tubes",
        DeviceID::DamageControl => "Damage Control",
        DeviceID::Shields => "Shields",
        DeviceID::Computer => "Computer",
        _ => "Subsystem",
    }
}
