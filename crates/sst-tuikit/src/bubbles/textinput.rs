//! `bubbles/textinput`: a single-line text input.
//!
//! Go `ti := textinput.New(); ti.Prompt = "COMMAND> "` ->
//! `let mut ti = textinput::new(); ti.prompt = "COMMAND> ".into();`.
//! `m.Model, cmd = m.Model.Update(msg)` -> `let cmd = m.model.update(&msg);`.
//!
//! The value is a `Vec<char>` owned by the model. (Go's `[]rune` slices can
//! alias between copies of a model after in-place edits; nothing in the game
//! observes that.)

use std::rc::Rc;
use std::time::Duration;

use super::cursor;
use super::key::{self, Binding};
use super::runeutil::Sanitizer;
use crate::ansi;
use crate::lipgloss::{self, Style};
use crate::tea::{self, Cmd, KeyMsg, Msg};
use crate::unicode;

/// Clipboard contents (`pasteMsg`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PasteMsg(pub String);

/// Clipboard error (`pasteErrMsg`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PasteErrMsg(pub String);

/// `textinput.EchoMode`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EchoMode {
    #[default]
    EchoNormal,
    EchoPassword,
    EchoNone,
}

/// `textinput.ValidateFunc` (an error is a message string).
pub type ValidateFunc = Rc<dyn Fn(&str) -> Option<String>>;

/// `textinput.KeyMap`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyMap {
    pub character_forward: Binding,
    pub character_backward: Binding,
    pub word_forward: Binding,
    pub word_backward: Binding,
    pub delete_word_backward: Binding,
    pub delete_word_forward: Binding,
    pub delete_after_cursor: Binding,
    pub delete_before_cursor: Binding,
    pub delete_character_backward: Binding,
    pub delete_character_forward: Binding,
    pub line_start: Binding,
    pub line_end: Binding,
    pub paste: Binding,
    pub accept_suggestion: Binding,
    pub next_suggestion: Binding,
    pub prev_suggestion: Binding,
}

/// `textinput.DefaultKeyMap`.
pub fn default_key_map() -> KeyMap {
    let b = |k: &[&str]| Binding::new().with_keys(k);
    KeyMap {
        character_forward: b(&["right", "ctrl+f"]),
        character_backward: b(&["left", "ctrl+b"]),
        word_forward: b(&["alt+right", "ctrl+right", "alt+f"]),
        word_backward: b(&["alt+left", "ctrl+left", "alt+b"]),
        delete_word_backward: b(&["alt+backspace", "ctrl+w"]),
        delete_word_forward: b(&["alt+delete", "alt+d"]),
        delete_after_cursor: b(&["ctrl+k"]),
        delete_before_cursor: b(&["ctrl+u"]),
        delete_character_backward: b(&["backspace", "ctrl+h"]),
        delete_character_forward: b(&["delete", "ctrl+d"]),
        line_start: b(&["home", "ctrl+a"]),
        line_end: b(&["end", "ctrl+e"]),
        paste: b(&["ctrl+v"]),
        accept_suggestion: b(&["tab"]),
        next_suggestion: b(&["down", "ctrl+n"]),
        prev_suggestion: b(&["up", "ctrl+p"]),
    }
}

/// The text input model (`textinput.Model`).
#[derive(Clone)]
pub struct Model {
    pub err: Option<String>,
    pub prompt: String,
    pub placeholder: String,
    pub echo_mode: EchoMode,
    pub echo_character: char,
    pub cursor: cursor::Model,
    /// Deprecated in Go (unused).
    pub blink_speed: Duration,
    pub prompt_style: Style,
    pub text_style: Style,
    pub placeholder_style: Style,
    pub completion_style: Style,
    /// Deprecated in Go (unused).
    pub cursor_style: Style,
    pub char_limit: i32,
    pub width: i32,
    pub key_map: KeyMap,
    value: Vec<char>,
    focus: bool,
    pos: i32,
    offset: i32,
    offset_right: i32,
    pub validate: Option<ValidateFunc>,
    rsan: Option<Sanitizer>,
    pub show_suggestions: bool,
    suggestions: Vec<Vec<char>>,
    matched_suggestions: Vec<Vec<char>>,
    current_suggestion_index: i32,
}

impl std::fmt::Debug for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("textinput::Model")
            .field("prompt", &self.prompt)
            .field("value", &self.value())
            .field("pos", &self.pos)
            .field("focus", &self.focus)
            .finish_non_exhaustive()
    }
}

impl Default for Model {
    fn default() -> Self {
        new()
    }
}

/// `textinput.New()`.
pub fn new() -> Model {
    Model {
        err: None,
        prompt: "> ".into(),
        placeholder: String::new(),
        echo_mode: EchoMode::EchoNormal,
        echo_character: '*',
        cursor: cursor::new(),
        blink_speed: Duration::ZERO,
        prompt_style: lipgloss::new_style(),
        text_style: lipgloss::new_style(),
        placeholder_style: lipgloss::new_style().foreground(lipgloss::color("240")),
        completion_style: lipgloss::new_style().foreground(lipgloss::color("240")),
        cursor_style: lipgloss::new_style(),
        char_limit: 0,
        width: 0,
        key_map: default_key_map(),
        value: Vec::new(),
        focus: false,
        pos: 0,
        offset: 0,
        offset_right: 0,
        validate: None,
        rsan: None,
        show_suggestions: false,
        suggestions: Vec::new(),
        matched_suggestions: Vec::new(),
        current_suggestion_index: 0,
    }
}

fn clamp(v: i32, low: i32, high: i32) -> i32 {
    let (low, high) = if high < low { (high, low) } else { (low, high) };
    high.min(low.max(v))
}

fn is_space(c: char) -> bool {
    unicode::go_is_space(c)
}

fn s(v: &[char]) -> String {
    v.iter().collect()
}

impl Model {
    fn san(&mut self) -> Sanitizer {
        if self.rsan.is_none() {
            self.rsan = Some(Sanitizer::new(" ", " "));
        }
        self.rsan.clone().unwrap_or_default()
    }

    fn run_validate(&self, v: &[char]) -> Option<String> {
        self.validate.as_ref().and_then(|f| f(&s(v)))
    }

    /// `Model.SetValue`.
    pub fn set_value(&mut self, v: &str) {
        let runes: Vec<char> = v.chars().collect();
        let runes = self.san().sanitize(&runes);
        let err = self.run_validate(&runes);
        self.set_value_internal(runes, err);
    }

    fn set_value_internal(&mut self, runes: Vec<char>, err: Option<String>) {
        self.err = err;
        let empty = self.value.is_empty();
        if self.char_limit > 0 && runes.len() as i32 > self.char_limit {
            self.value = runes[..self.char_limit as usize].to_vec();
        } else {
            self.value = runes;
        }
        if (self.pos == 0 && empty) || self.pos > self.value.len() as i32 {
            self.set_cursor(self.value.len() as i32);
        }
        self.handle_overflow();
    }

    /// `Model.Value`.
    pub fn value(&self) -> String {
        s(&self.value)
    }

    /// `Model.Position`.
    pub fn position(&self) -> i32 {
        self.pos
    }

    /// `Model.SetCursor`.
    pub fn set_cursor(&mut self, pos: i32) {
        self.pos = clamp(pos, 0, self.value.len() as i32);
        self.handle_overflow();
    }

    /// `Model.CursorStart`.
    pub fn cursor_start(&mut self) {
        self.set_cursor(0);
    }

    /// `Model.CursorEnd`.
    pub fn cursor_end(&mut self) {
        self.set_cursor(self.value.len() as i32);
    }

    /// `Model.Focused`.
    pub fn focused(&self) -> bool {
        self.focus
    }

    /// `Model.Focus`.
    pub fn focus(&mut self) -> Option<Cmd> {
        self.focus = true;
        self.cursor.focus()
    }

    /// `Model.Blur`.
    pub fn blur(&mut self) {
        self.focus = false;
        self.cursor.blur();
    }

    /// `Model.Reset`.
    pub fn reset(&mut self) {
        self.value = Vec::new();
        self.set_cursor(0);
    }

    /// `Model.SetSuggestions`.
    pub fn set_suggestions(&mut self, suggestions: &[&str]) {
        self.suggestions = suggestions.iter().map(|s| s.chars().collect()).collect();
        self.update_suggestions();
    }

    fn insert_runes_from_user_input(&mut self, v: &[char]) {
        let mut paste = self.san().sanitize(v);
        let mut avail_space = 0;
        if self.char_limit > 0 {
            avail_space = self.char_limit - self.value.len() as i32;
            if avail_space <= 0 {
                return;
            }
            if (avail_space as usize) < paste.len() {
                paste.truncate(avail_space as usize);
            }
        }
        let pos = self.pos as usize;
        let mut head: Vec<char> = self.value[..pos].to_vec();
        let tail: Vec<char> = self.value[pos..].to_vec();
        for r in paste {
            head.push(r);
            self.pos += 1;
            if self.char_limit > 0 {
                avail_space -= 1;
                if avail_space <= 0 {
                    break;
                }
            }
        }
        head.extend(tail);
        let err = self.run_validate(&head);
        self.set_value_internal(head, err);
    }

    fn handle_overflow(&mut self) {
        let len = self.value.len() as i32;
        if self.width <= 0 || unicode::uniseg_string_width(&s(&self.value)) <= self.width {
            self.offset = 0;
            self.offset_right = len;
            return;
        }
        self.offset_right = self.offset_right.min(len);
        if self.pos < self.offset {
            self.offset = self.pos;
            let mut w = 0;
            let mut i = 0usize;
            let runes = &self.value[self.offset as usize..];
            while i < runes.len() && w <= self.width {
                w += unicode::runewidth_rune_width(runes[i]);
                if w <= self.width + 1 {
                    i += 1;
                }
            }
            self.offset_right = self.offset + i as i32;
        } else if self.pos >= self.offset_right {
            self.offset_right = self.pos;
            let mut w = 0;
            let runes = &self.value[..self.offset_right as usize];
            let mut i = runes.len() as i32 - 1;
            while i > 0 && w < self.width {
                w += unicode::runewidth_rune_width(runes[i as usize]);
                if w <= self.width {
                    i -= 1;
                }
            }
            self.offset = self.offset_right - (runes.len() as i32 - 1 - i);
        }
    }

    fn delete_before_cursor(&mut self) {
        self.value = self.value[self.pos as usize..].to_vec();
        self.err = self.run_validate(&self.value);
        self.offset = 0;
        self.set_cursor(0);
    }

    fn delete_after_cursor(&mut self) {
        self.value.truncate(self.pos as usize);
        self.err = self.run_validate(&self.value);
        self.set_cursor(self.value.len() as i32);
    }

    fn delete_word_backward(&mut self) {
        if self.pos == 0 || self.value.is_empty() {
            return;
        }
        if self.echo_mode != EchoMode::EchoNormal {
            self.delete_before_cursor();
            return;
        }
        let old_pos = self.pos;
        self.set_cursor(self.pos - 1);
        while is_space(self.value[self.pos as usize]) {
            if self.pos <= 0 {
                break;
            }
            self.set_cursor(self.pos - 1);
        }
        while self.pos > 0 {
            if !is_space(self.value[self.pos as usize]) {
                self.set_cursor(self.pos - 1);
            } else {
                if self.pos > 0 {
                    self.set_cursor(self.pos + 1);
                }
                break;
            }
        }
        if old_pos > self.value.len() as i32 {
            self.value.truncate(self.pos as usize);
        } else {
            let mut v = self.value[..self.pos as usize].to_vec();
            v.extend_from_slice(&self.value[old_pos as usize..]);
            self.value = v;
        }
        self.err = self.run_validate(&self.value);
    }

    fn delete_word_forward(&mut self) {
        if self.pos >= self.value.len() as i32 || self.value.is_empty() {
            return;
        }
        if self.echo_mode != EchoMode::EchoNormal {
            self.delete_after_cursor();
            return;
        }
        let old_pos = self.pos;
        self.set_cursor(self.pos + 1);
        while (self.pos as usize) < self.value.len() && is_space(self.value[self.pos as usize]) {
            self.set_cursor(self.pos + 1);
            if self.pos >= self.value.len() as i32 {
                break;
            }
        }
        while self.pos < self.value.len() as i32 {
            if !is_space(self.value[self.pos as usize]) {
                self.set_cursor(self.pos + 1);
            } else {
                break;
            }
        }
        if self.pos > self.value.len() as i32 {
            self.value.truncate(old_pos as usize);
        } else {
            let mut v = self.value[..old_pos as usize].to_vec();
            v.extend_from_slice(&self.value[self.pos as usize..]);
            self.value = v;
        }
        self.err = self.run_validate(&self.value);
        self.set_cursor(old_pos);
    }

    fn word_backward(&mut self) {
        if self.pos == 0 || self.value.is_empty() {
            return;
        }
        if self.echo_mode != EchoMode::EchoNormal {
            self.cursor_start();
            return;
        }
        let mut i = self.pos - 1;
        while i >= 0 {
            if is_space(self.value[i as usize]) {
                self.set_cursor(self.pos - 1);
                i -= 1;
            } else {
                break;
            }
        }
        while i >= 0 {
            if !is_space(self.value[i as usize]) {
                self.set_cursor(self.pos - 1);
                i -= 1;
            } else {
                break;
            }
        }
    }

    fn word_forward(&mut self) {
        if self.pos >= self.value.len() as i32 || self.value.is_empty() {
            return;
        }
        if self.echo_mode != EchoMode::EchoNormal {
            self.cursor_end();
            return;
        }
        let mut i = self.pos as usize;
        while i < self.value.len() {
            if is_space(self.value[i]) {
                self.set_cursor(self.pos + 1);
                i += 1;
            } else {
                break;
            }
        }
        while i < self.value.len() {
            if !is_space(self.value[i]) {
                self.set_cursor(self.pos + 1);
                i += 1;
            } else {
                break;
            }
        }
    }

    fn echo_transform(&self, v: &str) -> String {
        match self.echo_mode {
            EchoMode::EchoPassword => self
                .echo_character
                .to_string()
                .repeat(unicode::uniseg_string_width(v).max(0) as usize),
            EchoMode::EchoNone => String::new(),
            EchoMode::EchoNormal => v.to_string(),
        }
    }

    /// `Model.Update`. Does nothing unless focused.
    pub fn update(&mut self, msg: &Msg) -> Option<Cmd> {
        if !self.focus {
            return None;
        }
        let key_msg = msg.downcast_ref::<KeyMsg>();
        if let Some(k) = key_msg {
            if key::matches(k, &[&self.key_map.accept_suggestion]) && self.can_accept_suggestion() {
                let sug = self.matched_suggestions[self.current_suggestion_index as usize].clone();
                if sug.len() > self.value.len() {
                    let tail = sug[self.value.len()..].to_vec();
                    self.value.extend(tail);
                }
                self.cursor_end();
            }
        }

        let old_pos = self.pos;
        if let Some(k) = key_msg {
            let km = self.key_map.clone();
            let m = |b: &Binding| key::matches(k, &[b]);
            if m(&km.delete_word_backward) {
                self.delete_word_backward();
            } else if m(&km.delete_character_backward) {
                self.err = None;
                if !self.value.is_empty() {
                    let cut = (self.pos - 1).max(0) as usize;
                    let mut v = self.value[..cut].to_vec();
                    v.extend_from_slice(&self.value[self.pos as usize..]);
                    self.value = v;
                    self.err = self.run_validate(&self.value);
                    if self.pos > 0 {
                        self.set_cursor(self.pos - 1);
                    }
                }
            } else if m(&km.word_backward) {
                self.word_backward();
            } else if m(&km.character_backward) {
                if self.pos > 0 {
                    self.set_cursor(self.pos - 1);
                }
            } else if m(&km.word_forward) {
                self.word_forward();
            } else if m(&km.character_forward) {
                if self.pos < self.value.len() as i32 {
                    self.set_cursor(self.pos + 1);
                }
            } else if m(&km.line_start) {
                self.cursor_start();
            } else if m(&km.delete_character_forward) {
                if !self.value.is_empty() && self.pos < self.value.len() as i32 {
                    self.value.remove(self.pos as usize);
                    self.err = self.run_validate(&self.value);
                }
            } else if m(&km.line_end) {
                self.cursor_end();
            } else if m(&km.delete_after_cursor) {
                self.delete_after_cursor();
            } else if m(&km.delete_before_cursor) {
                self.delete_before_cursor();
            } else if m(&km.paste) {
                return paste();
            } else if m(&km.delete_word_forward) {
                self.delete_word_forward();
            } else if m(&km.next_suggestion) {
                self.next_suggestion();
            } else if m(&km.prev_suggestion) {
                self.previous_suggestion();
            } else {
                let runes = k.runes.clone();
                self.insert_runes_from_user_input(&runes);
            }
            self.update_suggestions();
        } else if let Some(p) = msg.downcast_ref::<PasteMsg>() {
            let runes: Vec<char> = p.0.chars().collect();
            self.insert_runes_from_user_input(&runes);
        } else if let Some(e) = msg.downcast_ref::<PasteErrMsg>() {
            self.err = Some(e.0.clone());
        }

        let mut cmds = Vec::new();
        let c = self.cursor.update(msg);
        cmds.push(c);
        if old_pos != self.pos && self.cursor.mode() == cursor::Mode::CursorBlink {
            self.cursor.blink = false;
            cmds.push(self.cursor.blink_cmd());
        }
        self.handle_overflow();
        tea::batch(cmds)
    }

    /// `Model.View`.
    pub fn view(&self) -> String {
        if self.value.is_empty() && !self.placeholder.is_empty() {
            return self.placeholder_view();
        }
        let style_text = |x: &str| self.text_style.clone().inline(true).render(x);
        let mut cur = self.cursor.clone();
        let value = &self.value[self.offset as usize..self.offset_right as usize];
        let pos = (self.pos - self.offset).max(0) as usize;
        let mut v = style_text(&self.echo_transform(&s(&value[..pos.min(value.len())])));
        if pos < value.len() {
            let ch = self.echo_transform(&value[pos].to_string());
            cur.set_char(&ch);
            v += &cur.view();
            v += &style_text(&self.echo_transform(&s(&value[pos + 1..])));
            v += &self.completion_view(0);
        } else if self.focus && self.can_accept_suggestion() {
            let suggestion = &self.matched_suggestions[self.current_suggestion_index as usize];
            if value.len() < suggestion.len() {
                cur.text_style = self.completion_style.clone();
                cur.set_char(&self.echo_transform(&suggestion[pos].to_string()));
                v += &cur.view();
                v += &self.completion_view(1);
            } else {
                cur.set_char(" ");
                v += &cur.view();
            }
        } else {
            cur.set_char(" ");
            v += &cur.view();
        }

        let val_width = unicode::uniseg_string_width(&s(value));
        if self.width > 0 && val_width <= self.width {
            let mut padding = (self.width - val_width).max(0);
            if val_width + padding <= self.width && pos < value.len() {
                padding += 1;
            }
            v += &style_text(&" ".repeat(padding as usize));
        }
        self.prompt_style.render(&self.prompt) + &v
    }

    fn placeholder_view(&self) -> String {
        let style = |x: &str| self.placeholder_style.clone().inline(true).render(x);
        let p = self.prompt_style.render(&self.prompt);
        let mut cur = self.cursor.clone();
        cur.text_style = self.placeholder_style.clone();
        let (first, rest, _) = unicode::uniseg_first_grapheme_cluster(&self.placeholder);
        cur.set_char(first);
        let mut v = cur.view();
        if self.width < 1 && unicode::uniseg_string_width(rest) <= 1 {
            return self.prompt_style.render(&self.prompt) + &v;
        }
        if self.width > 0 {
            let width = self.width - lipgloss::width(&p) - lipgloss::width(&v);
            let placeholder_rest = ansi::truncate(rest, width, "…");
            let avail_width = (width - lipgloss::width(&placeholder_rest)).max(0);
            v += &style(&placeholder_rest);
            v += &" ".repeat(avail_width as usize);
        } else {
            v += &style(rest);
        }
        p + &v
    }

    /// `Model.CursorMode` (deprecated in Go).
    pub fn cursor_mode(&self) -> cursor::Mode {
        self.cursor.mode()
    }

    /// `Model.SetCursorMode` (deprecated in Go).
    pub fn set_cursor_mode(&mut self, mode: cursor::Mode) -> Option<Cmd> {
        self.cursor.set_mode(mode)
    }

    fn completion_view(&self, offset: usize) -> String {
        let style = |x: &str| self.placeholder_style.clone().inline(true).render(x);
        if self.can_accept_suggestion() {
            let suggestion = &self.matched_suggestions[self.current_suggestion_index as usize];
            if self.value.len() < suggestion.len() {
                let start = (self.value.len() + offset).min(suggestion.len());
                return style(&s(&suggestion[start..]));
            }
        }
        String::new()
    }

    /// `Model.AvailableSuggestions`.
    pub fn available_suggestions(&self) -> Vec<String> {
        self.suggestions.iter().map(|x| s(x)).collect()
    }

    /// `Model.MatchedSuggestions`.
    pub fn matched_suggestions(&self) -> Vec<String> {
        self.matched_suggestions.iter().map(|x| s(x)).collect()
    }

    /// `Model.CurrentSuggestionIndex`.
    pub fn current_suggestion_index(&self) -> i32 {
        self.current_suggestion_index
    }

    /// `Model.CurrentSuggestion`.
    pub fn current_suggestion(&self) -> String {
        if self.current_suggestion_index as usize >= self.matched_suggestions.len() {
            return String::new();
        }
        s(&self.matched_suggestions[self.current_suggestion_index as usize])
    }

    fn can_accept_suggestion(&self) -> bool {
        !self.matched_suggestions.is_empty()
    }

    fn update_suggestions(&mut self) {
        if !self.show_suggestions {
            return;
        }
        if self.value.is_empty() || self.suggestions.is_empty() {
            self.matched_suggestions = Vec::new();
            return;
        }
        let val = s(&self.value).to_lowercase();
        let matches: Vec<Vec<char>> = self
            .suggestions
            .iter()
            .filter(|sg| s(sg).to_lowercase().starts_with(&val))
            .cloned()
            .collect();
        if matches != self.matched_suggestions {
            self.current_suggestion_index = 0;
        }
        self.matched_suggestions = matches;
    }

    fn next_suggestion(&mut self) {
        self.current_suggestion_index += 1;
        if self.current_suggestion_index as usize >= self.matched_suggestions.len() {
            self.current_suggestion_index = 0;
        }
    }

    fn previous_suggestion(&mut self) {
        self.current_suggestion_index -= 1;
        if self.current_suggestion_index < 0 {
            self.current_suggestion_index = self.matched_suggestions.len() as i32 - 1;
        }
    }
}

/// `textinput.Blink` (as a command).
pub fn blink() -> Option<Cmd> {
    cursor::blink()
}

/// Reads the system clipboard (atotto/clipboard: `pbpaste`, `xclip`,
/// `xsel`, `wl-paste`, `powershell Get-Clipboard`).
fn read_clipboard() -> Result<String, String> {
    let candidates: &[(&str, &[&str])] = if cfg!(target_os = "macos") {
        &[("pbpaste", &[])]
    } else if cfg!(windows) {
        &[("powershell", &["-NoProfile", "-Command", "Get-Clipboard"])]
    } else {
        &[
            ("wl-paste", &["--no-newline"]),
            ("xclip", &["-out", "-selection", "clipboard"]),
            ("xsel", &["--output", "--clipboard"]),
        ]
    };
    let mut last = String::from("No clipboard utilities available.");
    for (cmd, args) in candidates {
        match std::process::Command::new(cmd).args(*args).output() {
            Ok(out) if out.status.success() => {
                return Ok(String::from_utf8_lossy(&out.stdout).into_owned())
            }
            Ok(out) => last = format!("exit status {}", out.status),
            Err(e) => last = e.to_string(),
        }
    }
    Err(last)
}

/// `textinput.Paste` (as a command).
pub fn paste() -> Option<Cmd> {
    tea::cmd_opt(|| {
        Some(match read_clipboard() {
            Ok(s) => tea::msg(PasteMsg(s)),
            Err(e) => tea::msg(PasteErrMsg(e)),
        })
    })
}
