//! Procedural 8 kHz 16-bit mono WAV synthesis (port of `pkg/audio/wav.go`).
//!
//! All transcendental functions come from [`crate::gomath`] so the output is
//! byte-identical to the Go implementation. Go folds constant expressions
//! such as `2.0 * math.Pi * 26.0` exactly before rounding; those products are
//! reproduced as the exact float64 bit patterns Go produces (`TWO_PI_*`).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use crate::gomath::{abs, cos, exp, log, pow, sin, PI};
use crate::sound::*;

pub(crate) const SAMPLE_RATE: i32 = 8000;

// Go-folded constants `2.0 * math.Pi * X` (exact product rounded once).
const TWO_PI: f64 = 2.0 * PI;
const TWO_PI_5_5: f64 = f64::from_bits(0x4041475cc9eedf01);
const TWO_PI_6_5: f64 = f64::from_bits(0x40446b9c347764a4);
const TWO_PI_8: f64 = f64::from_bits(0x404921fb54442d18);
const TWO_PI_11: f64 = f64::from_bits(0x4051475cc9eedf01);
const TWO_PI_14: f64 = f64::from_bits(0x4055fdbbe9bba775);
const TWO_PI_16: f64 = f64::from_bits(0x405921fb54442d18);
const TWO_PI_18: f64 = f64::from_bits(0x405c463abeccb2bb);
const TWO_PI_22: f64 = f64::from_bits(0x4061475cc9eedf01);
const TWO_PI_24: f64 = f64::from_bits(0x4062d97c7f3321d2);
const TWO_PI_26: f64 = f64::from_bits(0x40646b9c347764a4);
const TWO_PI_32: f64 = f64::from_bits(0x406921fb54442d18);
const TWO_PI_220: f64 = f64::from_bits(0x40959933fc6a96c1);
const TWO_PI_330: f64 = f64::from_bits(0x40a032e6fd4ff111);
const TWO_PI_660: f64 = f64::from_bits(0x40b032e6fd4ff111);
const TWO_PI_880: f64 = f64::from_bits(0x40b59933fc6a96c1);

type WavCacheKey = (SoundID, i32);

fn wav_cache() -> &'static Mutex<HashMap<WavCacheKey, Arc<Vec<u8>>>> {
    static C: OnceLock<Mutex<HashMap<WavCacheKey, Arc<Vec<u8>>>>> = OnceLock::new();
    C.get_or_init(Default::default)
}

fn raw_cache() -> &'static Mutex<HashMap<SoundID, Arc<Vec<i16>>>> {
    static C: OnceLock<Mutex<HashMap<SoundID, Arc<Vec<i16>>>>> = OnceLock::new();
    C.get_or_init(Default::default)
}

/// Full-volume WAV for `id` (Go `SynthesizeWav`).
pub fn synthesize_wav(id: &SoundID) -> Vec<u8> {
    synthesize_wav_with_volume(id, 100)
}

/// WAV for `id` scaled to `volume` (clamped to 0..=100) (Go `SynthesizeWavWithVolume`).
pub fn synthesize_wav_with_volume(id: &SoundID, volume: i32) -> Vec<u8> {
    let volume = volume.clamp(0, 100);
    let key = (id.clone(), volume);
    if let Some(c) = wav_cache().lock().unwrap().get(&key) {
        return c.as_ref().clone();
    }
    let raw = raw_samples(id);
    let wav = encode_wav(&scale_samples(&raw, volume, 0));
    let wav = Arc::new(wav);
    wav_cache().lock().unwrap().insert(key, wav.clone());
    wav.as_ref().clone()
}

/// Scales samples by `vol/100` (Go float64 multiply, clamp, truncating int16
/// conversion) and appends `pad` zero samples.
pub(crate) fn scale_samples(raw: &[i16], vol: i32, pad: usize) -> Vec<i16> {
    let mut scaled = vec![0i16; raw.len() + pad];
    let scale = vol as f64 / 100.0;
    for (i, &s) in raw.iter().enumerate() {
        let v = (s as f64 * scale).clamp(-32768.0, 32767.0);
        scaled[i] = v as i16;
    }
    scaled
}

/// Raw (full-volume) samples for `id`, cached (Go `rawSamples`).
pub(crate) fn raw_samples(id: &SoundID) -> Arc<Vec<i16>> {
    if let Some(c) = raw_cache().lock().unwrap().get(id) {
        return c.clone();
    }
    let samples = match id.as_str() {
        "phaser" => synthesize_phaser(),
        "torpedo_launch" => synthesize_torpedo_launch(),
        "explosion" => synthesize_explosion(),
        "red_alert" => synthesize_red_alert(),
        "dock" => synthesize_dock(),
        "warp" => synthesize_warp(),
        "damage" => synthesize_damage(),
        "shields" => synthesize_shields(),
        "victory" => synthesize_victory(),
        "defeat" => synthesize_defeat(),
        "cloak" => synthesize_cloak(),
        "decloak" => synthesize_decloak(),
        "plasma_launch" => synthesize_plasma_launch(),
        "plasma_impact" => synthesize_plasma_impact(),
        "tholian_web" => synthesize_tholian_web(),
        "web_breached" => synthesize_web_breached(),
        "point_defense" => synthesize_point_defense(),
        "comm_chime" => synthesize_comm_chime(),
        "computer_beep" => synthesize_computer_beep(),
        _ => synthesize_beep(),
    };
    let samples = Arc::new(samples);
    raw_cache()
        .lock()
        .unwrap()
        .insert(id.clone(), samples.clone());
    samples
}

/// RIFF/WAVE PCM encoder (Go `encodeWAV`).
pub(crate) fn encode_wav(samples: &[i16]) -> Vec<u8> {
    let data_len = samples.len() * 2;
    let riff_size = 36 + data_len;
    let mut buf = Vec::with_capacity(44 + data_len);
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&(riff_size as u32).to_le_bytes());
    buf.extend_from_slice(b"WAVE");
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes());
    buf.extend_from_slice(&(SAMPLE_RATE as u32).to_le_bytes());
    buf.extend_from_slice(&(SAMPLE_RATE as u32 * 2).to_le_bytes());
    buf.extend_from_slice(&2u16.to_le_bytes());
    buf.extend_from_slice(&16u16.to_le_bytes());
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&(data_len as u32).to_le_bytes());
    for s in samples {
        buf.extend_from_slice(&s.to_le_bytes());
    }
    buf
}

fn clamp_sample(v: f64) -> i16 {
    let v = v.clamp(-1.0, 1.0);
    (v * 32760.0) as i16
}

fn normalize_samples(buf: &[f64], target_peak: f64) -> Vec<i16> {
    let mut max_abs = 0.0f64;
    for &v in buf {
        let a = abs(v);
        if a > max_abs {
            max_abs = a;
        }
    }
    let mut out = vec![0i16; buf.len()];
    if max_abs < 1e-9 {
        return out;
    }
    let scale = target_peak / max_abs;
    for (o, &v) in out.iter_mut().zip(buf) {
        *o = clamp_sample(v * scale);
    }
    out[0] = 0;
    let n = out.len();
    if n > 1 {
        out[n - 1] = 0;
    }
    out
}

fn smooth_envelope(t: f64, duration: f64, attack: f64, release: f64) -> f64 {
    if t <= 0.0 || t >= duration {
        return 0.0;
    }
    let mut env = 1.0;
    if attack > 0.0 && t < attack {
        env = 0.5 * (1.0 - cos(PI * (t / attack)));
    } else if release > 0.0 && t > duration - release {
        let rem = (duration - t) / release;
        env = 0.5 * (1.0 - cos(PI * rem));
    }
    env
}

/// Go's deterministic LCG step and white-noise sample.
#[inline]
fn lcg_noise(rng: &mut u32) -> f64 {
    *rng = (rng.wrapping_mul(1103515245).wrapping_add(12345)) & 0x7fffffff;
    (*rng % 65536) as f64 / 32768.0 - 1.0
}

#[inline]
fn num_samples(duration: f64) -> usize {
    (SAMPLE_RATE as f64 * duration) as i32 as usize
}

#[inline]
fn time_at(i: usize) -> f64 {
    i as f64 / SAMPLE_RATE as f64
}

fn synthesize_phaser() -> Vec<i16> {
    let duration = 0.65;
    let n = num_samples(duration);
    let mut buf = vec![0.0; n];
    let mut rng: u32 = 0x41a7f3d1;
    let (mut lp_fast, mut lp_slow, mut lp_pink) = (0.0f64, 0.0f64, 0.0f64);
    let (mut phase1, mut phase2, mut phase_body) = (0.0f64, 0.0f64, 0.0f64);
    let dt = 1.0 / SAMPLE_RATE as f64;
    for (i, b) in buf.iter_mut().enumerate() {
        let t = time_at(i);
        let white = lcg_noise(&mut rng);
        lp_pink += 0.18 * (white - lp_pink);
        lp_fast += 0.68 * (white - lp_fast);
        lp_slow += 0.26 * (white - lp_slow);
        let beam_hiss = lp_fast - lp_slow;

        let f1 = 1360.0 + 80.0 * sin(TWO_PI_8 * t) + 28.0 * sin(TWO_PI_26 * t) + 95.0 * lp_pink;
        phase1 += TWO_PI * f1 * dt;
        let f2 = 2550.0
            + 130.0 * sin(TWO_PI_8 * t - 0.85)
            + 42.0 * sin(TWO_PI_26 * t + 0.5)
            + 140.0 * lp_pink;
        phase2 += TWO_PI * f2 * dt;
        let f_body = 560.0 + 30.0 * sin(TWO_PI_8 * t);
        phase_body += TWO_PI * f_body * dt;

        let am = 0.86 + 0.14 * sin(TWO_PI_26 * t);
        let tones = (0.44 * sin(phase1) + 0.44 * sin(phase2) + 0.12 * sin(phase_body)) * am;
        let env = smooth_envelope(t, duration, 0.018, 0.055);
        *b = (0.52 * tones + 0.48 * beam_hiss * (0.85 + 0.15 * sin(TWO_PI_16 * t))) * env;
    }
    normalize_samples(&buf, 0.285)
}

fn synthesize_torpedo_launch() -> Vec<i16> {
    let duration = 0.72;
    let n = num_samples(duration);
    let mut buf = vec![0.0; n];
    let tap_delays = [0.000, 0.105, 0.215, 0.330, 0.450];
    let tap_weights = [1.00, 0.82, 0.64, 0.48, 0.34];
    let tap_start = [1020.0, 900.0, 790.0, 700.0, 620.0];
    let tap_end = [150.0, 142.0, 135.0, 128.0, 120.0];
    let tap_durs = [0.19, 0.20, 0.21, 0.22, 0.24];
    let mut rng: u32 = 0x7b3c9e15;
    let (mut lp1, mut lp2) = (0.0f64, 0.0f64);
    for (i, b) in buf.iter_mut().enumerate() {
        let t = time_at(i);
        let mut spring_sum = 0.0;
        for tap in 0..5 {
            let dt = t - tap_delays[tap];
            let t_dur = tap_durs[tap];
            if dt >= 0.0 && dt < t_dur {
                let f0 = tap_start[tap];
                let f1 = tap_end[tap];
                let k = log(f1 / f0) / t_dur;
                let phi = TWO_PI * f0 * (exp(k * dt) - 1.0) / k;
                let wave = 0.52 * sin(phi) + 0.32 * sin(2.0 * phi) + 0.16 * sin(3.0 * phi);
                let tap_env = smooth_envelope(dt, t_dur, 0.006, 0.055) * exp(-1.8 * (dt / t_dur));
                spring_sum += wave * tap_env * tap_weights[tap];
            }
        }
        let mut punch = 0.0;
        if t < 0.14 {
            let phi_punch = TWO_PI * (260.0 * t - 660.0 * t * t);
            let p_env = smooth_envelope(t, 0.14, 0.004, 0.040) * exp(-8.0 * t);
            punch =
                (0.55 * sin(phi_punch) + 0.30 * sin(2.0 * phi_punch) + 0.15 * sin(3.0 * phi_punch))
                    * p_env;
        }
        let noise = lcg_noise(&mut rng);
        lp1 += 0.28 * (noise - lp1);
        lp2 += 0.28 * (lp1 - lp2);
        let pneumatic = lp2 * exp(-9.0 * t);
        let env = smooth_envelope(t, duration, 0.006, 0.080);
        *b = (0.76 * spring_sum + 0.18 * punch + 0.06 * pneumatic) * env;
    }
    normalize_samples(&buf, 0.290)
}

fn synthesize_explosion() -> Vec<i16> {
    let duration = 0.62;
    let n = num_samples(duration);
    let mut buf = vec![0.0; n];
    let mut rng: u32 = 0x12345678;
    let (mut lp1, mut lp2) = (0.0f64, 0.0f64);
    let f0 = 150.0;
    let f1 = 45.0;
    for (i, b) in buf.iter_mut().enumerate() {
        let t = time_at(i);
        let progress = t / duration;
        let raw_noise = lcg_noise(&mut rng);
        let alpha = 0.26 * (1.0 - progress) + 0.04;
        lp1 += alpha * (raw_noise - lp1);
        lp2 += alpha * (lp1 - lp2);
        let phi_boom = TWO_PI * (f0 * t + 0.5 * (f1 - f0) * t * t / duration);
        let boom = 0.52 * sin(phi_boom) + 0.32 * sin(2.0 * phi_boom) + 0.16 * sin(3.0 * phi_boom);
        let env = smooth_envelope(t, duration, 0.008, 0.100) * exp(-1.8 * progress);
        *b = (0.58 * boom + 0.42 * lp2 * 1.6) * env;
    }
    normalize_samples(&buf, 0.290)
}

fn synthesize_red_alert() -> Vec<i16> {
    let duration = 0.62;
    let n = num_samples(duration);
    let mut buf = vec![0.0; n];
    let mut phase = 0.0f64;
    let dt = 1.0 / SAMPLE_RATE as f64;
    for (i, b) in buf.iter_mut().enumerate() {
        let t = time_at(i);
        let mut sweep = t / 0.46;
        if sweep > 1.0 {
            sweep = 1.0;
        }
        let mut freq = 440.0 + 420.0 * pow(sweep, 0.75);
        freq += 8.0 * sin(TWO_PI_18 * t);
        phase += TWO_PI * freq * dt;
        let reed = 0.65 * sin(phase) + 0.24 * sin(2.0 * phase) + 0.11 * sin(3.0 * phase);
        let mut env = smooth_envelope(t, duration, 0.012, 0.095);
        if t > 0.46 {
            env *= exp(-3.5 * (t - 0.46));
        }
        *b = reed * env;
    }
    normalize_samples(&buf, 0.275)
}

fn synthesize_dock() -> Vec<i16> {
    let duration = 0.75;
    let n = num_samples(duration);
    let mut buf = vec![0.0; n];
    let mut rng: u32 = 0x31415926;
    let (mut lp1, mut lp2) = (0.0f64, 0.0f64);
    let split = 0.30;
    for (i, b) in buf.iter_mut().enumerate() {
        let t = time_at(i);
        let noise = lcg_noise(&mut rng);
        lp1 += 0.22 * (noise - lp1);
        lp2 += 0.22 * (lp1 - lp2);
        let pneumatic = lp2 * exp(-4.0 * t);

        let (freq, note_t, note_dur, clamp_thud);
        if t < split {
            freq = 220.00;
            note_t = t;
            note_dur = split;
            let phi_clamp = TWO_PI * (240.0 * note_t - 280.0 * note_t * note_t);
            clamp_thud =
                (0.60 * sin(phi_clamp) + 0.40 * sin(2.0 * phi_clamp)) * exp(-14.0 * note_t);
        } else {
            freq = 329.63;
            note_t = t - split;
            note_dur = duration - split;
            let phi_clamp = TWO_PI * (190.0 * note_t - 200.0 * note_t * note_t);
            clamp_thud =
                (0.60 * sin(phi_clamp) + 0.40 * sin(2.0 * phi_clamp)) * exp(-16.0 * note_t);
        }
        let phi = TWO_PI * freq * note_t;
        let tone = 0.50 * sin(phi) + 0.30 * sin(2.0 * phi) + 0.20 * sin(1.5 * phi);
        let tone_env = smooth_envelope(note_t, note_dur, 0.015, 0.070) * exp(-1.1 * note_t);
        let env = smooth_envelope(t, duration, 0.008, 0.080);
        *b = (0.72 * tone * tone_env + 0.20 * clamp_thud + 0.08 * pneumatic) * env;
    }
    normalize_samples(&buf, 0.280)
}

fn synthesize_warp() -> Vec<i16> {
    let duration = 0.42;
    let n = num_samples(duration);
    let mut buf = vec![0.0; n];
    let f0 = 140.0;
    let f1 = 260.0;
    let k = log(f1 / f0) / duration;
    for (i, b) in buf.iter_mut().enumerate() {
        let t = time_at(i);
        let progress = t / duration;
        let phi_base = TWO_PI * f0 * (exp(k * t) - 1.0) / k;
        let phi_shim = 2.0 * phi_base + 1.4 * sin(TWO_PI_18 * t);
        let hum = 0.42 * sin(phi_base) + 0.40 * sin(phi_shim) + 0.18 * sin(3.0 * phi_base);
        let env = smooth_envelope(t, duration, 0.004, 0.080) * exp(-1.1 * progress);
        *b = hum * env;
    }
    normalize_samples(&buf, 0.275)
}

fn synthesize_damage() -> Vec<i16> {
    let duration = 0.58;
    let n = num_samples(duration);
    let mut buf = vec![0.0; n];
    let mut rng: u32 = 0xabcdef01;
    let (mut lp_fast, mut lp_slow) = (0.0f64, 0.0f64);
    let disruptor_dur = 0.22;
    let df0 = 920.0;
    let df1 = 290.0;
    let dk = log(df1 / df0) / disruptor_dur;
    for (i, b) in buf.iter_mut().enumerate() {
        let t = time_at(i);
        let mut val = 0.0;
        if t < disruptor_dur {
            let d_phi = TWO_PI * df0 * (exp(dk * t) - 1.0) / dk + 1.8 * sin(TWO_PI_32 * t);
            let d_am = 0.74 + 0.26 * sin(TWO_PI_32 * t);
            let disruptor =
                (0.55 * sin(d_phi) + 0.30 * sin(2.0 * d_phi) + 0.15 * sin(3.0 * d_phi)) * d_am;
            let d_env = smooth_envelope(t, disruptor_dur, 0.006, 0.035);
            val += disruptor * d_env * 0.95;
        }
        if t >= 0.190 {
            let it = t - 0.190;
            let i_dur = duration - 0.190;
            let phi_hit = TWO_PI * (320.0 * it - 112.5 * it * it / i_dur);
            let thud =
                (0.52 * sin(phi_hit) + 0.32 * sin(2.0 * phi_hit) + 0.16 * sin(3.0 * phi_hit))
                    * exp(-4.5 * it);
            let noise = lcg_noise(&mut rng);
            lp_fast += 0.48 * (noise - lp_fast);
            lp_slow += 0.14 * (noise - lp_slow);
            let spark = (lp_fast - lp_slow) * (0.75 + 0.25 * sin(TWO_PI_26 * it)) * exp(-4.0 * it);
            let i_env = smooth_envelope(it, i_dur, 0.006, 0.065);
            val += (0.65 * thud + 0.45 * spark) * i_env;
        }
        *b = val;
    }
    normalize_samples(&buf, 0.290)
}

fn synthesize_shields() -> Vec<i16> {
    let duration = 0.45;
    let n = num_samples(duration);
    let mut buf = vec![0.0; n];
    for (i, b) in buf.iter_mut().enumerate() {
        let t = time_at(i);
        let m = 1.2 * sin(TWO_PI_14 * t);
        let phi1 = TWO_PI_220 * t + 0.5 * m;
        let phi2 = TWO_PI_330 * t + m;
        let phi3 = TWO_PI_660 * t + 1.5 * m;
        let am = 0.84 + 0.16 * sin(TWO_PI_14 * t);
        let chord = 0.50 * sin(phi1) + 0.34 * sin(phi2) + 0.16 * sin(phi3);
        let env = smooth_envelope(t, duration, 0.012, 0.080) * exp(-1.6 * (t / duration));
        *b = chord * am * env;
    }
    normalize_samples(&buf, 0.270)
}

fn note_index(t: f64, note_duration: f64, count: usize) -> usize {
    let idx = (t / note_duration) as i32 as usize;
    idx.min(count - 1)
}

fn synthesize_victory() -> Vec<i16> {
    let notes = [440.00, 554.37, 659.25, 880.00];
    let note_duration = 0.18;
    let total = note_duration * notes.len() as f64;
    let n = num_samples(total);
    let mut buf = vec![0.0; n];
    for (i, b) in buf.iter_mut().enumerate() {
        let t = time_at(i);
        let idx = note_index(t, note_duration, notes.len());
        let note_t = t - idx as f64 * note_duration;
        let freq = notes[idx];
        let phi = TWO_PI * freq * note_t + 0.4 * sin(TWO_PI_5_5 * note_t);
        let env = smooth_envelope(note_t, note_duration, 0.012, 0.035) * exp(-2.2 * note_t);
        *b = (0.75 * sin(phi) + 0.25 * sin(2.0 * phi)) * env;
    }
    normalize_samples(&buf, 0.275)
}

fn synthesize_defeat() -> Vec<i16> {
    let notes = [329.63, 261.63, 220.00];
    let note_duration = 0.25;
    let total = note_duration * notes.len() as f64;
    let n = num_samples(total);
    let mut buf = vec![0.0; n];
    for (i, b) in buf.iter_mut().enumerate() {
        let t = time_at(i);
        let idx = note_index(t, note_duration, notes.len());
        let note_t = t - idx as f64 * note_duration;
        let freq = notes[idx];
        let phi = TWO_PI * freq * note_t;
        let env = smooth_envelope(note_t, note_duration, 0.018, 0.050) * exp(-1.8 * note_t);
        *b = (0.70 * sin(phi) + 0.30 * sin(2.0 * phi)) * env;
    }
    normalize_samples(&buf, 0.275)
}

fn synthesize_beep() -> Vec<i16> {
    let duration = 0.16;
    let n = num_samples(duration);
    let mut buf = vec![0.0; n];
    for (i, b) in buf.iter_mut().enumerate() {
        let t = time_at(i);
        let phi = TWO_PI_880 * t;
        let env = smooth_envelope(t, duration, 0.010, 0.035);
        *b = sin(phi) * env;
    }
    normalize_samples(&buf, 0.250)
}

fn synthesize_cloak() -> Vec<i16> {
    let duration = 0.55;
    let n = num_samples(duration);
    let mut buf = vec![0.0; n];
    let f0 = 960.0;
    let f1 = 190.0;
    let k = log(f1 / f0) / duration;
    for (i, b) in buf.iter_mut().enumerate() {
        let t = time_at(i);
        let phi1 = TWO_PI * f0 * (exp(k * t) - 1.0) / k;
        let phi2 = 1.008 * phi1;
        let am = 0.75 + 0.25 * sin(TWO_PI_11 * t);
        let env = smooth_envelope(t, duration, 0.030, 0.090) * pow(1.0 - (t / duration), 0.45);
        *b = (0.50 * sin(phi1) + 0.35 * sin(phi2) + 0.15 * sin(2.0 * phi1)) * am * env;
    }
    normalize_samples(&buf, 0.275)
}

fn synthesize_decloak() -> Vec<i16> {
    let duration = 0.52;
    let n = num_samples(duration);
    let mut buf = vec![0.0; n];
    let f0 = 190.0;
    let f1 = 960.0;
    let k = log(f1 / f0) / duration;
    for (i, b) in buf.iter_mut().enumerate() {
        let t = time_at(i);
        let phi1 = TWO_PI * f0 * (exp(k * t) - 1.0) / k;
        let phi2 = 1.008 * phi1;
        let am = 0.75 + 0.25 * sin(TWO_PI_11 * t);
        let env = smooth_envelope(t, duration, 0.035, 0.080);
        *b = (0.50 * sin(phi1) + 0.35 * sin(phi2) + 0.15 * sin(2.0 * phi1)) * am * env;
    }
    normalize_samples(&buf, 0.275)
}

fn synthesize_plasma_launch() -> Vec<i16> {
    let duration = 0.58;
    let n = num_samples(duration);
    let mut buf = vec![0.0; n];
    let mut rng: u32 = 0x543210;
    let mut lp_hiss = 0.0f64;
    let f0 = 110.0;
    let f1 = 165.0;
    for (i, b) in buf.iter_mut().enumerate() {
        let t = time_at(i);
        let phi = TWO_PI * (f0 * t + 0.5 * (f1 - f0) * t * t / duration);
        let drone = 0.50 * sin(phi) + 0.32 * sin(2.0 * phi) + 0.18 * sin(4.0 * phi);
        let drone_mod = 0.72 + 0.28 * sin(TWO_PI_14 * t);
        let noise = lcg_noise(&mut rng);
        lp_hiss += 0.25 * (noise - lp_hiss);
        let env = smooth_envelope(t, duration, 0.025, 0.085) * exp(-1.2 * (t / duration));
        *b = (0.82 * drone * drone_mod + 0.18 * lp_hiss) * env;
    }
    normalize_samples(&buf, 0.280)
}

fn synthesize_plasma_impact() -> Vec<i16> {
    let duration = 0.62;
    let n = num_samples(duration);
    let mut buf = vec![0.0; n];
    let mut rng: u32 = 0x98765432;
    let (mut lp_wash, mut lp_rumble) = (0.0f64, 0.0f64);
    for (i, b) in buf.iter_mut().enumerate() {
        let t = time_at(i);
        let noise = lcg_noise(&mut rng);
        lp_wash += 0.28 * (noise - lp_wash);
        let wash_mod = 0.70 + 0.30 * sin(TWO_PI_22 * t);
        let wash = lp_wash * wash_mod * exp(-5.5 * t);
        let phi_rumble = TWO_PI * (90.0 * t - 48.0 * t * t / duration);
        lp_rumble += 0.10 * (noise - lp_rumble);
        let rumble = (0.58 * sin(phi_rumble)
            + 0.28 * sin(2.0 * phi_rumble)
            + 0.14 * sin(3.0 * phi_rumble)
            + 0.20 * lp_rumble)
            * exp(-2.8 * t);
        let env = smooth_envelope(t, duration, 0.012, 0.085);
        *b = (0.38 * wash + 0.62 * rumble) * env;
    }
    normalize_samples(&buf, 0.290)
}

fn synthesize_tholian_web() -> Vec<i16> {
    let duration = 0.45;
    let n = num_samples(duration);
    let mut buf = vec![0.0; n];
    let notes = [1174.66, 1479.98, 1760.00];
    let note_duration = duration / notes.len() as f64;
    for (i, b) in buf.iter_mut().enumerate() {
        let t = time_at(i);
        let idx = note_index(t, note_duration, notes.len());
        let note_t = t - idx as f64 * note_duration;
        let freq = notes[idx];
        let phi = TWO_PI * freq * note_t + 0.8 * sin(TWO_PI_24 * note_t);
        let env = smooth_envelope(note_t, note_duration, 0.008, 0.030) * exp(-4.0 * note_t);
        *b = (0.82 * sin(phi) + 0.18 * sin(2.0 * phi)) * env;
    }
    normalize_samples(&buf, 0.265)
}

fn synthesize_web_breached() -> Vec<i16> {
    let duration = 0.46;
    let n = num_samples(duration);
    let mut buf = vec![0.0; n];
    let mut rng: u32 = 0xfe4321;
    let mut lp_noise = 0.0f64;
    for (i, b) in buf.iter_mut().enumerate() {
        let t = time_at(i);
        let noise = lcg_noise(&mut rng);
        lp_noise += 0.25 * (noise - lp_noise);
        let phi1 = TWO_PI * (1760.0 * t - 1200.0 * t * t / duration);
        let phi2 = TWO_PI * (1320.0 * t - 900.0 * t * t / duration);
        let crystal = (0.55 * sin(phi1) + 0.45 * sin(phi2)) * exp(-4.5 * t);
        let env = smooth_envelope(t, duration, 0.010, 0.065);
        *b = (0.75 * crystal + 0.25 * lp_noise * exp(-4.0 * t)) * env;
    }
    normalize_samples(&buf, 0.270)
}

fn synthesize_point_defense() -> Vec<i16> {
    let duration = 0.24;
    let n = num_samples(duration);
    let mut buf = vec![0.0; n];
    let burst_duration = 0.08;
    for (i, b) in buf.iter_mut().enumerate() {
        let t = time_at(i);
        let burst_idx = (t / burst_duration) as i32;
        let burst_t = t - burst_idx as f64 * burst_duration;
        if burst_t < 0.065 {
            let b_env = smooth_envelope(burst_t, 0.065, 0.005, 0.015);
            let phi = TWO_PI * (1180.0 * burst_t - 150.0 * burst_t * burst_t / 0.065)
                + 1.5 * sin(TWO_PI_24 * burst_t);
            *b = (0.78 * sin(phi) + 0.22 * sin(2.0 * phi)) * b_env;
        }
    }
    normalize_samples(&buf, 0.265)
}

fn synthesize_comm_chime() -> Vec<i16> {
    let duration = 0.54;
    let n = num_samples(duration);
    let mut buf = vec![0.0; n];
    for (i, b) in buf.iter_mut().enumerate() {
        let t = time_at(i);
        let (freq, note_t, note_dur, decay_rate) = if t < 0.120 {
            (1318.51, t, 0.120, 2.5)
        } else if t < 0.250 {
            (880.00, t - 0.120, 0.130, 2.5)
        } else {
            (1318.51, t - 0.250, 0.290, 3.0)
        };
        let phi = TWO_PI * freq * note_t + 0.5 * sin(TWO_PI_6_5 * note_t);
        let env = smooth_envelope(note_t, note_dur, 0.010, 0.030) * exp(-decay_rate * note_t);
        *b = (0.84 * sin(phi) + 0.16 * sin(2.0 * phi)) * env;
    }
    normalize_samples(&buf, 0.270)
}

fn synthesize_computer_beep() -> Vec<i16> {
    let duration = 0.18;
    let n = num_samples(duration);
    let mut buf = vec![0.0; n];
    let split = 0.085;
    for (i, b) in buf.iter_mut().enumerate() {
        let t = time_at(i);
        let (freq, note_t, note_dur) = if t < split {
            (1046.50, t, split)
        } else {
            (1318.51, t - split, duration - split)
        };
        let env = smooth_envelope(note_t, note_dur, 0.008, 0.020);
        let phi = TWO_PI * freq * note_t;
        *b = sin(phi) * env;
    }
    normalize_samples(&buf, 0.250)
}
