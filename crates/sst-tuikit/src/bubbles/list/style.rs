//! `bubbles/list` style.go.

use crate::lipgloss::{self, adaptive_color, Style};

pub const BULLET: &str = "•";
pub const ELLIPSIS: &str = "…";

/// `list.Styles`.
#[derive(Clone, Debug)]
pub struct Styles {
    pub title_bar: Style,
    pub title: Style,
    pub spinner: Style,
    pub filter_prompt: Style,
    pub filter_cursor: Style,
    pub default_filter_character_match: Style,
    pub status_bar: Style,
    pub status_empty: Style,
    pub status_bar_active_filter: Style,
    pub status_bar_filter_count: Style,
    pub no_items: Style,
    pub pagination_style: Style,
    pub help_style: Style,
    pub active_pagination_dot: Style,
    pub inactive_pagination_dot: Style,
    pub arabic_pagination: Style,
    pub divider_dot: Style,
}

/// `list.DefaultStyles()`.
pub fn default_styles() -> Styles {
    let very_subdued = adaptive_color("#DDDADA", "#3C3C3C");
    let subdued = adaptive_color("#9B9B9B", "#5C5C5C");
    let n = lipgloss::new_style;
    Styles {
        title_bar: n().padding(&[0, 0, 1, 2]),
        title: n()
            .background(lipgloss::color("62"))
            .foreground(lipgloss::color("230"))
            .padding(&[0, 1]),
        spinner: n().foreground(adaptive_color("#8E8E8E", "#747373")),
        filter_prompt: n().foreground(adaptive_color("#04B575", "#ECFD65")),
        filter_cursor: n().foreground(adaptive_color("#EE6FF8", "#EE6FF8")),
        default_filter_character_match: n().underline(true),
        status_bar: n()
            .foreground(adaptive_color("#A49FA5", "#777777"))
            .padding(&[0, 0, 1, 2]),
        status_empty: n().foreground(subdued.clone()),
        status_bar_active_filter: n().foreground(adaptive_color("#1a1a1a", "#dddddd")),
        status_bar_filter_count: n().foreground(very_subdued.clone()),
        no_items: n().foreground(adaptive_color("#909090", "#626262")),
        arabic_pagination: n().foreground(subdued),
        pagination_style: n().padding_left(2),
        help_style: n().padding(&[1, 0, 0, 2]),
        active_pagination_dot: n()
            .foreground(adaptive_color("#847A85", "#979797"))
            .set_string(&[BULLET]),
        inactive_pagination_dot: n().foreground(very_subdued.clone()).set_string(&[BULLET]),
        divider_dot: n()
            .foreground(very_subdued)
            .set_string(&[&format!(" {BULLET} ")]),
    }
}
