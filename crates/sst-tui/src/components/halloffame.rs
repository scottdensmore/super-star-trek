//! Port of `pkg/tui/components/halloffame` (halloffame.go): the dual-tab
//! Mission Telemetry / Starfleet Hall of Fame modal with callsign entry.
//!
//! Go keeps a `*engine.Leaderboard` that the caller also holds, so a score
//! recorded here is visible through the caller's pointer; that is
//! [`LeaderboardRef`]. `time.Now()` (the entry date) comes from an injectable
//! [`Clock`].

use std::cell::RefCell;
use std::rc::Rc;

use sst_engine::gotime::{Clock, SystemClock};
use sst_engine::leaderboard::{Leaderboard, ScoreEntry};
use sst_engine::persist;
use sst_engine::score::{rank_for_score, ScoreBreakdown};
use sst_tuikit::ansi;
use sst_tuikit::bubbles::textinput;
use sst_tuikit::lipgloss::{self, Style};
use sst_tuikit::tea::{self, Cmd, KeyMsg, Msg};

use crate::theme::{default_theme, Styles, Theme};

const TAB_TELEMETRY: i32 = 0;
const TAB_LEADERBOARD: i32 = 1;

/// Shared leaderboard handle (Go `*engine.Leaderboard`).
pub type LeaderboardRef = Rc<RefCell<Leaderboard>>;

/// Go `CloseModalMsg`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CloseModalMsg;

/// Go `ScoreRecordedMsg`.
#[derive(Clone, Debug, PartialEq)]
pub struct ScoreRecordedMsg {
    pub entry: ScoreEntry,
}

/// Go `Model`.
#[derive(Clone)]
pub struct Model {
    width: i32,
    height: i32,
    active_tab: i32,
    score: ScoreBreakdown,
    leaderboard: Option<LeaderboardRef>,
    prompt_name: bool,
    text_input: textinput::Model,
    theme: Theme,
    storage_path: String,
    clock: Rc<dyn Clock>,
}

struct HofStyles {
    s: Styles,
    border: Style,
}

fn get_styles(th: Theme) -> HofStyles {
    let s = th.styles();
    let border = lipgloss::new_style().foreground(s.panel.get_border_top_foreground());
    HofStyles { s, border }
}

/// Go `New(th, width, height, storagePath)`; `None` is Go's nil theme.
pub fn new(th: impl Into<Option<Theme>>, width: i32, height: i32, storage_path: &str) -> Model {
    let th = th.into().unwrap_or_else(default_theme);
    let width = if width <= 0 { 66 } else { width };
    let height = if height <= 0 { 18 } else { height };

    let mut ti = textinput::new();
    ti.prompt = String::new();
    ti.placeholder = "Enter callsign...".into();
    ti.char_limit = 20;
    ti.width = 20;
    let styles = th.styles();
    ti.prompt_style = styles.prompt.clone();
    ti.text_style = styles.command_text.clone();

    let mut lb = Leaderboard::default_board();
    if !storage_path.is_empty() {
        if let Ok(loaded) = persist::load_leaderboard(Some(storage_path)) {
            lb = loaded;
        }
    }

    Model {
        width,
        height,
        active_tab: TAB_TELEMETRY,
        score: ScoreBreakdown::default(),
        leaderboard: Some(Rc::new(RefCell::new(lb))),
        prompt_name: false,
        text_input: ti,
        theme: th,
        storage_path: storage_path.to_string(),
        clock: Rc::new(SystemClock),
    }
}

impl Model {
    /// Injects the clock used for recorded entry dates (Go `time.Now()`).
    pub fn set_clock(&mut self, clock: Rc<dyn Clock>) {
        self.clock = clock;
    }

    /// `SetTheme`: also restyles the callsign input.
    pub fn set_theme(&mut self, th: impl Into<Option<Theme>>) {
        self.theme = th.into().unwrap_or_else(default_theme);
        let styles = self.theme.styles();
        self.text_input.prompt_style = styles.prompt.clone();
        self.text_input.text_style = styles.command_text.clone();
    }

    /// The active theme.
    pub fn theme(&self) -> Theme {
        self.theme
    }

    /// Width and height (`m.width`, `m.height`).
    pub fn size(&self) -> (i32, i32) {
        (self.width, self.height)
    }

    /// Whether the callsign prompt is active (`m.promptName`).
    pub fn prompt_name(&self) -> bool {
        self.prompt_name
    }

    /// 0 = telemetry tab, 1 = leaderboard tab (`m.activeTab`).
    pub fn active_tab(&self) -> i32 {
        self.active_tab
    }

    /// The shared leaderboard (`m.leaderboard`).
    pub fn leaderboard(&self) -> Option<LeaderboardRef> {
        self.leaderboard.clone()
    }

    /// `SetState(score, lb, promptName)`; `lb == None` keeps the current board.
    pub fn set_state(
        &mut self,
        score: ScoreBreakdown,
        lb: Option<LeaderboardRef>,
        prompt_name: bool,
    ) {
        self.score = score;
        if self.score.rank_badge.is_empty() {
            let (title, badge) = rank_for_score(self.score.total_score);
            self.score.rank_title = title.into();
            self.score.rank_badge = badge.into();
        }
        if lb.is_some() {
            self.leaderboard = lb;
        } else if self.leaderboard.is_none() {
            self.leaderboard = Some(Rc::new(RefCell::new(Leaderboard::default_board())));
        }
        self.prompt_name = prompt_name;
        if prompt_name {
            self.active_tab = TAB_LEADERBOARD;
            self.text_input.set_value("");
            let _ = self.text_input.focus();
        } else {
            self.active_tab = TAB_TELEMETRY;
        }
    }

    fn toggle_tab(&mut self) {
        self.active_tab = if self.active_tab == TAB_TELEMETRY {
            TAB_LEADERBOARD
        } else {
            TAB_TELEMETRY
        };
    }

    /// `Update`: emits [`CloseModalMsg`] / [`ScoreRecordedMsg`].
    pub fn update(&mut self, msg: &Msg) -> Option<Cmd> {
        if let Some(k) = msg.downcast_ref::<KeyMsg>() {
            if self.prompt_name {
                return match k.typ {
                    tea::KEY_ESC => {
                        self.prompt_name = false;
                        tea::cmd(|| CloseModalMsg)
                    }
                    tea::KEY_ENTER => {
                        let mut name = self.text_input.value().trim().to_string();
                        if name.is_empty() {
                            name = "Unknown Captain".into();
                        }
                        let entry = ScoreEntry {
                            captain_name: name,
                            score: self.score.total_score,
                            rank: self.score.rank_badge.clone(),
                            stardate: self.score.elapsed_stardates,
                            date: self.clock.now(),
                            game_won: self.score.game_won,
                            ..Default::default()
                        };
                        if let Some(lb) = &self.leaderboard {
                            lb.borrow_mut().add(entry.clone());
                            if !self.storage_path.is_empty() {
                                let _ = persist::save_leaderboard(&lb.borrow(), &self.storage_path);
                            }
                        }
                        self.prompt_name = false;
                        tea::cmd(move || ScoreRecordedMsg { entry })
                    }
                    _ => self.text_input.update(msg),
                };
            }

            match k.typ {
                tea::KEY_TAB | tea::KEY_SHIFT_TAB => {
                    self.toggle_tab();
                    return None;
                }
                tea::KEY_LEFT => {
                    self.active_tab = TAB_TELEMETRY;
                    return None;
                }
                tea::KEY_RIGHT => {
                    self.active_tab = TAB_LEADERBOARD;
                    return None;
                }
                tea::KEY_ESC | tea::KEY_ENTER => return tea::cmd(|| CloseModalMsg),
                _ => {}
            }

            match k.string().as_str() {
                "tab" | "shift+tab" => {
                    self.toggle_tab();
                    return None;
                }
                "1" => {
                    self.active_tab = TAB_TELEMETRY;
                    return None;
                }
                "2" => {
                    self.active_tab = TAB_LEADERBOARD;
                    return None;
                }
                "esc" | "enter" | "q" | "Q" | "h" | "H" | " " => return tea::cmd(|| CloseModalMsg),
                _ => {}
            }
        }

        if self.prompt_name {
            return self.text_input.update(msg);
        }
        None
    }

    /// `View`: the 66x18 bordered modal box.
    pub fn view(&self) -> String {
        let styles = get_styles(self.theme);
        let mut inner_w = self.width - 2;
        if inner_w < 10 {
            inner_w = 64;
        }
        let b = &styles.border;
        let format_row = |content: &str| -> String {
            let w = ansi::string_width(content);
            let mut pad = inner_w - w;
            let mut content = content.to_string();
            if pad < 0 {
                content = ansi::truncate(&content, inner_w, "");
                pad = 0;
            }
            format!(
                "{}{}{}{}",
                b.render("│"),
                content,
                " ".repeat(pad as usize),
                b.render("│")
            )
        };

        let (tab1, tab2) = if self.active_tab == TAB_TELEMETRY {
            (
                styles
                    .s
                    .panel_title
                    .render("[1] MISSION DEBRIEF & TELEMETRY"),
                styles.s.log_text.render("[2] HALL OF FAME"),
            )
        } else {
            (
                styles.s.log_text.render("[1] MISSION DEBRIEF & TELEMETRY"),
                styles.s.panel_title.render("[2] STARFLEET HALL OF FAME"),
            )
        };
        let prefix_w = 2 + ansi::string_width(&tab1) + 2 + ansi::string_width(&tab2);
        let top_fill = (self.width - prefix_w - 1).max(0);
        let border_top = format!(
            "{}{}{}{}{}{}",
            b.render("┌─"),
            tab1,
            b.render("──"),
            tab2,
            b.render(&"─".repeat(top_fill as usize)),
            b.render("┐")
        );
        let border_bottom = format!(
            "{}{}{}",
            b.render("└"),
            b.render(&"─".repeat(inner_w.max(0) as usize)),
            b.render("┘")
        );

        let mut rows = vec![border_top];
        if self.active_tab == TAB_TELEMETRY {
            rows.extend(self.render_telemetry_rows(&styles, inner_w, &format_row));
        } else {
            rows.extend(self.render_leaderboard_rows(&styles, inner_w, &format_row));
        }
        rows.push(border_bottom);
        rows.join("\n")
    }

    fn render_telemetry_rows(
        &self,
        styles: &HofStyles,
        inner_w: i32,
        format_row: &dyn Fn(&str) -> String,
    ) -> Vec<String> {
        let sc = &self.score;
        let combine_cols = |left: &str, right: &str| -> String {
            let (mut left, mut right) = (left.to_string(), right.to_string());
            let mut wl = ansi::string_width(&left);
            let mut wr = ansi::string_width(&right);
            let mut gap = inner_w - wl - wr;
            if gap >= 0 {
                return format!("{}{}{}", left, " ".repeat(gap as usize), right);
            }
            let budget_l = inner_w / 2;
            let budget_r = inner_w - budget_l;
            if wl > budget_l && wr <= budget_r {
                left = ansi::truncate(&left, inner_w - wr, "");
            } else if wr > budget_r && wl <= budget_l {
                right = ansi::truncate(&right, inner_w - wl, "");
            } else {
                left = ansi::truncate(&left, budget_l, "");
                right = ansi::truncate(&right, inner_w - ansi::string_width(&left), "");
            }
            wl = ansi::string_width(&left);
            wr = ansi::string_width(&right);
            gap = (inner_w - wl - wr).max(0);
            format!("{}{}{}", left, " ".repeat(gap as usize), right)
        };

        let row1 = combine_cols(
            &styles.s.gauge_label.render(" COMBAT ACHIEVEMENTS"),
            &styles.s.gauge_label.render("PENALTIES & LOSSES"),
        );

        let format_col = |l_label: &str,
                          l_val: i32,
                          l_pts: i32,
                          r_label: &str,
                          r_val: i32,
                          r_pts: i32|
         -> String {
            let mut l_str = format!(" {:<20} {:>2} (+{:>4})", l_label, l_val, l_pts);
            let mut r_str = format!(" {:<19} {:>3} (-{:>4})", r_label, r_val, r_pts);
            let gap = inner_w - ansi::string_width(&l_str) - ansi::string_width(&r_str);
            if gap < 0 {
                let l_pts_str = format!("(+{})", l_pts);
                let r_pts_str = format!("(-{})", r_pts);
                let l_prefix = format!(" {:<19} {:>2}", l_label, l_val);
                let r_prefix = format!(" {:<18} {:>3}", r_label, r_val);
                let l_gap =
                    (32 - ansi::string_width(&l_prefix) - ansi::string_width(&l_pts_str)).max(1);
                let r_gap =
                    (32 - ansi::string_width(&r_prefix) - ansi::string_width(&r_pts_str)).max(1);
                l_str = format!("{}{}{}", l_prefix, " ".repeat(l_gap as usize), l_pts_str);
                r_str = format!("{}{}{}", r_prefix, " ".repeat(r_gap as usize), r_pts_str);
            }
            combine_cols(&l_str, &r_str)
        };

        let row2 = format_col(
            "Klingons Destroyed:",
            sc.klingons_killed,
            sc.klingon_points,
            "Casualties:",
            sc.casualties,
            sc.casualty_penalty,
        );
        let row3 = format_col(
            "Commanders Killed:",
            sc.commanders_killed,
            sc.commander_points,
            "Starbases Lost:",
            sc.starbases_lost,
            sc.starbase_penalty,
        );
        let row4 = format_col(
            "Super-Cmdrs Killed:",
            sc.super_commanders_killed,
            sc.super_commander_points,
            "Distress Calls:",
            sc.help_calls,
            sc.help_penalty,
        );
        let row5 = format_col(
            "Romulans Destroyed:",
            sc.romulans_killed,
            sc.romulan_points,
            "Planets Destroyed:",
            sc.planets_destroyed,
            sc.planet_penalty,
        );
        let row6 = format_col(
            "Romulan Surrenders:",
            sc.romulans_surrendered,
            sc.surrendered_points,
            "Stars Destroyed:",
            sc.stars_destroyed,
            sc.star_penalty,
        );

        let rate_str = format!(" Kill Rate ({:4.2}/SD):", sc.kill_rate);
        let pts_str = if sc.kill_rate_points < 10000 {
            format!("(+{:>4})", sc.kill_rate_points)
        } else {
            format!("(+{})", sc.kill_rate_points)
        };
        let l_rate_gap = (32 - ansi::string_width(&rate_str) - ansi::string_width(&pts_str)).max(1);
        let l_rate = format!("{}{}{}", rate_str, " ".repeat(l_rate_gap as usize), pts_str);
        let r_ships = if sc.starship_penalty >= 10000 {
            format!(
                " {:<18} {:>3} (-{})",
                "Starships Lost:", sc.starships_lost, sc.starship_penalty
            )
        } else {
            format!(
                " {:<19} {:>3} (-{:>4})",
                "Starships Lost:", sc.starships_lost, sc.starship_penalty
            )
        };
        let row7 = combine_cols(&l_rate, &r_ships);

        let bonus_prefix = " Mission Victory Bonus:";
        let bonus_pts = if sc.win_bonus < 10000 {
            format!("(+{:>4})", sc.win_bonus)
        } else {
            format!("(+{})", sc.win_bonus)
        };
        let l_bonus_gap =
            (33 - ansi::string_width(bonus_prefix) - ansi::string_width(&bonus_pts)).max(1);
        let l_bonus = format!(
            "{}{}{}",
            bonus_prefix,
            " ".repeat(l_bonus_gap as usize),
            bonus_pts
        );
        let r_el_prefix = " Stardates Elapsed:";
        let r_el_val = format!("{:5.1}", sc.elapsed_stardates);
        let r_el_gap =
            (31 - ansi::string_width(r_el_prefix) - ansi::string_width(&r_el_val)).max(1);
        let r_elapsed = format!(
            "{}{}{}",
            r_el_prefix,
            " ".repeat(r_el_gap as usize),
            r_el_val
        );
        let row8 = combine_cols(&l_bonus, &r_elapsed);

        let divider = styles
            .border
            .render(&format!(" {}", "─".repeat((inner_w - 2).max(0) as usize)));

        let row10 = format!(
            " CURRENT NET SCORE:  {}",
            styles
                .s
                .gauge_value
                .render(&format!("{:+6}", sc.total_score))
        );
        let row11 = format!(
            " STARFLEET RANK:     {}",
            styles
                .s
                .panel_title
                .render(&format!("{:<8} {}", sc.rank_badge, sc.rank_title))
        );
        let row12 = format!(" RATING ASSESSMENT:  {:.2} kills/stardate", sc.kill_rate);
        let row15 = " [Tab / 1-2] Switch Tab                 [Esc / Q / H] Dismiss";

        [
            row1.as_str(),
            &row2,
            &row3,
            &row4,
            &row5,
            &row6,
            &row7,
            &row8,
            &divider,
            &row10,
            &row11,
            &row12,
            &divider,
            "",
            row15,
            "",
        ]
        .iter()
        .map(|r| format_row(r))
        .collect()
    }

    fn render_leaderboard_rows(
        &self,
        styles: &HofStyles,
        inner_w: i32,
        format_row: &dyn Fn(&str) -> String,
    ) -> Vec<String> {
        let h_str = format!(
            " {:>2}  {:<20} {:<9} {:>6}  {:<8}  {:<10}",
            "#", "CAPTAIN", "RANK", "SCORE", "STARDATE", "DATE"
        );
        let row1 = styles.s.gauge_label.render(&h_str);
        let divider = styles
            .border
            .render(&format!(" {}", "─".repeat((inner_w - 2).max(0) as usize)));

        let entries: Vec<ScoreEntry> = match &self.leaderboard {
            Some(lb) => lb.borrow().entries.iter().cloned().collect(),
            None => Vec::new(),
        };

        let mut entry_rows = Vec::with_capacity(10);
        for i in 0..10 {
            if i < entries.len() {
                let e = &entries[i];
                let mut c_name = e.captain_name.clone();
                if ansi::string_width(&c_name) > 20 {
                    c_name = ansi::truncate(&c_name, 20, "…");
                }
                let (y, mo, d, _, _, _) = e.date.civil();
                let date_str = format!("{:04}-{:02}-{:02}", y, mo, d);
                let line = format!(
                    " {:>2}. {:<20} {:<9} {:>6}   {:7.1}  {:<10}",
                    i + 1,
                    c_name,
                    e.rank,
                    e.score,
                    e.stardate,
                    date_str
                );
                if i == 0 {
                    entry_rows.push(styles.s.panel_title.render(&line));
                } else {
                    entry_rows.push(line);
                }
            } else {
                entry_rows.push(format!(
                    " {:>2}. {:<20} {:<9} {:>6}   {:>7}  {:<10}",
                    i + 1,
                    "--------------------",
                    "------",
                    "------",
                    "-------",
                    "----------"
                ));
            }
        }

        let (row13, row14, row15) = if self.prompt_name {
            (
                format!(" ENTER CALLSIGN: {}", self.text_input.view()),
                " [ENTER] Save Score to Hall of Fame     [ESC] Cancel".to_string(),
                String::new(),
            )
        } else {
            (
                divider.clone(),
                " Starfleet Command honors all officers serving the Federation.".to_string(),
                " [Tab / 1-2] Switch Tab                 [Esc / Q / H] Dismiss".to_string(),
            )
        };

        let mut result = Vec::with_capacity(16);
        result.push(format_row(&row1));
        result.push(format_row(&divider));
        for er in &entry_rows {
            result.push(format_row(er));
        }
        result.push(format_row(&row13));
        result.push(format_row(&row14));
        result.push(format_row(&row15));
        result.push(format_row(""));
        result
    }
}

#[cfg(test)]
mod parity_tests;
#[cfg(test)]
mod tests;
