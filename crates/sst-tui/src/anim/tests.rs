//! Ports of `anim_test.go`, `phaser_test.go` and `tactical_anim_test.go`.

use std::time::Duration;

use sst_engine::rules::{ANIM_SPEED_CINEMATIC, ANIM_SPEED_FAST, ANIM_SPEED_NORMAL, ANIM_SPEED_OFF};
use sst_tuikit::lipgloss;
use sst_tuikit::tea::{self, harness::Harness};

use super::*;

#[test]
fn bresenham() {
    assert_eq!(
        bresenham_line([2, 2], [2, 5]),
        vec![[2, 2], [2, 3], [2, 4], [2, 5]]
    );
    let w = bresenham_line([2, 5], [2, 2]);
    assert_eq!((w.len(), w[0], w[3]), (4, [2, 5], [2, 2]));
    let s = bresenham_line([1, 3], [4, 3]);
    assert_eq!((s.len(), s[0], s[3]), (4, [1, 3], [4, 3]));
    let n = bresenham_line([4, 3], [1, 3]);
    assert_eq!((n.len(), n[0], n[3]), (4, [4, 3], [1, 3]));
    let d = bresenham_line([1, 1], [4, 4]);
    assert_eq!(d.len(), 4);
    for (i, pt) in d.iter().enumerate() {
        assert_eq!(*pt, [i as i32 + 1, i as i32 + 1]);
    }
    let nw = bresenham_line([4, 4], [1, 1]);
    assert_eq!((nw.len(), nw[0], nw[3]), (4, [4, 4], [1, 1]));
    assert_eq!(bresenham_line([3, 3], [3, 3]), vec![[3, 3]]);
}

#[test]
fn torpedo_step_and_skip() {
    let mut a = new_torpedo_animation([1, 1], [1, 4], true, 2);
    assert!(!a.is_finished());
    assert!(a.total_duration() > Duration::ZERO);
    assert!(!a.frames().is_empty());
    assert!(!a.step().overrides.is_empty());
    let _ = a.skip();
    assert!(a.is_finished());
}

#[test]
fn torpedo_impact_and_miss() {
    let target = [1, 4];
    let mut hit = new_torpedo_animation([1, 1], target, true, 2);
    let frames = hit.frames();
    assert_eq!(frames.len(), 5);
    for f in &frames[..2] {
        assert_eq!(f.overrides[&target].glyph, "+K+");
    }
    let mut last = Frame::default();
    while !hit.is_finished() {
        last = hit.step();
    }
    assert_eq!(last.overrides[&target].glyph, "#*#");

    let mut miss = new_torpedo_animation([1, 1], target, false, 2);
    let mut last = Frame::default();
    while !miss.is_finished() {
        last = miss.step();
    }
    assert_eq!(last.overrides[&target].glyph, "   ");
}

#[test]
fn phaser_directional_glyphs() {
    let mut h = new_phaser_animation([3, 1], [3, 5], true, 2);
    let f = h.step();
    assert_eq!(f.overrides[&[3, 3]].glyph, "---");
    assert_eq!(f.overrides[&[3, 5]].glyph, "<K>");

    let f = new_phaser_animation([1, 4], [5, 4], true, 2).step();
    assert_eq!(f.overrides[&[3, 4]].glyph, " | ");
    let f = new_phaser_animation([1, 1], [4, 4], true, 2).step();
    assert_eq!(f.overrides[&[2, 2]].glyph, " \\ ");
    let f = new_phaser_animation([1, 4], [4, 1], true, 2).step();
    assert_eq!(f.overrides[&[2, 3]].glyph, " / ");

    let mut a = new_phaser_animation([3, 1], [3, 5], true, 2);
    assert_eq!(a.frames().len(), 4);
    a.step();
    assert!(!a.is_finished());
    a.step();
    a.step();
    a.step();
    assert!(a.is_finished());
}

#[test]
fn red_alert_badge() {
    let s0 = red_alert_badge_style(0);
    let s1 = red_alert_badge_style(1);
    let s2 = red_alert_badge_style(2);
    assert_ne!(s0.get_foreground(), s1.get_foreground());
    assert_eq!(s0.get_foreground(), s2.get_foreground());
    assert!(s0.get_bold() && !s1.get_bold());
}

#[test]
fn tick_cmd_produces_tick_msg() {
    let cmd = tick_cmd(42, 3, Duration::from_millis(10)).expect("non-nil cmd");
    let msg = cmd.call().expect("msg");
    let t = msg.downcast_ref::<TickMsg>().expect("TickMsg");
    assert_eq!(
        *t,
        TickMsg {
            anim_id: 42,
            step: 3
        }
    );
}

#[test]
fn cell_override_properties() {
    let co = CellOverride {
        glyph: " · ".into(),
        style: lipgloss::new_style().bold(true),
    };
    assert_eq!(co.glyph, " · ");
}

#[test]
fn frame_durations() {
    for (speed, ms) in [
        (ANIM_SPEED_OFF, 0),
        (ANIM_SPEED_FAST, 50),
        (ANIM_SPEED_NORMAL, 120),
        (ANIM_SPEED_CINEMATIC, 240),
        (-1, 120),
        (4, 120),
        (999, 120),
    ] {
        assert_eq!(frame_duration(speed), Duration::from_millis(ms), "{speed}");
    }
}

#[test]
fn multi_phaser_multiple_targets() {
    let t1 = [4, 7];
    let t2 = [1, 4];
    let t3 = [7, 7];
    let mut a = new_multi_phaser_animation([4, 4], &[t1, t2, t3], &[true, false, true], 2);
    assert!(!a.is_finished());
    assert_eq!(a.total_duration(), Duration::from_millis(480));
    assert_eq!(a.frames().len(), 4);
    let f = a.step();
    assert_eq!(f.overrides[&[4, 5]].glyph, "---");
    assert_eq!(f.overrides[&t1].glyph, "<K>");
    assert_eq!(f.overrides[&[2, 4]].glyph, " | ");
    assert!(!f.overrides.contains_key(&t2));
    assert_eq!(f.overrides[&[5, 5]].glyph, " \\ ");
    assert_eq!(f.overrides[&t3].glyph, "<K>");
    for _ in 0..3 {
        assert_eq!(a.step().overrides.len(), f.overrides.len());
    }
    assert!(a.is_finished());
}

#[test]
fn multi_phaser_empty_targets() {
    let mut a = new_multi_phaser_animation([4, 4], &[], &[], ANIM_SPEED_FAST);
    assert_eq!(a.frames().len(), 4);
    assert!(a.step().overrides.is_empty());
}

#[test]
fn multi_phaser_skip() {
    let mut a = new_multi_phaser_animation([4, 4], &[[4, 7]], &[true], ANIM_SPEED_CINEMATIC);
    a.skip();
    assert!(a.is_finished());
}

#[test]
fn phaser_delegation() {
    let mut a = new_phaser_animation([3, 3], [3, 6], true, ANIM_SPEED_NORMAL);
    assert!(!a.is_finished());
    assert_eq!(a.step().overrides[&[3, 6]].glyph, "<K>");
}

#[test]
fn plasma_progression() {
    let mut p = new_plasma_animation([2, 2], [4, 4], Duration::from_millis(100));
    assert!(!p.done());
    p.tick();
    p.tick();
    p.tick();
    assert!(p.done());
}

#[test]
fn plasma_animation_interface() {
    let from = [1, 1];
    let to = [1, 3];
    let mut p = new_plasma_animation(from, to, Duration::from_millis(50));
    assert_eq!((p.from(), p.to()), (from, to));
    assert!(!p.is_finished());
    assert_eq!(p.total_duration(), Duration::from_millis(150));
    assert_eq!(p.frames().len(), 3);
    assert_eq!(p.current_coord(), from);
    assert_eq!(p.step().overrides[&from].glyph, "*P*");
    p.skip();
    assert!(p.is_finished() && p.done());
}

fn stages_of(mut next: impl FnMut() -> Option<&'static str>) -> Vec<&'static str> {
    let mut v = Vec::new();
    while let Some(g) = next() {
        v.push(g);
    }
    v
}

#[test]
fn cloak_stages() {
    let mut c = new_cloak_animation([3, 3], true);
    let stages = stages_of(|| {
        if c.done() {
            return None;
        }
        let g = c.current_glyph();
        c.tick();
        Some(g)
    });
    assert_eq!(stages, vec!["+R+", "~R~", "·?·", " . "]);
}

#[test]
fn cloak_decloak() {
    let mut c = new_cloak_animation([4, 4], false);
    assert!(!c.is_cloaking());
    assert_eq!(c.coord(), [4, 4]);
    let stages = stages_of(|| {
        if c.done() {
            return None;
        }
        let g = c.current_glyph();
        c.tick();
        Some(g)
    });
    assert_eq!(stages, vec![" . ", "·?·", "~R~", "+R+"]);
}

#[test]
fn cloak_animation_interface() {
    let mut c = new_cloak_animation([2, 2], true);
    assert!(!c.is_finished());
    assert_eq!(c.frames().len(), 4);
    assert!(c.total_duration() > Duration::ZERO);
    assert_eq!(c.step().overrides[&[2, 2]].glyph, "+R+");
    c.skip();
    assert!(c.is_finished() && c.done());
}

#[test]
fn web_pulse() {
    let mut w = new_web_animation([5, 5], WebAction::Weave);
    assert!(!w.done());
    while !w.done() {
        w.tick();
    }
    assert!(w.done());
}

#[test]
fn web_breach() {
    let mut w = new_web_animation([6, 6], WebAction::Breach);
    assert!(!w.done());
    assert_eq!(w.action(), WebAction::Breach);
    assert_eq!(w.coord(), [6, 6]);
    let stages = stages_of(|| {
        if w.done() {
            return None;
        }
        let g = w.current_glyph();
        w.tick();
        Some(g)
    });
    assert!(stages.len() >= 3);
    assert!(w.done());
}

#[test]
fn web_animation_interface() {
    let mut w = new_web_animation([3, 3], WebAction::Weave);
    assert!(!w.is_finished());
    assert!(!w.frames().is_empty());
    assert!(w.total_duration() > Duration::ZERO);
    let _ = w.step();
    w.skip();
    assert!(w.is_finished() && w.done());
}

#[test]
fn web_action_string() {
    assert_eq!(WebAction::Weave.to_string(), "Weave");
    assert_eq!(WebAction::Breach.to_string(), "Breach");
    assert_eq!(WebAction(99).to_string(), "Unknown");
}

#[test]
fn tactical_glyph_rune_widths() {
    let anims: Vec<Box<dyn Animation>> = vec![
        Box::new(new_plasma_animation(
            [1, 1],
            [2, 2],
            Duration::from_millis(100),
        )),
        Box::new(new_cloak_animation([1, 1], true)),
        Box::new(new_cloak_animation([1, 1], false)),
        Box::new(new_web_animation([1, 1], WebAction::Weave)),
        Box::new(new_web_animation([1, 1], WebAction::Breach)),
    ];
    for a in &anims {
        for f in a.frames() {
            for co in f.overrides.values() {
                assert_eq!(co.glyph.chars().count(), 3, "{:?}", co.glyph);
            }
        }
    }
}

#[test]
fn anim_speed_off_immediately_finished() {
    let c = new_cloak_animation_with_speed([1, 1], true, ANIM_SPEED_OFF);
    assert!(c.done() && c.is_finished());
    let w = new_web_animation_with_speed([1, 1], WebAction::Weave, ANIM_SPEED_OFF);
    assert!(w.done() && w.is_finished());
    let p = new_plasma_animation([1, 1], [2, 2], Duration::ZERO);
    assert!(p.done() && p.is_finished());
}

/// The root model's playback loop (`startCombatAnimation` + `TickMsg`
/// handling in pkg/tui/update.go) on the tuikit harness.
struct Player {
    anim_id: i32,
    active: Option<Box<dyn Animation>>,
    shown: Vec<String>,
}

impl tea::Model for Player {
    fn init(&self) -> Option<tea::Cmd> {
        None
    }
    fn update(&mut self, msg: tea::Msg) -> Option<tea::Cmd> {
        if let Some(t) = msg.downcast_ref::<TickMsg>() {
            if t.anim_id == self.anim_id {
                if let Some(a) = self.active.as_mut() {
                    if a.is_finished() {
                        self.active = None;
                        self.shown.push("clear".into());
                        return None;
                    }
                    let f = a.step();
                    self.shown
                        .push(f.overrides.values().map(|o| o.glyph.clone()).collect());
                    return tick_cmd(self.anim_id, t.step + 1, f.duration);
                }
            }
        } else if msg.downcast_ref::<&str>().is_some() {
            self.anim_id += 1;
            let mut a: Box<dyn Animation> =
                Box::new(new_torpedo_animation([1, 1], [1, 4], true, 2));
            let f = a.step();
            self.shown
                .push(f.overrides.values().map(|o| o.glyph.clone()).collect());
            self.active = Some(a);
            return tick_cmd(self.anim_id, 1, f.duration);
        }
        None
    }
    fn view(&self) -> String {
        self.shown.join(",")
    }
}

#[test]
fn harness_playback() {
    let mut h = Harness::new(Player {
        anim_id: 0,
        active: None,
        shown: Vec::new(),
    });
    h.send("fire");
    assert_eq!(h.pending_ticks(), vec![Duration::from_millis(120)]);
    while h.fire_next_tick() {}
    assert_eq!(h.view(), " · +K+, o +K+, * ,***,#*#,clear");
}
