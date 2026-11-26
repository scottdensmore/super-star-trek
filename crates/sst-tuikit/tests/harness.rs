//! Deterministic harness behaviour: immediate commands run synchronously,
//! timers (tea::tick, cursor blink) wait for the virtual clock, cancelled
//! blinks are delivered at once.

use std::time::Duration;

use sst_tuikit::bubbles::textinput;
use sst_tuikit::tea::{self, harness::Harness, Cmd, KeyMsg, Model, Msg, KEY_ENTER};

struct Submitted(String);
struct Pulse;

struct Bar {
    input: textinput::Model,
    submitted: Vec<String>,
    pulses: u32,
}

impl Model for Bar {
    fn init(&self) -> Option<Cmd> {
        // Go's Init has a value receiver: focusing a copy here would bump
        // the copy's blink tag only (exactly like Go), so start the blink
        // through a message instead.
        tea::batch(vec![
            textinput::blink(),
            tea::tick(Duration::from_millis(600), |_| Pulse),
        ])
    }
    fn update(&mut self, msg: Msg) -> Option<Cmd> {
        if let Some(s) = msg.downcast_ref::<Submitted>() {
            self.submitted.push(s.0.clone());
            return None;
        }
        if msg.downcast_ref::<Pulse>().is_some() {
            self.pulses += 1;
            return tea::tick(Duration::from_millis(600), |_| Pulse);
        }
        if let Some(k) = msg.downcast_ref::<KeyMsg>() {
            if k.typ == KEY_ENTER {
                let v = self.input.value();
                self.input.reset();
                return tea::cmd(move || Submitted(v));
            }
        }
        self.input.update(&msg)
    }
    fn view(&self) -> String {
        self.input.view()
    }
}

#[test]
fn harness_blink_and_ticks() {
    sst_tuikit::lipgloss::set_color_profile(sst_tuikit::termenv::Ascii);
    let mut input = textinput::new();
    input.prompt = "COMMAND> ".into();
    let _ = input.focus();
    let mut h = Harness::new(Bar {
        input,
        submitted: vec![],
        pulses: 0,
    });
    h.init();
    // Blink (530ms) and pulse (600ms) pending.
    let mut p = h.pending_ticks();
    p.sort();
    assert_eq!(
        p,
        vec![Duration::from_millis(530), Duration::from_millis(600)]
    );
    assert!(!h.model.input.cursor.blink);

    h.advance(Duration::from_millis(530));
    assert!(h.model.input.cursor.blink, "cursor blinked off-phase");
    h.advance(Duration::from_millis(70));
    assert_eq!(h.model.pulses, 1);

    // Typing restarts the blink: the previous timer is cancelled at once.
    h.send(KeyMsg::runes("nav"));
    assert_eq!(h.model.input.value(), "nav");
    assert!(!h.model.input.cursor.blink);
    assert_eq!(h.pending_ticks().len(), 2);

    h.send(KeyMsg::new(KEY_ENTER));
    assert_eq!(h.model.submitted, vec!["nav".to_string()]);
    assert_eq!(h.view(), "COMMAND>  ");

    assert!(h.fire_next_tick());
    h.send_input(b"\x1b[A");
    assert!(!h.quit_requested());
}
