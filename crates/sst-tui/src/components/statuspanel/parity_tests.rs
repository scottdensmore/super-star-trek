//! Byte-for-byte parity with `testdata/parity/components/statuspanel.txt`,
//! recorded by `pkg/tui/components/statuspanel/zz_paritygen_test.go`.

#![allow(clippy::needless_range_loop, clippy::type_complexity)]

#[allow(clippy::duplicate_mod)]
#[path = "../parity_support.rs"]
mod pg;

use sst_engine::{DeviceID, ANIM_SPEED_OFF};

use super::*;

const DATA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/parity/components/statuspanel.txt"
));

fn telemetry(m: &Model) -> String {
    format!(
        "vol={} muted={} sound={}",
        m.audio_volume(),
        m.audio_muted(),
        m.sound_enabled()
    )
}

fn dev(d: DeviceID) -> usize {
    d.as_i32() as usize
}

type DataFn = fn(&mut PanelData);

fn generate() -> Vec<String> {
    let mut o = pg::Out::new();

    let games = pg::games();
    for (name, g) in &games {
        let r = |th: Theme| new(Some(th), &[]).view(Some(g));
        if name == "hazards" || name == "damaged" {
            o.each(&format!("game/{name}"), r);
        } else {
            o.rot(&format!("game/{name}"), r);
        }
    }

    let g1 = &games[0].1;
    let dmg = &games[games.len() - 4].1;
    let mut m = new(None, &[]);
    let steps: [(&str, fn(&mut Model)); 8] = [
        ("init", |_| {}),
        ("sound-off", |m| m.set_sound_enabled(false)),
        ("sound-on", |m| m.set_sound_enabled(true)),
        ("vol0", |m| m.set_audio_telemetry(0, false)),
        ("vol35", |m| m.set_audio_telemetry(35, false)),
        ("vol100-muted", |m| m.set_audio_telemetry(100, true)),
        ("vol-neg", |m| m.set_audio_telemetry(-5, false)),
        ("vol55", |m| m.set_audio_telemetry(55, false)),
    ];
    for (name, f) in steps {
        f(&mut m);
        o.w(&format!("audio/{name}"), &telemetry(&m));
        o.rot(&format!("audio/{name}/game"), |th| {
            m.set_theme(Some(th));
            m.view(Some(g1))
        });
        o.rot(&format!("audio/{name}/nogame"), |th| {
            m.set_theme(Some(th));
            m.view(None)
        });
    }
    for cycle in 0..4 {
        m.set_red_alert_cycle(cycle);
        o.rot(&format!("redalert/{cycle}"), |th| {
            m.set_theme(Some(th));
            m.view(Some(dmg))
        });
    }
    m.set_hazard_alerts(vec!["custom alert one".into(), "⚠️ two".into()]);
    o.lite("hazard-setter", |th| {
        m.set_theme(Some(th));
        m.view(None)
    });

    let lrs = dev(DeviceID::LRSensors);
    let datas: Vec<(&str, DataFn)> = vec![
        ("sample", |_| {}),
        ("lrs1", |d| d.devices[dev(DeviceID::LRSensors)] = 1.0),
        ("lrs2.5", |d| d.devices[dev(DeviceID::LRSensors)] = 2.5),
        ("lrs2.5-docked", |d| {
            d.devices[dev(DeviceID::LRSensors)] = 2.5;
            d.is_docked = true;
        }),
        ("rules-zero", |d| {
            d.rules = GameRules::default();
            d.devices[dev(DeviceID::LRSensors)] = 1.0;
        }),
        ("rules-zero-degr", |d| {
            d.rules = GameRules {
                sensor_degradation: true,
                anim_speed: ANIM_SPEED_OFF,
                ..Default::default()
            };
            d.devices[dev(DeviceID::LRSensors)] = 1.0;
            d.enterprise.condition = ConditionType::Red;
        }),
        ("no-degr", |d| {
            d.rules.sensor_degradation = false;
            d.devices[dev(DeviceID::LRSensors)] = 1.5;
        }),
        ("ent-devices", |d| {
            d.enterprise.devices[dev(DeviceID::Warp)] = 9.99;
            d.enterprise.devices[dev(DeviceID::Computer)] = 12.345;
        }),
        ("both-devices", |d| {
            d.enterprise.devices[dev(DeviceID::Warp)] = 9.99;
            d.devices[dev(DeviceID::Phasers)] = 0.04;
            d.devices[dev(DeviceID::Shields)] = -1.0;
        }),
        ("audio-neg", |d| {
            d.audio_volume = -1;
            d.sound_enabled = false;
        }),
        ("audio-neg-muted", |d| {
            d.audio_volume = -1;
            d.audio_muted = true;
        }),
        ("audio-0", |d| d.audio_volume = 0),
        ("alerts", |d| {
            d.hazard_alerts = vec!["A".into(), String::new(), "⚠️ B ⚠️".into()]
        }),
        ("cond-odd", |d| d.enterprise.condition = ConditionType(9)),
        ("cond-yellow", |d| {
            d.enterprise.condition = ConditionType::Yellow
        }),
        ("cond-docked", |d| {
            d.enterprise.condition = ConditionType::Docked
        }),
        ("cond-red", |d| d.enterprise.condition = ConditionType::Red),
        ("energy-odd", |d| {
            d.enterprise.energy = f64::NAN;
            d.enterprise.shields = f64::INFINITY;
            d.stardate = f64::NEG_INFINITY;
            d.time_remaining = -0.04;
        }),
        ("energy-big", |d| {
            d.enterprise.energy = 123456.78;
            d.enterprise.shields = -250.0;
            d.enterprise.torpedoes = -3;
            d.stardate = 2525.25;
            d.time_remaining = 0.05;
        }),
        ("corner", |d| {
            d.enterprise.quad = [1, 8];
            d.enterprise.sector = [8, 1];
            for r in 0..9 {
                for c in 0..9 {
                    d.galaxy_chart[r][c] = (r * 100 + c * 10 + (r + c) % 10) as i32;
                }
            }
        }),
        ("offgrid", |d| {
            d.enterprise.quad = [0, 9];
            d.galaxy_chart[1][8] = 7;
        }),
    ];
    for (name, f) in datas {
        let mut data = sample_panel_data();
        f(&mut data);
        o.rot(&format!("render/{name}"), |th| {
            new(Some(th), &[]).render(&data)
        });
    }
    let mut data = sample_panel_data();
    data.devices[lrs] = 1.0;
    o.each("render/lrs1-full", |th| {
        let mut mm = new(Some(th), &[30, 16]);
        mm.set_red_alert_cycle(1);
        mm.render(&data)
    });
    o.lite("render/after", |th| {
        let mm = new(Some(th), &[]);
        let _ = mm.render(&data);
        mm.view(None)
    });

    let mut ent = Enterprise {
        quad: [4, 4],
        sector: [2, 3],
        energy: 4500.0,
        shields: 1000.0,
        torpedoes: 8,
        condition: ConditionType::Green,
        ..Default::default()
    };
    ent.devices[lrs] = 2.5;
    let mut chart = [[0; 9]; 9];
    chart[4][4] = 105;
    chart[3][4] = 203;
    o.lite("setstate/new", |th| {
        let mut mm = new(Some(th), &[30, 16]);
        mm.set_state(ent.clone(), 25.0, 10, 3, false, chart, None);
        mm.view(None)
    });
    o.lite("setstate/docked", |th| {
        let mut mm = new(Some(th), &[]);
        mm.set_state(ent.clone(), 25.0, 10, 3, true, chart, None);
        mm.view(None)
    });
    o.lite("setstate/zero", |th| {
        let mut mm = Model::default();
        mm.set_theme(Some(th));
        mm.set_state(ent.clone(), 1.5, 0, 0, false, chart, None);
        mm.view(None)
    });
    o.lite("setstate/rules", |th| {
        let mut mm = Model::default();
        mm.set_theme(Some(th));
        let mut r = default_rules_for_profile(&DifficultyProfile::casual());
        r.sensor_degradation = false;
        let mut e2 = ent.clone();
        e2.devices[lrs] = 1.5;
        e2.condition = ConditionType::Red;
        mm.set_state(e2, 1.5, 0, 0, false, chart, Some(r));
        mm.view(None)
    });
    o.each("notelemetry", |th| {
        let mut mm = Model::default();
        mm.set_theme(Some(th));
        mm.view(None)
    });
    for (k, th) in pg::variants() {
        let mm = new(Some(th), &[]);
        o.w(
            &format!("theme/{k}"),
            &format!("{} {}", mm.theme().name(), telemetry(&mm)),
        );
    }
    o.lines
}

#[test]
fn parity_statuspanel() {
    pg::check(DATA, &generate());
}
