//! Ports of `drydockmodal_test.go`.

use std::cell::RefCell;
use std::rc::Rc;

use sst_engine::campaign::TourState;
use sst_tuikit::tea::{self, KeyMsg};

use super::*;
use crate::theme::{get_theme, THEME_MODERN};

fn tour(seed: i64) -> TourRef {
    let mut t = TourState::new_at(seed, 0);
    t.in_drydock = true;
    Rc::new(RefCell::new(t))
}

fn key(t: tea::KeyType) -> Msg {
    tea::msg(KeyMsg::new(t))
}

fn runes(s: &str) -> Msg {
    tea::msg(KeyMsg::runes(s))
}

fn tier(t: &TourRef, id: &RefitID) -> i32 {
    t.borrow()
        .installed_refits
        .as_ref()
        .and_then(|r| r.get(id).copied())
        .unwrap_or(0)
}

#[test]
fn navigation_and_selection() {
    let t = tour(404);
    t.borrow_mut().requisition_points = 2000;
    let mut m = new(Some(t), get_theme(THEME_MODERN));
    assert_eq!(m.selected_index, 0);
    for (msg, want) in [
        (key(tea::KEY_DOWN), 1),
        (key(tea::KEY_UP), 0),
        (key(tea::KEY_UP), 0),
        (runes("j"), 1),
        (runes("k"), 0),
    ] {
        let _ = m.update(&msg);
        assert_eq!(m.selected_index, want);
    }
    let n = refit_catalog().len() as i32;
    for _ in 0..n + 2 {
        let _ = m.update(&key(tea::KEY_DOWN));
    }
    assert_eq!(m.selected_index, n - 1);
}

#[test]
fn purchase_refit() {
    let t = tour(405);
    t.borrow_mut().requisition_points = 1000;
    let mut m = new(Some(t.clone()), get_theme(THEME_MODERN));
    m.selected_index = 0;
    let cmd = m.update(&key(tea::KEY_ENTER)).expect("cmd");
    assert_eq!(tier(&t, &RefitID::dilithium_core()), 1);
    assert_eq!(t.borrow().requisition_points, 500);
    let msg = cmd.call().unwrap();
    let p = msg.downcast_ref::<RefitPurchasedMsg>().unwrap();
    assert_eq!(p.refit_id, RefitID::dilithium_core());
    assert_eq!(p.tier, 1);
}

#[test]
fn purchase_refit_insufficient_points() {
    let t = tour(407);
    t.borrow_mut().requisition_points = 100;
    let mut m = new(Some(t.clone()), get_theme(THEME_MODERN));
    m.selected_index = 0;
    assert!(m.update(&key(tea::KEY_ENTER)).is_none());
    assert!(m.error_message.contains("insufficient requisition"));
    assert_eq!(tier(&t, &RefitID::dilithium_core()), 0);
    assert!(m.view().contains("insufficient requisition"));
    let _ = m.update(&key(tea::KEY_DOWN));
    assert!(m.error_message.is_empty());
}

#[test]
fn purchase_refit_max_tier() {
    let t = tour(408);
    t.borrow_mut().requisition_points = 10000;
    t.borrow_mut()
        .installed_refits
        .as_mut()
        .unwrap()
        .insert(RefitID::dilithium_core(), 3);
    let mut m = new(Some(t), get_theme(THEME_MODERN));
    assert!(m.update(&key(tea::KEY_ENTER)).is_none());
    assert!(!m.error_message.is_empty());
}

#[test]
fn disembark() {
    let mut m = new(Some(tour(406)), get_theme(THEME_MODERN));
    for msg in [key(tea::KEY_SPACE), runes("d"), runes("D")] {
        let r = m.update(&msg).expect("cmd").call().unwrap();
        assert!(r.downcast_ref::<DisembarkMsg>().is_some());
    }
}

#[test]
fn view() {
    crate::theme::detect::set_fixed_detection(true);
    let t = tour(409);
    {
        let mut tb = t.borrow_mut();
        tb.sectors_completed = 2;
        tb.requisition_points = 1250;
        tb.installed_refits
            .as_mut()
            .unwrap()
            .insert(RefitID::deflector_grid(), 1);
    }
    let v = new(Some(t), get_theme(THEME_MODERN)).view();
    for s in [
        "STARBASE 01 DRYDOCK & REFIT FACILITY",
        "Sector 2 Cleared",
        "1250 PTS",
        "NCC - 1701",
        "Modules Installed: 1/6",
        "Dilithium Core Tuning",
        "> Dilithium Core Tuning",
        "Reinforced Deflectors",
        "[Tier 1/3]",
        "Disembark",
        "Next Sector",
    ] {
        assert!(v.contains(s), "missing {s:?}:\n{v}");
    }
}
