//! Byte-for-byte parity with `testdata/parity/components/drydockmodal.txt`
//! (recorded by `pkg/tui/components/drydockmodal/zz_paritygen_test.go`).

#[path = "../modal_parity_util.rs"]
#[allow(clippy::duplicate_mod)]
mod util;

use std::cell::RefCell;
use std::rc::Rc;

use sst_engine::campaign::TourState;
use sst_engine::gofmt::quote;
use util::*;

use super::*;

const DATA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/parity/components/drydockmodal.txt"
));

struct TourCase {
    seed: i64,
    req: i32,
    secs: i32,
    refits: &'static [(&'static str, i32)],
    nil_map: bool,
    width: i32,
}

const TOURS: &[TourCase] = &[
    TourCase {
        seed: 404,
        req: 2000,
        secs: 1,
        refits: &[],
        nil_map: false,
        width: 0,
    },
    TourCase {
        seed: 405,
        req: 100,
        secs: 0,
        refits: &[],
        nil_map: false,
        width: 60,
    },
    TourCase {
        seed: 409,
        req: 1250,
        secs: 2,
        refits: &[("deflector_grid", 1)],
        nil_map: false,
        width: 0,
    },
    TourCase {
        seed: 777,
        req: 99999,
        secs: 5,
        refits: &[
            ("dilithium_core", 3),
            ("torpedo_bays", 2),
            ("damage_nanites", 0),
        ],
        nil_map: false,
        width: 120,
    },
    TourCase {
        seed: 12,
        req: 5000,
        secs: 3,
        refits: &[],
        nil_map: true,
        width: 1,
    },
];

const SEQUENCES: &[&[&str]] = &[
    &[
        "enter", "enter", "enter", "enter", "down", "enter", "j", "j", "enter", "k", "up", "up",
        "up", "enter",
    ],
    &[
        "down", "down", "down", "down", "down", "down", "down", "enter", "enter", "enter", "enter",
        "x", "space",
    ],
    &["d", "D", "space", "esc", "tab", "up"],
];

fn desc(m: &Msg) -> Option<String> {
    if m.downcast_ref::<DisembarkMsg>().is_some() {
        return Some("DisembarkMsg".into());
    }
    m.downcast_ref::<RefitPurchasedMsg>()
        .map(|v| format!("RefitPurchasedMsg id={} tier={}", v.refit_id, v.tier))
}

fn tour_state(t: &Option<TourRef>) -> String {
    let Some(t) = t else {
        return "tour=nil".into();
    };
    let t = t.borrow();
    let refits = match &t.installed_refits {
        None => "map[]".to_string(),
        Some(r) => format!(
            "map[{}]",
            r.iter()
                .map(|(k, v)| format!("{k}:{v}"))
                .collect::<Vec<_>>()
                .join(" ")
        ),
    };
    format!(
        "req={} secs={} refits={}",
        t.requisition_points, t.sectors_completed, refits
    )
}

fn make_tour(c: &TourCase) -> TourRef {
    let mut t = TourState::new_at(c.seed, 0);
    t.in_drydock = true;
    t.requisition_points = c.req;
    t.sectors_completed = c.secs;
    for (k, v) in c.refits {
        t.installed_refits
            .as_mut()
            .unwrap()
            .insert(RefitID::new(*k), *v);
    }
    if c.nil_map {
        t.installed_refits = None;
    }
    Rc::new(RefCell::new(t))
}

fn generate() -> Vec<String> {
    let mut o = Out::default();
    for (i, def) in refit_catalog().iter().enumerate() {
        let costs: Vec<String> = def.tier_costs.iter().map(|c| c.to_string()).collect();
        o.w(
            &format!("catalog/{i}"),
            &format!("{}|{}|[{}]", def.id, def.name, costs.join(" ")),
        );
    }
    for c in combos() {
        let (th, pre) = c.apply();
        let full = c.full();
        for (ti, tc) in TOURS.iter().enumerate() {
            if !full && ti != 2 {
                continue;
            }
            let mut m = new(Some(make_tour(tc)), th);
            if tc.width != 0 {
                m.width = tc.width;
            }
            o.w(&format!("{pre}tour{ti}/init"), &m.view());
            for (si, seq) in SEQUENCES.iter().enumerate() {
                if !full && si > 0 {
                    break;
                }
                for (ki, k) in seq.iter().enumerate() {
                    let cmd = m.update(&key(k));
                    let p = format!("{pre}tour{ti}/seq{si}/{ki}/{k}/");
                    o.w(&format!("{p}msg"), &describe_cmd(cmd, &desc));
                    o.w(
                        &format!("{p}state"),
                        &format!(
                            "sel={} err={} {}",
                            m.selected_index(),
                            quote(&m.error_message),
                            tour_state(&m.tour)
                        ),
                    );
                    if (full && ti == 0 && si == 0) || ki == seq.len() - 1 {
                        o.w(&format!("{p}view"), &m.view());
                    }
                }
            }
        }
        if full {
            let mut m = new(None, None);
            o.w(&format!("{pre}nil/view"), &m.view());
            let cmd = m.update(&key("enter"));
            o.w(
                &format!("{pre}nil/enter"),
                &format!("{} {}", describe_cmd(cmd, &desc), m.error_message),
            );
            m.set_theme(th);
            o.w(&format!("{pre}nil/view2"), &m.view());
            m.set_theme(None);
            o.w(
                &format!("{pre}nil/theme"),
                &format!("{} {}", m.theme.name(), m.theme.color_mode()),
            );
            m.tour = Some(make_tour(&TOURS[0]));
            m.selected_index = 99;
            let cmd = m.update(&key("enter"));
            o.w(
                &format!("{pre}oob/enter"),
                &format!("{} {}", describe_cmd(cmd, &desc), tour_state(&m.tour)),
            );
            m.selected_index = -1;
            let cmd = m.update(&key("enter"));
            o.w(
                &format!("{pre}neg/enter"),
                &format!("{} {}", describe_cmd(cmd, &desc), tour_state(&m.tour)),
            );
        }
    }
    o.0
}

#[test]
fn drydockmodal_parity() {
    // `New(nil, nil)` renders with the Auto default theme (Ascii profile,
    // so the palette is irrelevant); pin detection.
    crate::theme::detect::set_fixed_detection(true);
    let (lines, bytes) = compare(&generate(), DATA);
    eprintln!("drydockmodal parity: {lines} lines, {bytes} bytes");
}
