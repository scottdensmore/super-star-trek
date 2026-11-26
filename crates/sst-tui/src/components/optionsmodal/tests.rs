//! Ports of `modal_test.go`.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use sst_audio::Player;
use sst_tuikit::tea::{self, KeyMsg};

use super::*;
use crate::theme::detect;

fn setup() -> Model {
    detect::set_fixed_detection(true);
    new(
        default_theme(),
        default_rules_for_profile(&DifficultyProfile::normal()),
    )
}

fn key(t: tea::KeyType) -> Msg {
    tea::msg(KeyMsg::new(t))
}

fn runes(s: &str) -> Msg {
    tea::msg(KeyMsg::runes(s))
}

#[test]
fn navigation_and_cycle() {
    let mut m = setup();
    assert_eq!(m.selected_row, Row(0));
    let _ = m.update(&key(tea::KEY_RIGHT));
    assert_eq!(m.rules().profile, DifficultyProfile::hardcore());
    assert_eq!(m.rules().surveillance, SurveillanceMode::local());
    let _ = m.update(&key(tea::KEY_DOWN));
    assert_eq!(m.selected_row, Row(1));
    let _ = m.update(&key(tea::KEY_RIGHT));
    assert_eq!(m.rules().surveillance, SurveillanceMode::blackout());
    assert_eq!(m.rules().profile, DifficultyProfile::custom());
}

#[test]
fn render_layout() {
    let view = setup().view();
    for s in [
        "STARFLEET CONFIGURATION & RULES",
        "Difficulty Profile",
        "Starbase Surveillance",
        "Sensor Degradation",
        "Repair Multiplier",
        "Klingon Cloaking",
        "Combat Animations",
        "Color Mode",
        "Sound FX",
        "Audio Mode",
        "Audio Volume",
        "Sound Test",
        "[■■■■■■■■··] 80%",
    ] {
        assert!(view.contains(s), "missing {s:?}:\n{view}");
    }
}

#[test]
fn close_and_active() {
    let m = setup();
    assert!(m.active());
    assert!(!m.closed);
    let mut esc = m.clone();
    let _ = esc.update(&key(tea::KEY_ESC));
    assert!(esc.closed && !esc.active());
    let mut q = m.clone();
    let _ = q.update(&runes("q"));
    assert!(q.closed && !q.active());
    let mut done = m.clone();
    done.selected_row = Row::Done;
    let _ = done.update(&key(tea::KEY_ENTER));
    assert!(done.closed && !done.active());
}

#[test]
fn cycle_all_options() {
    let mut m = setup();
    m.selected_row = Row::SensorDegradation;
    let orig = m.rules().sensor_degradation;
    let _ = m.update(&key(tea::KEY_RIGHT));
    assert_ne!(m.rules().sensor_degradation, orig);
    assert_eq!(m.rules().profile, DifficultyProfile::custom());

    m.selected_row = Row::RepairMultiplier;
    let orig = m.rules().repair_multiplier;
    let _ = m.update(&key(tea::KEY_RIGHT));
    assert_ne!(m.rules().repair_multiplier, orig);

    m.selected_row = Row::KlingonCloak;
    let orig = m.rules().klingon_cloak;
    let _ = m.update(&key(tea::KEY_RIGHT));
    assert_ne!(m.rules().klingon_cloak, orig);

    m.selected_row = Row::TimeMargin;
    let orig = m.rules().time_margin;
    let _ = m.update(&key(tea::KEY_RIGHT));
    assert_ne!(m.rules().time_margin, orig);

    m.set_rules(default_rules_for_profile(&DifficultyProfile::casual()));
    assert_eq!(m.rules().profile, DifficultyProfile::casual());
}

#[test]
fn navigation_wrapping() {
    let mut m = setup();
    let _ = m.update(&key(tea::KEY_UP));
    assert_eq!(m.selected_row, Row::Done);
    let _ = m.update(&key(tea::KEY_DOWN));
    assert_eq!(m.selected_row, Row::Profile);
}

#[test]
fn color_mode_row() {
    detect::set_fixed_detection(true);
    let th = default_theme().with_color_mode(ColorMode::Auto);
    let mut m = new(th, default_rules_for_profile(&DifficultyProfile::normal()));
    m.selected_row = Row::ColorMode;
    assert_eq!(m.color_mode(), ColorMode::Auto);
    for (msg, want) in [
        (key(tea::KEY_RIGHT), ColorMode::Dark),
        (key(tea::KEY_RIGHT), ColorMode::Light),
        (key(tea::KEY_LEFT), ColorMode::Dark),
        (key(tea::KEY_SPACE), ColorMode::Light),
    ] {
        let _ = m.update(&msg);
        assert_eq!(m.color_mode(), want);
    }
    m.set_color_mode(ColorMode::Auto);
    assert_eq!(m.color_mode(), ColorMode::Auto);
    let view = m.view();
    assert!(view.contains("Color Mode"));
    assert!(view.contains("AUTO"));
}

#[test]
fn anim_speed_row() {
    let mut m = setup();
    assert_eq!(m.rules().anim_speed, 2);
    m.selected_row = Row::AnimSpeed;
    for (msg, want) in [
        (key(tea::KEY_RIGHT), 3),
        (key(tea::KEY_RIGHT), 0),
        (key(tea::KEY_SPACE), 1),
        (key(tea::KEY_LEFT), 0),
        (key(tea::KEY_LEFT), 3),
    ] {
        let _ = m.update(&msg);
        assert_eq!(m.rules().anim_speed, want);
    }
    let view = m.view();
    assert!(view.contains("Combat Animations"));
    assert!(view.contains("CINEMATIC"));
}

#[test]
fn audio_row() {
    let mut m = setup();
    assert!(m.audio_enabled());
    m.selected_row = Row::Audio;
    for (msg, want) in [
        (key(tea::KEY_RIGHT), false),
        (key(tea::KEY_LEFT), true),
        (key(tea::KEY_SPACE), false),
        (key(tea::KEY_ENTER), true),
    ] {
        let _ = m.update(&msg);
        assert_eq!(m.audio_enabled(), want);
    }
    let v = m.view();
    assert!(v.contains("Sound FX"));
    assert!(v.contains("[ENABLED]"));
    m.set_audio_enabled(false);
    assert!(!m.audio_enabled());
    assert!(m.view().contains("[DISABLED]"));

    m.selected_row = Row::Audio;
    let _ = m.update(&key(tea::KEY_DOWN));
    assert_eq!(m.selected_row, Row::AudioMode);
    m.selected_row = Row::AudioMode;
    let _ = m.update(&key(tea::KEY_UP));
    assert_eq!(m.selected_row, Row::Audio);
    m.selected_row = Row::Audio;
    let _ = m.update(&key(tea::KEY_UP));
    assert_eq!(m.selected_row, Row::ColorMode);
    let _ = m.update(&key(tea::KEY_DOWN));
    assert_eq!(m.selected_row, Row::Audio);
}

#[test]
fn audio_navigation() {
    let mut m = setup();
    m.selected_row = Row::ColorMode;
    for want in [
        Row::Audio,
        Row::AudioMode,
        Row::Volume,
        Row::SoundTest,
        Row::Done,
    ] {
        let _ = m.update(&key(tea::KEY_DOWN));
        assert_eq!(m.selected_row, want);
    }
    for want in [Row::SoundTest, Row::Volume, Row::AudioMode, Row::Audio] {
        let _ = m.update(&key(tea::KEY_UP));
        assert_eq!(m.selected_row, want);
    }
}

/// Go `mockAudioPlayer` (embeds `NullPlayer`, records `Play`/`SetVolume`).
#[derive(Default)]
struct MockPlayer {
    played: RefCell<SoundID>,
    play_count: Cell<i32>,
    volume: Cell<i32>,
    muted: Cell<bool>,
}

impl Player for MockPlayer {
    fn play(&self, s: SoundID) {
        *self.played.borrow_mut() = s;
        self.play_count.set(self.play_count.get() + 1);
    }
    fn set_muted(&self, m: bool) {
        self.muted.set(m);
    }
    fn is_muted(&self) -> bool {
        self.muted.get()
    }
    fn set_volume(&self, v: i32) {
        self.volume.set(v);
    }
    fn volume(&self) -> i32 {
        self.volume.get()
    }
}

fn play_msg(cmd: Option<Cmd>) -> SoundID {
    let msg = cmd.expect("cmd").call().expect("msg");
    msg.downcast_ref::<PlaySoundMsg>()
        .expect("PlaySoundMsg")
        .sound
        .clone()
}

#[test]
fn audio_rows() {
    let mut m = setup();
    m.set_volume(70);
    assert_eq!(m.volume(), 70);
    m.selected_row = Row::Volume;
    let _ = m.update(&key(tea::KEY_RIGHT));
    assert_eq!(m.volume(), 80);
    let _ = m.update(&key(tea::KEY_LEFT));
    assert_eq!(m.volume(), 70);
    m.set_volume(95);
    let _ = m.update(&key(tea::KEY_RIGHT));
    assert_eq!(m.volume(), 100);
    let _ = m.update(&key(tea::KEY_RIGHT));
    assert_eq!(m.volume(), 100);
    m.set_volume(5);
    let _ = m.update(&key(tea::KEY_LEFT));
    assert_eq!(m.volume(), 0);
    let _ = m.update(&key(tea::KEY_LEFT));
    assert_eq!(m.volume(), 0);

    m.set_audio_mode(AudioMode::native());
    assert_eq!(m.audio_mode(), AudioMode::native());
    m.selected_row = Row::AudioMode;
    m.set_audio_mode(AudioMode::auto());
    for want in [
        AudioMode::native(),
        AudioMode::bell(),
        AudioMode::off(),
        AudioMode::auto(),
    ] {
        let _ = m.update(&key(tea::KEY_RIGHT));
        assert_eq!(m.audio_mode(), want);
    }
    let _ = m.update(&key(tea::KEY_LEFT));
    assert_eq!(m.audio_mode(), AudioMode::off());

    m.selected_row = Row::SoundTest;
    let first = m.selected_sound();
    assert!(!first.as_str().is_empty());
    let _ = m.update(&key(tea::KEY_RIGHT));
    assert_ne!(m.selected_sound(), first);

    let cur = m.selected_sound();
    assert_eq!(play_msg(m.update(&key(tea::KEY_ENTER))), cur);
    assert_eq!(play_msg(m.update(&key(tea::KEY_SPACE))), cur);

    let mock = Rc::new(MockPlayer::default());
    m.set_player(mock.clone() as PlayerRef);
    let _ = m.update(&key(tea::KEY_ENTER));
    assert_eq!(*mock.played.borrow(), cur);
    assert_eq!(mock.play_count.get(), 1);
}

#[test]
fn audition_volume_sync() {
    let mut m = setup();
    let mock = Rc::new(MockPlayer::default());
    m.set_player(mock.clone() as PlayerRef);
    m.set_volume(50);
    m.selected_row = Row::SoundTest;
    let _ = m.update(&key(tea::KEY_ENTER));
    assert_eq!(mock.volume.get(), 50);
    m.selected_row = Row::Volume;
    let _ = m.update(&key(tea::KEY_RIGHT));
    assert_eq!(mock.volume.get(), 60);
    m.selected_row = Row::SoundTest;
    let _ = m.update(&key(tea::KEY_ENTER));
    assert_eq!(mock.volume.get(), 60);
}
