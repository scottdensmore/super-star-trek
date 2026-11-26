//! `bubbles/paginator`.

use super::key::{self, Binding};
use crate::tea::{Cmd, KeyMsg, Msg};

/// `paginator.Type`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Type {
    #[default]
    Arabic,
    Dots,
}

/// `paginator.KeyMap`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyMap {
    pub prev_page: Binding,
    pub next_page: Binding,
}

/// `paginator.DefaultKeyMap`.
pub fn default_key_map() -> KeyMap {
    KeyMap {
        prev_page: Binding::new().with_keys(&["pgup", "left", "h"]),
        next_page: Binding::new().with_keys(&["pgdown", "right", "l"]),
    }
}

/// `paginator.Model`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Model {
    pub typ: Type,
    pub page: i32,
    pub per_page: i32,
    pub total_pages: i32,
    pub active_dot: String,
    pub inactive_dot: String,
    /// A `%d/%d` format (only that form is supported).
    pub arabic_format: String,
    pub key_map: KeyMap,
}

/// `paginator.New()`.
pub fn new() -> Model {
    Model {
        typ: Type::Arabic,
        page: 0,
        per_page: 1,
        total_pages: 1,
        active_dot: "•".into(),
        inactive_dot: "○".into(),
        arabic_format: "%d/%d".into(),
        key_map: default_key_map(),
    }
}

impl Default for Model {
    fn default() -> Self {
        new()
    }
}

impl Model {
    /// `SetTotalPages`.
    pub fn set_total_pages(&mut self, items: i32) -> i32 {
        if items < 1 {
            return self.total_pages;
        }
        let mut n = items / self.per_page;
        if items % self.per_page > 0 {
            n += 1;
        }
        self.total_pages = n;
        n
    }

    /// `ItemsOnPage`.
    pub fn items_on_page(&self, total_items: i32) -> i32 {
        if total_items < 1 {
            return 0;
        }
        let (start, end) = self.get_slice_bounds(total_items);
        end - start
    }

    /// `GetSliceBounds`.
    pub fn get_slice_bounds(&self, length: i32) -> (i32, i32) {
        let start = self.page * self.per_page;
        let end = (self.page * self.per_page + self.per_page).min(length);
        (start, end)
    }

    pub fn prev_page(&mut self) {
        if self.page > 0 {
            self.page -= 1;
        }
    }

    pub fn next_page(&mut self) {
        if !self.on_last_page() {
            self.page += 1;
        }
    }

    pub fn on_last_page(&self) -> bool {
        self.page == self.total_pages - 1
    }

    pub fn on_first_page(&self) -> bool {
        self.page == 0
    }

    /// `Update`.
    pub fn update(&mut self, msg: &Msg) -> Option<Cmd> {
        if let Some(k) = msg.downcast_ref::<KeyMsg>() {
            if key::matches(k, &[&self.key_map.next_page]) {
                self.next_page();
            } else if key::matches(k, &[&self.key_map.prev_page]) {
                self.prev_page();
            }
        }
        None
    }

    /// `View`.
    pub fn view(&self) -> String {
        match self.typ {
            Type::Dots => self.dots_view(),
            Type::Arabic => self.arabic_view(),
        }
    }

    fn dots_view(&self) -> String {
        let mut s = String::new();
        for i in 0..self.total_pages {
            if i == self.page {
                s += &self.active_dot;
                continue;
            }
            s += &self.inactive_dot;
        }
        s
    }

    fn arabic_view(&self) -> String {
        self.arabic_format
            .replacen("%d", &(self.page + 1).to_string(), 1)
            .replacen("%d", &self.total_pages.to_string(), 1)
    }
}
