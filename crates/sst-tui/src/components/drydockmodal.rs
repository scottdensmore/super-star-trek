//! Port of `pkg/tui/components/drydockmodal` (drydockmodal.go): the
//! Starbase Drydock & Refit Facility overlay shown between tour sectors.
//! The tour is shared with the root model (Go `*engine.TourState`), so
//! purchases mutate it through [`TourRef`].

use sst_engine::campaign::TourRef;
use sst_engine::refits::{purchase_refit, refit_catalog, RefitID};
use sst_tuikit::lipgloss::{self, TerminalColor};
use sst_tuikit::tea::{self, Cmd, KeyMsg, Msg};

use crate::theme::{default_theme, Theme};

/// Go `DisembarkMsg`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisembarkMsg;

/// Go `RefitPurchasedMsg`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefitPurchasedMsg {
    pub refit_id: RefitID,
    pub tier: i32,
}

/// Go `Model`.
#[derive(Clone)]
pub struct Model {
    /// Go `Tour` (shared pointer; `None` is nil).
    pub tour: Option<TourRef>,
    /// Go `Theme`.
    pub theme: Theme,
    /// Go `Width`.
    pub width: i32,
    /// Go `Height`.
    pub height: i32,
    selected_index: i32,
    error_message: String,
}

struct DrydockStyles {
    border_normal: TerminalColor,
    foreground: TerminalColor,
    gauge_good: TerminalColor,
    active_command: TerminalColor,
    status_muted: TerminalColor,
    condition_red: TerminalColor,
}

fn get_styles(th: Theme) -> DrydockStyles {
    let b = th.styles();
    DrydockStyles {
        border_normal: b.panel.get_border_top_foreground(),
        foreground: b.command_text.get_foreground(),
        gauge_good: b.condition_green.get_foreground(),
        active_command: b.prompt.get_foreground(),
        status_muted: b.log_text.get_foreground(),
        condition_red: b.condition_red.get_foreground(),
    }
}

/// Go `New(tour, th)`; `None` is Go's nil.
pub fn new(tour: Option<TourRef>, th: impl Into<Option<Theme>>) -> Model {
    Model {
        tour,
        theme: th.into().unwrap_or_else(default_theme),
        width: 80,
        height: 24,
        selected_index: 0,
        error_message: String::new(),
    }
}

const SCHEMATIC: &str = "
      /================\\
     /                  \\
 ===|     NCC - 1701     |===
 #  \\                    /  #
 #   \\==================/   #
 #            ||            #
 #+-------+   ||   +-------+#
  | WARP  |=======| WARP  |
  +-------+       +-------+
";

impl Model {
    /// `SetTheme`.
    pub fn set_theme(&mut self, th: impl Into<Option<Theme>>) {
        self.theme = th.into().unwrap_or_else(default_theme);
    }

    /// `SelectedIndex`.
    pub fn selected_index(&self) -> i32 {
        self.selected_index
    }

    /// The last purchase error (cleared by the next key).
    pub fn error_message(&self) -> &str {
        &self.error_message
    }

    fn tier_of(&self, id: &RefitID) -> i32 {
        self.tour
            .as_ref()
            .and_then(|t| {
                t.borrow()
                    .installed_refits
                    .as_ref()
                    .map(|r| r.get(id).copied().unwrap_or(0))
            })
            .unwrap_or(0)
    }

    /// `Update`: emits [`RefitPurchasedMsg`] / [`DisembarkMsg`].
    pub fn update(&mut self, msg: &Msg) -> Option<Cmd> {
        let k = msg.downcast_ref::<KeyMsg>()?;
        self.error_message.clear();
        let catalog = refit_catalog();
        match k.string().as_str() {
            "up" | "k" => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            "down" | "j" => {
                if self.selected_index < catalog.len() as i32 - 1 {
                    self.selected_index += 1;
                }
            }
            "enter" => {
                if self.selected_index < 0 || self.selected_index as usize >= catalog.len() {
                    return None;
                }
                let def = &catalog[self.selected_index as usize];
                let res = match &self.tour {
                    Some(t) => purchase_refit(Some(&mut t.borrow_mut()), &def.id),
                    None => purchase_refit(None, &def.id),
                };
                if let Err(e) = res {
                    self.error_message = e.to_string();
                    return None;
                }
                let tier = self.tier_of(&def.id);
                let refit_id = def.id.clone();
                return tea::cmd(move || RefitPurchasedMsg { refit_id, tier });
            }
            " " | "space" | "d" | "D" => return tea::cmd(|| DisembarkMsg),
            _ => {}
        }
        None
    }

    /// `View`: the dual-pane dialog.
    pub fn view(&self) -> String {
        let st = get_styles(self.theme);
        let box_style = lipgloss::new_style()
            .border(lipgloss::rounded_border(), &[])
            .border_foreground(std::slice::from_ref(&st.border_normal))
            .width(78)
            .padding(&[0, 1]);
        let header_style = lipgloss::new_style()
            .foreground(st.foreground.clone())
            .bold(true);
        let accent_style = lipgloss::new_style()
            .foreground(st.gauge_good.clone())
            .bold(true);

        let (sec_idx, req_pts, num_installed) = match &self.tour {
            Some(t) => {
                let t = t.borrow();
                (
                    t.sectors_completed,
                    t.requisition_points,
                    t.installed_refits.as_ref().map(|r| r.len()).unwrap_or(0),
                )
            }
            None => (0, 0, 0),
        };
        let title = header_style.render(&format!(
            "★ STARBASE 01 DRYDOCK & REFIT FACILITY ★  (Sector {} Cleared)",
            sec_idx
        ));
        let req_text = accent_style.render(&format!("Requisition: {} PTS", req_pts));
        let gap = (self.width - 4 - lipgloss::width(&title) - lipgloss::width(&req_text)).max(1);
        let spacer = " ".repeat(gap as usize);
        let top_bar = lipgloss::join_horizontal(lipgloss::CENTER, &[&title, &spacer, &req_text]);

        let left_pane = lipgloss::new_style().width(30).render(&format!(
            "{}\nModules Installed: {}/6",
            SCHEMATIC, num_installed
        ));

        let mut items: Vec<String> = Vec::new();
        for (i, refit) in refit_catalog().iter().enumerate() {
            let selected = i as i32 == self.selected_index;
            let cursor = if selected { "> " } else { "  " };
            let tier = self.tier_of(&refit.id);
            let tier_str = format!("[Tier {}/3]", tier);
            let cost_str = if tier < 3 {
                format!("Cost: {}", refit.tier_costs[tier as usize])
            } else {
                "[MAX TIER]".to_string()
            };
            let mut item_style = lipgloss::new_style().foreground(st.foreground.clone());
            if selected {
                item_style = item_style.bold(true).foreground(st.active_command.clone());
            }
            let line = format!("{}{:<26} {:<12} {}", cursor, refit.name, tier_str, cost_str);
            let desc = format!("     {}", refit.description);
            items.push(item_style.render(&line));
            items.push(
                lipgloss::new_style()
                    .foreground(st.status_muted.clone())
                    .render(&desc),
            );
        }
        let right_pane = lipgloss::new_style().width(44).render(&items.join("\n"));
        let content = lipgloss::join_horizontal(lipgloss::TOP, &[&left_pane, &right_pane]);

        let footer = if !self.error_message.is_empty() {
            lipgloss::new_style()
                .foreground(st.condition_red.clone())
                .render(&format!("Error: {}", self.error_message))
        } else {
            lipgloss::new_style().foreground(st.status_muted.clone()).render(
                "[↑/↓ / j/k]: Select   [Enter]: Purchase Refit   [Space / D]: Disembark to Next Sector",
            )
        };

        box_style.render(&format!("{}\n\n{}\n\n{}", top_bar, content, footer))
    }
}

#[cfg(test)]
mod parity_tests;
#[cfg(test)]
mod tests;
