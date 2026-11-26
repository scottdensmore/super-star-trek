//! Byte-for-byte parity with `testdata/parity/components/optionsmodal.txt`
//! (recorded by `pkg/tui/components/optionsmodal/zz_paritygen_test.go`).

#[path = "../modal_parity_util.rs"]
#[allow(clippy::duplicate_mod)]
mod util;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use sst_audio::Player;
use sst_engine::gofmt::fmt_v;
use sst_tuikit::termenv::Profile;
use util::*;

use super::*;
use crate::theme::get_theme;

const DATA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/parity/components/optionsmodal.txt"
));

/// Go `pgPlayer`: records every call.
#[derive(Default)]
struct RecPlayer {
    log: RefCell<Vec<String>>,
    vol: Cell<i32>,
    muted: Cell<bool>,
}

impl Player for RecPlayer {
    fn play(&self, s: SoundID) {
        self.log.borrow_mut().push(format!("play={s}"));
    }
    fn set_muted(&self, m: bool) {
        self.muted.set(m);
        self.log.borrow_mut().push(format!("muted={m}"));
    }
    fn is_muted(&self) -> bool {
        self.muted.get()
    }
    fn set_volume(&self, v: i32) {
        self.vol.set(v);
        self.log.borrow_mut().push(format!("vol={v}"));
    }
    fn volume(&self) -> i32 {
        self.vol.get()
    }
}

impl RecPlayer {
    fn take(&self) -> String {
        std::mem::take(&mut *self.log.borrow_mut()).join(",")
    }
}

fn desc(m: &Msg) -> Option<String> {
    m.downcast_ref::<PlaySoundMsg>()
        .map(|v| format!("PlaySoundMsg {}", v.sound))
}

fn rules_str(r: &GameRules) -> String {
    format!(
        "profile={} surv={} deg={} rep={} cloak={} anom={} adv={} tm={} anim={}",
        r.profile,
        r.surveillance,
        r.sensor_degradation,
        fmt_v(r.repair_multiplier),
        r.klingon_cloak,
        r.spatial_anomalies,
        r.adversaries,
        fmt_v(r.time_margin),
        r.anim_speed
    )
}

fn state(m: &Model) -> String {
    format!(
        "row={} closed={} active={} cm={} audio={} mode={} vol={} sound={} rules{{{}}}",
        m.selected_row.0,
        m.closed,
        m.active(),
        m.color_mode(),
        m.audio_enabled(),
        m.audio_mode(),
        m.volume(),
        m.selected_sound(),
        rules_str(&m.rules())
    )
}

const SEQUENCES: &[&[&str]] = &[
    &[
        "right", "right", "right", "right", "right", "right", "right", "left", "left", "space",
        "enter", "l", "h",
    ],
    &[
        "down", "right", "right", "left", "down", "space", "down", "right", "right", "right",
        "right", "left", "down", "enter", "down", "right", "right", "right", "right", "left",
        "down", "right", "right", "left", "left", "left",
    ],
    &[
        "j", "j", "j", "j", "j", "j", "j", "right", "right", "right", "left", "j", "space",
        "enter", "j", "right", "right", "right", "right", "left", "left", "j", "left", "left",
        "right", "right", "right", "right", "right", "right", "right", "right", "right", "right",
        "right",
    ],
    &[
        "up", "up", "space", "right", "enter", "left", "left", "space", "l", "l", "l", "enter",
        "down", "space", "up", "down", "enter",
    ],
    &["x", "tab", "k", "q"],
    &["esc", "down", "enter"],
];

fn generate() -> Vec<String> {
    let mut o = Out::default();
    for c in combos() {
        let (th, pre) = c.apply();
        let full = c.full();
        for (si, seq) in SEQUENCES.iter().enumerate() {
            let mut m = new(th, default_rules_for_profile(&DifficultyProfile::normal()));
            let pl = Rc::new(RecPlayer::default());
            pl.vol.set(80);
            if si != 5 {
                m.set_player(pl.clone() as PlayerRef);
            }
            if full || si == 0 {
                o.w(&format!("{pre}seq{si}/init"), &m.view());
            }
            for (ki, k) in seq.iter().enumerate() {
                let cmd = m.update(&key(k));
                let p = format!("{pre}seq{si}/{ki}/{k}/");
                let msg = describe_cmd(cmd, &desc);
                let played = pl.take();
                if c.prof == Profile::Ascii {
                    o.w(&format!("{p}msg"), &msg);
                    o.w(&format!("{p}player"), &played);
                    o.w(&format!("{p}state"), &state(&m));
                }
                if full || (ki == seq.len() - 1 && si % 3 == 0) {
                    o.w(&format!("{p}view"), &m.view());
                }
            }
        }
        if !full {
            continue;
        }
        for prof in [
            "casual",
            "normal",
            "hardcore",
            "nightmare",
            "expert",
            "emeritus",
            "custom",
            "bogus",
        ] {
            let mut m = new(th, default_rules_for_profile(&DifficultyProfile::new(prof)));
            m.selected_row = Row::Done;
            o.w(
                &format!("{pre}profile/{prof}"),
                &format!("{}\n{}", state(&m), m.view()),
            );
        }
        let mut m = new(None, GameRules::default());
        o.w(&format!("{pre}zero"), &state(&m));
        let pl = Rc::new(RecPlayer::default());
        m.set_player(pl.clone() as PlayerRef);
        for v in [150, -3, 55, 100, 0] {
            m.set_volume(v);
            o.w(
                &format!("{pre}vol/{v}"),
                &format!("{} {}", m.volume(), pl.take()),
            );
        }
        for md in ["native", "bell", "off", "auto", "weird", ""] {
            m.set_audio_mode(AudioMode(md.into()));
            o.w(&format!("{pre}mode/{md}"), m.audio_mode().as_str());
        }
        for s in ["dock", "nope", "computer_beep"] {
            m.set_selected_sound(&SoundID::new(s));
            o.w(&format!("{pre}sound/{s}"), m.selected_sound().as_str());
        }
        m.set_theme(get_theme("lcars").with_color_mode(ColorMode::Light));
        o.w(&format!("{pre}settheme"), m.color_mode().as_str());
        m.set_color_mode(ColorMode::Dark);
        m.set_audio_enabled(false);
        m.set_active(false);
        o.w(&format!("{pre}inactive"), &state(&m));
        m.closed = true;
        m.set_active(true);
        o.w(&format!("{pre}reactivated"), &state(&m));
        m.selected_row = Row::RepairMultiplier;
        let mut r = m.rules();
        r.repair_multiplier = 1.3;
        r.time_margin = 0.333;
        r.anim_speed = 7;
        m.set_rules(r);
        o.w(&format!("{pre}odd/view"), &m.view());
        let _ = m.update(&key("left"));
        o.w(&format!("{pre}odd/left"), &state(&m));
        m.selected_row = Row::AnimSpeed;
        let _ = m.update(&key("left"));
        o.w(&format!("{pre}odd/anim"), &state(&m));
        m.selected_row = Row(20);
        let _ = m.update(&key("right"));
        o.w(
            &format!("{pre}odd/row20"),
            &format!("{}\n{}", state(&m), m.view()),
        );
        o.w(
            &format!("{pre}player/nil"),
            &new(th, GameRules::default()).player().is_none().to_string(),
        );
    }
    o.0
}

#[test]
fn optionsmodal_parity() {
    // `New(nil, ...)` uses the Auto default theme; its colour mode is all
    // that is recorded, but pin detection anyway.
    crate::theme::detect::set_fixed_detection(true);
    let (lines, bytes) = compare(&generate(), DATA);
    eprintln!("optionsmodal parity: {lines} lines, {bytes} bytes");
}
