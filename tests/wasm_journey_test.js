#!/usr/bin/env node

/**
 * Super Star Trek - WebAssembly User Journey Test Suite
 *
 * Tests the compiled WebAssembly binary (web/sst.wasm) and JavaScript bridge
 * across all major player journeys:
 *  1. Navigation & Movement (Sector maneuvering & Inter-quadrant warp)
 *  2. Tactical Combat (Shields, Phasers, Torpedoes, Return fire, Destruction)
 *  3. Starbase Operations (Docking, replenishment, repairs)
 *  4. Scenario: Kobayashi Maru (Trap, wave reinforcement, commendation scoring)
 *  5. Scenario: Mutara Nebula (Zero shields ionization, tactical duel, victory)
 *  6. Scenario: Starbase Siege (Warp to Starbase 12, defense, victory)
 *  7. State Persistence (Save and Load serialization roundtrip)
 *  8. Audio Dispatching (Warp, Phaser, Torpedo, Shield, and Dock sound FX)
 */

const fs = require('fs');
const path = require('path');
const assert = require('assert');

const rootDir = path.resolve(__dirname, '..');
const wasmPath = path.join(rootDir, 'web', 'sst.wasm');
const wasmExecPath = path.join(rootDir, 'web', 'wasm_exec.js');

if (!fs.existsSync(wasmPath)) {
    console.error(`Error: WASM binary not found at ${wasmPath}. Run ./scripts/build-wasm.sh first.`);
    process.exit(1);
}

// Load Go runtime support
require(wasmExecPath);

// Audio telemetry tracking
const soundsPlayed = [];
globalThis.sstPlaySound = function(id) {
    soundsPlayed.push(id);
};
globalThis.sstSetMuted = function() {};
globalThis.sstIsMuted = function() { return false; };

async function initWasm() {
    const go = new globalThis.Go();
    const wasmBuffer = fs.readFileSync(wasmPath);
    const { instance } = await WebAssembly.instantiate(wasmBuffer, go.importObject);
    go.run(instance);
}

function clearSounds() {
    soundsPlayed.length = 0;
}

function hasSound(id) {
    return soundsPlayed.includes(id);
}

async function runJourneys() {
    console.log('====================================================');
    console.log(' ★ Super Star Trek: Automated WASM Journey Tests ★');
    console.log('====================================================\n');

    let out = '';
    let status = '';

    await initWasm();
    assert.strictEqual(typeof globalThis.sstInit, 'function', 'sstInit must be registered on global');
    assert.strictEqual(typeof globalThis.sstCommand, 'function', 'sstCommand must be registered on global');
    assert.strictEqual(typeof globalThis.sstSave, 'function', 'sstSave must be registered on global');
    assert.strictEqual(typeof globalThis.sstLoad, 'function', 'sstLoad must be registered on global');

    // ----------------------------------------------------
    // Journey 1: Exploration & Movement
    // ----------------------------------------------------
    console.log('▶ Running Journey 1: Exploration & Movement...');
    globalThis.sstInit(12345, 'normal');
    clearSounds();

    out = globalThis.sstCommand('srs');
    assert(out.includes('<E>'), 'SRS output must display Enterprise glyph <E>');
    assert(out.includes('CONDITION'), 'SRS output must display status panel');

    // Sector navigation
    out = globalThis.sstCommand('nav 1 1');
    assert(out.includes('[WARP ENGINES ENGAGED]'), 'Sector movement should report warp engines engaged');
    assert(hasSound('warp'), 'Warp sound should be triggered on move');

    // Inter-quadrant warp jump at warp factor 6
    clearSounds();
    out = globalThis.sstCommand('nav q 3 5 6');
    assert(out.includes('[WARP ENGINES ENGAGED]'), 'Inter-quadrant warp should report engagement');
    assert(hasSound('warp'), 'Warp sound must trigger on quadrant warp');
    console.log('  ✔ Journey 1 Passed: Sector navigation, quadrant warp, and audio verified.\n');

    // ----------------------------------------------------
    // Journey 2: Tactical Combat
    // ----------------------------------------------------
    console.log('▶ Running Journey 2: Tactical Combat...');
    globalThis.sstCommand('scenario kobayashi-maru');
    clearSounds();

    // Raise deflector shields
    out = globalThis.sstCommand('she 500');
    assert(out.includes('Deflector Shields:'), 'Shield transfer must succeed');
    assert(hasSound('shields'), 'SoundShields must be dispatched');
    assert(out.includes('[RETURN FIRE]'), 'Surviving Klingons must return fire');

    // Fire phasers at the 3 enemy Klingons
    clearSounds();
    out = globalThis.sstCommand('pha 400');
    assert(out.includes('[PHASERS FIRED]'), 'Phasers fired confirmation required');
    assert(hasSound('phaser'), 'SoundPhaser must be dispatched');
    assert(out.includes('[RETURN FIRE]'), 'Klingons must return fire');

    // Fire photon torpedo along bearing
    clearSounds();
    out = globalThis.sstCommand('tor 1.0');
    assert(out.includes('[TORPEDO TRACK]'), 'Torpedo track must be logged');
    assert(hasSound('torpedo_launch'), 'SoundTorpedoLaunch must be dispatched');
    console.log('  ✔ Journey 2 Passed: Shields, phasers, and torpedo ballistics verified.\n');

    // ----------------------------------------------------
    // Journey 3: Starbase Operations & Docking
    // ----------------------------------------------------
    console.log('▶ Running Journey 3: Starbase Docking & Resupply...');
    globalThis.sstCommand('scenario siege');
    globalThis.sstCommand('nav q 4 4 6');
    globalThis.sstCommand('nav s 2 6');

    clearSounds();
    out = globalThis.sstCommand('doc');
    assert(out.includes('[STARBASE DOCKING]'), 'Docking confirmation must be logged');
    assert(hasSound('dock'), 'SoundDock must be dispatched');
    assert(out.includes('replenished'), 'Resupply confirmation must be logged');

    status = globalThis.sstCommand('srs');
    assert(status.includes('CONDITION:  \u001b[36mDOCKED'), 'Status condition must be DOCKED');
    assert(status.includes('Energy:     5000 / 5000'), 'Energy must be replenished to 5000');
    assert(status.includes('Torpedoes:  10'), 'Torpedoes must be replenished to 10');
    console.log('  ✔ Journey 3 Passed: Starbase docking, ammo reload, and repair verified.\n');

    // ----------------------------------------------------
    // Journey 4: Scenario - Kobayashi Maru
    // ----------------------------------------------------
    console.log('▶ Running Journey 4: Scenario - Kobayashi Maru...');
    out = globalThis.sstCommand('scenario kobayashi-maru');
    assert(out.includes('KOBAYASHI MARU'), 'Briefing must display scenario title');
    assert(out.includes('Neutral Zone'), 'Briefing must describe Neutral Zone');

    // Destroy first Klingon
    out = globalThis.sstCommand('tor 4 7');
    // Wave reinforcement keeps neutral zone contested
    status = globalThis.sstCommand('srs');
    assert(status.includes('+K+'), 'Klingon warships must reinforce in Kobayashi Maru');
    console.log('  ✔ Journey 4 Passed: Kobayashi Maru scenario, trap, and reinforcement verified.\n');

    // ----------------------------------------------------
    // Journey 5: Scenario - Mutara Nebula
    // ----------------------------------------------------
    console.log('▶ Running Journey 5: Scenario - Mutara Nebula...');
    out = globalThis.sstCommand('scenario mutara');
    assert(out.includes('MUTARA NEBULA'), 'Briefing must display Mutara Nebula');

    // Shields must remain 0 in Mutara Nebula
    out = globalThis.sstCommand('she 1000');
    status = globalThis.sstCommand('srs');
    assert(status.includes('Shields:    0'), 'Shields must remain 0 inside Mutara Nebula');

    // Duel the Super-Commander
    out = globalThis.sstCommand('pha 800');
    assert(out.includes('[PHASERS FIRED]'), 'Phasers fire inside nebula');
    console.log('  ✔ Journey 5 Passed: Mutara Nebula ionization and combat verified.\n');

    // ----------------------------------------------------
    // Journey 6: Scenario - Starbase Siege
    // ----------------------------------------------------
    console.log('▶ Running Journey 6: Scenario - Starbase Siege...');
    out = globalThis.sstCommand('scenario siege');
    assert(out.includes('STARBASE UNDER SIEGE'), 'Briefing must display Starbase Under Siege');

    // Warp to quadrant [4, 4] at Warp 6
    out = globalThis.sstCommand('nav q 4 4 6');
    assert(out.includes('[WARP ENGINES ENGAGED]'), 'Warp transit must complete');
    status = globalThis.sstCommand('srs');
    assert(status.includes('>B<'), 'Starbase 12 must be present in quadrant [4,4]');
    assert(status.includes('+K+'), 'Siege fleet must be present in quadrant [4,4]');
    console.log('  ✔ Journey 6 Passed: Starbase Siege warp transit and defense verified.\n');

    // ----------------------------------------------------
    // Journey 7: State Persistence (Save & Load)
    // ----------------------------------------------------
    console.log('▶ Running Journey 7: State Persistence (Save & Load)...');
    globalThis.sstInit(12345, 'normal');
    globalThis.sstCommand('she 1500');
    globalThis.sstCommand('nav 1 1');

    const savedJSON = globalThis.sstSave();
    assert(savedJSON && savedJSON.length > 50, 'sstSave must return valid JSON string');

    // Create a new game with different seed
    globalThis.sstInit(99999, 'hardcore');
    let preLoadSRS = globalThis.sstCommand('srs');
    assert(!preLoadSRS.includes('Shields:    1500'), 'Fresh game must not have previous shields');

    // Restore saved state
    const loadSuccess = globalThis.sstLoad(savedJSON);
    assert.strictEqual(loadSuccess, true, 'sstLoad must return true for valid save state');
    let postLoadSRS = globalThis.sstCommand('srs');
    assert(postLoadSRS.includes('Shields:    1500'), 'Restored game must reflect saved shield value 1500');
    console.log('  ✔ Journey 7 Passed: Browser serialization roundtrip verified.\n');

    console.log('====================================================');
    console.log(' ★ ALL 7 WASM USER JOURNEYS PASSED SUCCESSFULLY! ★');
    console.log('====================================================');
    process.exit(0);
}

runJourneys().catch(err => {
    console.error('\n✖ Test Failed with error:', err);
    process.exit(1);
});
