//! `bubbles/help`.

use super::key::Binding;
use crate::lipgloss::{self, Style};

/// `help.KeyMap`.
pub trait KeyMap {
    fn short_help(&self) -> Vec<Binding>;
    fn full_help(&self) -> Vec<Vec<Binding>>;
}

/// `help.Styles`.
#[derive(Clone, Debug)]
pub struct Styles {
    pub ellipsis: Style,
    pub short_key: Style,
    pub short_desc: Style,
    pub short_separator: Style,
    pub full_key: Style,
    pub full_desc: Style,
    pub full_separator: Style,
}

/// `help.Model`.
#[derive(Clone, Debug)]
pub struct Model {
    pub width: i32,
    pub show_all: bool,
    pub short_separator: String,
    pub full_separator: String,
    pub ellipsis: String,
    pub styles: Styles,
}

/// `help.New()`.
pub fn new() -> Model {
    let key_style =
        lipgloss::new_style().foreground(lipgloss::adaptive_color("#909090", "#626262"));
    let desc_style =
        lipgloss::new_style().foreground(lipgloss::adaptive_color("#B2B2B2", "#4A4A4A"));
    let sep_style =
        lipgloss::new_style().foreground(lipgloss::adaptive_color("#DDDADA", "#3C3C3C"));
    Model {
        width: 0,
        show_all: false,
        short_separator: " • ".into(),
        full_separator: "    ".into(),
        ellipsis: "…".into(),
        styles: Styles {
            short_key: key_style.clone(),
            short_desc: desc_style.clone(),
            short_separator: sep_style.clone(),
            ellipsis: sep_style.clone(),
            full_key: key_style,
            full_desc: desc_style,
            full_separator: sep_style,
        },
    }
}

impl Default for Model {
    fn default() -> Self {
        new()
    }
}

impl Model {
    /// `View`.
    pub fn view(&self, k: &dyn KeyMap) -> String {
        if self.show_all {
            return self.full_help_view(&k.full_help());
        }
        self.short_help_view(&k.short_help())
    }

    /// `ShortHelpView`.
    pub fn short_help_view(&self, bindings: &[Binding]) -> String {
        if bindings.is_empty() {
            return String::new();
        }
        let mut b = String::new();
        let mut total_width = 0;
        let separator = self
            .styles
            .short_separator
            .clone()
            .inline(true)
            .render(&self.short_separator);
        for (i, kb) in bindings.iter().enumerate() {
            if !kb.enabled() {
                continue;
            }
            let sep = if total_width > 0 && i < bindings.len() {
                separator.as_str()
            } else {
                ""
            };
            let s = format!(
                "{sep}{} {}",
                self.styles
                    .short_key
                    .clone()
                    .inline(true)
                    .render(&kb.help().key),
                self.styles
                    .short_desc
                    .clone()
                    .inline(true)
                    .render(&kb.help().desc)
            );
            let w = lipgloss::width(&s);
            if let Some(tail) = self.should_add_item(total_width, w) {
                if !tail.is_empty() {
                    b += &tail;
                }
                break;
            }
            total_width += w;
            b += &s;
        }
        b
    }

    /// `FullHelpView`.
    pub fn full_help_view(&self, groups: &[Vec<Binding>]) -> String {
        if groups.is_empty() {
            return String::new();
        }
        let mut out: Vec<String> = Vec::new();
        let mut total_width = 0;
        let separator = self
            .styles
            .full_separator
            .clone()
            .inline(true)
            .render(&self.full_separator);
        for (i, group) in groups.iter().enumerate() {
            if !group.iter().any(|b| b.enabled()) {
                continue;
            }
            let sep = if total_width > 0 && i < groups.len() {
                separator.clone()
            } else {
                String::new()
            };
            let mut keys = Vec::new();
            let mut descriptions = Vec::new();
            for kb in group {
                if !kb.enabled() {
                    continue;
                }
                keys.push(kb.help().key.clone());
                descriptions.push(kb.help().desc.clone());
            }
            let k = self.styles.full_key.render(&keys.join("\n"));
            let d = self.styles.full_desc.render(&descriptions.join("\n"));
            let col = lipgloss::join_horizontal(lipgloss::TOP, &[&sep, &k, " ", &d]);
            let w = lipgloss::width(&col);
            if let Some(tail) = self.should_add_item(total_width, w) {
                if !tail.is_empty() {
                    out.push(tail);
                }
                break;
            }
            total_width += w;
            out.push(col);
        }
        let refs: Vec<&str> = out.iter().map(|s| s.as_str()).collect();
        lipgloss::join_horizontal(lipgloss::TOP, &refs)
    }

    /// `shouldAddItem`: `None` = ok, `Some(tail)` = stop (tail may be "").
    fn should_add_item(&self, total_width: i32, width: i32) -> Option<String> {
        if self.width > 0 && total_width + width > self.width {
            let tail = format!(
                " {}",
                self.styles
                    .ellipsis
                    .clone()
                    .inline(true)
                    .render(&self.ellipsis)
            );
            if total_width + lipgloss::width(&tail) < self.width {
                return Some(tail);
            }
        }
        None
    }
}
