//! Byte-for-byte parity against reference data recorded from the Go
//! implementation (`pkg/audio/zz_paritygen_test.go`, stored in
//! `testdata/parity/audio/`).

use std::io::Write;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use sst_audio::*;
use sst_engine::*;

fn parity(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/../../testdata/parity/audio/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

/// Minimal SHA-256 (FIPS 180-4) so the test needs no extra dependencies.
fn sha256_hex(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut msg = data.to_vec();
    let bit_len = (data.len() as u64) * 8;
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    for chunk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes(chunk[i * 4..i * 4 + 4].try_into().unwrap());
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut v = h;
        for i in 0..64 {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
            let t1 = v[7]
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let t2 = s0.wrapping_add(maj);
            v = [
                t1.wrapping_add(t2),
                v[0],
                v[1],
                v[2],
                v[3].wrapping_add(t1),
                v[4],
                v[5],
                v[6],
            ];
        }
        for i in 0..8 {
            h[i] = h[i].wrapping_add(v[i]);
        }
    }
    h.iter().map(|x| format!("{x:08x}")).collect()
}

#[test]
fn sha256_self_check() {
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn wav_hashes_match_go() {
    let data = parity("wav_hashes.txt");
    let mut n = 0;
    let mut bytes = 0;
    for line in data.lines() {
        let p: Vec<&str> = line.split(' ').collect();
        let id = SoundID::new(p[1]);
        let vol: i32 = p[2].parse().unwrap();
        let wav = match p[0] {
            "wav" => synthesize_wav_with_volume(&id, vol),
            "os" => synthesize_os_playback_wav(&id, vol),
            k => panic!("unknown kind {k}"),
        };
        assert_eq!(wav.len(), p[3].parse::<usize>().unwrap(), "{line}");
        assert_eq!(sha256_hex(&wav), p[4], "{line}");
        n += 1;
        bytes += wav.len();
    }
    assert_eq!(n, 520);
    eprintln!("wav parity: {n} buffers, {bytes} bytes");
}

#[test]
fn full_wav_files_match_go() {
    let dir = format!(
        "{}/../../testdata/parity/audio/wav",
        env!("CARGO_MANIFEST_DIR")
    );
    let mut n = 0;
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let id = path.file_stem().unwrap().to_string_lossy().into_owned();
        let want = std::fs::read(&path).unwrap();
        let got = synthesize_wav(&SoundID::new(id.clone()));
        assert!(got == want, "{id}: WAV bytes differ");
        n += 1;
    }
    assert_eq!(n, 20);
}

struct Recorder {
    played: std::cell::RefCell<Vec<SoundID>>,
    muted: std::cell::Cell<bool>,
}

impl Player for Recorder {
    fn play(&self, s: SoundID) {
        self.played.borrow_mut().push(s);
    }
    fn set_muted(&self, m: bool) {
        self.muted.set(m);
    }
    fn is_muted(&self) -> bool {
        self.muted.get()
    }
    fn set_volume(&self, _: i32) {}
    fn volume(&self) -> i32 {
        0
    }
}

fn parse_event(line: &str) -> Event {
    let mut parts = line.split(' ').skip(1);
    let kind = parts.next().unwrap();
    let mut kv = std::collections::HashMap::new();
    for p in parts {
        let (k, v) = p.split_once('=').unwrap();
        kv.insert(k, v);
    }
    let f = |k: &str| -> f64 { kv[k].parse().unwrap() };
    let b = |k: &str| -> bool { kv[k] == "true" };
    let i = |k: &str| -> i32 { kv[k].parse().unwrap() };
    match kind {
        "ShieldTransfer" => Event::ShieldTransfer(EventShieldTransfer {
            new_shields: 1.0,
            new_energy: 2.0,
        }),
        "TorpedoFired" => Event::TorpedoFired(Default::default()),
        "TorpedoHit" => Event::TorpedoHit(EventTorpedoHit {
            damage: f("damage"),
            destroyed: b("destroyed"),
            ..Default::default()
        }),
        "PhaserFired" => Event::PhaserFired(EventPhaserFired { energy: 100.0 }),
        "PhaserHit" => Event::PhaserHit(EventPhaserHit {
            damage: f("damage"),
            destroyed: b("destroyed"),
            ..Default::default()
        }),
        "Docked" => Event::Docked(Default::default()),
        "ShipMoved" => Event::ShipMoved(Default::default()),
        "ObstacleEncountered" => Event::ObstacleEncountered(Default::default()),
        "ConditionChanged" => Event::ConditionChanged(EventConditionChanged {
            from: ConditionType::from_i32(i("from")),
            to: ConditionType::from_i32(i("to")),
        }),
        "GameOver" => Event::GameOver(EventGameOver {
            reason: GameOverReason::from_i32(i("reason")),
            score: 10.0,
        }),
        "KlingonCounterAttack" => Event::KlingonCounterAttack(EventKlingonCounterAttack {
            damage: f("damage"),
            ..Default::default()
        }),
        "SubsystemDamaged" => Event::SubsystemDamaged(Default::default()),
        "SubsystemRepaired" => Event::SubsystemRepaired(Default::default()),
        "LRScanCompleted" => Event::LRScanCompleted(Default::default()),
        "StarbaseSurveillance" => Event::StarbaseSurveillance(Default::default()),
        "KlingonCloakState" => Event::KlingonCloakState(EventKlingonCloakState {
            cloaked: b("cloaked"),
            ..Default::default()
        }),
        "HelpCalled" => Event::HelpCalled(Default::default()),
        "HazardTriggered" => Event::HazardTriggered(Default::default()),
        "WormholeJump" => Event::WormholeJump(Default::default()),
        "SingularityAbsorption" => Event::SingularityAbsorption(Default::default()),
        "AnomalyDiscovered" => Event::AnomalyDiscovered(Default::default()),
        "KlingonScreening" => Event::KlingonScreening(Default::default()),
        "RomulanDecloak" => Event::RomulanDecloak(Default::default()),
        "PlasmaLaunched" => Event::PlasmaLaunched(Default::default()),
        "PlasmaMoved" => Event::PlasmaMoved(Default::default()),
        "PlasmaImpact" => Event::PlasmaImpact(Default::default()),
        "PlasmaDissipated" => Event::PlasmaDissipated(Default::default()),
        "PlasmaIntercepted" => Event::PlasmaIntercepted(Default::default()),
        "EntityMove" => Event::EntityMove(Default::default()),
        "WebSegmentLaid" => Event::WebSegmentLaid(Default::default()),
        "WebBreached" => Event::WebBreached(Default::default()),
        "WebCollapsed" => Event::WebCollapsed(Default::default()),
        k => panic!("unknown event {k}"),
    }
}

#[test]
fn dispatcher_matches_go() {
    let data = parity("dispatcher.txt");
    let mut lines = data.lines().peekable();
    let (mut cases, mut events_total, mut sounds_total) = (0, 0, 0);
    while let Some(header) = lines.next() {
        assert!(header.starts_with("case "), "{header}");
        let muted = header.ends_with("muted=true");
        let mut events = Vec::new();
        while lines.peek().is_some_and(|l| l.starts_with("ev ")) {
            events.push(parse_event(lines.next().unwrap()));
        }
        let sounds = lines.next().unwrap().strip_prefix("sounds ").unwrap_or("");
        let rec = Rc::new(Recorder {
            played: Default::default(),
            muted: std::cell::Cell::new(muted),
        });
        Dispatcher::new(Some(rec.clone() as PlayerRef)).dispatch_events(&events);
        let got: Vec<String> = rec.played.borrow().iter().map(|s| s.to_string()).collect();
        assert_eq!(got.join(","), sounds, "{header}");
        cases += 1;
        events_total += events.len();
        sounds_total += got.len();
    }
    assert_eq!(cases, 400);
    eprintln!("dispatcher parity: {cases} cases, {events_total} events, {sounds_total} sounds");
}

#[derive(Clone, Default)]
struct SharedBuf(Arc<Mutex<Vec<u8>>>);

impl Write for SharedBuf {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Decodes the Go `%q` strings appearing in bell.txt (only `\a` escapes).
fn unquote_bells(q: &str) -> Vec<u8> {
    let inner = q.strip_prefix('"').unwrap().strip_suffix('"').unwrap();
    let s = inner.replace("\\a", "\x07");
    assert!(!s.contains('\\'), "unexpected escape in {q}");
    s.into_bytes()
}

#[test]
fn bell_player_matches_go() {
    let data = parity("bell.txt");
    let mut player: Option<TerminalBellPlayer> = None;
    let mut buf = SharedBuf::default();
    let visual = Arc::new(Mutex::new(0));
    let mut ops = 0;
    for line in data.lines() {
        if let Some(rest) = line.strip_prefix("case ") {
            let has_writer = rest.contains("writer=true");
            let has_visual = rest.contains("visual=true");
            buf = SharedBuf::default();
            *visual.lock().unwrap() = 0;
            let v = visual.clone();
            let vb: Option<VisualBell> = if has_visual {
                Some(Arc::new(move || *v.lock().unwrap() += 1))
            } else {
                None
            };
            let w = if has_writer {
                BellWriter::Writer(Box::new(buf.clone()))
            } else {
                BellWriter::None
            };
            player = Some(TerminalBellPlayer::new(w, vb));
            continue;
        }
        let p = player.as_ref().unwrap();
        let (op, expect) = line.split_once(" -> ").unwrap();
        let (cmd, arg) = op.split_once(' ').unwrap();
        match cmd {
            "play" => p.play(SoundID::new(arg)),
            "mute" => p.set_muted(arg == "true"),
            "vol" => p.set_volume(arg.parse().unwrap()),
            _ => panic!("{line}"),
        }
        // bytes="..." visual=N muted=B volume=N
        let q_end = expect.rfind("\" visual=").unwrap() + 1;
        let want_bytes = unquote_bells(&expect["bytes=".len()..q_end]);
        let rest: Vec<&str> = expect[q_end + 1..].split(' ').collect();
        assert_eq!(*buf.0.lock().unwrap(), want_bytes, "{line}");
        assert_eq!(
            format!("visual={}", visual.lock().unwrap()),
            rest[0],
            "{line}"
        );
        assert_eq!(format!("muted={}", p.is_muted()), rest[1], "{line}");
        assert_eq!(format!("volume={}", p.volume()), rest[2], "{line}");
        ops += 1;
    }
    eprintln!("bell parity: {ops} operations");
}
