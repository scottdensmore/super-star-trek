//! Port of `pkg/tui/components/commandpalette` (palette.go): the Spock
//! Command Palette, a fuzzy-filtered `bubbles/list` of command prefixes.

use std::rc::Rc;

use sst_tuikit::bubbles::list::{self, DefaultItem, Item};
use sst_tuikit::lipgloss;
use sst_tuikit::tea::{self, Cmd, KeyMsg, Msg, WindowSizeMsg};

use crate::theme::{default_theme, Theme};

/// Go `PaletteItem`: an entry of the command catalog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaletteItem {
    title: String,
    desc: String,
    prefix: String,
    parameterized: bool,
}

impl PaletteItem {
    fn new(title: &str, desc: &str, prefix: &str, parameterized: bool) -> Self {
        PaletteItem {
            title: title.into(),
            desc: desc.into(),
            prefix: prefix.into(),
            parameterized,
        }
    }
    /// `Title()`.
    pub fn title(&self) -> String {
        self.title.clone()
    }
    /// `Description()`.
    pub fn description(&self) -> String {
        self.desc.clone()
    }
    /// `FilterValue()`.
    pub fn filter_value(&self) -> String {
        format!("{} {}", self.title, self.desc)
    }
    /// `Prefix()`.
    pub fn prefix(&self) -> String {
        self.prefix.clone()
    }
    /// `Parameterized()`.
    pub fn parameterized(&self) -> bool {
        self.parameterized
    }
}

impl Item for PaletteItem {
    fn filter_value(&self) -> String {
        PaletteItem::filter_value(self)
    }
    fn as_default_item(&self) -> Option<&dyn DefaultItem> {
        Some(self)
    }
}

impl DefaultItem for PaletteItem {
    fn title(&self) -> String {
        self.title.clone()
    }
    fn description(&self) -> String {
        self.desc.clone()
    }
}

/// Go `CommandSelectedMsg`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandSelectedMsg {
    pub command_prefix: String,
    pub parameterized: bool,
}

/// Go `ClosePaletteMsg`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClosePaletteMsg;

/// Go `Model`.
#[derive(Clone)]
pub struct Model {
    theme: Theme,
    list: list::Model,
    catalog: Vec<PaletteItem>,
    query: String,
    width: i32,
    height: i32,
}

fn default_catalog() -> Vec<PaletteItem> {
    let p = PaletteItem::new;
    vec![
        // Tactical & Combat
        p(
            "TOR",
            "Fire photon torpedo along bearing or sector (tor <course|r c>)",
            "tor ",
            true,
        ),
        p(
            "PHA",
            "Fire ship phaser bank with energy (pha <energy>)",
            "pha ",
            true,
        ),
        p(
            "SHE",
            "Transfer energy between warp and shields (she <amount>)",
            "she ",
            true,
        ),
        p("TARGET", "Open Tactical Target Lock HUD", "target", false),
        // Navigation
        p(
            "NAV",
            "Maneuver: nav q <r c> (quad), nav s <r c> (sector), or nav <course warp>",
            "nav ",
            true,
        ),
        p(
            "DOC",
            "Dock with adjacent Starbase for fuel & repair",
            "doc",
            false,
        ),
        p(
            "SRSCAN",
            "Refresh short-range sensor tactical grid",
            "srscan",
            false,
        ),
        p(
            "LRSCAN",
            "Scan adjacent 3x3 quadrant cluster",
            "lrscan",
            false,
        ),
        // Reports
        p(
            "STATUS",
            "Review ship status, condition, and stardate",
            "status",
            false,
        ),
        p(
            "DAM",
            "Review subsystem device damage repair times",
            "dam",
            false,
        ),
        p(
            "CHART",
            "Interactive 8x8 galactic star chart and warp planner (Ctrl+G)",
            "chart",
            false,
        ),
        p(
            "SAVES",
            "Browse, inspect, and load saved missions (Ctrl+O)",
            "saves",
            false,
        ),
        p(
            "SCENARIOS",
            "Browse and launch tactical challenge scenarios",
            "scenarios",
            false,
        ),
        // Settings & Help
        p(
            "THEME: Modern",
            "Switch theme to Starfleet Modern (Electric Cyan/Navy)",
            "theme modern",
            false,
        ),
        p(
            "THEME: LCARS",
            "Switch theme to 24th Century LCARS (Gold/Purple)",
            "theme lcars",
            false,
        ),
        p(
            "THEME: CRT",
            "Switch theme to Phosphor Monochrome CRT (Green/Amber)",
            "theme crt",
            false,
        ),
        p(
            "THEME MODE AUTO",
            "Set theme adaptation to automatic terminal background detection",
            "theme mode auto",
            false,
        ),
        p(
            "THEME MODE DARK",
            "Force dark theme palette",
            "theme mode dark",
            false,
        ),
        p(
            "THEME MODE LIGHT",
            "Force light theme palette",
            "theme mode light",
            false,
        ),
        p(
            "HELP",
            "Display tactical command reference summary",
            "help",
            false,
        ),
        p(
            "HELP NAV",
            "Navigation reference: quadrant warping, course angles, and impulse",
            "help nav",
            false,
        ),
        p("QUIT", "Abandon mission and exit to shell", "quit", false),
    ]
}

fn to_items(c: &[PaletteItem]) -> Vec<Rc<dyn Item>> {
    c.iter()
        .map(|it| Rc::new(it.clone()) as Rc<dyn Item>)
        .collect()
}

/// `(panelStyle, innerWidth, innerHeight)` shared by `SetSize` and `View`.
fn frame(th: Theme, width: i32, height: i32) -> (lipgloss::Style, i32, i32) {
    let styles = th.styles();
    let panel_style = styles
        .panel
        .clone()
        .border(styles.border.clone(), &[true])
        .padding(&[0, 0]);
    let mut border_h = panel_style.get_horizontal_border_size();
    if border_h == 0 {
        border_h = 2;
    }
    let mut border_v = panel_style.get_vertical_border_size();
    if border_v == 0 {
        border_v = 2;
    }
    let inner_w = (width - border_h).max(10);
    let inner_h = (height - border_v).max(1);
    (panel_style, inner_w, inner_h)
}

/// Go `New(th, width, height)`; `None` is Go's nil theme.
pub fn new(th: impl Into<Option<Theme>>, width: i32, height: i32) -> Model {
    let th = th.into().unwrap_or_else(default_theme);
    let width = if width <= 0 { 56 } else { width };
    let height = if height <= 0 { 16 } else { height };

    let catalog = default_catalog();
    let mut delegate = list::new_default_delegate();
    delegate.show_description = true;
    delegate.set_spacing(0);

    let mut l = list::new(to_items(&catalog), delegate, width, height);
    l.title = "COMMAND PALETTE".into();
    l.set_show_title(true);
    l.set_show_status_bar(false);
    l.set_show_pagination(false);
    l.set_show_help(false);
    l.disable_quit_keybindings();

    let mut m = Model {
        theme: th,
        list: l,
        catalog,
        query: String::new(),
        width,
        height,
    };
    m.update_styles();
    m.set_size(width, height);
    m
}

impl Model {
    /// `SetTheme`.
    pub fn set_theme(&mut self, th: impl Into<Option<Theme>>) {
        self.theme = th.into().unwrap_or_else(default_theme);
        self.update_styles();
        self.set_size(self.width, self.height);
    }

    /// `Theme()`.
    pub fn theme(&self) -> Theme {
        self.theme
    }

    /// `SetSize`.
    pub fn set_size(&mut self, width: i32, height: i32) {
        let width = if width <= 0 { 56 } else { width };
        let height = if height <= 0 { 16 } else { height };
        self.width = width;
        self.height = height;
        let (_, iw, ih) = frame(self.theme, width, height);
        self.list.set_size(iw, ih);
    }

    fn update_styles(&mut self) {
        let styles = self.theme.styles();
        let mut d = list::new_default_delegate();
        d.show_description = true;
        d.set_spacing(0);

        d.styles.normal_title = styles.command_text.clone().padding(&[0, 0, 0, 2]);
        d.styles.normal_desc = styles.gauge_label.clone().padding(&[0, 0, 0, 2]);

        let accent = styles.prompt.get_foreground();
        d.styles.selected_title = styles
            .prompt
            .clone()
            .border(lipgloss::normal_border(), &[false, false, false, true])
            .border_foreground(std::slice::from_ref(&accent))
            .padding(&[0, 0, 0, 1]);
        d.styles.selected_desc = styles
            .gauge_label
            .clone()
            .border(lipgloss::normal_border(), &[false, false, false, true])
            .border_foreground(&[accent])
            .padding(&[0, 0, 0, 1]);
        d.styles.filter_match = lipgloss::new_style().underline(true);

        self.list.set_delegate(d);
        self.list.styles.title = styles.panel_title.clone().padding(&[0, 1]);
        self.list.styles.no_items = styles.condition_red.clone().padding(&[0, 0, 0, 2]);
    }

    /// `Reset`: clears the query and selects the first item.
    pub fn reset(&mut self) {
        self.query.clear();
        self.apply_filter();
        self.list.select(0);
    }

    fn apply_filter(&mut self) {
        if self.query.is_empty() {
            self.list.title = "COMMAND PALETTE".into();
            let items = to_items(&self.catalog);
            let _ = self.list.set_items(items);
            self.list.filter_input.reset();
            self.list.select(0);
            return;
        }
        self.list.title = format!("COMMAND PALETTE: {}", self.query);
        self.list.filter_input.set_value(&self.query);
        let targets: Vec<String> = self.catalog.iter().map(|it| it.filter_value()).collect();
        let ranks = list::default_filter(&self.query, &targets);
        let filtered: Vec<Rc<dyn Item>> = ranks
            .iter()
            .map(|r| Rc::new(self.catalog[r.index as usize].clone()) as Rc<dyn Item>)
            .collect();
        let _ = self.list.set_items(filtered);
        self.list.select(0);
    }

    /// `Items()`: a copy of the catalog.
    pub fn items(&self) -> Vec<PaletteItem> {
        self.catalog.clone()
    }

    /// `VisibleItems()`.
    pub fn visible_items(&self) -> Vec<PaletteItem> {
        self.list
            .visible_items()
            .iter()
            .filter_map(|it| it.as_any().downcast_ref::<PaletteItem>().cloned())
            .collect()
    }

    /// `SelectedItem()` (`None` is Go's nil).
    pub fn selected_item(&self) -> Option<PaletteItem> {
        self.list
            .selected_item()
            .and_then(|it| it.as_any().downcast_ref::<PaletteItem>().cloned())
    }

    /// `FilterValue()`: the current query.
    pub fn filter_value(&self) -> String {
        self.query.clone()
    }

    /// `Update`: emits [`CommandSelectedMsg`] / [`ClosePaletteMsg`].
    pub fn update(&mut self, msg: &Msg) -> Option<Cmd> {
        if let Some(k) = msg.downcast_ref::<KeyMsg>() {
            if let Some(r) = self.handle_key(k) {
                return r;
            }
        } else if msg.downcast_ref::<WindowSizeMsg>().is_some() {
            // Fixed dialog dimensions (e.g. 56x16).
            return None;
        }
        self.list.update(msg)
    }

    /// `Some(result)` when the palette fully handles the key.
    fn handle_key(&mut self, k: &KeyMsg) -> Option<Option<Cmd>> {
        let key_str = k.string();
        if k.typ == tea::KEY_ESC || key_str == "esc" {
            return Some(tea::cmd(|| ClosePaletteMsg));
        }
        if k.typ == tea::KEY_ENTER || key_str == "enter" {
            return Some(self.selected_item().and_then(|item| {
                tea::cmd(move || CommandSelectedMsg {
                    command_prefix: item.prefix,
                    parameterized: item.parameterized,
                })
            }));
        }
        if k.typ == tea::KEY_UP || key_str == "up" {
            self.list.cursor_up();
            return Some(None);
        }
        if k.typ == tea::KEY_DOWN || key_str == "down" {
            self.list.cursor_down();
            return Some(None);
        }
        if k.typ == tea::KEY_PG_UP || key_str == "pgup" {
            self.list.prev_page();
            return Some(None);
        }
        if k.typ == tea::KEY_PG_DOWN || key_str == "pgdown" {
            self.list.next_page();
            return Some(None);
        }
        if k.typ == tea::KEY_HOME || key_str == "home" {
            self.list.select(0);
            return Some(None);
        }
        if k.typ == tea::KEY_END || key_str == "end" {
            let n = self.list.visible_items().len() as i32;
            if n > 0 {
                self.list.select(n - 1);
            }
            return Some(None);
        }
        if k.typ == tea::KEY_BACKSPACE
            || k.typ == tea::KEY_DELETE
            || key_str == "backspace"
            || key_str == "delete"
        {
            if self.query.pop().is_some() {
                self.apply_filter();
            }
            return Some(None);
        }
        if k.typ == tea::KEY_SPACE || key_str == " " {
            self.query.push(' ');
            self.apply_filter();
            return Some(None);
        }
        if k.typ == tea::KEY_RUNES && !k.runes.is_empty() {
            self.query.extend(k.runes.iter());
            self.apply_filter();
            return Some(None);
        }
        let b = key_str.as_bytes();
        if b.len() == 1 && (32..=126).contains(&b[0]) {
            self.query.push_str(&key_str);
            self.apply_filter();
            return Some(None);
        }
        None
    }

    /// `View`.
    pub fn view(&self) -> String {
        let (panel_style, iw, ih) = frame(self.theme, self.width, self.height);
        let list_view = self.list.view();
        panel_style.width(iw).height(ih).render(&list_view)
    }
}

#[cfg(test)]
mod parity_tests;
#[cfg(test)]
mod tests;
