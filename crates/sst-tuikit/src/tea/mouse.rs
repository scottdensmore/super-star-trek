//! Mouse messages (`mouse.go`).

use std::fmt;

/// `tea.MouseAction`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct MouseAction(pub i32);
pub const MOUSE_ACTION_PRESS: MouseAction = MouseAction(0);
pub const MOUSE_ACTION_RELEASE: MouseAction = MouseAction(1);
pub const MOUSE_ACTION_MOTION: MouseAction = MouseAction(2);

/// `tea.MouseButton`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct MouseButton(pub i32);
pub const MOUSE_BUTTON_NONE: MouseButton = MouseButton(0);
pub const MOUSE_BUTTON_LEFT: MouseButton = MouseButton(1);
pub const MOUSE_BUTTON_MIDDLE: MouseButton = MouseButton(2);
pub const MOUSE_BUTTON_RIGHT: MouseButton = MouseButton(3);
pub const MOUSE_BUTTON_WHEEL_UP: MouseButton = MouseButton(4);
pub const MOUSE_BUTTON_WHEEL_DOWN: MouseButton = MouseButton(5);
pub const MOUSE_BUTTON_WHEEL_LEFT: MouseButton = MouseButton(6);
pub const MOUSE_BUTTON_WHEEL_RIGHT: MouseButton = MouseButton(7);
pub const MOUSE_BUTTON_BACKWARD: MouseButton = MouseButton(8);
pub const MOUSE_BUTTON_FORWARD: MouseButton = MouseButton(9);
pub const MOUSE_BUTTON_10: MouseButton = MouseButton(10);
pub const MOUSE_BUTTON_11: MouseButton = MouseButton(11);

/// `tea.MouseEventType` (deprecated in Go, still filled in).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct MouseEventType(pub i32);
pub const MOUSE_UNKNOWN: MouseEventType = MouseEventType(0);
pub const MOUSE_LEFT: MouseEventType = MouseEventType(1);
pub const MOUSE_RIGHT: MouseEventType = MouseEventType(2);
pub const MOUSE_MIDDLE: MouseEventType = MouseEventType(3);
pub const MOUSE_RELEASE: MouseEventType = MouseEventType(4);
pub const MOUSE_WHEEL_UP: MouseEventType = MouseEventType(5);
pub const MOUSE_WHEEL_DOWN: MouseEventType = MouseEventType(6);
pub const MOUSE_WHEEL_LEFT: MouseEventType = MouseEventType(7);
pub const MOUSE_WHEEL_RIGHT: MouseEventType = MouseEventType(8);
pub const MOUSE_BACKWARD: MouseEventType = MouseEventType(9);
pub const MOUSE_FORWARD: MouseEventType = MouseEventType(10);
pub const MOUSE_MOTION: MouseEventType = MouseEventType(11);

/// A mouse event (`tea.MouseMsg` / `tea.MouseEvent`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MouseMsg {
    pub x: i32,
    pub y: i32,
    pub shift: bool,
    pub alt: bool,
    pub ctrl: bool,
    pub action: MouseAction,
    pub button: MouseButton,
    /// Go `Type` (deprecated).
    pub typ: MouseEventType,
}

/// `tea.MouseEvent`.
pub type MouseEvent = MouseMsg;

fn mouse_action_name(a: MouseAction) -> &'static str {
    match a.0 {
        0 => "press",
        1 => "release",
        2 => "motion",
        _ => "",
    }
}

fn mouse_button_name(b: MouseButton) -> &'static str {
    match b.0 {
        0 => "none",
        1 => "left",
        2 => "middle",
        3 => "right",
        4 => "wheel up",
        5 => "wheel down",
        6 => "wheel left",
        7 => "wheel right",
        8 => "backward",
        9 => "forward",
        10 => "button 10",
        11 => "button 11",
        _ => "",
    }
}

impl MouseMsg {
    /// `MouseEvent.IsWheel`.
    pub fn is_wheel(&self) -> bool {
        matches!(self.button.0, 4..=7)
    }

    /// `MouseEvent.String()`, e.g. "ctrl+left press", "wheel up".
    pub fn string(&self) -> String {
        let mut s = String::new();
        if self.ctrl {
            s.push_str("ctrl+");
        }
        if self.alt {
            s.push_str("alt+");
        }
        if self.shift {
            s.push_str("shift+");
        }
        if self.button == MOUSE_BUTTON_NONE {
            if self.action == MOUSE_ACTION_MOTION || self.action == MOUSE_ACTION_RELEASE {
                s.push_str(mouse_action_name(self.action));
            } else {
                s.push_str("unknown");
            }
        } else if self.is_wheel() {
            s.push_str(mouse_button_name(self.button));
        } else {
            let btn = mouse_button_name(self.button);
            if !btn.is_empty() {
                s.push_str(btn);
            }
            let act = mouse_action_name(self.action);
            if !act.is_empty() {
                s.push(' ');
                s.push_str(act);
            }
        }
        s
    }
}

impl fmt::Display for MouseMsg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.string())
    }
}

const X10_MOUSE_BYTE_OFFSET: i32 = 32;

/// `parseMouseButton`.
pub(crate) fn parse_mouse_button(b: i32, is_sgr: bool) -> MouseMsg {
    let mut m = MouseMsg::default();
    let mut e = b;
    if !is_sgr {
        e -= X10_MOUSE_BYTE_OFFSET;
    }
    const BIT_SHIFT: i32 = 0b0000_0100;
    const BIT_ALT: i32 = 0b0000_1000;
    const BIT_CTRL: i32 = 0b0001_0000;
    const BIT_MOTION: i32 = 0b0010_0000;
    const BIT_WHEEL: i32 = 0b0100_0000;
    const BIT_ADD: i32 = 0b1000_0000;
    const BITS_MASK: i32 = 0b0000_0011;

    if e & BIT_ADD != 0 {
        m.button = MouseButton(MOUSE_BUTTON_BACKWARD.0 + (e & BITS_MASK));
    } else if e & BIT_WHEEL != 0 {
        m.button = MouseButton(MOUSE_BUTTON_WHEEL_UP.0 + (e & BITS_MASK));
    } else {
        m.button = MouseButton(MOUSE_BUTTON_LEFT.0 + (e & BITS_MASK));
        if e & BITS_MASK == BITS_MASK {
            m.action = MOUSE_ACTION_RELEASE;
            m.button = MOUSE_BUTTON_NONE;
        }
    }
    if e & BIT_MOTION != 0 && !m.is_wheel() {
        m.action = MOUSE_ACTION_MOTION;
    }
    m.alt = e & BIT_ALT != 0;
    m.ctrl = e & BIT_CTRL != 0;
    m.shift = e & BIT_SHIFT != 0;

    let press = m.action == MOUSE_ACTION_PRESS;
    m.typ = if m.button == MOUSE_BUTTON_LEFT && press {
        MOUSE_LEFT
    } else if m.button == MOUSE_BUTTON_MIDDLE && press {
        MOUSE_MIDDLE
    } else if m.button == MOUSE_BUTTON_RIGHT && press {
        MOUSE_RIGHT
    } else if m.button == MOUSE_BUTTON_NONE && m.action == MOUSE_ACTION_RELEASE {
        MOUSE_RELEASE
    } else if m.button == MOUSE_BUTTON_WHEEL_UP && press {
        MOUSE_WHEEL_UP
    } else if m.button == MOUSE_BUTTON_WHEEL_DOWN && press {
        MOUSE_WHEEL_DOWN
    } else if m.button == MOUSE_BUTTON_WHEEL_LEFT && press {
        MOUSE_WHEEL_LEFT
    } else if m.button == MOUSE_BUTTON_WHEEL_RIGHT && press {
        MOUSE_WHEEL_RIGHT
    } else if m.button == MOUSE_BUTTON_BACKWARD && press {
        MOUSE_BACKWARD
    } else if m.button == MOUSE_BUTTON_FORWARD && press {
        MOUSE_FORWARD
    } else if m.action == MOUSE_ACTION_MOTION {
        match m.button {
            MOUSE_BUTTON_LEFT => MOUSE_LEFT,
            MOUSE_BUTTON_MIDDLE => MOUSE_MIDDLE,
            MOUSE_BUTTON_RIGHT => MOUSE_RIGHT,
            MOUSE_BUTTON_BACKWARD => MOUSE_BACKWARD,
            MOUSE_BUTTON_FORWARD => MOUSE_FORWARD,
            _ => MOUSE_MOTION,
        }
    } else {
        MOUSE_UNKNOWN
    };
    m
}
