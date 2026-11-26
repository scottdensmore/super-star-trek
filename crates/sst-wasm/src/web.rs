//! wasm-bindgen glue (port of `cmd/wasm/bridge_js.go` and `main.go`).
//!
//! Go's `main` registers the bridge and blocks in `select {}`; here the
//! `#[wasm_bindgen(start)]` function registers the same globals on
//! `globalThis` (`window` in a browser): `sstInit`, `sstCommand`, `sstSave`,
//! `sstLoad`, `sstReset`. The closures are leaked with `forget`, so they stay
//! callable for the lifetime of the page, just like Go's `js.FuncOf` values.

use std::cell::RefCell;
use std::rc::Rc;

use js_sys::{Function, Reflect};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use sst_engine::gofmt::format_float;
use sst_engine::DifficultyProfile;

use crate::bridge::{AudioHost, JsAudioPlayer};
use crate::formatter::format_srs;
use crate::session::Session;

fn global() -> JsValue {
    js_sys::global().into()
}

/// Go `getJSFunction`: `globalThis[name]`, else `window[name]`, if a function.
fn get_js_function(name: &str) -> Option<Function> {
    let g = global();
    if let Ok(v) = Reflect::get(&g, &JsValue::from_str(name)) {
        if let Some(f) = v.dyn_ref::<Function>() {
            return Some(f.clone());
        }
    }
    let win = Reflect::get(&g, &JsValue::from_str("window")).ok()?;
    if win.is_object() {
        if let Ok(v) = Reflect::get(&win, &JsValue::from_str(name)) {
            return v.dyn_into::<Function>().ok();
        }
    }
    None
}

/// The browser's audio hooks (defined by `web/audio.js`).
pub struct WebHost;

impl AudioHost for WebHost {
    fn play_sound(&self, id: &str) {
        if let Some(f) = get_js_function("sstPlaySound") {
            let _ = f.call1(&JsValue::UNDEFINED, &JsValue::from_str(id));
        }
    }

    fn set_muted(&self, muted: bool) {
        if let Some(f) = get_js_function("sstSetMuted") {
            let _ = f.call1(&JsValue::UNDEFINED, &JsValue::from_bool(muted));
        }
    }

    fn is_muted(&self) -> Option<bool> {
        let f = get_js_function("sstIsMuted")?;
        f.call0(&JsValue::UNDEFINED).ok()?.as_bool()
    }

    fn set_volume(&self, vol: i32) {
        if let Some(f) = get_js_function("sstSetVolume") {
            let _ = f.call1(&JsValue::UNDEFINED, &JsValue::from_f64(vol as f64));
        }
    }

    fn get_volume(&self) -> Option<i32> {
        let f = get_js_function("sstGetVolume")?;
        f.call0(&JsValue::UNDEFINED)
            .ok()?
            .as_f64()
            .map(|v| v as i64 as i32)
    }
}

/// Go `js.Value.String()` (non-strings render as `<type: value>`).
fn go_js_string(v: &JsValue) -> String {
    if let Some(s) = v.as_string() {
        return s;
    }
    if v.is_undefined() {
        return "<undefined>".into();
    }
    if v.is_null() {
        return "<null>".into();
    }
    if let Some(b) = v.as_bool() {
        return format!("<boolean: {b}>");
    }
    if let Some(n) = v.as_f64() {
        return format!("<number: {}>", format_float(n, 'g', -1));
    }
    if v.is_symbol() {
        return "<symbol>".into();
    }
    if v.is_function() {
        return "<function>".into();
    }
    "<object>".into()
}

fn js_type_name(v: &JsValue) -> &'static str {
    if v.is_string() {
        "string"
    } else if v.as_bool().is_some() {
        "boolean"
    } else if v.is_symbol() {
        "symbol"
    } else if v.is_function() {
        "function"
    } else {
        "object"
    }
}

/// Go `js.Value.Int()`. Go panics (killing the runtime) on a non-number; this
/// throws the same message as a JS exception instead.
fn go_js_int(v: &JsValue) -> i64 {
    match v.as_f64() {
        Some(n) => n as i64,
        None => wasm_bindgen::throw_str(&format!(
            "syscall/js: call of Value.Int on {}",
            js_type_name(v)
        )),
    }
}

fn is_set(v: &JsValue) -> bool {
    !v.is_null() && !v.is_undefined()
}

thread_local! {
    static PLAYER: Rc<JsAudioPlayer<WebHost>> = Rc::new(JsAudioPlayer::new(WebHost, 80));
    static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) };
}

fn new_session(seed: i64, diff: DifficultyProfile) -> String {
    let mut s = Session::new(seed, Some(diff));
    s.set_audio_player(PLAYER.with(|p| p.clone()));
    let out = format_srs(Some(s.game()));
    SESSION.with(|cell| *cell.borrow_mut() = Some(s));
    out
}

fn with_session<R>(f: impl FnOnce(&mut Session) -> R) -> R {
    SESSION.with(|cell| {
        let mut slot = cell.borrow_mut();
        let s = slot.get_or_insert_with(|| {
            let mut s = Session::new(12345, Some(DifficultyProfile::normal()));
            s.set_audio_player(PLAYER.with(|p| p.clone()));
            s
        });
        f(s)
    })
}

fn set_global(name: &str, f: &JsValue) {
    let _ = Reflect::set(&global(), &JsValue::from_str(name), f);
}

/// Go `registerBridge`: creates the default session and installs the globals.
pub fn register_bridge() {
    new_session(12345, DifficultyProfile::normal());

    let init =
        Closure::<dyn Fn(JsValue, JsValue) -> JsValue>::new(|seed: JsValue, diff: JsValue| {
            let seed = if is_set(&seed) { go_js_int(&seed) } else { 0 };
            let diff = if is_set(&diff) {
                DifficultyProfile::new(go_js_string(&diff))
            } else {
                DifficultyProfile::normal()
            };
            JsValue::from_str(&new_session(seed, diff))
        });
    set_global("sstInit", init.as_ref());
    init.forget();

    // Go returns "" when called with no arguments; an explicit `undefined`
    // argument is indistinguishable from a missing one here.
    let command = Closure::<dyn Fn(JsValue) -> JsValue>::new(|input: JsValue| {
        if input.is_undefined() {
            return JsValue::from_str("");
        }
        let input = go_js_string(&input);
        JsValue::from_str(&with_session(|s| s.execute(&input)))
    });
    set_global("sstCommand", command.as_ref());
    command.forget();

    let save = Closure::<dyn Fn() -> JsValue>::new(|| {
        JsValue::from_str(&with_session(|s| s.save().unwrap_or_default()))
    });
    set_global("sstSave", save.as_ref());
    save.forget();

    let load = Closure::<dyn Fn(JsValue) -> JsValue>::new(|data: JsValue| {
        if data.is_undefined() {
            return JsValue::FALSE;
        }
        let data = go_js_string(&data);
        JsValue::from_bool(with_session(|s| s.load(&data).is_ok()))
    });
    set_global("sstLoad", load.as_ref());
    load.forget();

    let reset = Closure::<dyn Fn() -> JsValue>::new(|| {
        JsValue::from_str(&new_session(0, DifficultyProfile::normal()))
    });
    set_global("sstReset", reset.as_ref());
    reset.forget();
}

/// Module entry point (Go `main`).
#[wasm_bindgen(start)]
pub fn start() {
    register_bridge();
}
