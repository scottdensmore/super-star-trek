//! Ports of the Go unit tests in `pkg/audio/*_test.go`.

use std::cell::{Cell, RefCell};
use std::io::Write;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use sst_engine::*;

use crate::wav::{raw_samples, SAMPLE_RATE};
use crate::*;

fn u16le(b: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([b[i], b[i + 1]])
}
fn u32le(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
}

const FIRST_TEN: [SoundID; 10] = [
    SOUND_PHASER,
    SOUND_TORPEDO_LAUNCH,
    SOUND_EXPLOSION,
    SOUND_RED_ALERT,
    SOUND_DOCK,
    SOUND_WARP,
    SOUND_DAMAGE,
    SOUND_SHIELDS,
    SOUND_VICTORY,
    SOUND_DEFEAT,
];

const NEW_NINE: [SoundID; 9] = [
    SOUND_CLOAK,
    SOUND_DECLOAK,
    SOUND_PLASMA_LAUNCH,
    SOUND_PLASMA_IMPACT,
    SOUND_THOLIAN_WEB,
    SOUND_WEB_BREACHED,
    SOUND_POINT_DEFENSE,
    SOUND_COMM_CHIME,
    SOUND_COMPUTER_BEEP,
];

// ---- wav_test.go ----

#[test]
fn synthesize_wav_header_valid() {
    for s in FIRST_TEN {
        let wav = synthesize_wav(&s);
        assert!(wav.len() >= 44, "{s}: WAV buffer too small");
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(u32le(&wav, 4) as usize, wav.len() - 8, "{s}: RIFF size");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[12..16], b"fmt ");
        assert_eq!(u32le(&wav, 16), 16);
        assert_eq!(u16le(&wav, 20), 1);
        assert_eq!(u16le(&wav, 22), 1);
        assert_eq!(u32le(&wav, 24), 8000);
        assert_eq!(u32le(&wav, 28), 16000);
        assert_eq!(u16le(&wav, 32), 2);
        assert_eq!(u16le(&wav, 34), 16);
        assert_eq!(&wav[36..40], b"data");
        let data_len = u32le(&wav, 40) as usize;
        assert_eq!(data_len, wav.len() - 44);
        assert!(data_len > 0, "{s}: expected audio data samples");
    }
}

#[test]
fn synthesize_wav_deterministic() {
    assert_eq!(synthesize_wav(&SOUND_PHASER), synthesize_wav(&SOUND_PHASER));
}

#[test]
fn synthesize_wav_unknown_sound_id() {
    let wav = synthesize_wav(&SoundID::new("unknown_sound"));
    assert!(wav.len() >= 44);
    assert_eq!(&wav[0..4], b"RIFF");
}

#[test]
fn new_procedural_sound_synthesis() {
    for id in NEW_NINE {
        let wav = synthesize_wav(&id);
        assert!(wav.len() > 44, "{id}");
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
    }
}

#[test]
fn volume_attenuation_scaling() {
    let silent = synthesize_wav_with_volume(&SOUND_PHASER, 0);
    for i in (44..silent.len()).step_by(2) {
        assert_eq!(u16le(&silent, i) as i16, 0, "byte {i}");
    }
    let full = synthesize_wav_with_volume(&SOUND_PHASER, 100);
    let half = synthesize_wav_with_volume(&SOUND_PHASER, 50);
    assert_eq!(full.len(), half.len());
    let mut diff_found = false;
    for i in (44..full.len()).step_by(2) {
        let v_full = u16le(&full, i) as i16;
        let v_half = u16le(&half, i) as i16;
        if v_full != 0 {
            diff_found = true;
            let expected_half = (v_full as f64 * 0.5) as i16;
            // Go computes int16 subtraction (wrapping) before converting.
            let diff = (v_half.wrapping_sub(expected_half) as f64).abs();
            assert!(
                diff <= 2.0,
                "byte {i}: expected ~{expected_half}, got {v_half}"
            );
        }
    }
    assert!(diff_found);
}

#[test]
fn volume_attenuation_bounds() {
    assert_eq!(
        synthesize_wav_with_volume(&SOUND_PHASER, -20),
        synthesize_wav_with_volume(&SOUND_PHASER, 0)
    );
    assert_eq!(
        synthesize_wav_with_volume(&SOUND_PHASER, 150),
        synthesize_wav_with_volume(&SOUND_PHASER, 100)
    );
}

#[test]
fn synthesize_wav_subtle_amplitude_and_smooth_envelope() {
    for id in FIRST_TEN.iter().chain(NEW_NINE.iter()) {
        let samples = raw_samples(id);
        assert!(!samples.is_empty(), "{id}");
        // Go: `abs := s; if abs < 0 { abs = -abs }` on int16 (wrapping).
        let peak = samples
            .iter()
            .map(|&s| if s < 0 { s.wrapping_neg() } else { s })
            .max()
            .unwrap();
        assert!(peak >= 500, "{id}: peak {peak}");
        assert!(peak <= 10000, "{id}: peak {peak}");
        let first = samples[0].wrapping_abs();
        let last = samples[samples.len() - 1].wrapping_abs();
        assert!(first <= 100, "{id}: first {first}");
        assert!(last <= 100, "{id}: last {last}");
    }
}

/// Go `TestAudition` plays sounds through afplay when SST_AUDITION is set.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn audition() {
    let raw = std::env::var("SST_AUDITION").unwrap_or_default();
    let raw = raw.trim();
    if raw.is_empty() {
        eprintln!("set SST_AUDITION=1 (or comma-separated sound IDs) to play sounds");
        return;
    }
    let list: Vec<SoundID> = if raw == "1" || raw.eq_ignore_ascii_case("all") {
        vec![
            SOUND_TORPEDO_LAUNCH,
            SOUND_DOCK,
            SOUND_PHASER,
            SOUND_EXPLOSION,
            SOUND_DAMAGE,
            SOUND_WARP,
            SOUND_SHIELDS,
            SOUND_RED_ALERT,
        ]
    } else {
        raw.split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(SoundID::new)
            .collect()
    };
    let dir = std::env::temp_dir().join(format!("sst-audition-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let warm = dir.join("warmup.wav");
    let warm_samples = vec![0i16; (SAMPLE_RATE * 250 / 1000) as usize];
    if std::fs::write(&warm, crate::wav::encode_wav(&warm_samples)).is_ok() {
        let _ = std::process::Command::new("afplay").arg(&warm).status();
    }
    let lead = (SAMPLE_RATE * 80 / 1000) as usize;
    let tail = (SAMPLE_RATE * 220 / 1000) as usize;
    for (idx, id) in list.iter().enumerate() {
        let src = raw_samples(id);
        let mut padded = vec![0i16; lead + src.len() + tail];
        padded[lead..lead + src.len()].copy_from_slice(&src);
        let path = dir.join(format!("{id}.wav"));
        std::fs::write(&path, crate::wav::encode_wav(&padded)).unwrap();
        let st = std::process::Command::new("afplay")
            .arg(&path)
            .status()
            .unwrap();
        assert!(st.success(), "afplay failed for {id}");
        if idx < list.len() - 1 {
            std::thread::sleep(std::time::Duration::from_millis(850));
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

// ---- player_test.go ----

#[derive(Clone, Default)]
struct SharedBuf(Arc<Mutex<Vec<u8>>>);

impl SharedBuf {
    fn string(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
    fn len(&self) -> usize {
        self.0.lock().unwrap().len()
    }
    fn reset(&self) {
        self.0.lock().unwrap().clear();
    }
    fn writer(&self) -> BellWriter {
        BellWriter::Writer(Box::new(self.clone()))
    }
}

impl Write for SharedBuf {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn visual_flag() -> (Arc<Mutex<bool>>, VisualBell) {
    let flag = Arc::new(Mutex::new(false));
    let f = flag.clone();
    (flag, Arc::new(move || *f.lock().unwrap() = true))
}

#[test]
fn null_player() {
    let p = NullPlayer::new();
    assert!(!p.is_muted());
    p.play(SOUND_PHASER);
    p.set_muted(true);
    assert!(p.is_muted());
    p.play(SOUND_EXPLOSION);
}

#[test]
fn terminal_bell_player_sounds() {
    for s in [
        SOUND_RED_ALERT,
        SOUND_DAMAGE,
        SOUND_EXPLOSION,
        SOUND_PHASER,
        SOUND_TORPEDO_LAUNCH,
    ] {
        let buf = SharedBuf::default();
        let (called, vb) = visual_flag();
        let p = TerminalBellPlayer::new(buf.writer(), Some(vb));
        p.play(s.clone());
        assert_eq!(buf.string(), "\x07", "{s}");
        assert!(*called.lock().unwrap(), "{s}");
    }
}

#[test]
fn terminal_bell_player_ignored_sounds() {
    for s in [
        SOUND_DOCK,
        SOUND_WARP,
        SOUND_SHIELDS,
        SOUND_VICTORY,
        SOUND_DEFEAT,
    ] {
        let buf = SharedBuf::default();
        let (called, vb) = visual_flag();
        let p = TerminalBellPlayer::new(buf.writer(), Some(vb));
        p.play(s.clone());
        assert_eq!(buf.len(), 0, "{s}");
        assert!(!*called.lock().unwrap(), "{s}");
    }
}

#[test]
fn terminal_bell_player_muted() {
    let buf = SharedBuf::default();
    let (called, vb) = visual_flag();
    let p = TerminalBellPlayer::new(buf.writer(), Some(vb));
    p.set_muted(true);
    assert!(p.is_muted());
    p.play(SOUND_RED_ALERT);
    assert_eq!(buf.len(), 0);
    assert!(!*called.lock().unwrap());
}

#[test]
fn terminal_bell_player_stdout_silent_in_tests() {
    // Go skips writing to os.Stdout/os.Stderr under `go test`; the visual bell still fires.
    let (called, vb) = visual_flag();
    let p = TerminalBellPlayer::new(BellWriter::Stdout, Some(vb));
    p.play(SOUND_PHASER);
    assert!(*called.lock().unwrap());
}

#[cfg(not(target_arch = "wasm32"))]
mod native_tests {
    use super::*;

    #[test]
    fn new_native_os_player() {
        let buf = SharedBuf::default();
        let p = NativeOSPlayer::new(buf.writer(), None);
        assert!(!p.is_muted());
        p.play(SOUND_RED_ALERT);
    }

    #[test]
    fn native_os_player_fallback_when_no_cmd() {
        let buf = SharedBuf::default();
        let (called, vb) = visual_flag();
        let fallback: SharedPlayer = Arc::new(TerminalBellPlayer::new(buf.writer(), Some(vb)));
        let p = NativeOSPlayer::with_cmd(Some(fallback), "");
        assert!(!p.is_muted());
        p.play(SOUND_RED_ALERT);
        assert_eq!(buf.string(), "\x07");
        assert!(*called.lock().unwrap());

        p.set_muted(true);
        assert!(p.is_muted());
        buf.reset();
        *called.lock().unwrap() = false;
        p.play(SOUND_RED_ALERT);
        assert!(buf.len() == 0 && !*called.lock().unwrap());
    }

    #[test]
    fn native_os_player_custom_cmd() {
        let buf = SharedBuf::default();
        let fallback: SharedPlayer = Arc::new(TerminalBellPlayer::new(buf.writer(), None));
        let p = NativeOSPlayer::with_cmd(Some(fallback), "true");
        p.play(SOUND_PHASER);
        p.set_muted(true);
        p.play(SOUND_PHASER);
    }

    #[test]
    fn native_os_player_concurrency() {
        let buf = SharedBuf::default();
        let fallback: SharedPlayer = Arc::new(TerminalBellPlayer::new(buf.writer(), None));
        let p = Arc::new(NativeOSPlayer::with_cmd(Some(fallback), "true"));
        let mut handles = Vec::new();
        for idx in 0..20 {
            let a = p.clone();
            handles.push(std::thread::spawn(move || {
                a.play(SOUND_PHASER);
                a.play(SOUND_EXPLOSION);
            }));
            let b = p.clone();
            handles.push(std::thread::spawn(move || {
                b.set_muted(idx % 2 == 0);
                let _ = b.is_muted();
            }));
            let c = p.clone();
            handles.push(std::thread::spawn(move || {
                c.set_volume(idx * 5);
                let _ = c.volume();
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
    }

    #[test]
    fn player_volume_interface() {
        let np = NullPlayer::new();
        assert_eq!(np.volume(), 100);
        np.set_volume(60);
        assert_eq!(np.volume(), 60);

        let buf = SharedBuf::default();
        let tb = TerminalBellPlayer::new(buf.writer(), None);
        assert_eq!(tb.volume(), 80);
        tb.set_volume(40);
        assert_eq!(tb.volume(), 40);

        let nos = NativeOSPlayer::new(buf.writer(), None);
        assert_eq!(nos.volume(), 80);
        nos.set_volume(50);
        assert_eq!(nos.volume(), 50);
    }

    #[test]
    fn automated_test_silence_guarantees() {
        let buf = SharedBuf::default();
        let p = NativeOSPlayer::new(buf.writer(), None);
        p.play(SOUND_RED_ALERT);
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn player_volume_clamping() {
        let buf = SharedBuf::default();
        let players: Vec<PlayerRef> = vec![
            Rc::new(NullPlayer::new()),
            Rc::new(TerminalBellPlayer::new(buf.writer(), None)),
            Rc::new(NativeOSPlayer::new(buf.writer(), None)),
        ];
        for p in players {
            p.set_volume(-10);
            assert_eq!(p.volume(), 0);
            p.set_volume(150);
            assert_eq!(p.volume(), 100);
        }
    }
}

#[test]
fn terminal_bell_player_zero_volume_silence() {
    let buf = SharedBuf::default();
    let (called, vb) = visual_flag();
    let tb = TerminalBellPlayer::new(buf.writer(), Some(vb));
    tb.set_volume(0);
    tb.play(SOUND_RED_ALERT);
    assert_eq!(buf.len(), 0);
    assert!(!*called.lock().unwrap());
}

// ---- dispatcher_test.go ----

#[derive(Default)]
struct MockPlayer {
    played: RefCell<Vec<SoundID>>,
    muted: Cell<bool>,
    volume: Cell<i32>,
}

impl Player for MockPlayer {
    fn play(&self, s: SoundID) {
        if !self.muted.get() {
            self.played.borrow_mut().push(s);
        }
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

fn run(events: Vec<Event>, muted: bool) -> Vec<SoundID> {
    let p = Rc::new(MockPlayer::default());
    p.muted.set(muted);
    let d = Dispatcher::new(Some(p.clone() as PlayerRef));
    d.dispatch_events(&events);
    let v = p.played.borrow().clone();
    v
}

#[test]
fn dispatcher_event_mapping() {
    let events = vec![
        Event::PhaserFired(EventPhaserFired { energy: 500.0 }),
        Event::TorpedoFired(EventTorpedoFired {
            origin: [1, 1],
            angle: 1.57,
        }),
        Event::PhaserHit(EventPhaserHit {
            target: [2, 2],
            damage: 100.0,
            destroyed: false,
            ..Default::default()
        }),
        Event::TorpedoHit(EventTorpedoHit {
            target: [3, 3],
            damage: 200.0,
            destroyed: true,
            ..Default::default()
        }),
        Event::KlingonCounterAttack(EventKlingonCounterAttack {
            enemy_id: 1,
            damage: 50.0,
            ..Default::default()
        }),
        Event::SubsystemDamaged(EventSubsystemDamaged {
            device: DeviceID::Shields,
            repair_time: 2.0,
        }),
        Event::ShieldTransfer(EventShieldTransfer {
            new_shields: 500.0,
            new_energy: 1000.0,
        }),
        Event::Docked(EventDocked { starbase: [4, 4] }),
        Event::ConditionChanged(EventConditionChanged {
            from: ConditionType::Green,
            to: ConditionType::Red,
        }),
        Event::AnomalyDiscovered(EventAnomalyDiscovered {
            quad: [5, 5],
            env: EnvironmentType::Nebula,
        }),
        Event::WormholeJump(EventWormholeJump {
            from_quad: [1, 1],
            to_quad: [8, 8],
            ..Default::default()
        }),
        Event::ShipMoved(EventShipMoved {
            from_quad: [1, 1],
            to_quad: [2, 2],
            warp: 2.0,
            ..Default::default()
        }),
        Event::GameOver(EventGameOver {
            reason: GameOverReason::Won,
            score: 5000.0,
        }),
        Event::GameOver(EventGameOver {
            reason: GameOverReason::Destroyed,
            score: 100.0,
        }),
    ];
    let expected = vec![
        SOUND_PHASER,
        SOUND_TORPEDO_LAUNCH,
        SOUND_EXPLOSION,
        SOUND_EXPLOSION,
        SOUND_DAMAGE,
        SOUND_DAMAGE,
        SOUND_SHIELDS,
        SOUND_DOCK,
        SOUND_RED_ALERT,
        SOUND_RED_ALERT,
        SOUND_WARP,
        SOUND_WARP,
        SOUND_VICTORY,
        SOUND_DEFEAT,
    ];
    assert_eq!(run(events, false), expected);
}

#[test]
fn dispatcher_nil_player() {
    let d = Dispatcher::new(None);
    d.dispatch_events(&[Event::PhaserFired(EventPhaserFired { energy: 500.0 })]);
}

#[test]
fn dispatcher_muted_player() {
    let played = run(
        vec![
            Event::PhaserFired(EventPhaserFired { energy: 500.0 }),
            Event::Docked(EventDocked { starbase: [1, 1] }),
        ],
        true,
    );
    assert!(played.is_empty());
}

#[test]
fn dispatcher_ignored_events() {
    let played = run(
        vec![
            Event::SubsystemRepaired(EventSubsystemRepaired {
                device: DeviceID::Shields,
            }),
            Event::ConditionChanged(EventConditionChanged {
                from: ConditionType::Yellow,
                to: ConditionType::Green,
            }),
            Event::PhaserHit(EventPhaserHit {
                target: [1, 1],
                ..Default::default()
            }),
            Event::TorpedoHit(EventTorpedoHit {
                target: [1, 1],
                ..Default::default()
            }),
            Event::KlingonCounterAttack(EventKlingonCounterAttack {
                enemy_id: 1,
                ..Default::default()
            }),
        ],
        false,
    );
    assert!(played.is_empty(), "{played:?}");
}

#[test]
fn dispatcher_extended_event_mapping() {
    let events = vec![
        Event::KlingonCloakState(EventKlingonCloakState {
            klingon_id: 1,
            cloaked: true,
        }),
        Event::KlingonCloakState(EventKlingonCloakState {
            klingon_id: 1,
            cloaked: false,
        }),
        Event::RomulanDecloak(EventRomulanDecloak {
            romulan_id: 2,
            sector: [2, 2],
        }),
        Event::PlasmaLaunched(EventPlasmaLaunched {
            torpedo_id: 1,
            source_id: 2,
            sector: [2, 2],
            ..Default::default()
        }),
        Event::PlasmaImpact(EventPlasmaImpact {
            torpedo_id: 1,
            shield_damage: 100.0,
            hull_damage: 50.0,
        }),
        Event::PlasmaIntercepted(EventPlasmaIntercepted {
            torpedo_id: 1,
            sector: [3, 3],
            ..Default::default()
        }),
        Event::WebSegmentLaid(EventWebSegmentLaid {
            spinner_id: 3,
            coord: [4, 4],
        }),
        Event::WebBreached(EventWebBreached { coord: [4, 4] }),
        Event::HelpCalled(EventHelpCalled {}),
        Event::LRScanCompleted(EventLRScanCompleted {
            center_quad: [1, 1],
            ..Default::default()
        }),
    ];
    let expected = vec![
        SOUND_CLOAK,
        SOUND_DECLOAK,
        SOUND_DECLOAK,
        SOUND_PLASMA_LAUNCH,
        SOUND_PLASMA_IMPACT,
        SOUND_POINT_DEFENSE,
        SOUND_THOLIAN_WEB,
        SOUND_WEB_BREACHED,
        SOUND_COMM_CHIME,
        SOUND_COMPUTER_BEEP,
    ];
    assert_eq!(run(events, false), expected);
}
