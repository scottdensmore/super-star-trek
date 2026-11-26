//! `bubbles/key`: key bindings and matching.
//!
//! Go `key.NewBinding(key.WithKeys("up", "k"), key.WithHelp("↑/k", "up"))` ->
//! `Binding::new().with_keys(&["up", "k"]).with_help("↑/k", "up")`;
//! `key.Matches(msg, b1, b2)` -> `key::matches(&msg, &[&b1, &b2])`.

use crate::tea::KeyMsg;

/// Help text of a binding (`key.Help`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Help {
    pub key: String,
    pub desc: String,
}

/// A key binding (`key.Binding`). `keys == None` is Go's nil slice.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Binding {
    keys: Option<Vec<String>>,
    help: Help,
    disabled: bool,
}

impl Binding {
    /// `key.NewBinding()` with no options.
    pub fn new() -> Binding {
        Binding::default()
    }
    /// `key.WithKeys(keys...)` (no keys -> nil, i.e. disabled).
    pub fn with_keys(mut self, keys: &[&str]) -> Binding {
        self.keys = if keys.is_empty() {
            None
        } else {
            Some(keys.iter().map(|s| s.to_string()).collect())
        };
        self
    }
    /// `key.WithHelp(key, desc)`.
    pub fn with_help(mut self, key: &str, desc: &str) -> Binding {
        self.help = Help {
            key: key.into(),
            desc: desc.into(),
        };
        self
    }
    /// `key.WithDisabled()`.
    pub fn with_disabled(mut self) -> Binding {
        self.disabled = true;
        self
    }
    /// `Binding.SetKeys`.
    pub fn set_keys(&mut self, keys: &[&str]) {
        self.keys = if keys.is_empty() {
            None
        } else {
            Some(keys.iter().map(|s| s.to_string()).collect())
        };
    }
    /// `Binding.Keys`.
    pub fn keys(&self) -> Vec<String> {
        self.keys.clone().unwrap_or_default()
    }
    /// `Binding.SetHelp`.
    pub fn set_help(&mut self, key: &str, desc: &str) {
        self.help = Help {
            key: key.into(),
            desc: desc.into(),
        };
    }
    /// `Binding.Help`.
    pub fn help(&self) -> &Help {
        &self.help
    }
    /// `Binding.Enabled`.
    pub fn enabled(&self) -> bool {
        !self.disabled && self.keys.is_some()
    }
    /// `Binding.SetEnabled`.
    pub fn set_enabled(&mut self, v: bool) {
        self.disabled = !v;
    }
    /// `Binding.Unbind`.
    pub fn unbind(&mut self) {
        self.keys = None;
        self.help = Help::default();
    }
}

/// `key.Matches(k, bindings...)`.
pub fn matches(k: &KeyMsg, bindings: &[&Binding]) -> bool {
    let keys = k.string();
    for b in bindings {
        if let Some(ks) = &b.keys {
            for v in ks {
                if *v == keys && b.enabled() {
                    return true;
                }
            }
        }
    }
    false
}
