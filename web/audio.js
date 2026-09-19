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

// Mute state management with localStorage persistence
let isMuted = false;
try {
    if (typeof localStorage !== 'undefined') {
        const saved = localStorage.getItem('sst_sound_muted');
        if (saved !== null) {
            isMuted = (saved === 'true');
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
 * Procedural Synthesizers
 */

// Rapid downward frequency sweep (1200 Hz -> 200 Hz)
function playPhaser(ctx = getAudioContext()) {
    if (!ctx) return;
    try {
        const now = ctx.currentTime;
        const duration = 0.25;

        const osc = ctx.createOscillator();
        const gain = ctx.createGain();

        osc.type = 'sawtooth';
        osc.frequency.setValueAtTime(1200, now);
        osc.frequency.exponentialRampToValueAtTime(200, now + duration);

        gain.gain.setValueAtTime(0.001, now);
        gain.gain.linearRampToValueAtTime(0.25, now + 0.01);
        gain.gain.exponentialRampToValueAtTime(0.001, now + duration);

        osc.connect(gain);
        gain.connect(ctx.destination);

        osc.start(now);
        osc.stop(now + duration + 0.05);
    } catch (e) {}
}

// Rising whistling chirp (500 Hz -> 1200 Hz)
function playTorpedoLaunch(ctx = getAudioContext()) {
    if (!ctx) return;
    try {
        const now = ctx.currentTime;
        const duration = 0.20;

        const osc = ctx.createOscillator();
        const gain = ctx.createGain();

        osc.type = 'triangle';
        osc.frequency.setValueAtTime(500, now);
        osc.frequency.exponentialRampToValueAtTime(1200, now + duration);

        gain.gain.setValueAtTime(0.001, now);
        gain.gain.linearRampToValueAtTime(0.3, now + 0.01);
        gain.gain.exponentialRampToValueAtTime(0.001, now + duration);

        osc.connect(gain);
        gain.connect(ctx.destination);

        osc.start(now);
        osc.stop(now + duration + 0.05);
    } catch (e) {}
}

// White noise burst through lowpass resonant filter with decay (600 ms)
function playExplosion(ctx = getAudioContext()) {
    if (!ctx) return;
    try {
        const now = ctx.currentTime;
        const duration = 0.60;

        const noise = ctx.createBufferSource();
        noise.buffer = createNoiseBuffer(ctx, duration);

        const filter = ctx.createBiquadFilter();
        filter.type = 'lowpass';
        filter.frequency.setValueAtTime(800, now);
        filter.frequency.exponentialRampToValueAtTime(60, now + duration);
        filter.Q.setValueAtTime(3.0, now);

        const gain = ctx.createGain();
        gain.gain.setValueAtTime(0.4, now);
        gain.gain.exponentialRampToValueAtTime(0.001, now + duration);

        noise.connect(filter);
        filter.connect(gain);
        gain.connect(ctx.destination);

        noise.start(now);
        noise.stop(now + duration + 0.05);
    } catch (e) {}
}

// Two-tone pulsing warble / siren (600 ms)
function playRedAlert(ctx = getAudioContext()) {
    if (!ctx) return;
    try {
        const now = ctx.currentTime;
        const duration = 0.60;

        const osc = ctx.createOscillator();
        const lfo = ctx.createOscillator();
        const lfoGain = ctx.createGain();
        const gain = ctx.createGain();

        osc.type = 'sine';
        osc.frequency.setValueAtTime(700, now);

        lfo.type = 'sine';
        lfo.frequency.setValueAtTime(4, now); // 4 Hz modulation rate
        lfoGain.gain.setValueAtTime(150, now); // +/- 150 Hz deviation

        lfo.connect(lfoGain);
        lfoGain.connect(osc.frequency);

        gain.gain.setValueAtTime(0.001, now);
        gain.gain.linearRampToValueAtTime(0.25, now + 0.02);
        gain.gain.setValueAtTime(0.25, now + duration - 0.05);
        gain.gain.exponentialRampToValueAtTime(0.001, now + duration);

        osc.connect(gain);
        gain.connect(ctx.destination);

        lfo.start(now);
        osc.start(now);
        lfo.stop(now + duration + 0.05);
        osc.stop(now + duration + 0.05);
    } catch (e) {}
}

// Ascending major triad chime (C5, E5, G5 - 600 ms)
function playDock(ctx = getAudioContext()) {
    if (!ctx) return;
    try {
        const now = ctx.currentTime;
        const notes = [523.25, 659.25, 783.99]; // C5, E5, G5
        const noteDuration = 0.18;

        notes.forEach((freq, idx) => {
            const t0 = now + idx * noteDuration;
            const osc = ctx.createOscillator();
            const gain = ctx.createGain();

            osc.type = 'sine';
            osc.frequency.setValueAtTime(freq, t0);

            gain.gain.setValueAtTime(0.001, now);
            gain.gain.setValueAtTime(0.001, t0);
            gain.gain.linearRampToValueAtTime(0.25, t0 + 0.01);
            gain.gain.exponentialRampToValueAtTime(0.001, t0 + noteDuration + 0.05);

            osc.connect(gain);
            gain.connect(ctx.destination);

            osc.start(t0);
            osc.stop(t0 + noteDuration + 0.06);
        });
    } catch (e) {}
}

// Low-frequency accelerating drone (55 Hz -> 165 Hz over 700 ms)
function playWarp(ctx = getAudioContext()) {
    if (!ctx) return;
    try {
        const now = ctx.currentTime;
        const duration = 0.70;

        const osc = ctx.createOscillator();
        const filter = ctx.createBiquadFilter();
        const gain = ctx.createGain();

        osc.type = 'sawtooth';
        osc.frequency.setValueAtTime(55, now);
        osc.frequency.exponentialRampToValueAtTime(165, now + duration);

        filter.type = 'lowpass';
        filter.frequency.setValueAtTime(350, now);

        gain.gain.setValueAtTime(0.001, now);
        gain.gain.linearRampToValueAtTime(0.25, now + 0.05);
        gain.gain.setValueAtTime(0.25, now + duration - 0.05);
        gain.gain.exponentialRampToValueAtTime(0.001, now + duration);

        osc.connect(filter);
        filter.connect(gain);
        gain.connect(ctx.destination);

        osc.start(now);
        osc.stop(now + duration + 0.05);
    } catch (e) {}
}

// Harsh crunchy impact noise with low-frequency square wave (350 ms)
function playDamage(ctx = getAudioContext()) {
    if (!ctx) return;
    try {
        const now = ctx.currentTime;
        const duration = 0.35;

        const noise = ctx.createBufferSource();
        noise.buffer = createNoiseBuffer(ctx, duration);

        const osc = ctx.createOscillator();
        osc.type = 'square';
        osc.frequency.setValueAtTime(110, now);

        const filter = ctx.createBiquadFilter();
        filter.type = 'lowpass';
        filter.frequency.setValueAtTime(600, now);

        const noiseGain = ctx.createGain();
        noiseGain.gain.setValueAtTime(0.3, now);

        const oscGain = ctx.createGain();
        oscGain.gain.setValueAtTime(0.2, now);

        const masterGain = ctx.createGain();
        masterGain.gain.setValueAtTime(0.4, now);
        masterGain.gain.exponentialRampToValueAtTime(0.001, now + duration);

        noise.connect(noiseGain);
        noiseGain.connect(filter);
        osc.connect(oscGain);
        oscGain.connect(filter);

        filter.connect(masterGain);
        masterGain.connect(ctx.destination);

        noise.start(now);
        osc.start(now);
        noise.stop(now + duration + 0.05);
        osc.stop(now + duration + 0.05);
    } catch (e) {}
}

// Resonant flare shimmer with frequency modulation (400 ms)
function playShields(ctx = getAudioContext()) {
    if (!ctx) return;
    try {
        const now = ctx.currentTime;
        const duration = 0.40;

        const osc = ctx.createOscillator();
        const lfo = ctx.createOscillator();
        const lfoGain = ctx.createGain();
        const gain = ctx.createGain();

        osc.type = 'sine';
        osc.frequency.setValueAtTime(440, now);

        lfo.type = 'sine';
        lfo.frequency.setValueAtTime(25, now); // 25 Hz shimmer rate
        lfoGain.gain.setValueAtTime(60, now);

        lfo.connect(lfoGain);
        lfoGain.connect(osc.frequency);

        gain.gain.setValueAtTime(0.001, now);
        gain.gain.linearRampToValueAtTime(0.25, now + 0.04);
        gain.gain.exponentialRampToValueAtTime(0.001, now + duration);

        osc.connect(gain);
        gain.connect(ctx.destination);

        lfo.start(now);
        osc.start(now);
        lfo.stop(now + duration + 0.05);
        osc.stop(now + duration + 0.05);
    } catch (e) {}
}

// Ascending 4-note victory fanfare (C5, E5, G5, C6 - 600 ms)
function playVictory(ctx = getAudioContext()) {
    if (!ctx) return;
    try {
        const now = ctx.currentTime;
        const notes = [523.25, 659.25, 783.99, 1046.50]; // C5, E5, G5, C6
        const noteDuration = 0.15;

        notes.forEach((freq, idx) => {
            const t0 = now + idx * noteDuration;
            const osc = ctx.createOscillator();
            const gain = ctx.createGain();

            osc.type = 'triangle';
            osc.frequency.setValueAtTime(freq, t0);

            gain.gain.setValueAtTime(0.001, now);
            gain.gain.setValueAtTime(0.001, t0);
            gain.gain.linearRampToValueAtTime(0.28, t0 + 0.01);
            gain.gain.exponentialRampToValueAtTime(0.001, t0 + noteDuration + 0.04);

            osc.connect(gain);
            gain.connect(ctx.destination);

            osc.start(t0);
            osc.stop(t0 + noteDuration + 0.05);
        });
    } catch (e) {}
}

// Descending mournful slide (420 Hz -> 110 Hz over 700 ms)
function playDefeat(ctx = getAudioContext()) {
    if (!ctx) return;
    try {
        const now = ctx.currentTime;
        const duration = 0.70;

        const osc = ctx.createOscillator();
        const gain = ctx.createGain();

        osc.type = 'sawtooth';
        osc.frequency.setValueAtTime(420, now);
        osc.frequency.exponentialRampToValueAtTime(110, now + duration);

        gain.gain.setValueAtTime(0.001, now);
        gain.gain.linearRampToValueAtTime(0.25, now + 0.02);
        gain.gain.exponentialRampToValueAtTime(0.001, now + duration);

        osc.connect(gain);
        gain.connect(ctx.destination);

        osc.start(now);
        osc.stop(now + duration + 0.05);
    } catch (e) {}
}

// Central sound dispatcher
function sstPlaySound(soundID) {
    if (isMuted) {
        return;
    }
    const ctx = getAudioContext();
    if (!ctx) {
        return;
    }
    switch (soundID) {
        case 'phaser':
            playPhaser(ctx);
            break;
        case 'torpedo_launch':
            playTorpedoLaunch(ctx);
            break;
        case 'explosion':
            playExplosion(ctx);
            break;
        case 'red_alert':
            playRedAlert(ctx);
            break;
        case 'dock':
            playDock(ctx);
            break;
        case 'warp':
            playWarp(ctx);
            break;
        case 'damage':
            playDamage(ctx);
            break;
        case 'shields':
            playShields(ctx);
            break;
        case 'victory':
            playVictory(ctx);
            break;
        case 'defeat':
            playDefeat(ctx);
            break;
    }
}

// Attach to window object for WASM bridge and UI
if (typeof window !== 'undefined') {
    window.sstPlaySound = sstPlaySound;
    window.sstSetMuted = sstSetMuted;
    window.sstIsMuted = sstIsMuted;

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
}

if (typeof globalThis !== 'undefined' && typeof window === 'undefined') {
    globalThis.sstPlaySound = sstPlaySound;
    globalThis.sstSetMuted = sstSetMuted;
    globalThis.sstIsMuted = sstIsMuted;
}

if (typeof module !== 'undefined' && module.exports) {
    module.exports = {
        sstPlaySound,
        sstSetMuted,
        sstIsMuted,
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
    };
}
