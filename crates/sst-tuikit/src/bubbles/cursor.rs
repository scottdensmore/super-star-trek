//! `bubbles/cursor`: the blinking cursor of text inputs.
//!
//! Go shares the blink context (`*blinkCtx`) between copies of the model;
//! here it is an `Rc<RefCell<_>>`, so clones share it the same way. A new
//! blink command cancels the previous one, which then delivers
//! [`BlinkCanceled`] immediately.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use crate::lipgloss::{self, Style};
use crate::tea::{self, BlurMsg, CancelToken, Cmd, FocusMsg, Msg};

const DEFAULT_BLINK_SPEED: Duration = Duration::from_millis(530);

/// `initialBlinkMsg`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InitialBlinkMsg;

/// `cursor.BlinkMsg`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlinkMsg {
    id: i32,
    tag: i32,
}

/// `blinkCanceled`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlinkCanceled;

#[derive(Debug, Default)]
struct BlinkCtx {
    cancel: Option<CancelToken>,
}

/// `cursor.Mode`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Mode {
    #[default]
    CursorBlink = 0,
    CursorStatic = 1,
    CursorHide = 2,
}

impl Mode {
    pub fn string(self) -> &'static str {
        match self {
            Mode::CursorBlink => "blink",
            Mode::CursorStatic => "static",
            Mode::CursorHide => "hidden",
        }
    }
}

/// The cursor model (`cursor.Model`).
#[derive(Clone, Debug)]
pub struct Model {
    pub blink_speed: Duration,
    pub style: Style,
    pub text_style: Style,
    char: String,
    id: i32,
    focus: bool,
    pub blink: bool,
    blink_ctx: Rc<RefCell<BlinkCtx>>,
    blink_tag: i32,
    mode: Mode,
}

impl Default for Model {
    fn default() -> Self {
        new()
    }
}

/// `cursor.New()`.
pub fn new() -> Model {
    Model {
        blink_speed: DEFAULT_BLINK_SPEED,
        style: lipgloss::new_style(),
        text_style: lipgloss::new_style(),
        char: String::new(),
        id: 0,
        focus: false,
        blink: true,
        blink_ctx: Rc::new(RefCell::new(BlinkCtx::default())),
        blink_tag: 0,
        mode: Mode::CursorBlink,
    }
}

/// `cursor.Blink` (as a command).
pub fn blink() -> Option<Cmd> {
    tea::msg_cmd(InitialBlinkMsg)
}

impl Model {
    /// `Model.Update`.
    pub fn update(&mut self, msg: &Msg) -> Option<Cmd> {
        if msg.downcast_ref::<InitialBlinkMsg>().is_some() {
            if self.mode != Mode::CursorBlink || !self.focus {
                return None;
            }
            return self.blink_cmd();
        }
        if msg.downcast_ref::<FocusMsg>().is_some() {
            return self.focus();
        }
        if msg.downcast_ref::<BlurMsg>().is_some() {
            self.blur();
            return None;
        }
        if let Some(bm) = msg.downcast_ref::<BlinkMsg>() {
            if self.mode != Mode::CursorBlink || !self.focus {
                return None;
            }
            if bm.id != self.id || bm.tag != self.blink_tag {
                return None;
            }
            let mut cmd = None;
            if self.mode == Mode::CursorBlink {
                self.blink = !self.blink;
                cmd = self.blink_cmd();
            }
            return cmd;
        }
        None
    }

    /// `Model.Mode`.
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// `Model.SetMode`.
    pub fn set_mode(&mut self, mode: Mode) -> Option<Cmd> {
        self.mode = mode;
        self.blink = self.mode == Mode::CursorHide || !self.focus;
        if mode == Mode::CursorBlink {
            return blink();
        }
        None
    }

    /// `Model.BlinkCmd`: restarts the blink timer.
    pub fn blink_cmd(&mut self) -> Option<Cmd> {
        if self.mode != Mode::CursorBlink {
            return None;
        }
        {
            let ctx = self.blink_ctx.borrow();
            if let Some(c) = &ctx.cancel {
                c.set(true);
            }
        }
        let token: CancelToken = Rc::new(Cell::new(false));
        self.blink_ctx.borrow_mut().cancel = Some(token.clone());
        self.blink_tag += 1;
        let bm = BlinkMsg {
            id: self.id,
            tag: self.blink_tag,
        };
        tea::cancellable_tick(
            self.blink_speed,
            token,
            move || Some(tea::msg(bm)),
            || Some(tea::msg(BlinkCanceled)),
        )
    }

    /// `Model.Focus`.
    pub fn focus(&mut self) -> Option<Cmd> {
        self.focus = true;
        self.blink = self.mode == Mode::CursorHide;
        if self.mode == Mode::CursorBlink && self.focus {
            return self.blink_cmd();
        }
        None
    }

    /// `Model.Blur`.
    pub fn blur(&mut self) {
        self.focus = false;
        self.blink = true;
    }

    /// `Model.SetChar`.
    pub fn set_char(&mut self, c: &str) {
        self.char = c.to_string();
    }

    /// `Model.View`.
    pub fn view(&self) -> String {
        if self.blink {
            return self.text_style.clone().inline(true).render(&self.char);
        }
        self.style
            .clone()
            .inline(true)
            .reverse(true)
            .render(&self.char)
    }
}
