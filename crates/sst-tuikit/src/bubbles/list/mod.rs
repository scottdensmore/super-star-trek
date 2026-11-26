//! `bubbles/list` (list.go): a paginated, filterable list.
//!
//! Items are `Rc<dyn Item>`; Go's type assertions become
//! `item.as_any().downcast_ref::<MyItem>()` and `item.(DefaultItem)` becomes
//! [`Item::as_default_item`]. Delegates are `Rc<dyn ItemDelegate>`.

mod defaultitem;
mod keys;
mod style;

use std::any::Any;
use std::rc::Rc;
use std::time::Duration;

pub use defaultitem::{
    new_default_delegate, new_default_item_styles, DefaultDelegate, DefaultItem, DefaultItemStyles,
};
pub use keys::{default_key_map, KeyMap};
pub use style::{default_styles, Styles, BULLET, ELLIPSIS};

use super::help::{self, KeyMap as HelpKeyMap};
use super::key::{self, Binding};
use super::paginator;
use super::spinner;
use super::textinput;
use crate::ansi;
use crate::fuzzy;
use crate::lipgloss;
use crate::tea::{self, CancelToken, Cmd, KeyMsg, Msg};

/// Gives `&dyn Any` access to trait objects.
pub trait AsAny: Any {
    fn as_any(&self) -> &dyn Any;
}

impl<T: Any> AsAny for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// `list.Item`.
pub trait Item: AsAny {
    /// `FilterValue`.
    fn filter_value(&self) -> String;
    /// `item.(list.DefaultItem)`: implement to return `Some(self)` for items
    /// with a title and description.
    fn as_default_item(&self) -> Option<&dyn DefaultItem> {
        None
    }
}

/// `list.ItemDelegate`.
pub trait ItemDelegate {
    fn render(&self, w: &mut String, m: &Model, index: i32, item: &Rc<dyn Item>);
    fn height(&self) -> i32;
    fn spacing(&self) -> i32;
    fn update(&self, msg: &Msg, m: &mut Model) -> Option<Cmd>;
    /// `delegate.(help.KeyMap).ShortHelp()`, `None` if not implemented.
    fn short_help(&self) -> Option<Vec<Binding>> {
        None
    }
    /// `delegate.(help.KeyMap).FullHelp()`, `None` if not implemented.
    fn full_help(&self) -> Option<Vec<Vec<Binding>>> {
        None
    }
}

/// `filteredItem`.
#[derive(Clone)]
pub struct FilteredItem {
    pub index: i32,
    pub item: Rc<dyn Item>,
    pub matches: Vec<i32>,
}

/// `list.FilterMatchesMsg`.
#[derive(Clone)]
pub struct FilterMatchesMsg(pub Vec<FilteredItem>);

/// `list.Rank`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Rank {
    pub index: i32,
    pub matched_indexes: Vec<i32>,
}

/// `list.FilterFunc`.
pub type FilterFunc = Rc<dyn Fn(&str, &[String]) -> Vec<Rank>>;

/// `list.DefaultFilter`: sahilm/fuzzy ranking (sorted twice, like Go).
pub fn default_filter(term: &str, targets: &[String]) -> Vec<Rank> {
    let mut ranks = fuzzy::find(term, targets);
    fuzzy::sort_matches(&mut ranks);
    ranks
        .into_iter()
        .map(|r| Rank {
            index: r.index,
            matched_indexes: r.matched_indexes,
        })
        .collect()
}

/// `list.UnsortedFilter`.
pub fn unsorted_filter(term: &str, targets: &[String]) -> Vec<Rank> {
    fuzzy::find_no_sort(term, targets)
        .into_iter()
        .map(|r| Rank {
            index: r.index,
            matched_indexes: r.matched_indexes,
        })
        .collect()
}

/// `statusMessageTimeoutMsg`.
#[derive(Clone, Copy, Debug)]
pub struct StatusMessageTimeoutMsg;

/// `list.FilterState`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FilterState {
    #[default]
    Unfiltered,
    Filtering,
    FilterApplied,
}

impl FilterState {
    pub fn string(self) -> &'static str {
        match self {
            FilterState::Unfiltered => "unfiltered",
            FilterState::Filtering => "filtering",
            FilterState::FilterApplied => "filter applied",
        }
    }
}

fn clamp(v: i32, low: i32, high: i32) -> i32 {
    let (low, high) = if low > high { (high, low) } else { (low, high) };
    high.min(low.max(v))
}

/// `list.Model`.
#[derive(Clone)]
pub struct Model {
    show_title: bool,
    show_filter: bool,
    show_status_bar: bool,
    show_pagination: bool,
    show_help: bool,
    filtering_enabled: bool,
    item_name_singular: String,
    item_name_plural: String,
    pub title: String,
    pub styles: Styles,
    pub infinite_scrolling: bool,
    pub key_map: KeyMap,
    pub filter: FilterFunc,
    disable_quit_keybindings: bool,
    pub additional_short_help_keys: Option<Rc<dyn Fn() -> Vec<Binding>>>,
    pub additional_full_help_keys: Option<Rc<dyn Fn() -> Vec<Binding>>>,
    spinner: spinner::Model,
    show_spinner: bool,
    pub(crate) width: i32,
    height: i32,
    pub paginator: paginator::Model,
    cursor: i32,
    pub help: help::Model,
    pub filter_input: textinput::Model,
    filter_state: FilterState,
    pub status_message_lifetime: Duration,
    status_message: String,
    status_message_timer: Option<CancelToken>,
    items: Vec<Rc<dyn Item>>,
    pub(crate) filtered_items: Vec<FilteredItem>,
    filtered_is_nil: bool,
    delegate: Rc<dyn ItemDelegate>,
}

/// `list.New(items, delegate, width, height)`.
pub fn new(
    items: Vec<Rc<dyn Item>>,
    delegate: impl ItemDelegate + 'static,
    width: i32,
    height: i32,
) -> Model {
    let styles = default_styles();
    let mut sp = spinner::new();
    sp.spinner = spinner::line();
    sp.style = styles.spinner.clone();

    let mut filter_input = textinput::new();
    filter_input.prompt = "Filter: ".into();
    filter_input.prompt_style = styles.filter_prompt.clone();
    filter_input.cursor.style = styles.filter_cursor.clone();
    filter_input.char_limit = 64;
    let _ = filter_input.focus();

    let mut p = paginator::new();
    p.typ = paginator::Type::Dots;
    p.active_dot = styles.active_pagination_dot.string();
    p.inactive_dot = styles.inactive_pagination_dot.string();

    let mut m = Model {
        show_title: true,
        show_filter: true,
        show_status_bar: true,
        show_pagination: true,
        show_help: true,
        filtering_enabled: true,
        item_name_singular: "item".into(),
        item_name_plural: "items".into(),
        title: "List".into(),
        styles,
        infinite_scrolling: false,
        key_map: default_key_map(),
        filter: Rc::new(default_filter),
        disable_quit_keybindings: false,
        additional_short_help_keys: None,
        additional_full_help_keys: None,
        spinner: sp,
        show_spinner: false,
        width,
        height,
        paginator: p,
        cursor: 0,
        help: help::new(),
        filter_input,
        filter_state: FilterState::Unfiltered,
        status_message_lifetime: Duration::from_secs(1),
        status_message: String::new(),
        status_message_timer: None,
        items,
        filtered_items: Vec::new(),
        filtered_is_nil: true,
        delegate: Rc::new(delegate),
    };
    m.update_pagination();
    m.update_keybindings();
    m
}

impl Model {
    pub fn set_filtering_enabled(&mut self, v: bool) {
        self.filtering_enabled = v;
        if !v {
            self.reset_filtering();
        }
        self.update_keybindings();
    }
    pub fn filtering_enabled(&self) -> bool {
        self.filtering_enabled
    }
    pub fn set_show_title(&mut self, v: bool) {
        self.show_title = v;
        self.update_pagination();
    }

    /// `SetFilterText`.
    pub fn set_filter_text(&mut self, filter: &str) {
        self.filter_state = FilterState::Filtering;
        self.filter_input.set_value(filter);
        let c = filter_items(self);
        if let Some(m) = c.and_then(|c| c.call()) {
            if let Some(fmm) = m.downcast_ref::<FilterMatchesMsg>() {
                self.set_filtered(fmm.0.clone());
            } else {
                self.set_filtered(Vec::new());
            }
        }
        self.filter_state = FilterState::FilterApplied;
        self.go_to_start();
        self.filter_input.cursor_end();
        self.update_pagination();
        self.update_keybindings();
    }

    /// `SetFilterState`.
    pub fn set_filter_state(&mut self, state: FilterState) {
        self.go_to_start();
        self.filter_state = state;
        self.filter_input.cursor_end();
        let _ = self.filter_input.focus();
        self.update_keybindings();
    }

    pub fn show_title(&self) -> bool {
        self.show_title
    }
    pub fn set_show_filter(&mut self, v: bool) {
        self.show_filter = v;
        self.update_pagination();
    }
    pub fn show_filter(&self) -> bool {
        self.show_filter
    }
    pub fn set_show_status_bar(&mut self, v: bool) {
        self.show_status_bar = v;
        self.update_pagination();
    }
    pub fn show_status_bar(&self) -> bool {
        self.show_status_bar
    }
    pub fn set_status_bar_item_name(&mut self, singular: &str, plural: &str) {
        self.item_name_singular = singular.into();
        self.item_name_plural = plural.into();
    }
    pub fn status_bar_item_name(&self) -> (String, String) {
        (
            self.item_name_singular.clone(),
            self.item_name_plural.clone(),
        )
    }
    pub fn set_show_pagination(&mut self, v: bool) {
        self.show_pagination = v;
        self.update_pagination();
    }
    pub fn show_pagination(&self) -> bool {
        self.show_pagination
    }
    pub fn set_show_help(&mut self, v: bool) {
        self.show_help = v;
        self.update_pagination();
    }
    pub fn show_help(&self) -> bool {
        self.show_help
    }
    pub fn items(&self) -> Vec<Rc<dyn Item>> {
        self.items.clone()
    }

    fn set_filtered(&mut self, v: Vec<FilteredItem>) {
        self.filtered_items = v;
        self.filtered_is_nil = false;
    }

    /// `SetItems`.
    pub fn set_items(&mut self, i: Vec<Rc<dyn Item>>) -> Option<Cmd> {
        let mut cmd = None;
        self.items = i;
        if self.filter_state != FilterState::Unfiltered {
            self.filtered_items = Vec::new();
            self.filtered_is_nil = true;
            cmd = filter_items(self);
        }
        self.update_pagination();
        self.update_keybindings();
        cmd
    }

    /// `Select`.
    pub fn select(&mut self, index: i32) {
        self.paginator.page = index / self.paginator.per_page;
        self.cursor = index % self.paginator.per_page;
    }
    pub fn reset_selected(&mut self) {
        self.select(0);
    }
    pub fn reset_filter(&mut self) {
        self.reset_filtering();
    }

    /// `SetItem`.
    pub fn set_item(&mut self, index: usize, item: Rc<dyn Item>) -> Option<Cmd> {
        let mut cmd = None;
        self.items[index] = item;
        if self.filter_state != FilterState::Unfiltered {
            cmd = filter_items(self);
        }
        self.update_pagination();
        cmd
    }

    /// `InsertItem`.
    pub fn insert_item(&mut self, index: i32, item: Rc<dyn Item>) -> Option<Cmd> {
        let mut cmd = None;
        if index as usize >= self.items.len() {
            self.items.push(item);
        } else {
            self.items.insert(index.max(0) as usize, item);
        }
        if self.filter_state != FilterState::Unfiltered {
            cmd = filter_items(self);
        }
        self.update_pagination();
        self.update_keybindings();
        cmd
    }

    /// `RemoveItem`.
    pub fn remove_item(&mut self, index: usize) {
        if index < self.items.len() {
            self.items.remove(index);
        }
        if self.filter_state != FilterState::Unfiltered {
            if index < self.filtered_items.len() {
                self.filtered_items.remove(index);
            }
            if self.filtered_items.is_empty() {
                self.reset_filtering();
            }
        }
        self.update_pagination();
    }

    /// `SetDelegate`.
    pub fn set_delegate(&mut self, d: impl ItemDelegate + 'static) {
        self.delegate = Rc::new(d);
        self.update_pagination();
    }

    /// `VisibleItems`.
    pub fn visible_items(&self) -> Vec<Rc<dyn Item>> {
        if self.filter_state != FilterState::Unfiltered {
            return self.filtered_items.iter().map(|f| f.item.clone()).collect();
        }
        self.items.clone()
    }

    fn visible_len(&self) -> i32 {
        if self.filter_state != FilterState::Unfiltered {
            self.filtered_items.len() as i32
        } else {
            self.items.len() as i32
        }
    }

    /// `SelectedItem` (`None` is Go's nil).
    pub fn selected_item(&self) -> Option<Rc<dyn Item>> {
        let i = self.index();
        let items = self.visible_items();
        if i < 0 || items.is_empty() || items.len() as i32 <= i {
            return None;
        }
        Some(items[i as usize].clone())
    }

    /// `MatchesForItem`.
    pub fn matches_for_item(&self, index: i32) -> Vec<i32> {
        if self.filtered_is_nil || index as usize >= self.filtered_items.len() {
            return Vec::new();
        }
        self.filtered_items[index as usize].matches.clone()
    }

    /// `Index`.
    pub fn index(&self) -> i32 {
        self.paginator.page * self.paginator.per_page + self.cursor
    }

    /// `GlobalIndex`.
    pub fn global_index(&self) -> i32 {
        let index = self.index();
        if self.filtered_is_nil || index as usize >= self.filtered_items.len() {
            return index;
        }
        self.filtered_items[index as usize].index
    }

    /// `Cursor`.
    pub fn cursor(&self) -> i32 {
        self.cursor
    }

    /// `CursorUp`.
    pub fn cursor_up(&mut self) {
        self.cursor -= 1;
        if self.cursor < 0 && self.paginator.on_first_page() {
            if self.infinite_scrolling {
                self.go_to_end();
                return;
            }
            self.cursor = 0;
            return;
        }
        if self.cursor >= 0 {
            return;
        }
        self.paginator.prev_page();
        self.cursor = self.max_cursor_index();
    }

    /// `CursorDown`.
    pub fn cursor_down(&mut self) {
        let max_cursor_index = self.max_cursor_index();
        self.cursor += 1;
        if self.cursor <= max_cursor_index {
            return;
        }
        if !self.paginator.on_last_page() {
            self.paginator.next_page();
            self.cursor = 0;
            return;
        }
        self.cursor = 0.max(max_cursor_index);
        if self.infinite_scrolling {
            self.go_to_start();
        }
    }

    pub fn go_to_start(&mut self) {
        self.paginator.page = 0;
        self.cursor = 0;
    }

    pub fn go_to_end(&mut self) {
        self.paginator.page = 0.max(self.paginator.total_pages - 1);
        self.cursor = self.max_cursor_index();
    }

    /// `PrevPage`.
    pub fn prev_page(&mut self) {
        self.paginator.prev_page();
        self.cursor = clamp(self.cursor, 0, self.max_cursor_index());
    }

    /// `NextPage`.
    pub fn next_page(&mut self) {
        self.paginator.next_page();
        self.cursor = clamp(self.cursor, 0, self.max_cursor_index());
    }

    fn max_cursor_index(&self) -> i32 {
        0.max(self.paginator.items_on_page(self.visible_len()) - 1)
    }

    pub fn filter_state(&self) -> FilterState {
        self.filter_state
    }
    pub fn filter_value(&self) -> String {
        self.filter_input.value()
    }
    pub fn setting_filter(&self) -> bool {
        self.filter_state == FilterState::Filtering
    }
    pub fn is_filtered(&self) -> bool {
        self.filter_state == FilterState::FilterApplied
    }
    pub fn width(&self) -> i32 {
        self.width
    }
    pub fn height(&self) -> i32 {
        self.height
    }
    pub fn set_spinner(&mut self, s: spinner::Spinner) {
        self.spinner.spinner = s;
    }
    pub fn toggle_spinner(&mut self) -> Option<Cmd> {
        if !self.show_spinner {
            return self.start_spinner();
        }
        self.stop_spinner();
        None
    }
    pub fn start_spinner(&mut self) -> Option<Cmd> {
        self.show_spinner = true;
        self.spinner.tick()
    }
    pub fn stop_spinner(&mut self) {
        self.show_spinner = false;
    }

    /// `DisableQuitKeybindings`.
    pub fn disable_quit_keybindings(&mut self) {
        self.disable_quit_keybindings = true;
        self.key_map.quit.set_enabled(false);
        self.key_map.force_quit.set_enabled(false);
    }

    /// `NewStatusMessage`.
    pub fn new_status_message(&mut self, s: &str) -> Option<Cmd> {
        self.status_message = s.to_string();
        if let Some(t) = &self.status_message_timer {
            t.set(true);
        }
        let token: CancelToken = Rc::new(std::cell::Cell::new(false));
        self.status_message_timer = Some(token.clone());
        tea::cancellable_tick(
            self.status_message_lifetime,
            token,
            || Some(tea::msg(StatusMessageTimeoutMsg)),
            || None,
        )
    }

    pub fn set_width(&mut self, v: i32) {
        self.set_size(v, self.height);
    }
    pub fn set_height(&mut self, v: i32) {
        self.set_size(self.width, v);
    }

    /// `SetSize`.
    pub fn set_size(&mut self, width: i32, height: i32) {
        let prompt_width = lipgloss::width(&self.styles.title.render(&self.filter_input.prompt));
        self.width = width;
        self.height = height;
        self.help.width = width;
        self.filter_input.width = width - prompt_width - lipgloss::width(&self.spinner_view());
        self.update_pagination();
        self.update_keybindings();
    }

    fn reset_filtering(&mut self) {
        if self.filter_state == FilterState::Unfiltered {
            return;
        }
        self.filter_state = FilterState::Unfiltered;
        self.filter_input.reset();
        self.filtered_items = Vec::new();
        self.filtered_is_nil = true;
        self.update_pagination();
        self.update_keybindings();
    }

    fn items_as_filter_items(&self) -> Vec<FilteredItem> {
        self.items
            .iter()
            .map(|it| FilteredItem {
                index: 0,
                item: it.clone(),
                matches: Vec::new(),
            })
            .collect()
    }

    fn update_keybindings(&mut self) {
        match self.filter_state {
            FilterState::Filtering => {
                let km = &mut self.key_map;
                km.cursor_up.set_enabled(false);
                km.cursor_down.set_enabled(false);
                km.next_page.set_enabled(false);
                km.prev_page.set_enabled(false);
                km.go_to_start.set_enabled(false);
                km.go_to_end.set_enabled(false);
                km.filter.set_enabled(false);
                km.clear_filter.set_enabled(false);
                km.cancel_while_filtering.set_enabled(true);
                let v = !self.filter_input.value().is_empty();
                let km = &mut self.key_map;
                km.accept_while_filtering.set_enabled(v);
                km.quit.set_enabled(false);
                km.show_full_help.set_enabled(false);
                km.close_full_help.set_enabled(false);
            }
            _ => {
                let has_items = !self.items.is_empty();
                let has_pages = self.paginator.total_pages > 1;
                let filtering_enabled = self.filtering_enabled;
                let applied = self.filter_state == FilterState::FilterApplied;
                let quit = !self.disable_quit_keybindings;
                let km = &mut self.key_map;
                km.cursor_up.set_enabled(has_items);
                km.cursor_down.set_enabled(has_items);
                km.next_page.set_enabled(has_pages);
                km.prev_page.set_enabled(has_pages);
                km.go_to_start.set_enabled(has_items);
                km.go_to_end.set_enabled(has_items);
                km.filter.set_enabled(filtering_enabled && has_items);
                km.clear_filter.set_enabled(applied);
                km.cancel_while_filtering.set_enabled(false);
                km.accept_while_filtering.set_enabled(false);
                km.quit.set_enabled(quit);
                if self.help.show_all {
                    self.key_map.show_full_help.set_enabled(true);
                    self.key_map.close_full_help.set_enabled(true);
                } else {
                    let min_help = count_enabled_bindings(&self.full_help()) > 1;
                    self.key_map.show_full_help.set_enabled(min_help);
                    self.key_map.close_full_help.set_enabled(min_help);
                }
            }
        }
    }

    fn update_pagination(&mut self) {
        let index = self.index();
        let mut avail_height = self.height;
        if self.show_title || (self.show_filter && self.filtering_enabled) {
            avail_height -= lipgloss::height(&self.title_view());
        }
        if self.show_status_bar {
            avail_height -= lipgloss::height(&self.status_view());
        }
        if self.show_pagination {
            avail_height -= lipgloss::height(&self.pagination_view());
        }
        if self.show_help {
            avail_height -= lipgloss::height(&self.help_view());
        }
        let per = self.delegate.height() + self.delegate.spacing();
        self.paginator.per_page = 1.max(go_div(avail_height, per));
        let pages = self.visible_len();
        if pages < 1 {
            self.paginator.set_total_pages(1);
        } else {
            self.paginator.set_total_pages(pages);
        }
        self.paginator.page = index / self.paginator.per_page;
        self.cursor = index % self.paginator.per_page;
        if self.paginator.page >= self.paginator.total_pages - 1 {
            self.paginator.page = 0.max(self.paginator.total_pages - 1);
        }
    }

    fn hide_status_message(&mut self) {
        self.status_message.clear();
        if let Some(t) = &self.status_message_timer {
            t.set(true);
        }
    }

    /// `Update`.
    pub fn update(&mut self, msg: &Msg) -> Option<Cmd> {
        let mut cmds = Vec::new();
        if let Some(k) = msg.downcast_ref::<KeyMsg>() {
            if key::matches(k, &[&self.key_map.force_quit]) {
                return tea::quit();
            }
        } else if let Some(fm) = msg.downcast_ref::<FilterMatchesMsg>() {
            self.set_filtered(fm.0.clone());
            return None;
        } else if msg.downcast_ref::<spinner::TickMsg>().is_some() {
            let c = self.spinner.update(msg);
            if self.show_spinner {
                cmds.push(c);
            }
        } else if msg.downcast_ref::<StatusMessageTimeoutMsg>().is_some() {
            self.hide_status_message();
        }

        if self.filter_state == FilterState::Filtering {
            cmds.push(self.handle_filtering(msg));
        } else {
            cmds.push(self.handle_browsing(msg));
        }
        tea::batch(cmds)
    }

    fn handle_browsing(&mut self, msg: &Msg) -> Option<Cmd> {
        let mut cmds = Vec::new();
        if let Some(k) = msg.downcast_ref::<KeyMsg>() {
            let km = self.key_map.clone();
            let m = |b: &Binding| key::matches(k, &[b]);
            if m(&km.clear_filter) {
                self.reset_filtering();
            } else if m(&km.quit) {
                return tea::quit();
            } else if m(&km.cursor_up) {
                self.cursor_up();
            } else if m(&km.cursor_down) {
                self.cursor_down();
            } else if m(&km.prev_page) {
                self.paginator.prev_page();
            } else if m(&km.next_page) {
                self.paginator.next_page();
            } else if m(&km.go_to_start) {
                self.go_to_start();
            } else if m(&km.go_to_end) {
                self.go_to_end();
            } else if m(&km.filter) {
                self.hide_status_message();
                if self.filter_input.value().is_empty() {
                    let fi = self.items_as_filter_items();
                    self.set_filtered(fi);
                }
                self.go_to_start();
                self.filter_state = FilterState::Filtering;
                self.filter_input.cursor_end();
                let _ = self.filter_input.focus();
                self.update_keybindings();
                return textinput::blink();
            } else if m(&km.show_full_help) || m(&km.close_full_help) {
                self.help.show_all = !self.help.show_all;
                self.update_pagination();
            }
        }
        let delegate = self.delegate.clone();
        let c = delegate.update(msg, self);
        cmds.push(c);
        self.cursor = clamp(self.cursor, 0, self.max_cursor_index());
        tea::batch(cmds)
    }

    fn handle_filtering(&mut self, msg: &Msg) -> Option<Cmd> {
        let mut cmds = Vec::new();
        if let Some(k) = msg.downcast_ref::<KeyMsg>() {
            if key::matches(k, &[&self.key_map.cancel_while_filtering]) {
                self.reset_filtering();
                self.key_map.filter.set_enabled(true);
                self.key_map.clear_filter.set_enabled(false);
            } else if key::matches(k, &[&self.key_map.accept_while_filtering]) {
                self.hide_status_message();
                if !self.items.is_empty() {
                    let h = self.visible_items();
                    if h.is_empty() {
                        self.reset_filtering();
                    } else {
                        self.filter_input.blur();
                        self.filter_state = FilterState::FilterApplied;
                        self.update_keybindings();
                        if self.filter_input.value().is_empty() {
                            self.reset_filtering();
                        }
                    }
                }
            }
        }
        let before = self.filter_input.value();
        let input_cmd = self.filter_input.update(msg);
        let filter_changed = before != self.filter_input.value();
        cmds.push(input_cmd);
        if filter_changed {
            cmds.push(filter_items(self));
            let v = !self.filter_input.value().is_empty();
            self.key_map.accept_while_filtering.set_enabled(v);
        }
        self.update_pagination();
        tea::batch(cmds)
    }

    /// `View`.
    pub fn view(&self) -> String {
        let mut sections: Vec<String> = Vec::new();
        let mut avail_height = self.height;
        if self.show_title || (self.show_filter && self.filtering_enabled) {
            let v = self.title_view();
            avail_height -= lipgloss::height(&v);
            sections.push(v);
        }
        if self.show_status_bar {
            let v = self.status_view();
            avail_height -= lipgloss::height(&v);
            sections.push(v);
        }
        let mut pagination = String::new();
        if self.show_pagination {
            pagination = self.pagination_view();
            avail_height -= lipgloss::height(&pagination);
        }
        let mut help = String::new();
        if self.show_help {
            help = self.help_view();
            avail_height -= lipgloss::height(&help);
        }
        let content = lipgloss::new_style()
            .height(avail_height)
            .render(&self.populated_view());
        sections.push(content);
        if self.show_pagination {
            sections.push(pagination);
        }
        if self.show_help {
            sections.push(help);
        }
        let refs: Vec<&str> = sections.iter().map(|s| s.as_str()).collect();
        lipgloss::join_vertical(lipgloss::LEFT, &refs)
    }

    fn title_view(&self) -> String {
        let mut view = String::new();
        let mut title_bar_style = self.styles.title_bar.clone();
        let spinner_view = self.spinner_view();
        let spinner_width = lipgloss::width(&spinner_view);
        let spinner_left_gap = " ";
        let spinner_on_left = title_bar_style.get_padding_left()
            >= spinner_width + lipgloss::width(spinner_left_gap)
            && self.show_spinner;

        if self.show_filter && self.filter_state == FilterState::Filtering {
            view += &self.filter_input.view();
        } else if self.show_title {
            if self.show_spinner && spinner_on_left {
                view += &spinner_view;
                view += spinner_left_gap;
                let gap = title_bar_style.get_padding_left();
                title_bar_style = title_bar_style
                    .padding_left(gap - spinner_width - lipgloss::width(spinner_left_gap));
            }
            view += &self.styles.title.render(&self.title);
            if self.filter_state != FilterState::Filtering {
                view += "  ";
                view += &self.status_message;
                view = ansi::truncate(&view, self.width - spinner_width, ELLIPSIS);
            }
        }

        if self.show_spinner && !spinner_on_left {
            let avail_space = self.width - lipgloss::width(&self.styles.title_bar.render(&view));
            if avail_space > spinner_width {
                view += &" ".repeat((avail_space - spinner_width) as usize);
                view += &spinner_view;
            }
        }

        if !view.is_empty() {
            return title_bar_style.render(&view);
        }
        view
    }

    fn status_view(&self) -> String {
        let mut status = String::new();
        let total_items = self.items.len() as i32;
        let visible_items = self.visible_len();
        let item_name = if visible_items != 1 {
            &self.item_name_plural
        } else {
            &self.item_name_singular
        };
        let items_display = format!("{visible_items} {item_name}");

        if self.filter_state == FilterState::Filtering {
            if visible_items == 0 {
                status = self.styles.status_empty.render("Nothing matched");
            } else {
                status = items_display;
            }
        } else if self.items.is_empty() {
            status = self
                .styles
                .status_empty
                .render(&format!("No {}", self.item_name_plural));
        } else {
            if self.filter_state == FilterState::FilterApplied {
                let f = self.filter_input.value().trim().to_string();
                let f = ansi::truncate(&f, 10, "…");
                status += &format!("“{f}” ");
            }
            status += &items_display;
        }

        let num_filtered = total_items - visible_items;
        if num_filtered > 0 {
            status += &self.styles.divider_dot.string();
            status += &self
                .styles
                .status_bar_filter_count
                .render(&format!("{num_filtered} filtered"));
        }
        self.styles.status_bar.render(&status)
    }

    fn pagination_view(&self) -> String {
        if self.paginator.total_pages < 2 {
            return String::new();
        }
        let mut s = self.paginator.view();
        if ansi::string_width(&s) > self.width {
            let mut p = self.paginator.clone();
            p.typ = paginator::Type::Arabic;
            s = self.styles.arabic_pagination.render(&p.view());
        }
        let mut style = self.styles.pagination_style.clone();
        if self.delegate.spacing() == 0 && style.get_margin_top() == 0 {
            style = style.margin_top(1);
        }
        style.render(&s)
    }

    fn populated_view(&self) -> String {
        let items = self.visible_items();
        let mut b = String::new();
        if items.is_empty() {
            if self.filter_state == FilterState::Filtering {
                return String::new();
            }
            return self
                .styles
                .no_items
                .render(&format!("No {}.", self.item_name_plural));
        }
        let (start, end) = self.paginator.get_slice_bounds(items.len() as i32);
        let docs = &items[start.max(0) as usize..end.max(start).max(0) as usize];
        for (i, item) in docs.iter().enumerate() {
            self.delegate.render(&mut b, self, i as i32 + start, item);
            if i != docs.len() - 1 {
                b += &"\n".repeat((self.delegate.spacing() + 1).max(0) as usize);
            }
        }
        let items_on_page = self.paginator.items_on_page(items.len() as i32);
        if items_on_page < self.paginator.per_page {
            let n = (self.paginator.per_page - items_on_page)
                * (self.delegate.height() + self.delegate.spacing());
            b += &"\n".repeat(n.max(0) as usize);
        }
        b
    }

    fn help_view(&self) -> String {
        self.styles.help_style.render(&self.help.view(self))
    }

    fn spinner_view(&self) -> String {
        self.spinner.view()
    }
}

impl HelpKeyMap for Model {
    fn short_help(&self) -> Vec<Binding> {
        let mut kb = vec![
            self.key_map.cursor_up.clone(),
            self.key_map.cursor_down.clone(),
        ];
        let filtering = self.filter_state == FilterState::Filtering;
        if !filtering {
            if let Some(b) = self.delegate.short_help() {
                kb.extend(b);
            }
        }
        kb.extend([
            self.key_map.filter.clone(),
            self.key_map.clear_filter.clone(),
            self.key_map.accept_while_filtering.clone(),
            self.key_map.cancel_while_filtering.clone(),
        ]);
        if !filtering {
            if let Some(f) = &self.additional_short_help_keys {
                kb.extend(f());
            }
        }
        kb.push(self.key_map.quit.clone());
        kb.push(self.key_map.show_full_help.clone());
        kb
    }

    fn full_help(&self) -> Vec<Vec<Binding>> {
        let mut kb = vec![vec![
            self.key_map.cursor_up.clone(),
            self.key_map.cursor_down.clone(),
            self.key_map.next_page.clone(),
            self.key_map.prev_page.clone(),
            self.key_map.go_to_start.clone(),
            self.key_map.go_to_end.clone(),
        ]];
        let filtering = self.filter_state == FilterState::Filtering;
        if !filtering {
            if let Some(b) = self.delegate.full_help() {
                kb.extend(b);
            }
        }
        let mut list_level = vec![
            self.key_map.filter.clone(),
            self.key_map.clear_filter.clone(),
            self.key_map.accept_while_filtering.clone(),
            self.key_map.cancel_while_filtering.clone(),
        ];
        if !filtering {
            if let Some(f) = &self.additional_full_help_keys {
                list_level.extend(f());
            }
        }
        kb.push(list_level);
        kb.push(vec![
            self.key_map.quit.clone(),
            self.key_map.close_full_help.clone(),
        ]);
        kb
    }
}

/// Go integer division (truncates toward zero; Rust `/` does too).
fn go_div(a: i32, b: i32) -> i32 {
    if b == 0 {
        // Go panics on integer division by zero.
        panic!("integer divide by zero");
    }
    a / b
}

/// `filterItems(m)`: a command computing the filter matches from a copy of
/// the model state.
fn filter_items(m: &Model) -> Option<Cmd> {
    let value = m.filter_input.value();
    let state = m.filter_state;
    let items = m.items.clone();
    let filter = m.filter.clone();
    tea::cmd(move || {
        if value.is_empty() || state == FilterState::Unfiltered {
            return FilterMatchesMsg(
                items
                    .iter()
                    .map(|it| FilteredItem {
                        index: 0,
                        item: it.clone(),
                        matches: Vec::new(),
                    })
                    .collect(),
            );
        }
        let targets: Vec<String> = items.iter().map(|t| t.filter_value()).collect();
        let mut matches = Vec::new();
        for r in filter(&value, &targets) {
            matches.push(FilteredItem {
                index: r.index,
                item: items[r.index as usize].clone(),
                matches: r.matched_indexes,
            });
        }
        FilterMatchesMsg(matches)
    })
}

fn count_enabled_bindings(groups: &[Vec<Binding>]) -> i32 {
    groups.iter().flatten().filter(|b| b.enabled()).count() as i32
}
