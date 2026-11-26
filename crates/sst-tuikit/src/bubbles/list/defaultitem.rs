//! `bubbles/list` defaultitem.go.

use std::rc::Rc;

use super::style::ELLIPSIS;
use super::{FilterState, Item, ItemDelegate, Model};
use crate::ansi;
use crate::bubbles::key::Binding;
use crate::lipgloss::{self, adaptive_color, Style};
use crate::tea::{Cmd, Msg};

/// `list.DefaultItemStyles`.
#[derive(Clone, Debug)]
pub struct DefaultItemStyles {
    pub normal_title: Style,
    pub normal_desc: Style,
    pub selected_title: Style,
    pub selected_desc: Style,
    pub dimmed_title: Style,
    pub dimmed_desc: Style,
    pub filter_match: Style,
}

/// `list.NewDefaultItemStyles()`.
pub fn new_default_item_styles() -> DefaultItemStyles {
    let normal_title = lipgloss::new_style()
        .foreground(adaptive_color("#1a1a1a", "#dddddd"))
        .padding(&[0, 0, 0, 2]);
    let normal_desc = normal_title
        .clone()
        .foreground(adaptive_color("#A49FA5", "#777777"));
    let selected_title = lipgloss::new_style()
        .border(lipgloss::normal_border(), &[false, false, false, true])
        .border_foreground(&[adaptive_color("#F793FF", "#AD58B4")])
        .foreground(adaptive_color("#EE6FF8", "#EE6FF8"))
        .padding(&[0, 0, 0, 1]);
    let selected_desc = selected_title
        .clone()
        .foreground(adaptive_color("#F793FF", "#AD58B4"));
    let dimmed_title = lipgloss::new_style()
        .foreground(adaptive_color("#A49FA5", "#777777"))
        .padding(&[0, 0, 0, 2]);
    let dimmed_desc = dimmed_title
        .clone()
        .foreground(adaptive_color("#C2B8C2", "#4D4D4D"));
    DefaultItemStyles {
        normal_title,
        normal_desc,
        selected_title,
        selected_desc,
        dimmed_title,
        dimmed_desc,
        filter_match: lipgloss::new_style().underline(true),
    }
}

/// `list.DefaultItem`: an item with a title and a description.
pub trait DefaultItem {
    fn title(&self) -> String;
    fn description(&self) -> String;
}

type UpdateFunc = Rc<dyn Fn(&Msg, &mut Model) -> Option<Cmd>>;

/// `list.DefaultDelegate`.
#[derive(Clone)]
pub struct DefaultDelegate {
    pub show_description: bool,
    pub styles: DefaultItemStyles,
    pub update_func: Option<UpdateFunc>,
    pub short_help_func: Option<Rc<dyn Fn() -> Vec<Binding>>>,
    pub full_help_func: Option<Rc<dyn Fn() -> Vec<Vec<Binding>>>>,
    height: i32,
    spacing: i32,
}

/// `list.NewDefaultDelegate()`.
pub fn new_default_delegate() -> DefaultDelegate {
    DefaultDelegate {
        show_description: true,
        styles: new_default_item_styles(),
        update_func: None,
        short_help_func: None,
        full_help_func: None,
        height: 2,
        spacing: 1,
    }
}

impl DefaultDelegate {
    /// `SetHeight`.
    pub fn set_height(&mut self, i: i32) {
        self.height = i;
    }
    /// `SetSpacing`.
    pub fn set_spacing(&mut self, i: i32) {
        self.spacing = i;
    }
}

impl ItemDelegate for DefaultDelegate {
    fn height(&self) -> i32 {
        if self.show_description {
            self.height
        } else {
            1
        }
    }

    fn spacing(&self) -> i32 {
        self.spacing
    }

    fn update(&self, msg: &Msg, m: &mut Model) -> Option<Cmd> {
        match &self.update_func {
            None => None,
            Some(f) => f(msg, m),
        }
    }

    fn render(&self, w: &mut String, m: &Model, index: i32, item: &Rc<dyn Item>) {
        let s = &self.styles;
        let Some(di) = item.as_default_item() else {
            return;
        };
        let mut title = di.title();
        let mut desc = di.description();
        if m.width <= 0 {
            return;
        }
        let textwidth =
            m.width - s.normal_title.get_padding_left() - s.normal_title.get_padding_right();
        title = ansi::truncate(&title, textwidth, ELLIPSIS);
        if self.show_description {
            let mut lines = Vec::new();
            for (i, line) in desc.split('\n').enumerate() {
                if i as i32 >= self.height - 1 {
                    break;
                }
                lines.push(ansi::truncate(line, textwidth, ELLIPSIS));
            }
            desc = lines.join("\n");
        }

        let is_selected = index == m.index();
        let empty_filter =
            m.filter_state() == FilterState::Filtering && m.filter_value().is_empty();
        let is_filtered = m.filter_state() == FilterState::Filtering
            || m.filter_state() == FilterState::FilterApplied;

        let mut matched_runes: Vec<i32> = Vec::new();
        if is_filtered && (index as usize) < m.filtered_items.len() {
            matched_runes = m.matches_for_item(index);
        }

        if empty_filter {
            title = s.dimmed_title.render(&title);
            desc = s.dimmed_desc.render(&desc);
        } else if is_selected && m.filter_state() != FilterState::Filtering {
            if is_filtered {
                let unmatched = s.selected_title.clone().inline(true);
                let matched = unmatched.clone().inherit(&s.filter_match);
                title = lipgloss::style_runes(&title, &matched_runes, &matched, &unmatched);
            }
            title = s.selected_title.render(&title);
            desc = s.selected_desc.render(&desc);
        } else {
            if is_filtered {
                let unmatched = s.normal_title.clone().inline(true);
                let matched = unmatched.clone().inherit(&s.filter_match);
                title = lipgloss::style_runes(&title, &matched_runes, &matched, &unmatched);
            }
            title = s.normal_title.render(&title);
            desc = s.normal_desc.render(&desc);
        }

        if self.show_description {
            w.push_str(&format!("{title}\n{desc}"));
            return;
        }
        w.push_str(&title);
    }

    fn short_help(&self) -> Option<Vec<Binding>> {
        Some(
            self.short_help_func
                .as_ref()
                .map(|f| f())
                .unwrap_or_default(),
        )
    }

    fn full_help(&self) -> Option<Vec<Vec<Binding>>> {
        Some(
            self.full_help_func
                .as_ref()
                .map(|f| f())
                .unwrap_or_default(),
        )
    }
}
