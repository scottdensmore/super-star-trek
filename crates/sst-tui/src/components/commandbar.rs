//! Interactive command bar (port of `pkg/tui/components/commandbar/bar.go`).
//!
//! Go embeds `textinput.Model`; here it is the public field [`Model::model`]
//! and the bar derefs to it, so `bar.value()`, `bar.set_value(..)`,
//! `bar.reset()`, `bar.focused()`, `bar.prompt` work like Go's promoted
//! members. `Update` (a Go value receiver whose result the caller assigns
//! back) is `update(&mut self, &Msg)`.

use std::ops::{Deref, DerefMut};

use sst_tuikit::bubbles::textinput;
use sst_tuikit::tea::{self, Cmd, KeyMsg, Msg, KEY_DOWN, KEY_ENTER, KEY_TAB, KEY_UP};
use sst_tuikit::unicode::go_is_space;

use crate::theme::{self, Theme};

#[cfg(test)]
mod parity_tests;
#[cfg(test)]
mod tests;

/// Dispatched when the player submits a non-empty command with Enter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandSubmittedMsg {
    pub text: String,
}

/// The command bar component (Go `commandbar.Model`).
#[derive(Clone, Debug)]
pub struct Model {
    /// The embedded `textinput.Model`.
    pub model: textinput::Model,
    theme: Option<Theme>,
    messages: Vec<String>,
    max_messages: usize,
    width: i32,
    visual_bell: bool,

    history: Vec<String>,
    history_idx: i32,
    draft_input: String,
    tab_matches: Vec<String>,
    tab_match_idx: usize,
}

/// Go `CommandBarModel = Model`.
pub type CommandBarModel = Model;

const CANONICAL_KEYWORDS: [&str; 16] = [
    "chart", "damage", "doc", "help", "lrscan", "nav ", "pha ", "quit", "saves", "she ", "srscan",
    "status", "target", "thaw", "theme ", "tor ",
];

fn trim_space(s: &str) -> &str {
    s.trim_matches(go_is_space)
}

/// `New`: prompt "COMMAND> ", styles from `th` (nil: default theme), focused.
pub fn new(th: Option<Theme>) -> Model {
    let th = th.unwrap_or_else(theme::default_theme);
    let mut ti = textinput::new();
    ti.prompt = "COMMAND> ".into();
    let styles = th.styles();
    ti.prompt_style = styles.prompt;
    ti.text_style = styles.command_text;
    let _ = ti.focus();

    Model {
        model: ti,
        theme: Some(th),
        messages: Vec::with_capacity(4),
        max_messages: 4,
        width: 0,
        visual_bell: false,
        history: Vec::new(),
        history_idx: -1,
        draft_input: String::new(),
        tab_matches: Vec::new(),
        tab_match_idx: 0,
    }
}

/// `NewCommandBar` (alias of [`new`]).
pub fn new_command_bar(th: Option<Theme>) -> CommandBarModel {
    new(th)
}

impl Deref for Model {
    type Target = textinput::Model;
    fn deref(&self) -> &textinput::Model {
        &self.model
    }
}

impl DerefMut for Model {
    fn deref_mut(&mut self) -> &mut textinput::Model {
        &mut self.model
    }
}

impl Model {
    /// `SetTheme`: updates the theme and reapplies the input styles.
    pub fn set_theme(&mut self, th: Option<Theme>) {
        let th = th.unwrap_or_else(theme::default_theme);
        self.theme = Some(th);
        let styles = th.styles();
        self.model.prompt_style = styles.prompt;
        self.model.text_style = styles.command_text;
    }

    /// `Theme`.
    pub fn theme(&self) -> Theme {
        self.theme.unwrap_or_else(theme::default_theme)
    }

    /// `AddMessage`: appends to the 4-line log; each line of a multi-line
    /// message is its own entry and old entries scroll off.
    pub fn add_message(&mut self, msg: &str) {
        if self.max_messages == 0 {
            self.max_messages = 4;
        }
        let msg = msg.trim_end_matches(['\r', '\n']);
        if msg.is_empty() {
            return;
        }
        for line in msg.split('\n') {
            let line = line.trim_end_matches('\r');
            if self.messages.len() >= self.max_messages {
                self.messages.remove(0);
            }
            self.messages.push(line.to_string());
        }
    }

    /// `Messages`: a copy of the log buffer.
    pub fn messages(&self) -> Vec<String> {
        self.messages.clone()
    }

    /// `ClearMessages`.
    pub fn clear_messages(&mut self) {
        self.messages.clear();
    }

    /// `SetWidth`: panel width; the input gets `w - 4` columns.
    pub fn set_width(&mut self, w: i32) {
        self.width = w;
        if w > 0 {
            let avail = w - 4;
            if avail > 0 {
                self.model.width = avail;
            }
        } else {
            self.model.width = 0;
        }
    }

    /// `Width`: the configured panel width (the input's width is
    /// `self.model.width`).
    pub fn width(&self) -> i32 {
        self.width
    }

    /// `SetVisualBell`.
    pub fn set_visual_bell(&mut self, active: bool) {
        self.visual_bell = active;
    }

    /// `VisualBell`.
    pub fn visual_bell(&self) -> bool {
        self.visual_bell
    }

    /// `Focus`.
    pub fn focus(&mut self) -> Option<Cmd> {
        self.model.focus()
    }

    /// `Blur`.
    pub fn blur(&mut self) {
        self.model.blur();
    }

    fn set_value_end(&mut self, v: &str) {
        self.model.set_value(v);
        self.model.cursor_end();
    }

    /// `Update`: Enter submits, Up/Down walk the history, Tab completes
    /// keywords, everything else goes to the text input.
    pub fn update(&mut self, msg: &Msg) -> Option<Cmd> {
        if let Some(key) = msg.downcast_ref::<KeyMsg>() {
            match key.typ {
                KEY_UP => {
                    self.tab_matches.clear();
                    self.tab_match_idx = 0;
                    if self.history_idx == -1 {
                        if !self.history.is_empty() {
                            self.draft_input = self.model.value();
                            self.history_idx = self.history.len() as i32 - 1;
                            let v = self.history[self.history_idx as usize].clone();
                            self.set_value_end(&v);
                        }
                    } else if self.history_idx > 0 {
                        self.history_idx -= 1;
                        let v = self.history[self.history_idx as usize].clone();
                        self.set_value_end(&v);
                    }
                    return None;
                }
                KEY_DOWN => {
                    self.tab_matches.clear();
                    self.tab_match_idx = 0;
                    if self.history_idx != -1 {
                        let last = self.history.len() as i32 - 1;
                        if self.history_idx < last {
                            self.history_idx += 1;
                            let v = self.history[self.history_idx as usize].clone();
                            self.set_value_end(&v);
                        } else if self.history_idx == last {
                            self.history_idx = -1;
                            let v = self.draft_input.clone();
                            self.set_value_end(&v);
                        }
                    }
                    return None;
                }
                KEY_TAB => {
                    let value = self.model.value();
                    if self.tab_matches.is_empty()
                        || self.tab_match_idx >= self.tab_matches.len()
                        || value != self.tab_matches[self.tab_match_idx]
                    {
                        let prefix = trim_space(&value).to_lowercase();
                        if !prefix.is_empty() {
                            let matches: Vec<String> = CANONICAL_KEYWORDS
                                .iter()
                                .filter(|kw| trim_space(kw).starts_with(&prefix))
                                .map(|kw| kw.to_string())
                                .collect();
                            if !matches.is_empty() {
                                self.tab_matches = matches;
                                self.tab_match_idx = 0;
                                let v = self.tab_matches[0].clone();
                                self.set_value_end(&v);
                            }
                        }
                    } else {
                        self.tab_match_idx = (self.tab_match_idx + 1) % self.tab_matches.len();
                        let v = self.tab_matches[self.tab_match_idx].clone();
                        self.set_value_end(&v);
                    }
                    return None;
                }
                KEY_ENTER => {
                    let val = trim_space(&self.model.value()).to_string();
                    if !val.is_empty() {
                        if self.history.last() != Some(&val) {
                            self.history.push(val.clone());
                        }
                        self.history_idx = -1;
                        self.draft_input.clear();
                        self.tab_matches.clear();
                        self.tab_match_idx = 0;
                        self.model.reset();
                        return tea::cmd(move || CommandSubmittedMsg { text: val });
                    }
                    return None;
                }
                _ => {
                    // Any key other than Tab resets active tab cycling.
                    self.tab_matches.clear();
                    self.tab_match_idx = 0;
                }
            }
        }
        self.model.update(msg)
    }

    /// `View`: the 4-line message log and the styled input line in a panel.
    pub fn view(&self) -> String {
        let styles = self.theme().styles();

        const LOG_BUFFER_LINES: i32 = 4;
        let n = self.messages.len() as i32;
        let mut b = String::new();
        for i in 0..LOG_BUFFER_LINES {
            let idx = i - (LOG_BUFFER_LINES - n);
            if idx >= 0 && idx < n {
                b.push_str(&styles.log_text.render(&self.messages[idx as usize]));
            }
            b.push('\n');
        }
        b.push_str(&self.model.view());

        let mut panel_style = styles.panel.clone();
        if self.visual_bell {
            let bell_color = styles.condition_red.get_foreground();
            panel_style = panel_style.border_foreground(&[bell_color]);
        }
        if self.width > 0 {
            let border_width = panel_style.get_horizontal_border_size();
            let content_width = self.width - border_width;
            if content_width > 0 {
                panel_style = panel_style.width(content_width);
            }
        }
        panel_style.render(&b)
    }
}
