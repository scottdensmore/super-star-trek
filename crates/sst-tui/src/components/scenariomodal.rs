//! Port of `pkg/tui/components/scenariomodal` (modal.go): the two-column
//! tactical scenario browser. Top records come from the per-scenario
//! leaderboards; the loader is injectable ([`new_with_loader`]) so tests do
//! not depend on files under the user's config directory.

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use sst_engine::actions::EngineError;
use sst_engine::leaderboard::{Leaderboard, ScoreEntry};
use sst_engine::persist;
use sst_engine::scenarios::{list_scenarios, Scenario, ScenarioID};
use sst_tuikit::ansi;
use sst_tuikit::lipgloss::{self, Style};
use sst_tuikit::tea::{self, Cmd, KeyMsg, Msg};

use crate::theme::{default_theme, Styles, Theme};

/// Go `MsgLaunchScenario`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MsgLaunchScenario {
    pub scenario_id: ScenarioID,
}

/// Go `MsgCloseScenarioModal`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MsgCloseScenarioModal;

/// Loads a scenario leaderboard (Go `engine.LoadScenarioLeaderboard`).
pub type LeaderboardLoader = Rc<dyn Fn(&ScenarioID) -> Result<Leaderboard, EngineError>>;

fn default_loader() -> LeaderboardLoader {
    Rc::new(persist::load_scenario_leaderboard)
}

/// Go `Model`. `Model::default()` is Go's zero `Model{}`.
#[derive(Clone)]
pub struct Model {
    theme: Theme,
    width: i32,
    height: i32,
    scenarios: Vec<Arc<Scenario>>,
    selected_idx: i32,
    leaderboard_cache: HashMap<ScenarioID, Option<ScoreEntry>>,
    loader: LeaderboardLoader,
}

impl Default for Model {
    fn default() -> Self {
        Model {
            theme: default_theme(),
            width: 0,
            height: 0,
            scenarios: Vec::new(),
            selected_idx: 0,
            leaderboard_cache: HashMap::new(),
            loader: default_loader(),
        }
    }
}

struct ModalStyles {
    s: Styles,
    border_fg: Style,
}

fn get_styles(th: Theme) -> ModalStyles {
    let s = th.styles();
    let border_fg = lipgloss::new_style().foreground(s.panel.get_border_top_foreground());
    ModalStyles { s, border_fg }
}

fn pad_right(s: &str, width: i32) -> String {
    let w = ansi::string_width(s);
    if w >= width {
        return ansi::truncate(s, width, "");
    }
    format!("{}{}", s, " ".repeat((width - w) as usize))
}

fn format_badge(difficulty: &str, styles: &ModalStyles) -> String {
    let up = difficulty.to_uppercase();
    let badge = format!("[{}]", up);
    match up.as_str() {
        "EXTREME" => styles.s.condition_red.clone().bold(true).render(&badge),
        "HARD" => styles.s.condition_yellow.clone().bold(true).render(&badge),
        "CHALLENGE" => styles.s.condition_green.clone().bold(true).render(&badge),
        _ => styles.s.gauge_value.render(&badge),
    }
}

fn operational_constraints(id: &ScenarioID) -> &'static [&'static str] {
    if *id == ScenarioID::kobayashi_maru() {
        &[
            "• Neutral Zone border violation (illegal entry)",
            "• Waves of enemy Klingon battle cruisers assault",
        ]
    } else if *id == ScenarioID::mutara_nebula() {
        &[
            "• Shields offline; cloaked flagship hunting",
            "• No sensor lock: torpedo its last bearing",
        ]
    } else if *id == ScenarioID::starbase_siege() {
        &[
            "• Starbase 12 under persistent bombardment",
            "• Must neutralize Klingon fleet before base falls",
        ]
    } else {
        &["• Standard Starfleet tactical engagement rules"]
    }
}

/// Go `New(th, width, height)`; `None` is Go's nil theme.
pub fn new(th: impl Into<Option<Theme>>, width: i32, height: i32) -> Model {
    new_with_loader(th, width, height, default_loader())
}

/// [`new`] with an injected scenario leaderboard loader.
pub fn new_with_loader(
    th: impl Into<Option<Theme>>,
    width: i32,
    height: i32,
    loader: LeaderboardLoader,
) -> Model {
    let mut m = Model {
        theme: th.into().unwrap_or_else(default_theme),
        width: if width <= 0 { 72 } else { width },
        height: if height <= 0 { 18 } else { height },
        scenarios: list_scenarios(),
        selected_idx: 0,
        leaderboard_cache: HashMap::new(),
        loader,
    };
    m.refresh_cache();
    m
}

/// Go `NewModel(th)`: 72x18.
pub fn new_model(th: impl Into<Option<Theme>>) -> Model {
    new(th, 72, 18)
}

impl Model {
    fn refresh_cache(&mut self) {
        for sc in &self.scenarios {
            let top = match (self.loader)(&sc.id) {
                Ok(lb) if !lb.entries.is_empty() => Some(lb.entries[0].clone()),
                _ => None,
            };
            self.leaderboard_cache.insert(sc.id.clone(), top);
        }
    }

    /// `SetDimensions`: non-positive values are ignored.
    pub fn set_dimensions(&mut self, width: i32, height: i32) {
        if width > 0 {
            self.width = width;
        }
        if height > 0 {
            self.height = height;
        }
    }

    /// `SetSize` (same as [`Model::set_dimensions`]).
    pub fn set_size(&mut self, width: i32, height: i32) {
        self.set_dimensions(width, height);
    }

    /// `SetTheme`.
    pub fn set_theme(&mut self, th: impl Into<Option<Theme>>) {
        self.theme = th.into().unwrap_or_else(default_theme);
    }

    /// The active theme.
    pub fn theme(&self) -> Theme {
        self.theme
    }

    /// `SelectedScenario()` (`None` is Go's nil).
    pub fn selected_scenario(&self) -> Option<Arc<Scenario>> {
        if self.selected_idx < 0 {
            return None;
        }
        self.scenarios.get(self.selected_idx as usize).cloned()
    }

    /// `Reset`: re-lists scenarios, refreshes top records, selects the first.
    pub fn reset(&mut self) {
        self.scenarios = list_scenarios();
        self.selected_idx = 0;
        self.refresh_cache();
    }

    fn step(&mut self, d: i32) {
        let n = self.scenarios.len() as i32;
        if n > 0 {
            self.selected_idx = (self.selected_idx + d + n) % n;
        }
    }

    /// `Update`: emits [`MsgLaunchScenario`] / [`MsgCloseScenarioModal`].
    pub fn update(&mut self, msg: &Msg) -> Option<Cmd> {
        let k = msg.downcast_ref::<KeyMsg>()?;
        match k.typ {
            tea::KEY_ESC => return tea::cmd(|| MsgCloseScenarioModal),
            tea::KEY_ENTER => {
                return self.selected_scenario().and_then(|sel| {
                    let scenario_id = sel.id.clone();
                    tea::cmd(move || MsgLaunchScenario { scenario_id })
                });
            }
            tea::KEY_UP => {
                self.step(-1);
                return None;
            }
            tea::KEY_DOWN => {
                self.step(1);
                return None;
            }
            _ => {}
        }
        match k.string().as_str() {
            "esc" | "q" | "Q" => tea::cmd(|| MsgCloseScenarioModal),
            "k" => {
                self.step(-1);
                None
            }
            "j" => {
                self.step(1);
                None
            }
            _ => None,
        }
    }

    /// `View`: the fixed 72x18 dialog.
    pub fn view(&self) -> String {
        let styles = get_styles(self.theme);
        const W: i32 = 72;
        const SIDEBAR_W: i32 = 25;
        const DETAIL_W: i32 = 44;
        let bf = &styles.border_fg;

        let title_str = "[TACTICAL CHALLENGE SIMULATOR]";
        let styled_title = styles.s.panel_title.render(title_str);
        let dashes_top = (W - 3 - ansi::string_width(title_str) - 2).max(0);
        let border_top = format!(
            "{}{} {}{}",
            bf.render("┌─ "),
            styled_title,
            bf.render(&"─".repeat(dashes_top as usize)),
            bf.render("┐")
        );

        let footer_str = "[↑/↓/k/j] Navigate  [Enter] Launch  [Esc/q] Dismiss";
        let styled_footer = styles.s.log_text.render(footer_str);
        let dashes_bottom = (W - 3 - ansi::string_width(footer_str) - 2).max(0);
        let border_bottom = format!(
            "{}{} {}{}",
            bf.render("└─ "),
            styled_footer,
            bf.render(&"─".repeat(dashes_bottom as usize)),
            bf.render("┘")
        );

        let left = self.render_catalog_rows(&styles, SIDEBAR_W);
        let right = self.render_detail_rows(&styles, DETAIL_W);
        let mut rows = Vec::with_capacity(18);
        rows.push(border_top);
        let bar = bf.render("│");
        for i in 0..16 {
            rows.push(format!("{}{}{}{}{}", bar, left[i], bar, right[i], bar));
        }
        rows.push(border_bottom);
        rows.join("\n")
    }

    fn render_catalog_rows(&self, styles: &ModalStyles, width: i32) -> Vec<String> {
        let mut rows = vec![String::new(); 16];
        rows[0] = pad_right(
            &format!(" {}", styles.s.panel_title.render("SCENARIO CATALOG")),
            width,
        );
        rows[1] = pad_right(
            &format!(
                " {}",
                styles.border_fg.render(&"─".repeat((width - 2) as usize))
            ),
            width,
        );
        let mut cur = 2usize;
        for (i, s) in self.scenarios.iter().enumerate() {
            if cur + 2 > 16 {
                break;
            }
            let badge = format_badge(&s.difficulty, styles);
            let name_label = if i as i32 == self.selected_idx {
                format!(
                    " ▶ {}",
                    styles.s.panel_title.clone().bold(true).render(&s.name)
                )
            } else {
                format!("   {}", styles.s.log_text.render(&s.name))
            };
            rows[cur] = pad_right(&name_label, width);
            cur += 1;
            rows[cur] = pad_right(&format!("   {}", badge), width);
            cur += 1;
            if cur < 16 {
                rows[cur] = pad_right("", width);
                cur += 1;
            }
        }
        while cur < 16 {
            rows[cur] = pad_right("", width);
            cur += 1;
        }
        rows
    }

    fn render_detail_rows(&self, styles: &ModalStyles, width: i32) -> Vec<String> {
        let Some(s) = self.selected_scenario() else {
            return vec![pad_right("", width); 16];
        };
        let mut rows = vec![String::new(); 16];
        let rule = || {
            pad_right(
                &format!(
                    " {}",
                    styles.border_fg.render(&"─".repeat((width - 2) as usize))
                ),
                width,
            )
        };
        let badge = format_badge(&s.difficulty, styles);
        rows[0] = pad_right(
            &format!(
                " {} {}",
                styles
                    .s
                    .panel_title
                    .clone()
                    .bold(true)
                    .render(&s.name.to_uppercase()),
                badge
            ),
            width,
        );
        rows[1] = pad_right(
            &format!(
                " {}",
                styles.s.log_text.clone().italic(true).render(&s.subtitle)
            ),
            width,
        );
        rows[2] = rule();
        rows[3] = pad_right(
            &format!(" {}", styles.s.gauge_label.render("BRIEFING:")),
            width,
        );
        for i in 0..4 {
            let line = s.briefing.get(i).map(|l| l.as_str()).unwrap_or("");
            rows[4 + i] = pad_right(&format!(" {}", styles.s.log_text.render(line)), width);
        }
        rows[8] = rule();
        rows[9] = pad_right(
            &format!(
                " {}",
                styles.s.gauge_label.render("OPERATIONAL CONSTRAINTS:")
            ),
            width,
        );
        let c = operational_constraints(&s.id);
        let c1 = c.first().copied().unwrap_or("");
        let c2 = c.get(1).copied().unwrap_or("");
        rows[10] = pad_right(&format!(" {}", styles.s.log_text.render(c1)), width);
        rows[11] = pad_right(&format!(" {}", styles.s.log_text.render(c2)), width);
        rows[12] = rule();
        rows[13] = pad_right(
            &format!(
                " {}",
                styles.s.gauge_label.render("TOP LEADERBOARD RECORD:")
            ),
            width,
        );
        let top = self.leaderboard_cache.get(&s.id).cloned().flatten();
        if let Some(top) = top {
            let rec1 = format!(
                "{}  {}",
                styles.s.panel_title.render(&top.captain_name),
                styles.s.gauge_value.render(&format!("{} pts", top.score))
            );
            let (y, mo, d, _, _, _) = top.date.civil();
            let rec2 = format!(
                "Rank: {} | Stardate: {:.1} | {:04}-{:02}-{:02}",
                top.rank, top.stardate, y, mo, d
            );
            rows[14] = pad_right(&format!(" {}", rec1), width);
            rows[15] = pad_right(&format!(" {}", styles.s.log_text.render(&rec2)), width);
        } else {
            rows[14] = pad_right(
                &format!(" {}", styles.s.log_text.render("No record recorded yet.")),
                width,
            );
            rows[15] = pad_right(
                &format!(
                    " {}",
                    styles
                        .s
                        .log_text
                        .render("Be the first captain to conquer this challenge.")
                ),
                width,
            );
        }
        rows
    }
}

#[cfg(test)]
mod parity_tests;
#[cfg(test)]
mod tests;
