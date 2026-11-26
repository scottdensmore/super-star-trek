//! Port of `pkg/tui/components/manual` (manual.go, chapters.go): the
//! dual-pane Starfleet Technical Manual modal (fixed 66x18).

mod chapters;

use sst_tuikit::ansi;
use sst_tuikit::lipgloss::{self, Style};
use sst_tuikit::tea::{self, Cmd, KeyMsg, Msg};

use crate::theme::{default_theme, Styles, Theme};

/// Go `FocusPane`: which pane receives directional keys.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FocusPane {
    #[default]
    FocusChapters,
    FocusContent,
}

/// Go `Chapter`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chapter {
    pub id: String,
    pub number: i32,
    pub title: String,
    pub short_tag: String,
    pub lines: Vec<String>,
}

fn default_chapters() -> Vec<Chapter> {
    chapters::CHAPTERS
        .iter()
        .map(|c| Chapter {
            id: c.id.into(),
            number: c.number,
            title: c.title.into(),
            short_tag: c.short_tag.into(),
            lines: c.lines.iter().map(|l| l.to_string()).collect(),
        })
        .collect()
}

/// Go `CloseModalMsg`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CloseModalMsg;

/// Go `Model`.
///
/// Go's `scrollOffsets` slice is shared between model copies; the root
/// model always stores the `Update` result back, so a plain `Vec` is
/// observably identical.
#[derive(Clone, Debug)]
pub struct Model {
    width: i32,
    height: i32,
    theme: Theme,
    chapters: Vec<Chapter>,
    selected_idx: i32,
    scroll_offsets: Vec<i32>,
    focus: FocusPane,
}

struct ManualStyles {
    s: Styles,
    border_fg: Style,
}

fn get_styles(th: Theme) -> ManualStyles {
    let s = th.styles();
    let border_fg = lipgloss::new_style().foreground(s.panel.get_border_top_foreground());
    ManualStyles { s, border_fg }
}

/// Go `New(th, width, height)`; `None` is Go's nil theme.
pub fn new(th: impl Into<Option<Theme>>, width: i32, height: i32) -> Model {
    let chs = default_chapters();
    let n = chs.len();
    Model {
        width: if width <= 0 { 66 } else { width },
        height: if height <= 0 { 18 } else { height },
        theme: th.into().unwrap_or_else(default_theme),
        chapters: chs,
        selected_idx: 0,
        scroll_offsets: vec![0; n],
        focus: FocusPane::FocusChapters,
    }
}

fn alias(s: &str) -> Option<i32> {
    Some(match s {
        "sys" | "device" | "devices" | "subsystems" | "computer" | "srs" | "lrs" => 0,
        "navigation" | "warp" | "impulse" | "course" | "chart" => 1,
        "combat" | "weapons" | "tor" | "torpedo" | "torpedoes" | "pha" | "phaser" | "phasers"
        | "target" | "aim" => 2,
        "shield" | "shields" | "she" | "deflector" | "dam" | "damage" | "repair" => 3,
        "starbase" | "starbases" | "base" | "bases" | "doc" | "dock" | "docking"
        | "surveillance" => 4,
        "tactics" | "tac" | "klingon" | "klingons" | "threat" | "threats" | "cloak"
        | "cloaking" | "romulan" | "romulans" | "commander" => 5,
        "scoring" | "score" | "scores" | "rank" | "ranks" | "rating" | "points" => 6,
        "command" | "commands" | "cmd" | "cmds" | "keys" | "hotkeys" | "cheatsheet" | "help"
        | "guide" => 7,
        _ => return None,
    })
}

impl Model {
    /// `SetTheme`.
    pub fn set_theme(&mut self, th: impl Into<Option<Theme>>) {
        self.theme = th.into().unwrap_or_else(default_theme);
    }

    /// The active theme.
    pub fn theme(&self) -> Theme {
        self.theme
    }

    /// The configured size (the view is always 66x18, like Go).
    pub fn size(&self) -> (i32, i32) {
        (self.width, self.height)
    }

    /// The chapter catalog.
    pub fn chapters(&self) -> &[Chapter] {
        &self.chapters
    }

    /// The selected chapter index (`m.selectedIdx`).
    pub fn selected_index(&self) -> i32 {
        self.selected_idx
    }

    /// The focused pane.
    pub fn focus(&self) -> FocusPane {
        self.focus
    }

    /// `SelectChapter`: by numeric string ("1"-"8"), chapter ID, or alias.
    pub fn select_chapter(&mut self, topic_or_num: &str) {
        let s = topic_or_num.trim().to_lowercase();
        if s.is_empty() {
            return;
        }
        let b = s.as_bytes();
        if b.len() == 1 && (b'1'..=b'8').contains(&b[0]) {
            let idx = (b[0] - b'1') as i32;
            if (idx as usize) < self.chapters.len() {
                self.selected_idx = idx;
                return;
            }
        }
        for (i, ch) in self.chapters.iter().enumerate() {
            if go_equal_fold(&ch.id, &s) {
                self.selected_idx = i as i32;
                return;
            }
        }
        if let Some(idx) = alias(&s) {
            if idx >= 0 && (idx as usize) < self.chapters.len() {
                self.selected_idx = idx;
            }
        }
    }

    /// `Update`: emits [`CloseModalMsg`].
    pub fn update(&mut self, msg: &Msg) -> Option<Cmd> {
        let k = msg.downcast_ref::<KeyMsg>()?;
        let key_str = k.string();
        let n = self.chapters.len() as i32;
        if n == 0 {
            return None;
        }
        if self.selected_idx < 0 {
            self.selected_idx = 0;
        } else if self.selected_idx >= n {
            self.selected_idx = n - 1;
        }
        if (self.scroll_offsets.len() as i32) < n {
            self.scroll_offsets.resize(n as usize, 0);
        }

        if k.typ == tea::KEY_F1
            || key_str == "f1"
            || key_str == "F1"
            || key_str == "q"
            || key_str == "Q"
            || key_str == "?"
        {
            return tea::cmd(|| CloseModalMsg);
        }
        if self.focus == FocusPane::FocusChapters && (k.typ == tea::KEY_ESC || key_str == "esc") {
            return tea::cmd(|| CloseModalMsg);
        }
        if k.typ == tea::KEY_TAB
            || key_str == "tab"
            || k.typ == tea::KEY_SHIFT_TAB
            || key_str == "shift+tab"
        {
            self.focus = if self.focus == FocusPane::FocusChapters {
                FocusPane::FocusContent
            } else {
                FocusPane::FocusChapters
            };
            return None;
        }
        let kb = key_str.as_bytes();
        if kb.len() == 1 && (b'1'..=b'8').contains(&kb[0]) {
            let idx = (kb[0] - b'1') as i32;
            if idx < n {
                self.selected_idx = idx;
                if (idx as usize) < self.scroll_offsets.len() {
                    self.scroll_offsets[idx as usize] = 0;
                }
            }
            return None;
        }

        let is = |t: tea::KeyType, names: &[&str]| k.typ == t || names.contains(&key_str.as_str());
        match self.focus {
            FocusPane::FocusChapters => {
                if is(tea::KEY_UP, &["up", "k"]) {
                    self.selected_idx = (self.selected_idx - 1 + n) % n;
                } else if is(tea::KEY_DOWN, &["down", "j"]) {
                    self.selected_idx = (self.selected_idx + 1) % n;
                } else if is(tea::KEY_ENTER, &["enter"]) || is(tea::KEY_RIGHT, &["right", "l"]) {
                    self.focus = FocusPane::FocusContent;
                }
                None
            }
            FocusPane::FocusContent => {
                let sel = self.selected_idx as usize;
                let max_offset = (self.chapters[sel].lines.len() as i32 - 14).max(0);
                if is(tea::KEY_LEFT, &["left", "h"]) || is(tea::KEY_ESC, &["esc"]) {
                    self.focus = FocusPane::FocusChapters;
                    return None;
                }
                let off = &mut self.scroll_offsets[sel];
                if is(tea::KEY_UP, &["up", "k"]) {
                    *off -= 1;
                } else if is(tea::KEY_DOWN, &["down", "j"]) {
                    *off += 1;
                } else if is(tea::KEY_PG_UP, &["pgup", "b"]) || is(tea::KEY_CTRL_U, &["ctrl+u"]) {
                    *off -= 10;
                } else if is(tea::KEY_PG_DOWN, &["pgdown"])
                    || is(tea::KEY_SPACE, &[" ", "space"])
                    || is(tea::KEY_CTRL_D, &["ctrl+d"])
                {
                    *off += 10;
                } else if is(tea::KEY_HOME, &["home", "g"]) {
                    *off = 0;
                } else if is(tea::KEY_END, &["end", "G"]) {
                    *off = max_offset;
                }
                if *off < 0 {
                    *off = 0;
                }
                if *off > max_offset {
                    *off = max_offset;
                }
                None
            }
        }
    }

    /// `View`: the fixed 66x18 wireframe.
    pub fn view(&self) -> String {
        let styles = get_styles(self.theme);
        const W: i32 = 66;
        const SIDEBAR_W: i32 = 20;
        const READER_W: i32 = 43;

        let title_str = "[F1] STARFLEET TECHNICAL MANUAL & LIBRARY COMPUTER";
        let styled_title = styles.s.panel_title.render(title_str);
        let dashes_top = (W - 3 - ansi::string_width(title_str) - 2).max(0);
        let border_top = format!(
            "┌─ {} {}┐",
            styled_title,
            styles.border_fg.render(&"─".repeat(dashes_top as usize))
        );

        let footer_str = "[Tab: Pane]  [↑/↓: Move]  [1-8: Jump]  [Esc/Q/F1: Close]";
        let styled_footer = styles.s.log_text.render(footer_str);
        let dashes_bottom = (W - 3 - ansi::string_width(footer_str) - 2).max(0);
        let border_bottom = format!(
            "└─ {} {}┘",
            styled_footer,
            styles.border_fg.render(&"─".repeat(dashes_bottom as usize))
        );

        let sidebar = self.render_sidebar_rows(&styles, SIDEBAR_W);
        let reader = self.render_reader_rows(&styles, READER_W);

        let mut rows = Vec::with_capacity(18);
        rows.push(border_top);
        for i in 0..16 {
            rows.push(format!("│{}│{}│", sidebar[i], reader[i]));
        }
        rows.push(border_bottom);
        rows.join("\n")
    }

    fn render_sidebar_rows(&self, styles: &ManualStyles, width: i32) -> Vec<String> {
        let mut rows = vec![String::new(); 16];
        rows[0] = pad_right(
            &format!(" {}", styles.s.panel_title.render("CHAPTERS")),
            width,
        );
        rows[1] = pad_right("", width);
        for i in 0..8usize {
            if i >= self.chapters.len() {
                rows[2 + i] = pad_right("", width);
                continue;
            }
            let ch = &self.chapters[i];
            let label = if i as i32 == self.selected_idx {
                if self.focus == FocusPane::FocusChapters {
                    format!(
                        "▶ {}",
                        styles
                            .s
                            .panel_title
                            .clone()
                            .bold(true)
                            .render(&ch.short_tag)
                    )
                } else {
                    format!("▶ {}", styles.s.panel_title.render(&ch.short_tag))
                }
            } else {
                format!("  {}", styles.s.log_text.render(&ch.short_tag))
            };
            rows[2 + i] = pad_right(&label, width);
        }
        rows[10] = pad_right("", width);
        rows[11] = pad_right(
            &format!(" {}", styles.border_fg.render(&"─".repeat(18))),
            width,
        );
        rows[12] = pad_right(
            &format!(" {}", styles.s.log_text.render("[Tab] Focus Reader")),
            width,
        );
        rows[13] = pad_right(
            &format!(" {}", styles.s.log_text.render("[1-8] Direct Jump")),
            width,
        );
        rows[14] = pad_right("", width);
        rows[15] = pad_right("", width);
        rows
    }

    fn render_reader_rows(&self, styles: &ManualStyles, width: i32) -> Vec<String> {
        if self.selected_idx < 0 || self.selected_idx as usize >= self.chapters.len() {
            return vec![pad_right("", width); 16];
        }
        let mut rows = vec![String::new(); 16];
        let ch = &self.chapters[self.selected_idx as usize];
        let total = ch.lines.len() as i32;
        let mut offset = self
            .scroll_offsets
            .get(self.selected_idx as usize)
            .copied()
            .unwrap_or(0);
        let max_offset = (total - 14).max(0);
        if offset > max_offset {
            offset = max_offset;
        }
        if offset < 0 {
            offset = 0;
        }

        let counter = format!("[{:02}/{:02}] ", offset + 1, total);
        let counter_w = ansi::string_width(&counter);
        let max_title_w = width - counter_w - 2;
        let mut title_text = format!("CHAPTER {}: {}", ch.number, ch.title.to_uppercase());
        if ansi::string_width(&title_text) > max_title_w {
            title_text = ansi::truncate(&title_text, max_title_w, "…");
        }
        let styled_title = styles.s.panel_title.render(&format!(" {}", title_text));
        let styled_counter = styles.s.panel_title.render(&counter);
        let gap = (width - ansi::string_width(&styled_title) - counter_w).max(0);
        rows[0] = pad_right(
            &format!(
                "{}{}{}",
                styled_title,
                " ".repeat(gap as usize),
                styled_counter
            ),
            width,
        );

        for i in 0..14 {
            let line_idx = offset + i;
            rows[1 + i as usize] = if line_idx < total {
                let raw = &ch.lines[line_idx as usize];
                let line_str = if ansi::string_width(raw) < width && !raw.is_empty() {
                    format!(" {}", raw)
                } else {
                    raw.clone()
                };
                pad_right(&styles.s.log_text.render(&line_str), width)
            } else {
                pad_right("", width)
            };
        }

        let hint = "▲ [↑/↓, J/K to scroll 14 lines] ▼";
        let hint_gap = (width - ansi::string_width(hint)).max(0);
        let left = hint_gap / 2;
        let right = hint_gap - left;
        rows[15] = pad_right(
            &format!(
                "{}{}{}",
                " ".repeat(left as usize),
                styles.s.log_text.render(hint),
                " ".repeat(right as usize)
            ),
            width,
        );
        rows
    }
}

fn pad_right(s: &str, target: i32) -> String {
    let w = ansi::string_width(s);
    if w >= target {
        return ansi::truncate(s, target, "");
    }
    format!("{}{}", s, " ".repeat((target - w) as usize))
}

/// Go `strings.EqualFold` (simple Unicode case folding).
fn go_equal_fold(a: &str, b: &str) -> bool {
    let mut x = a.chars();
    let mut y = b.chars();
    loop {
        match (x.next(), y.next()) {
            (None, None) => return true,
            (Some(c), Some(d)) => {
                if c != d && !c.to_lowercase().eq(d.to_lowercase()) {
                    return false;
                }
            }
            _ => return false,
        }
    }
}

#[cfg(test)]
mod parity_tests;
#[cfg(test)]
mod tests;
