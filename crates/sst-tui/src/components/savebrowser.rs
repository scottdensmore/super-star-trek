//! Port of `pkg/tui/components/savebrowser` (browser.go): the interactive
//! `*.TRK` save browser. Filesystem access goes through [`SaveFs`] so tests
//! can pin modification times; [`OsFs`] is the real filesystem.

use std::rc::Rc;

use sst_engine::actions::EngineError;
use sst_engine::gopath;
use sst_engine::gotime::GoTime;
use sst_engine::persist;
use sst_engine::save::SaveMetadata;
use sst_engine::types::{ConditionType, SkillLevel};
use sst_tuikit::lipgloss;
use sst_tuikit::tea::{self, Cmd, KeyMsg, Msg};

use crate::theme::{default_theme, Theme};

/// Go `LoadGameMsg`: the player confirmed thawing a mission.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadGameMsg {
    pub path: String,
}

/// Go `CloseBrowserMsg`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CloseBrowserMsg;

/// One `os.ReadDir` entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
}

/// The filesystem operations the browser performs. Error strings use Go's
/// `*fs.PathError` text (`open <dir>: no such file or directory`).
pub trait SaveFs {
    /// Go `os.ReadDir(dir)`: entries sorted by name.
    fn read_dir(&self, dir: &str) -> Result<Vec<DirEntry>, String>;
    /// Go `engine.InspectSave(path)`.
    fn inspect_save(&self, path: &str) -> Result<SaveMetadata, EngineError>;
    /// Go `os.Remove(path)`.
    fn remove(&self, path: &str) -> Result<(), String>;
}

/// The real filesystem (mod times in the local zone, like Go).
#[derive(Clone, Copy, Debug, Default)]
pub struct OsFs;

impl SaveFs for OsFs {
    fn read_dir(&self, dir: &str) -> Result<Vec<DirEntry>, String> {
        let rd = std::fs::read_dir(dir).map_err(|e| persist::path_error("open", dir, &e).0)?;
        let mut out = Vec::new();
        for ent in rd {
            let ent = ent.map_err(|e| persist::path_error("readdirent", dir, &e).0)?;
            let name = ent.file_name().to_string_lossy().into_owned();
            let is_dir = ent.file_type().map(|t| t.is_dir()).unwrap_or(false);
            out.push(DirEntry { name, is_dir });
        }
        out.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
        Ok(out)
    }
    fn inspect_save(&self, path: &str) -> Result<SaveMetadata, EngineError> {
        persist::inspect_save(path)
    }
    fn remove(&self, path: &str) -> Result<(), String> {
        let p = std::path::Path::new(path);
        let r = if p.is_dir() {
            std::fs::remove_dir(p)
        } else {
            std::fs::remove_file(p)
        };
        r.map_err(|e| persist::path_error("remove", path, &e).0)
    }
}

/// Go `Model`.
#[derive(Clone)]
pub struct Model {
    theme: Theme,
    directory: String,
    saves: Vec<SaveMetadata>,
    cursor: i32,
    deleting: bool,
    error_message: String,
    width: i32,
    height: i32,
    fs: Rc<dyn SaveFs>,
}

/// Go `New(th, dir)`; `None` is Go's nil theme.
pub fn new(th: impl Into<Option<Theme>>, dir: &str) -> Model {
    new_with_fs(th, dir, Rc::new(OsFs))
}

/// [`new`] with an injected filesystem view.
pub fn new_with_fs(th: impl Into<Option<Theme>>, dir: &str, fs: Rc<dyn SaveFs>) -> Model {
    Model {
        theme: th.into().unwrap_or_else(default_theme),
        directory: if dir.is_empty() {
            ".".into()
        } else {
            dir.into()
        },
        saves: Vec::new(),
        cursor: 0,
        deleting: false,
        error_message: String::new(),
        width: 62,
        height: 14,
        fs,
    }
}

fn after(a: &GoTime, b: &GoTime) -> bool {
    (a.unix, a.nsec) > (b.unix, b.nsec)
}

/// Go `sort.Slice` (pdqsort): insertion sort for n <= 12, which is all the
/// browser ever shows in practice; larger inputs fall back to a stable sort.
fn sort_by_mod_time_desc(v: &mut [SaveMetadata]) {
    if v.len() <= 12 {
        for i in 1..v.len() {
            let mut j = i;
            while j > 0 && after(&v[j].mod_time, &v[j - 1].mod_time) {
                v.swap(j, j - 1);
                j -= 1;
            }
        }
    } else {
        v.sort_by(|a, b| {
            (b.mod_time.unix, b.mod_time.nsec).cmp(&(a.mod_time.unix, a.mod_time.nsec))
        });
    }
}

impl Model {
    /// `SetTheme`.
    pub fn set_theme(&mut self, th: impl Into<Option<Theme>>) {
        self.theme = th.into().unwrap_or_else(default_theme);
    }

    /// `SetSize`.
    pub fn set_size(&mut self, width: i32, height: i32) {
        self.width = width;
        self.height = height;
    }

    /// `Reset`: cursor, deletion state and error message.
    pub fn reset(&mut self) {
        self.cursor = 0;
        self.deleting = false;
        self.error_message.clear();
    }

    /// The scanned saves (most recent first).
    pub fn saves(&self) -> &[SaveMetadata] {
        &self.saves
    }

    /// The cursor index.
    pub fn cursor(&self) -> i32 {
        self.cursor
    }

    /// Whether the delete confirmation is showing.
    pub fn deleting(&self) -> bool {
        self.deleting
    }

    /// The footer error message.
    pub fn error_message(&self) -> &str {
        &self.error_message
    }

    /// `Refresh`: scans the directory for `*.TRK` files.
    pub fn refresh(&mut self) -> Result<(), String> {
        self.error_message.clear();
        let entries = match self.fs.read_dir(&self.directory) {
            Ok(e) => e,
            Err(e) => {
                self.error_message = format!("Failed to read directory: {}", e);
                self.saves.clear();
                self.cursor = 0;
                return Err(e);
            }
        };
        let mut loaded = Vec::new();
        for entry in entries {
            if entry.is_dir {
                continue;
            }
            if entry.name.to_uppercase().ends_with(".TRK") {
                let full = gopath::join(&[&self.directory, &entry.name]);
                if let Ok(meta) = self.fs.inspect_save(&full) {
                    loaded.push(meta);
                }
            }
        }
        sort_by_mod_time_desc(&mut loaded);
        self.saves = loaded;
        if self.cursor >= self.saves.len() as i32 {
            self.cursor = self.saves.len() as i32 - 1;
        }
        if self.cursor < 0 {
            self.cursor = 0;
        }
        Ok(())
    }

    fn up(&mut self) {
        if !self.deleting && self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    fn down(&mut self) {
        if !self.deleting && self.cursor < self.saves.len() as i32 - 1 {
            self.cursor += 1;
        }
    }

    fn in_range(&self) -> bool {
        !self.saves.is_empty() && self.cursor >= 0 && (self.cursor as usize) < self.saves.len()
    }

    /// `Update`: emits [`LoadGameMsg`] / [`CloseBrowserMsg`].
    pub fn update(&mut self, msg: &Msg) -> Option<Cmd> {
        let k = msg.downcast_ref::<KeyMsg>()?;
        match k.typ {
            tea::KEY_UP => self.up(),
            tea::KEY_DOWN => self.down(),
            tea::KEY_ENTER => {
                if self.deleting {
                    if self.in_range() {
                        let target = self.saves[self.cursor as usize].path.clone();
                        match self.fs.remove(&target) {
                            Err(e) => self.error_message = format!("Delete failed: {}", e),
                            Ok(()) => {
                                let _ = self.refresh();
                            }
                        }
                        self.deleting = false;
                    }
                    return None;
                }
                if self.in_range() {
                    let path = self.saves[self.cursor as usize].path.clone();
                    return tea::cmd(move || LoadGameMsg { path });
                }
            }
            tea::KEY_ESC => {
                if self.deleting {
                    self.deleting = false;
                    return None;
                }
                return tea::cmd(|| CloseBrowserMsg);
            }
            tea::KEY_DELETE => {
                if !self.saves.is_empty() {
                    self.deleting = !self.deleting;
                }
            }
            tea::KEY_RUNES => match k.string().as_str() {
                "k" | "K" => self.up(),
                "j" | "J" => self.down(),
                "d" | "D" if !self.saves.is_empty() => self.deleting = !self.deleting,
                _ => {}
            },
            _ => {}
        }
        None
    }

    /// `View`: the 62x14 modal dialog.
    pub fn view(&self) -> String {
        let styles = self.theme.styles();
        let dialog_width = if self.width >= 2 {
            self.width - 2
        } else if self.width > 0 {
            0
        } else {
            60
        };

        let header = styles.title.render(" SAVED MISSIONS (Ctrl+O) ");
        let col_header = styles
            .gauge_label
            .render("  FILE       STARDATE  SKILL   COND    KLINGONS  MODIFIED");

        const VISIBLE_ROWS: usize = 7;
        let mut rows: Vec<String> = Vec::new();
        let start = if self.cursor >= VISIBLE_ROWS as i32 {
            (self.cursor - VISIBLE_ROWS as i32 + 1) as usize
        } else {
            0
        };
        let end = (start + VISIBLE_ROWS).min(self.saves.len());

        if self.saves.is_empty() {
            rows.push(styles.log_text.render("  No saved missions (*.TRK) found."));
        } else {
            for i in start..end {
                let s = &self.saves[i];
                let mark = if i as i32 == self.cursor { "> " } else { "  " };
                let (_, mo, d, hh, mm, _) = s.mod_time.civil();
                let mod_str = format!("{:02}-{:02} {:02}:{:02}", mo, d, hh, mm);
                let base: String = s.filename.chars().take(10).collect();
                let line = format!(
                    "{}{:<10} {:<9} {:<7} {:<7} {:<9} {}",
                    mark,
                    base,
                    format!("{:.1}", s.stardate),
                    format_skill(s.skill),
                    format_condition(s.condition),
                    s.klingons_left,
                    mod_str
                );
                if i as i32 == self.cursor {
                    rows.push(styles.command_text.render(&line));
                } else {
                    rows.push(styles.log_text.render(&line));
                }
            }
        }
        while rows.len() < VISIBLE_ROWS {
            rows.push(String::new());
        }

        let border_fg = styles.panel.get_border_top_foreground();
        let divider_str = if dialog_width > 0 {
            "─".repeat(dialog_width as usize)
        } else {
            String::new()
        };
        let divider = lipgloss::new_style()
            .foreground(border_fg)
            .render(&divider_str);

        let footer =
            if self.deleting && !self.saves.is_empty() && (self.cursor as usize) < self.saves.len()
            {
                styles.gauge_value.render(&format!(
                    "Delete '{}'? [Enter: Confirm / Esc: Cancel]",
                    self.saves[self.cursor as usize].filename
                ))
            } else if !self.error_message.is_empty() {
                styles.gauge_value.render(&self.error_message)
            } else {
                styles
                    .log_text
                    .render("[Enter] Thaw  [D] Delete  [↑/↓] Select  [Esc] Close")
            };

        let body = format!(
            "{}\n\n{}\n{}\n{}\n{}",
            header,
            col_header,
            rows.join("\n"),
            divider,
            footer
        );
        styles.panel.clone().width(dialog_width).render(&body)
    }
}

fn format_skill(skill: SkillLevel) -> &'static str {
    match skill {
        SkillLevel::Novice => "NOVICE",
        SkillLevel::Fair => "FAIR",
        SkillLevel::Good => "GOOD",
        SkillLevel::Expert => "EXPERT",
        SkillLevel::Emeritus => "EMERITUS",
        _ => "UNKNOWN",
    }
}

fn format_condition(c: ConditionType) -> &'static str {
    match c {
        ConditionType::Green => "GREEN",
        ConditionType::Yellow => "YELLOW",
        ConditionType::Red => "RED",
        ConditionType::Docked => "DOCKED",
        _ => "UNKNOWN",
    }
}

#[cfg(test)]
mod parity_tests;
#[cfg(test)]
mod tests;
