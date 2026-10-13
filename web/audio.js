/**
 * Super Star Trek - Procedural Web Audio API Sound Engine
 * Authentic retro sound FX synthesized procedurally using AudioContext,
 * OscillatorNode, GainNode, procedural white noise AudioBuffer, and BiquadFilterNode.
 */

let audioCtx = null;

function getAudioContext() {
    if (!audioCtx) {
        const AudioContextClass = typeof window !== 'undefined'
            ? (window.AudioContext || window.webkitAudioContext)
            : (typeof AudioContext !== 'undefined' ? AudioContext : null);
        if (AudioContextClass) {
            audioCtx = new AudioContextClass();
        }
    }
    if (audioCtx && audioCtx.state === 'suspended') {
        audioCtx.resume().catch(() => {});
    }
    return audioCtx;
}

// Autoplay unlock on user interaction
function unlockAudio() {
    const ctx = getAudioContext();
    if (ctx && ctx.state === 'suspended') {
        ctx.resume().catch(() => {});
    }
}

if (typeof window !== 'undefined' && typeof window.addEventListener === 'function') {
    window.addEventListener('click', unlockAudio, { passive: true });
    window.addEventListener('keydown', unlockAudio, { passive: true });
    window.addEventListener('touchstart', unlockAudio, { passive: true });
}

// Mute & volume state management with localStorage persistence
let isMuted = false;
let currentVolume = 80;
try {
    if (typeof localStorage !== 'undefined') {
        const saved = localStorage.getItem('sst_sound_muted');
        if (saved !== null) {
            isMuted = (saved === 'true');
        }
        const savedVol = localStorage.getItem('sst_sound_volume');
        if (savedVol !== null) {
            const parsed = parseInt(savedVol, 10);
            if (!Number.isNaN(parsed)) {
                currentVolume = Math.max(0, Math.min(100, parsed));
            }
        }
    }
} catch (e) {
    // LocalStorage inaccessible in restricted environments
}

function sstSetMuted(muted) {
    isMuted = Boolean(muted);
    try {
        if (typeof localStorage !== 'undefined') {
            localStorage.setItem('sst_sound_muted', isMuted ? 'true' : 'false');
        }
    } catch (e) {}
}

function sstIsMuted() {
    return isMuted;
}

function sstSetVolume(vol) {
    const n = Number(vol);
    if (!Number.isNaN(n)) {
        currentVolume = Math.max(0, Math.min(100, Math.round(n)));
        try {
            if (typeof localStorage !== 'undefined') {
                localStorage.setItem('sst_sound_volume', String(currentVolume));
            }
        } catch (e) {}
    }
}

function sstGetVolume() {
    return currentVolume;
}

function volumeScale() {
    return Math.max(0.0001, currentVolume / 100.0);
}

// Procedural white noise AudioBuffer generator
function createNoiseBuffer(ctx, durationSeconds) {
    const bufferSize = Math.max(1, Math.floor(ctx.sampleRate * durationSeconds));
    const buffer = ctx.createBuffer(1, bufferSize, ctx.sampleRate);
    const output = buffer.getChannelData(0);
    let rng = 0x12345678;
    for (let i = 0; i < bufferSize; i++) {
        rng = (Math.imul(rng, 1103515245) + 12345) & 0x7fffffff;
        output[i] = (rng % 65536) / 32768.0 - 1.0;
    }
    return buffer;
}

/**
 * Subtle Star Trek: The Original Series (TOS) Procedural Synthesizers
 */

// TOS Enterprise Ship Phaser Beam: twin oscillating tape-feedback carriers (~1360 Hz & ~2550 Hz) + 560 Hz emitter body + 1200-2800 Hz ion beam hiss (650ms)
function playPhaser(ctx = getAudioContext(), startTime) {
    if (!ctx) return;
    try {
        const now = startTime !== undefined ? startTime : ctx.currentTime;
        const duration = 0.65;
        const scale = volumeScale();

        const osc1 = ctx.createOscillator();
        const osc2 = ctx.createOscillator();
        const oscBody = ctx.createOscillator();
        const bodyGain = ctx.createGain();
        const lfoSlow = ctx.createOscillator();
        const lfoSlowGain1 = ctx.createGain();
        const lfoSlowGain2 = ctx.createGain();
        const lfoFast = ctx.createOscillator();
        const lfoFastGain = ctx.createGain();
        const noise = ctx.createBufferSource();
        const noiseFilter = ctx.createBiquadFilter();
        const noiseGain = ctx.createGain();
        const masterGain = ctx.createGain();

        // Twin TOS ship phaser carriers: ~1360 Hz and ~2550 Hz
        osc1.type = 'sine';
        osc1.frequency.setValueAtTime(1360, now);

        osc2.type = 'sine';
        osc2.frequency.setValueAtTime(2550, now);

        oscBody.type = 'triangle';
        oscBody.frequency.setValueAtTime(560, now);
        bodyGain.gain.setValueAtTime(0.28, now);

        // 8 Hz primary warble sweep (1280-1440 Hz and 2420-2680 Hz)
        lfoSlow.type = 'sine';
        lfoSlow.frequency.setValueAtTime(8.0, now);
        lfoSlowGain1.gain.setValueAtTime(80, now);
        lfoSlowGain2.gain.setValueAtTime(130, now);

        // 26 Hz tape-feedback flutter
        lfoFast.type = 'sine';
        lfoFast.frequency.setValueAtTime(26.0, now);
        lfoFastGain.gain.setValueAtTime(35, now);

        lfoSlow.connect(lfoSlowGain1);
        lfoSlow.connect(lfoSlowGain2);
        lfoSlowGain1.connect(osc1.frequency);
        lfoSlowGain2.connect(osc2.frequency);

        lfoFast.connect(lfoFastGain);
        lfoFastGain.connect(osc1.frequency);
        lfoFastGain.connect(osc2.frequency);

        // High-energy bandpass ion beam hiss (1200-2800 Hz)
        noise.buffer = createNoiseBuffer(ctx, duration);
        noiseFilter.type = 'bandpass';
        noiseFilter.frequency.setValueAtTime(1950, now);
        noiseFilter.Q.setValueAtTime(1.4, now);
        noiseGain.gain.setValueAtTime(0.95, now);

        noise.connect(noiseFilter);
        noiseFilter.connect(noiseGain);
        noiseGain.connect(masterGain);

        oscBody.connect(bodyGain);
        bodyGain.connect(masterGain);
        osc1.connect(masterGain);
        osc2.connect(masterGain);

        // Flat, high-energy sustained plateau with crisp 18ms attack and 55ms release
        masterGain.gain.setValueAtTime(0.0001, now);
        masterGain.gain.linearRampToValueAtTime(0.095 * scale, now + 0.018);
        masterGain.gain.setValueAtTime(0.095 * scale, now + duration - 0.055);
        masterGain.gain.exponentialRampToValueAtTime(0.0001, now + duration);
        masterGain.connect(ctx.destination);

        lfoSlow.start(now);
        lfoFast.start(now);
        osc1.start(now);
        osc2.start(now);
        oscBody.start(now);
        noise.start(now);

        const stopAt = now + duration + 0.04;
        lfoSlow.stop(stopAt);
        lfoFast.stop(stopAt);
        osc1.stop(stopAt);
        osc2.stop(stopAt);
        oscBody.stop(stopAt);
        noise.stop(stopAt);
    } catch (e) {}
}

// TOS Photon Torpedo Launch: iconic 1966 retro dispersive spring-wire twang + tape slapback echo ("ptew-ew-ew-ew") + pneumatic tube punch (720ms)
function playTorpedoLaunch(ctx = getAudioContext(), startTime) {
    if (!ctx) return;
    try {
        const now = startTime !== undefined ? startTime : ctx.currentTime;
        const scale = volumeScale();

        // 1. 5 dispersive spring-wire chirp taps over 720ms
        const taps = [
            { delay: 0.000, dur: 0.19, fStart: 1020, fEnd: 150, weight: 1.00 },
            { delay: 0.105, dur: 0.20, fStart: 900,  fEnd: 142, weight: 0.82 },
            { delay: 0.215, dur: 0.21, fStart: 790,  fEnd: 135, weight: 0.64 },
            { delay: 0.330, dur: 0.22, fStart: 700,  fEnd: 128, weight: 0.48 },
            { delay: 0.450, dur: 0.24, fStart: 620,  fEnd: 120, weight: 0.34 },
        ];

        taps.forEach((tap) => {
            const t0 = now + tap.delay;
            const oscFund = ctx.createOscillator();
            const oscHarm = ctx.createOscillator();
            const harmGain = ctx.createGain();
            const filter = ctx.createBiquadFilter();
            const tapGain = ctx.createGain();

            oscFund.type = 'sine';
            oscFund.frequency.setValueAtTime(tap.fStart, t0);
            oscFund.frequency.exponentialRampToValueAtTime(tap.fEnd, t0 + tap.dur);

            // Metallic spring overtone (2x harmonic)
            oscHarm.type = 'triangle';
            oscHarm.frequency.setValueAtTime(tap.fStart * 2.0, t0);
            oscHarm.frequency.exponentialRampToValueAtTime(tap.fEnd * 2.0, t0 + tap.dur);
            harmGain.gain.setValueAtTime(0.45, t0);

            filter.type = 'lowpass';
            filter.frequency.setValueAtTime(2100, t0);
            filter.frequency.exponentialRampToValueAtTime(380, t0 + tap.dur);

            const peak = 0.095 * tap.weight * scale;
            tapGain.gain.setValueAtTime(0.0001, now);
            tapGain.gain.setValueAtTime(0.0001, t0);
            tapGain.gain.linearRampToValueAtTime(Math.max(0.0002, peak), t0 + 0.006);
            tapGain.gain.exponentialRampToValueAtTime(0.0001, t0 + tap.dur);

            oscHarm.connect(harmGain);
            harmGain.connect(filter);
            oscFund.connect(filter);
            filter.connect(tapGain);
            tapGain.connect(ctx.destination);

            oscFund.start(t0);
            oscHarm.start(t0);
            oscFund.stop(t0 + tap.dur + 0.03);
            oscHarm.stop(t0 + tap.dur + 0.03);
        });

        // 2. Pneumatic torpedo tube launch punch (low-mid body at 0-140ms)
        const punchOsc = ctx.createOscillator();
        const punchGain = ctx.createGain();
        punchOsc.type = 'triangle';
        punchOsc.frequency.setValueAtTime(260, now);
        punchOsc.frequency.exponentialRampToValueAtTime(75, now + 0.14);

        punchGain.gain.setValueAtTime(0.0001, now);
        punchGain.gain.linearRampToValueAtTime(0.065 * scale, now + 0.005);
        punchGain.gain.exponentialRampToValueAtTime(0.0001, now + 0.14);

        punchOsc.connect(punchGain);
        punchGain.connect(ctx.destination);
        punchOsc.start(now);
        punchOsc.stop(now + 0.16);
    } catch (e) {}
}

// TOS Deep-Space Detonation: warm low-frequency boom (85 Hz -> 30 Hz) with low-pass rumble (500 ms)
function playExplosion(ctx = getAudioContext(), startTime) {
    if (!ctx) return;
    try {
        const now = startTime !== undefined ? startTime : ctx.currentTime;
        const duration = 0.50;
        const scale = volumeScale();

        const noise = ctx.createBufferSource();
        noise.buffer = createNoiseBuffer(ctx, duration);

        const filter = ctx.createBiquadFilter();
        filter.type = 'lowpass';
        filter.frequency.setValueAtTime(420, now);
        filter.frequency.exponentialRampToValueAtTime(55, now + duration);
        filter.Q.setValueAtTime(1.8, now);

        const subOsc = ctx.createOscillator();
        subOsc.type = 'sine';
        subOsc.frequency.setValueAtTime(85, now);
        subOsc.frequency.exponentialRampToValueAtTime(30, now + duration);

        const gain = ctx.createGain();
        gain.gain.setValueAtTime(0.0001, now);
        gain.gain.linearRampToValueAtTime(0.11 * scale, now + 0.018);
        gain.gain.exponentialRampToValueAtTime(0.0001, now + duration);

        noise.connect(filter);
        filter.connect(gain);
        subOsc.connect(gain);
        gain.connect(ctx.destination);

        noise.start(now);
        subOsc.start(now);
        noise.stop(now + duration + 0.04);
        subOsc.stop(now + duration + 0.04);
    } catch (e) {}
}

// TOS "Condition Red" Klaxon ("Reed Alert"): subtle upward-curving reed sweep (440 Hz -> 860 Hz)
function playRedAlert(ctx = getAudioContext(), startTime) {
    if (!ctx) return;
    try {
        const now = startTime !== undefined ? startTime : ctx.currentTime;
        const duration = 0.52;
        const scale = volumeScale();

        const osc = ctx.createOscillator();
        const lfo = ctx.createOscillator();
        const lfoGain = ctx.createGain();
        const filter = ctx.createBiquadFilter();
        const gain = ctx.createGain();

        osc.type = 'triangle';
        osc.frequency.setValueAtTime(440, now);
        osc.frequency.exponentialRampToValueAtTime(860, now + 0.38);
        osc.frequency.setValueAtTime(860, now + duration);

        lfo.type = 'sine';
        lfo.frequency.setValueAtTime(18, now);
        lfoGain.gain.setValueAtTime(8, now);

        lfo.connect(lfoGain);
        lfoGain.connect(osc.frequency);

        filter.type = 'lowpass';
        filter.frequency.setValueAtTime(1500, now);

        gain.gain.setValueAtTime(0.0001, now);
        gain.gain.linearRampToValueAtTime(0.08 * scale, now + 0.025);
        gain.gain.setValueAtTime(0.08 * scale, now + 0.36);
        gain.gain.exponentialRampToValueAtTime(0.0001, now + duration);

        osc.connect(filter);
        filter.connect(gain);
        gain.connect(ctx.destination);

        lfo.start(now);
        osc.start(now);
        lfo.stop(now + duration + 0.04);
        osc.stop(now + duration + 0.04);
    } catch (e) {}
}

// TOS Starbase Docking Sequence: low mechanical docking clamp latch + soft pneumatic airlock seal + warm low-register confirmation (220 Hz -> 330 Hz over 750ms)
function playDock(ctx = getAudioContext(), startTime) {
    if (!ctx) return;
    try {
        const now = startTime !== undefined ? startTime : ctx.currentTime;
        const scale = volumeScale();

        // 1. Mechanical docking clamp latch (240 Hz -> 90 Hz, 0-120ms)
        const clampOsc = ctx.createOscillator();
        const clampGain = ctx.createGain();
        clampOsc.type = 'triangle';
        clampOsc.frequency.setValueAtTime(240, now);
        clampOsc.frequency.exponentialRampToValueAtTime(90, now + 0.12);

        clampGain.gain.setValueAtTime(0.0001, now);
        clampGain.gain.linearRampToValueAtTime(0.075 * scale, now + 0.008);
        clampGain.gain.exponentialRampToValueAtTime(0.0001, now + 0.12);

        clampOsc.connect(clampGain);
        clampGain.connect(ctx.destination);
        clampOsc.start(now);
        clampOsc.stop(now + 0.14);

        // 2. Pneumatic airlock equalization hiss (low-passed noise, 30-350ms)
        const hissDur = 0.32;
        const hiss = ctx.createBufferSource();
        hiss.buffer = createNoiseBuffer(ctx, hissDur);
        const hissFilter = ctx.createBiquadFilter();
        hissFilter.type = 'lowpass';
        hissFilter.frequency.setValueAtTime(520, now + 0.03);
        const hissGain = ctx.createGain();
        hissGain.gain.setValueAtTime(0.0001, now);
        hissGain.gain.setValueAtTime(0.0001, now + 0.03);
        hissGain.gain.linearRampToValueAtTime(0.03 * scale, now + 0.07);
        hissGain.gain.exponentialRampToValueAtTime(0.0001, now + 0.03 + hissDur);

        hiss.connect(hissFilter);
        hissFilter.connect(hissGain);
        hissGain.connect(ctx.destination);
        hiss.start(now + 0.03);
        hiss.stop(now + 0.03 + hissDur + 0.02);

        // 3. Warm low-register confirmation hum (A3 220 Hz -> E4 329.63 Hz with rich 2x/1.5x overtones)
        const tones = [
            { freq: 220.0,  offset: 0.00, dur: 0.30 },
            { freq: 329.63, offset: 0.30, dur: 0.45 },
        ];

        tones.forEach((tone) => {
            const t0 = now + tone.offset;
            const osc = ctx.createOscillator();
            const harmOsc = ctx.createOscillator();
            const harmGain = ctx.createGain();
            const gain = ctx.createGain();

            osc.type = 'sine';
            osc.frequency.setValueAtTime(tone.freq, t0);

            harmOsc.type = 'triangle';
            harmOsc.frequency.setValueAtTime(tone.freq * 2.0, t0);
            harmGain.gain.setValueAtTime(0.35, t0);

            gain.gain.setValueAtTime(0.0001, now);
            gain.gain.setValueAtTime(0.0001, t0);
            gain.gain.linearRampToValueAtTime(0.08 * scale, t0 + 0.02);
            gain.gain.exponentialRampToValueAtTime(0.0001, t0 + tone.dur);

            harmOsc.connect(harmGain);
            harmGain.connect(gain);
            osc.connect(gain);
            gain.connect(ctx.destination);

            osc.start(t0);
            harmOsc.start(t0);
            osc.stop(t0 + tone.dur + 0.03);
            harmOsc.stop(t0 + tone.dur + 0.03);
        });
    } catch (e) {}
}

// TOS Enterprise Impulse/Warp Drive Engage Pulse (130 Hz -> 220 Hz + 260 Hz -> 440 Hz turbine shimmer, 200ms)
function playWarp(ctx = getAudioContext(), startTime) {
    if (!ctx) return;
    try {
        const now = startTime !== undefined ? startTime : ctx.currentTime;
        const duration = 0.20;
        const scale = volumeScale();

        const oscLow = ctx.createOscillator();
        const oscHigh = ctx.createOscillator();
        const filter = ctx.createBiquadFilter();
        const gain = ctx.createGain();

        oscLow.type = 'sine';
        oscLow.frequency.setValueAtTime(130, now);
        oscLow.frequency.exponentialRampToValueAtTime(220, now + duration);

        oscHigh.type = 'triangle';
        oscHigh.frequency.setValueAtTime(260, now);
        oscHigh.frequency.exponentialRampToValueAtTime(440, now + duration);

        filter.type = 'lowpass';
        filter.frequency.setValueAtTime(650, now);

        gain.gain.setValueAtTime(0.0001, now);
        gain.gain.linearRampToValueAtTime(0.08 * scale, now + 0.005);
        gain.gain.exponentialRampToValueAtTime(0.0001, now + duration);

        oscLow.connect(filter);
        oscHigh.connect(filter);
        filter.connect(gain);
        gain.connect(ctx.destination);

        oscLow.start(now);
        oscHigh.start(now);
        oscLow.stop(now + duration + 0.03);
        oscHigh.stop(now + duration + 0.03);
    } catch (e) {}
}

// TOS Klingon Return Fire: Phase 1 (0..135ms) Klingon disruptor bolt (920 -> 290 Hz) + Phase 2 (120..380ms) shield/hull hit & sparks
function playDamage(ctx = getAudioContext(), startTime) {
    if (!ctx) return;
    try {
        const now = startTime !== undefined ? startTime : ctx.currentTime;
        const duration = 0.38;
        const scale = volumeScale();

        // Phase 1: Incoming Klingon disruptor bolt (0..135ms)
        const disOsc = ctx.createOscillator();
        const disLfo = ctx.createOscillator();
        const disLfoGain = ctx.createGain();
        const disGain = ctx.createGain();

        disOsc.type = 'triangle';
        disOsc.frequency.setValueAtTime(920, now);
        disOsc.frequency.exponentialRampToValueAtTime(290, now + 0.135);

        disLfo.type = 'sine';
        disLfo.frequency.setValueAtTime(32, now);
        disLfoGain.gain.setValueAtTime(55, now);

        disLfo.connect(disLfoGain);
        disLfoGain.connect(disOsc.frequency);

        disGain.gain.setValueAtTime(0.0001, now);
        disGain.gain.linearRampToValueAtTime(0.095 * scale, now + 0.008);
        disGain.gain.exponentialRampToValueAtTime(0.0001, now + 0.14);

        disOsc.connect(disGain);
        disGain.connect(ctx.destination);

        disLfo.start(now);
        disOsc.start(now);
        disLfo.stop(now + 0.15);
        disOsc.stop(now + 0.15);

        // Phase 2: Enterprise shield/hull impact & console sparks (120..380ms)
        const hitTime = now + 0.12;
        const hitDur = duration - 0.12;

        const noise = ctx.createBufferSource();
        noise.buffer = createNoiseBuffer(ctx, hitDur);

        const hitOsc = ctx.createOscillator();
        hitOsc.type = 'triangle';
        hitOsc.frequency.setValueAtTime(320, hitTime);
        hitOsc.frequency.exponentialRampToValueAtTime(95, hitTime + hitDur);

        const filter = ctx.createBiquadFilter();
        filter.type = 'bandpass';
        filter.frequency.setValueAtTime(1200, hitTime);
        filter.Q.setValueAtTime(1.8, hitTime);

        const noiseGain = ctx.createGain();
        noiseGain.gain.setValueAtTime(0.32, hitTime);

        const oscGain = ctx.createGain();
        oscGain.gain.setValueAtTime(0.65, hitTime);

        const masterGain = ctx.createGain();
        masterGain.gain.setValueAtTime(0.0001, now);
        masterGain.gain.setValueAtTime(0.0001, hitTime);
        masterGain.gain.linearRampToValueAtTime(0.105 * scale, hitTime + 0.01);
        masterGain.gain.exponentialRampToValueAtTime(0.0001, hitTime + hitDur);

        noise.connect(filter);
        filter.connect(noiseGain);
        noiseGain.connect(masterGain);

        hitOsc.connect(oscGain);
        oscGain.connect(masterGain);

        masterGain.connect(ctx.destination);

        noise.start(hitTime);
        hitOsc.start(hitTime);
        noise.stop(hitTime + hitDur + 0.03);
        hitOsc.stop(hitTime + hitDur + 0.03);
    } catch (e) {}
}

// TOS Deflector Screen Harmonic Hum/Swell (220 Hz + 330 Hz with 14 Hz shimmer)
function playShields(ctx = getAudioContext(), startTime) {
    if (!ctx) return;
    try {
        const now = startTime !== undefined ? startTime : ctx.currentTime;
        const duration = 0.38;
        const scale = volumeScale();

        const osc1 = ctx.createOscillator();
        const osc2 = ctx.createOscillator();
        const lfo = ctx.createOscillator();
        const lfoGain = ctx.createGain();
        const gain = ctx.createGain();

        osc1.type = 'sine';
        osc1.frequency.setValueAtTime(220, now);
        osc2.type = 'sine';
        osc2.frequency.setValueAtTime(330, now);

        lfo.type = 'sine';
        lfo.frequency.setValueAtTime(14, now);
        lfoGain.gain.setValueAtTime(12, now);

        lfo.connect(lfoGain);
        lfoGain.connect(osc1.frequency);
        lfoGain.connect(osc2.frequency);

        gain.gain.setValueAtTime(0.0001, now);
        gain.gain.linearRampToValueAtTime(0.075 * scale, now + 0.045);
        gain.gain.exponentialRampToValueAtTime(0.0001, now + duration);

        osc1.connect(gain);
        osc2.connect(gain);
        gain.connect(ctx.destination);

        lfo.start(now);
        osc1.start(now);
        osc2.start(now);
        lfo.stop(now + duration + 0.04);
        osc1.stop(now + duration + 0.04);
        osc2.stop(now + duration + 0.04);
    } catch (e) {}
}

// TOS Starfleet Victory Fanfare (A4, C#5, E5, A5)
function playVictory(ctx = getAudioContext(), startTime) {
    if (!ctx) return;
    try {
        const now = startTime !== undefined ? startTime : ctx.currentTime;
        const scale = volumeScale();
        const notes = [440.0, 554.37, 659.25, 880.0];
        const noteDuration = 0.15;

        notes.forEach((freq, idx) => {
            const t0 = now + idx * noteDuration;
            const osc = ctx.createOscillator();
            const gain = ctx.createGain();

            osc.type = 'sine';
            osc.frequency.setValueAtTime(freq, t0);

            gain.gain.setValueAtTime(0.0001, now);
            gain.gain.setValueAtTime(0.0001, t0);
            gain.gain.linearRampToValueAtTime(0.08 * scale, t0 + 0.015);
            gain.gain.exponentialRampToValueAtTime(0.0001, t0 + noteDuration + 0.03);

            osc.connect(gain);
            gain.connect(ctx.destination);

            osc.start(t0);
            osc.stop(t0 + noteDuration + 0.04);
        });
    } catch (e) {}
}

// TOS Somber Descending Cadence (E4 -> C4 -> A3)
function playDefeat(ctx = getAudioContext(), startTime) {
    if (!ctx) return;
    try {
        const now = startTime !== undefined ? startTime : ctx.currentTime;
        const scale = volumeScale();
        const notes = [329.63, 261.63, 220.0];
        const noteDuration = 0.22;

        notes.forEach((freq, idx) => {
            const t0 = now + idx * noteDuration;
            const osc = ctx.createOscillator();
            const gain = ctx.createGain();

            osc.type = 'sine';
            osc.frequency.setValueAtTime(freq, t0);

            gain.gain.setValueAtTime(0.0001, now);
            gain.gain.setValueAtTime(0.0001, t0);
            gain.gain.linearRampToValueAtTime(0.075 * scale, t0 + 0.02);
            gain.gain.exponentialRampToValueAtTime(0.0001, t0 + noteDuration + 0.03);

            osc.connect(gain);
            gain.connect(ctx.destination);

            osc.start(t0);
            osc.stop(t0 + noteDuration + 0.04);
        });
    } catch (e) {}
}

// TOS Romulan Cloaking Veil (960 Hz -> 190 Hz ethereal phase sweep)
function playCloak(ctx = getAudioContext(), startTime) {
    if (!ctx) return;
    try {
        const now = startTime !== undefined ? startTime : ctx.currentTime;
        const duration = 0.42;
        const scale = volumeScale();

        const osc = ctx.createOscillator();
        const gain = ctx.createGain();

        osc.type = 'sine';
        osc.frequency.setValueAtTime(960, now);
        osc.frequency.exponentialRampToValueAtTime(190, now + duration);

        gain.gain.setValueAtTime(0.0001, now);
        gain.gain.linearRampToValueAtTime(0.075 * scale, now + 0.03);
        gain.gain.exponentialRampToValueAtTime(0.0001, now + duration);

        osc.connect(gain);
        gain.connect(ctx.destination);

        osc.start(now);
        osc.stop(now + duration + 0.04);
    } catch (e) {}
}

// TOS Romulan Decloaking Phase-In (190 Hz -> 960 Hz)
function playDecloak(ctx = getAudioContext(), startTime) {
    if (!ctx) return;
    try {
        const now = startTime !== undefined ? startTime : ctx.currentTime;
        const duration = 0.38;
        const scale = volumeScale();

        const osc = ctx.createOscillator();
        const gain = ctx.createGain();

        osc.type = 'sine';
        osc.frequency.setValueAtTime(190, now);
        osc.frequency.exponentialRampToValueAtTime(960, now + duration);

        gain.gain.setValueAtTime(0.0001, now);
        gain.gain.linearRampToValueAtTime(0.075 * scale, now + 0.035);
        gain.gain.exponentialRampToValueAtTime(0.0001, now + duration);

        osc.connect(gain);
        gain.connect(ctx.destination);

        osc.start(now);
        osc.stop(now + duration + 0.04);
    } catch (e) {}
}

// TOS Romulan Plasma Torpedo Launch Hum (110 Hz -> 165 Hz pulsing drone)
function playPlasmaLaunch(ctx = getAudioContext(), startTime) {
    if (!ctx) return;
    try {
        const now = startTime !== undefined ? startTime : ctx.currentTime;
        const duration = 0.45;
        const scale = volumeScale();

        const osc = ctx.createOscillator();
        const gain = ctx.createGain();

        osc.type = 'triangle';
        osc.frequency.setValueAtTime(110, now);
        osc.frequency.exponentialRampToValueAtTime(165, now + duration);

        gain.gain.setValueAtTime(0.0001, now);
        gain.gain.linearRampToValueAtTime(0.08 * scale, now + 0.03);
        gain.gain.exponentialRampToValueAtTime(0.0001, now + duration);

        osc.connect(gain);
        gain.connect(ctx.destination);

        osc.start(now);
        osc.stop(now + duration + 0.04);
    } catch (e) {}
}

// TOS Romulan Plasma Torpedo Impact (deep 60 Hz -> 34 Hz hull resonance)
function playPlasmaImpact(ctx = getAudioContext(), startTime) {
    if (!ctx) return;
    try {
        const now = startTime !== undefined ? startTime : ctx.currentTime;
        const duration = 0.50;
        const scale = volumeScale();

        const osc = ctx.createOscillator();
        const gain = ctx.createGain();

        osc.type = 'sine';
        osc.frequency.setValueAtTime(60, now);
        osc.frequency.exponentialRampToValueAtTime(34, now + duration);

        gain.gain.setValueAtTime(0.0001, now);
        gain.gain.linearRampToValueAtTime(0.095 * scale, now + 0.015);
        gain.gain.exponentialRampToValueAtTime(0.0001, now + duration);

        osc.connect(gain);
        gain.connect(ctx.destination);

        osc.start(now);
        osc.stop(now + duration + 0.04);
    } catch (e) {}
}

// TOS Tholian Energy Web Triad (D6, F#6, A6)
function playTholianWeb(ctx = getAudioContext(), startTime) {
    if (!ctx) return;
    try {
        const now = startTime !== undefined ? startTime : ctx.currentTime;
        const scale = volumeScale();
        const notes = [1174.66, 1479.98, 1760.0];
        const noteDuration = 0.10;

        notes.forEach((freq, idx) => {
            const t0 = now + idx * noteDuration;
            const osc = ctx.createOscillator();
            const gain = ctx.createGain();

            osc.type = 'sine';
            osc.frequency.setValueAtTime(freq, t0);

            gain.gain.setValueAtTime(0.0001, now);
            gain.gain.setValueAtTime(0.0001, t0);
            gain.gain.linearRampToValueAtTime(0.065 * scale, t0 + 0.01);
            gain.gain.exponentialRampToValueAtTime(0.0001, t0 + noteDuration);

            osc.connect(gain);
            gain.connect(ctx.destination);

            osc.start(t0);
            osc.stop(t0 + noteDuration + 0.02);
        });
    } catch (e) {}
}

// TOS Tholian Web Breached Release
function playWebBreached(ctx = getAudioContext(), startTime) {
    if (!ctx) return;
    try {
        const now = startTime !== undefined ? startTime : ctx.currentTime;
        const duration = 0.32;
        const scale = volumeScale();

        const osc = ctx.createOscillator();
        const gain = ctx.createGain();

        osc.type = 'sine';
        osc.frequency.setValueAtTime(1760, now);
        osc.frequency.exponentialRampToValueAtTime(550, now + duration);

        gain.gain.setValueAtTime(0.0001, now);
        gain.gain.linearRampToValueAtTime(0.075 * scale, now + 0.015);
        gain.gain.exponentialRampToValueAtTime(0.0001, now + duration);

        osc.connect(gain);
        gain.connect(ctx.destination);

        osc.start(now);
        osc.stop(now + duration + 0.04);
    } catch (e) {}
}

// TOS Phaser Point Defense Micro-Pulses
function playPointDefense(ctx = getAudioContext(), startTime) {
    if (!ctx) return;
    try {
        const now = startTime !== undefined ? startTime : ctx.currentTime;
        const duration = 0.12;
        const scale = volumeScale();

        const osc = ctx.createOscillator();
        const gain = ctx.createGain();

        osc.type = 'sine';
        osc.frequency.setValueAtTime(1480, now);
        osc.frequency.exponentialRampToValueAtTime(1180, now + duration);

        gain.gain.setValueAtTime(0.0001, now);
        gain.gain.linearRampToValueAtTime(0.065 * scale, now + 0.01);
        gain.gain.exponentialRampToValueAtTime(0.0001, now + duration);

        osc.connect(gain);
        gain.connect(ctx.destination);

        osc.start(now);
        osc.stop(now + duration + 0.03);
    } catch (e) {}
}

// TOS 3-Note Bridge Intercom Bosun Whistle (E6 -> A5 -> E6)
function playCommChime(ctx = getAudioContext(), startTime) {
    if (!ctx) return;
    try {
        const now = startTime !== undefined ? startTime : ctx.currentTime;
        const scale = volumeScale();
        const notes = [
            { freq: 1318.51, offset: 0.0, dur: 0.095 },
            { freq: 880.0, offset: 0.095, dur: 0.105 },
            { freq: 1318.51, offset: 0.20, dur: 0.22 },
        ];

        notes.forEach((n) => {
            const t0 = now + n.offset;
            const osc = ctx.createOscillator();
            const gain = ctx.createGain();

            osc.type = 'sine';
            osc.frequency.setValueAtTime(n.freq, t0);

            gain.gain.setValueAtTime(0.0001, now);
            gain.gain.setValueAtTime(0.0001, t0);
            gain.gain.linearRampToValueAtTime(0.075 * scale, t0 + 0.01);
            gain.gain.exponentialRampToValueAtTime(0.0001, t0 + n.dur);

            osc.connect(gain);
            gain.connect(ctx.destination);

            osc.start(t0);
            osc.stop(t0 + n.dur + 0.02);
        });
    } catch (e) {}
}

// TOS Library Computer Dual-Tone Blip (1320 Hz -> 1760 Hz)
function playComputerBeep(ctx = getAudioContext(), startTime) {
    if (!ctx) return;
    try {
        const now = startTime !== undefined ? startTime : ctx.currentTime;
        const scale = volumeScale();
        const notes = [
            { freq: 1320.0, offset: 0.0, dur: 0.042 },
            { freq: 1760.0, offset: 0.042, dur: 0.048 },
        ];

        notes.forEach((n) => {
            const t0 = now + n.offset;
            const osc = ctx.createOscillator();
            const gain = ctx.createGain();

            osc.type = 'sine';
            osc.frequency.setValueAtTime(n.freq, t0);

            gain.gain.setValueAtTime(0.0001, now);
            gain.gain.setValueAtTime(0.0001, t0);
            gain.gain.linearRampToValueAtTime(0.055 * scale, t0 + 0.006);
            gain.gain.exponentialRampToValueAtTime(0.0001, t0 + n.dur);

            osc.connect(gain);
            gain.connect(ctx.destination);

            osc.start(t0);
            osc.stop(t0 + n.dur + 0.02);
        });
    } catch (e) {}
}

let lastScheduledSound = '';
let lastScheduledTime = 0;
let nextScheduleSlot = 0;

const primaryActionSounds = new Set([
    'warp',
    'phaser',
    'torpedo_launch',
    'shields',
    'dock',
    'comm_chime',
    'computer_beep',
]);

function soundStaggerStep(soundID) {
    switch (soundID) {
        case 'phaser':
            return 0.70; // 650ms beam + 50ms separation gap
        case 'torpedo_launch':
            return 0.77; // 720ms torpedo + 50ms separation gap
        case 'dock':
            return 0.80; // 750ms dock + 50ms separation gap
        case 'explosion':
        case 'plasma_impact':
        case 'red_alert':
            return 0.56;
        case 'damage':
            return 0.44;
        case 'warp':
        case 'shields':
            return 0.28;
        default:
            return 0.35;
    }
}

// Central sound dispatcher
function sstPlaySound(soundID) {
    if (isMuted || currentVolume <= 0) {
        return;
    }
    const ctx = getAudioContext();
    if (!ctx) {
        return;
    }
    const now = ctx.currentTime || 0;
    if (soundID === lastScheduledSound && (now - lastScheduledTime) < 0.08) {
        return;
    }
    lastScheduledSound = soundID;
    lastScheduledTime = now;

    const step = soundStaggerStep(soundID);
    let startAt = now;
    if (primaryActionSounds.has(soundID)) {
        startAt = now;
        nextScheduleSlot = now + step;
    } else if (now < nextScheduleSlot) {
        startAt = Math.min(nextScheduleSlot, now + 2.50);
        nextScheduleSlot = startAt + step;
    } else {
        nextScheduleSlot = now + step;
    }

    switch (soundID) {
        case 'phaser':
            playPhaser(ctx, startAt);
            break;
        case 'torpedo_launch':
            playTorpedoLaunch(ctx, startAt);
            break;
        case 'explosion':
            playExplosion(ctx, startAt);
            break;
        case 'red_alert':
            playRedAlert(ctx, startAt);
            break;
        case 'dock':
            playDock(ctx, startAt);
            break;
        case 'warp':
            playWarp(ctx, startAt);
            break;
        case 'damage':
            playDamage(ctx, startAt);
            break;
        case 'shields':
            playShields(ctx, startAt);
            break;
        case 'victory':
            playVictory(ctx, startAt);
            break;
        case 'defeat':
            playDefeat(ctx, startAt);
            break;
        case 'cloak':
            playCloak(ctx, startAt);
            break;
        case 'decloak':
            playDecloak(ctx, startAt);
            break;
        case 'plasma_launch':
            playPlasmaLaunch(ctx, startAt);
            break;
        case 'plasma_impact':
            playPlasmaImpact(ctx, startAt);
            break;
        case 'tholian_web':
            playTholianWeb(ctx, startAt);
            break;
        case 'web_breached':
            playWebBreached(ctx, startAt);
            break;
        case 'point_defense':
            playPointDefense(ctx, startAt);
            break;
        case 'comm_chime':
            playCommChime(ctx, startAt);
            break;
        case 'computer_beep':
            playComputerBeep(ctx, startAt);
            break;
    }
}

// Attach to window object for WASM bridge and UI
if (typeof window !== 'undefined') {
    window.sstPlaySound = sstPlaySound;
    window.sstSetMuted = sstSetMuted;
    window.sstIsMuted = sstIsMuted;
    window.sstSetVolume = sstSetVolume;
    window.sstGetVolume = sstGetVolume;

    window.playPhaser = playPhaser;
    window.playTorpedoLaunch = playTorpedoLaunch;
    window.playExplosion = playExplosion;
    window.playRedAlert = playRedAlert;
    window.playDock = playDock;
    window.playWarp = playWarp;
    window.playDamage = playDamage;
    window.playShields = playShields;
    window.playVictory = playVictory;
    window.playDefeat = playDefeat;
    window.playCloak = playCloak;
    window.playDecloak = playDecloak;
    window.playPlasmaLaunch = playPlasmaLaunch;
    window.playPlasmaImpact = playPlasmaImpact;
    window.playTholianWeb = playTholianWeb;
    window.playWebBreached = playWebBreached;
    window.playPointDefense = playPointDefense;
    window.playCommChime = playCommChime;
    window.playComputerBeep = playComputerBeep;
}

if (typeof globalThis !== 'undefined' && typeof window === 'undefined') {
    globalThis.sstPlaySound = sstPlaySound;
    globalThis.sstSetMuted = sstSetMuted;
    globalThis.sstIsMuted = sstIsMuted;
    globalThis.sstSetVolume = sstSetVolume;
    globalThis.sstGetVolume = sstGetVolume;
}

if (typeof module !== 'undefined' && module.exports) {
    module.exports = {
        sstPlaySound,
        sstSetMuted,
        sstIsMuted,
        sstSetVolume,
        sstGetVolume,
        playPhaser,
        playTorpedoLaunch,
        playExplosion,
        playRedAlert,
        playDock,
        playWarp,
        playDamage,
        playShields,
        playVictory,
        playDefeat,
        playCloak,
        playDecloak,
        playPlasmaLaunch,
        playPlasmaImpact,
        playTholianWeb,
        playWebBreached,
        playPointDefense,
        playCommChime,
        playComputerBeep,
    };
}

