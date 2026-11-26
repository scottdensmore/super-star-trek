//! bubbletea's standard renderer (`standard_renderer.go`): frame diffing,
//! only the last `height` lines, every line truncated to the terminal width,
//! alt-screen / cursor / mouse / paste mode sequences.

use std::io::Write;

use crate::ansi;

/// The renderer state. Output is accumulated by [`StandardRenderer::flush`]
/// and the mode methods into an internal buffer that the program writes to
/// the terminal (see [`StandardRenderer::take_output`]).
pub struct StandardRenderer {
    out: Vec<u8>,
    buf: String,
    has_buf: bool,
    last_render: String,
    last_rendered_lines: Vec<String>,
    lines_rendered: i32,
    alt_lines_rendered: i32,
    cursor_hidden: bool,
    alt_screen_active: bool,
    bp_active: bool,
    reporting_focus: bool,
    pub(crate) width: i32,
    pub(crate) height: i32,
    queued_message_lines: Vec<String>,
}

impl Default for StandardRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl StandardRenderer {
    pub fn new() -> Self {
        StandardRenderer {
            out: Vec::new(),
            buf: String::new(),
            has_buf: false,
            last_render: String::new(),
            last_rendered_lines: Vec::new(),
            lines_rendered: 0,
            alt_lines_rendered: 0,
            cursor_hidden: false,
            alt_screen_active: false,
            bp_active: false,
            reporting_focus: false,
            width: 0,
            height: 0,
            queued_message_lines: Vec::new(),
        }
    }

    fn execute(&mut self, seq: &str) {
        self.out.extend_from_slice(seq.as_bytes());
    }

    /// Bytes produced so far (cleared).
    pub fn take_output(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.out)
    }

    /// Writes pending output to `w`.
    pub fn write_to(&mut self, w: &mut dyn Write) -> std::io::Result<()> {
        if !self.out.is_empty() {
            w.write_all(&self.out)?;
            w.flush()?;
            self.out.clear();
        }
        Ok(())
    }

    /// `write`: sets the next frame.
    pub fn write(&mut self, s: &str) {
        self.buf.clear();
        let s = if s.is_empty() { " " } else { s };
        self.buf.push_str(s);
        self.has_buf = true;
    }

    fn last_lines_rendered(&self) -> i32 {
        if self.alt_screen_active {
            self.alt_lines_rendered
        } else {
            self.lines_rendered
        }
    }

    /// `flush`: renders the pending frame if it changed.
    pub fn flush(&mut self) {
        if !self.has_buf || self.buf.is_empty() || self.buf == self.last_render {
            return;
        }
        let mut buf = String::new();
        if self.alt_screen_active {
            buf.push_str(ansi::CURSOR_HOME_POSITION);
        } else if self.lines_rendered > 1 {
            buf.push_str(&ansi::cursor_up(self.lines_rendered - 1));
        }

        let mut new_lines: Vec<String> = self.buf.split('\n').map(|s| s.to_string()).collect();
        if self.height > 0 && new_lines.len() > self.height as usize {
            new_lines = new_lines.split_off(new_lines.len() - self.height as usize);
        }

        let flush_queued = !self.queued_message_lines.is_empty() && !self.alt_screen_active;
        if flush_queued {
            for line in std::mem::take(&mut self.queued_message_lines) {
                let mut line = line;
                if ansi::string_width(&line) < self.width {
                    line.push_str(ansi::ERASE_LINE_RIGHT);
                }
                buf.push_str(&line);
                buf.push_str("\r\n");
            }
        }

        let n = new_lines.len();
        for i in 0..n {
            let can_skip = !flush_queued
                && self.last_rendered_lines.len() > i
                && self.last_rendered_lines[i] == new_lines[i];
            if can_skip {
                if i < n - 1 {
                    buf.push('\n');
                }
                continue;
            }
            if i == 0 && self.last_render.is_empty() {
                buf.push('\r');
            }
            let mut line = new_lines[i].clone();
            if self.width > 0 {
                line = ansi::truncate(&line, self.width, "");
            }
            if ansi::string_width(&line) < self.width {
                line.push_str(ansi::ERASE_LINE_RIGHT);
            }
            buf.push_str(&line);
            if i < n - 1 {
                buf.push_str("\r\n");
            }
        }

        if self.last_lines_rendered() > n as i32 {
            buf.push_str(ansi::ERASE_SCREEN_BELOW);
        }
        if self.alt_screen_active {
            self.alt_lines_rendered = n as i32;
        } else {
            self.lines_rendered = n as i32;
        }
        if self.alt_screen_active {
            buf.push_str(&ansi::cursor_position(0, n as i32));
        } else {
            buf.push('\r');
        }
        self.out.extend_from_slice(buf.as_bytes());
        self.last_render = self.buf.clone();
        self.last_rendered_lines = new_lines;
        self.buf.clear();
        self.has_buf = false;
    }

    /// `stop`: final flush and line erase.
    pub fn stop(&mut self) {
        self.flush();
        self.execute(ansi::ERASE_ENTIRE_LINE);
        self.execute("\r");
    }

    /// `kill`: erase without flushing.
    pub fn kill(&mut self) {
        self.execute(ansi::ERASE_ENTIRE_LINE);
        self.execute("\r");
    }

    pub fn repaint(&mut self) {
        self.last_render.clear();
        self.last_rendered_lines.clear();
    }

    pub fn clear_screen(&mut self) {
        self.execute(ansi::ERASE_ENTIRE_SCREEN);
        self.execute(ansi::CURSOR_HOME_POSITION);
        self.repaint();
    }

    pub fn alt_screen(&self) -> bool {
        self.alt_screen_active
    }

    pub fn enter_alt_screen(&mut self) {
        if self.alt_screen_active {
            return;
        }
        self.alt_screen_active = true;
        self.execute(ansi::SET_ALT_SCREEN_SAVE_CURSOR_MODE);
        self.execute(ansi::ERASE_ENTIRE_SCREEN);
        self.execute(ansi::CURSOR_HOME_POSITION);
        if self.cursor_hidden {
            self.execute(ansi::HIDE_CURSOR);
        } else {
            self.execute(ansi::SHOW_CURSOR);
        }
        self.alt_lines_rendered = 0;
        self.repaint();
    }

    pub fn exit_alt_screen(&mut self) {
        if !self.alt_screen_active {
            return;
        }
        self.alt_screen_active = false;
        self.execute(ansi::RESET_ALT_SCREEN_SAVE_CURSOR_MODE);
        if self.cursor_hidden {
            self.execute(ansi::HIDE_CURSOR);
        } else {
            self.execute(ansi::SHOW_CURSOR);
        }
        self.repaint();
    }

    pub fn show_cursor(&mut self) {
        self.cursor_hidden = false;
        self.execute(ansi::SHOW_CURSOR);
    }
    pub fn hide_cursor(&mut self) {
        self.cursor_hidden = true;
        self.execute(ansi::HIDE_CURSOR);
    }
    pub fn enable_mouse_cell_motion(&mut self) {
        self.execute(ansi::SET_BUTTON_EVENT_MOUSE_MODE);
    }
    pub fn disable_mouse_cell_motion(&mut self) {
        self.execute(ansi::RESET_BUTTON_EVENT_MOUSE_MODE);
    }
    pub fn enable_mouse_all_motion(&mut self) {
        self.execute(ansi::SET_ANY_EVENT_MOUSE_MODE);
    }
    pub fn disable_mouse_all_motion(&mut self) {
        self.execute(ansi::RESET_ANY_EVENT_MOUSE_MODE);
    }
    pub fn enable_mouse_sgr_mode(&mut self) {
        self.execute(ansi::SET_SGR_EXT_MOUSE_MODE);
    }
    pub fn disable_mouse_sgr_mode(&mut self) {
        self.execute(ansi::RESET_SGR_EXT_MOUSE_MODE);
    }
    pub fn enable_bracketed_paste(&mut self) {
        self.execute(ansi::SET_BRACKETED_PASTE_MODE);
        self.bp_active = true;
    }
    pub fn disable_bracketed_paste(&mut self) {
        self.execute(ansi::RESET_BRACKETED_PASTE_MODE);
        self.bp_active = false;
    }
    pub fn bracketed_paste_active(&self) -> bool {
        self.bp_active
    }
    pub fn enable_report_focus(&mut self) {
        self.execute(ansi::SET_FOCUS_EVENT_MODE);
        self.reporting_focus = true;
    }
    pub fn disable_report_focus(&mut self) {
        self.execute(ansi::RESET_FOCUS_EVENT_MODE);
        self.reporting_focus = false;
    }
    pub fn report_focus(&self) -> bool {
        self.reporting_focus
    }
    pub fn set_window_title(&mut self, title: &str) {
        self.execute(&ansi::set_window_title(title));
    }
    /// `handleMessages` for `WindowSizeMsg`.
    pub fn resize(&mut self, width: i32, height: i32) {
        self.width = width;
        self.height = height;
        self.repaint();
    }
    /// Queues a line printed above the program (`Println`, inline mode only).
    pub fn queue_message_line(&mut self, line: &str) {
        if !self.alt_screen_active {
            self.queued_message_lines
                .extend(line.split('\n').map(|s| s.to_string()));
            self.repaint();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flush_alt_screen_frame() {
        let mut r = StandardRenderer::new();
        r.enter_alt_screen();
        r.resize(10, 2);
        let _ = r.take_output();
        r.write("hello\nworld\nthird line that is long");
        r.flush();
        let out = String::from_utf8(r.take_output()).unwrap();
        // Only the last 2 lines, truncated to 10 cells.
        assert_eq!(out, "\x1b[H\rworld\x1b[K\r\nthird line\x1b[2;H");
        // Same frame again: nothing written.
        r.write("hello\nworld\nthird line that is long");
        r.flush();
        assert!(r.take_output().is_empty());
    }
}
