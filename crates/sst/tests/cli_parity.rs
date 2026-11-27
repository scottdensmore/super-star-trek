//! Replays `testdata/parity/cli/cli.txt` (recorded from Go `run()` by
//! `cmd/sst/zz_paritygen_test.go`): exit code, stdout, stderr and, for
//! seeded TUI launches, model facts and the initial `View()`, byte for byte.

mod common;

use common::{capture, Env};
use sst_engine::gofmt::{fmt_v, quote};

const DATA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/parity/cli/cli.txt"
));

struct Reader<'a> {
    s: &'a str,
}

impl<'a> Reader<'a> {
    fn line(&mut self) -> &'a str {
        let i = self.s.find('\n').expect("line");
        let l = &self.s[..i];
        self.s = &self.s[i + 1..];
        l
    }
    fn section(&mut self, key: &str) -> &'a str {
        let l = self.line();
        let n: usize = l
            .strip_prefix(key)
            .and_then(|r| r.strip_prefix(' '))
            .unwrap_or_else(|| panic!("expected {key}, got {l:?}"))
            .parse()
            .unwrap();
        let v = &self.s[..n];
        assert_eq!(&self.s[n..n + 1], "\n");
        self.s = &self.s[n + 1..];
        v
    }
}

fn facts(m: &sst_tui::Model) -> String {
    let mut b = String::new();
    b += &format!("sound_enabled={}\n", m.sound_enabled());
    match &m.audio_player {
        Some(p) => b += &format!("player volume={} muted={}\n", p.volume(), p.is_muted()),
        None => b += "player nil\n",
    }
    match &m.theme {
        Some(t) => b += &format!("theme={} mode={}\n", t.name(), t.color_mode()),
        None => b += "theme nil\n",
    }
    match &m.game {
        Some(g) => {
            let g = g.borrow();
            let r = &g.rules;
            b += &format!(
                "rules profile={} surveillance={} sensor={} repair={} cloak={} anomalies={} adversaries={} margin={} anim={}\n",
                r.profile,
                r.surveillance,
                r.sensor_degradation,
                fmt_v(r.repair_multiplier),
                r.klingon_cloak,
                r.spatial_anomalies,
                r.adversaries,
                fmt_v(r.time_margin),
                r.anim_speed
            );
            let e = &g.enterprise;
            b += &format!(
                "game scenario={} stardate={} klingons={} quad=[{} {}] sector=[{} {}] energy={}\n",
                quote(g.scenario.as_str()),
                fmt_v(g.stardate),
                g.remaining_klingons,
                e.quad[0],
                e.quad[1],
                e.sector[0],
                e.sector[1],
                fmt_v(e.energy)
            );
        }
        None => b += "game nil\n",
    }
    match &m.tour {
        Some(t) => {
            let t = t.borrow();
            b += &format!(
                "tour active={} sector={} sectors={}\n",
                t.active,
                t.current_sector_index,
                t.sectors.len()
            );
        }
        None => b += "tour nil\n",
    }
    b += &format!("size={}x{}\n", m.width, m.height);
    b
}

#[test]
fn cli_parity() {
    let mut env = Env::new();
    let mut r = Reader { s: DATA };
    let n: usize = r.line().strip_prefix("cases ").unwrap().parse().unwrap();
    let (mut bytes, mut views) = (0usize, 0usize);
    let mut failures = Vec::new();
    for _ in 0..n {
        let head = r.line();
        let name = head.splitn(3, ' ').nth(2).unwrap().to_string();
        let nargs: usize = r.line().strip_prefix("args ").unwrap().parse().unwrap();
        let args: Vec<String> = (0..nargs).map(|_| r.section("arg").to_string()).collect();
        let config = r.section("config");
        let progerr = r.line() == "progerr true";
        let exit: i32 = r.line().strip_prefix("exit ").unwrap().parse().unwrap();
        // Version output is recorded with placeholders for the build values.
        let stamp = |s: &str| {
            s.replace("{VERSION}", super_star_trek::VERSION)
                .replace("{COMMIT}", super_star_trek::COMMIT)
                .replace("{DATE}", super_star_trek::DATE)
        };
        let want_out = stamp(r.section("stdout"));
        let want_err = stamp(r.section("stderr"));
        let (want_out, want_err) = (want_out.as_str(), want_err.as_str());
        let model_line = r.line();

        let home = env.fresh_home();
        if !config.is_empty() {
            let dir = home.join(".super-star-trek");
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("config.json"), config).unwrap();
        }
        sst_tui::theme::detect::reset_detection_cache();
        let c = capture(if progerr {
            Err("terminal failure".into())
        } else {
            Ok(())
        });
        let mut input: &[u8] = b"";
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = super_star_trek::run(&args, &mut input, &mut out, &mut err);
        let out = String::from_utf8(out).unwrap();
        let err = String::from_utf8(err).unwrap();
        bytes += want_out.len() + want_err.len();
        if code != exit {
            failures.push(format!("{name}: exit {code} want {exit}"));
        }
        if out != want_out {
            failures.push(format!("{name}: stdout\n got {out:?}\nwant {want_out:?}"));
        }
        if err != want_err {
            failures.push(format!("{name}: stderr\n got {err:?}\nwant {want_err:?}"));
        }
        let cap = c.borrow();
        if model_line == "model none" {
            if cap.calls != 0 {
                failures.push(format!("{name}: runProgram called"));
            }
        } else {
            let opts: usize = model_line.rsplit("opts=").next().unwrap().parse().unwrap();
            if cap.calls != 1 || cap.opts.len() != opts {
                failures.push(format!(
                    "{name}: runProgram calls={} opts={}",
                    cap.calls,
                    cap.opts.len()
                ));
            }
            if model_line.starts_with("model seeded") {
                let want_facts = r.section("facts");
                let want_view = r.section("view");
                if let Some(m) = &cap.model {
                    let f = facts(m);
                    if f != want_facts {
                        failures.push(format!("{name}: facts\n got {f}\nwant {want_facts}"));
                    }
                    let v = m.view();
                    if v != want_view {
                        failures.push(format!(
                            "{name}: view mismatch\n got:\n{v}\nwant:\n{want_view}"
                        ));
                    }
                    views += 1;
                    bytes += want_facts.len() + want_view.len();
                }
            }
        }
        drop(cap);
        assert_eq!(r.line(), "end");
    }
    eprintln!("cli parity: {n} cases, {views} views, {bytes} bytes compared");
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
