//! `bubbles/list` keys.go.

use crate::bubbles::key::Binding;

/// `list.KeyMap`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyMap {
    pub cursor_up: Binding,
    pub cursor_down: Binding,
    pub next_page: Binding,
    pub prev_page: Binding,
    pub go_to_start: Binding,
    pub go_to_end: Binding,
    pub filter: Binding,
    pub clear_filter: Binding,
    pub cancel_while_filtering: Binding,
    pub accept_while_filtering: Binding,
    pub show_full_help: Binding,
    pub close_full_help: Binding,
    pub quit: Binding,
    pub force_quit: Binding,
}

/// `list.DefaultKeyMap()`.
pub fn default_key_map() -> KeyMap {
    let b = |k: &[&str], hk: &str, hd: &str| Binding::new().with_keys(k).with_help(hk, hd);
    KeyMap {
        cursor_up: b(&["up", "k"], "↑/k", "up"),
        cursor_down: b(&["down", "j"], "↓/j", "down"),
        prev_page: b(&["left", "h", "pgup", "b", "u"], "←/h/pgup", "prev page"),
        next_page: b(&["right", "l", "pgdown", "f", "d"], "→/l/pgdn", "next page"),
        go_to_start: b(&["home", "g"], "g/home", "go to start"),
        go_to_end: b(&["end", "G"], "G/end", "go to end"),
        filter: b(&["/"], "/", "filter"),
        clear_filter: b(&["esc"], "esc", "clear filter"),
        cancel_while_filtering: b(&["esc"], "esc", "cancel"),
        accept_while_filtering: b(
            &[
                "enter",
                "tab",
                "shift+tab",
                "ctrl+k",
                "up",
                "ctrl+j",
                "down",
            ],
            "enter",
            "apply filter",
        ),
        show_full_help: b(&["?"], "?", "more"),
        close_full_help: b(&["?"], "?", "close help"),
        quit: b(&["q", "esc"], "q", "quit"),
        force_quit: Binding::new().with_keys(&["ctrl+c"]),
    }
}
