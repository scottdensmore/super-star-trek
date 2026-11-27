#!/usr/bin/env node

/**
 * Replays the Go reference sessions (testdata/parity/wasm/sessions.txt,
 * recorded by cmd/wasm/zz_paritygen_test.go) through the built WebAssembly
 * module's JS globals and compares every output byte-for-byte. This checks
 * the real wasm32 build, not just the native `cargo test` build.
 */

const fs = require('fs');
const path = require('path');
const vm = require('vm');

const rootDir = path.resolve(__dirname, '..');
const pkgDir = process.env.SST_WASM_PKG_DIR || path.join(rootDir, 'web', 'pkg');
const wasmPath = path.join(pkgDir, 'sst_wasm_bg.wasm');
const gluePath = path.join(pkgDir, 'sst_wasm.js');
const dataPath = path.join(rootDir, 'testdata', 'parity', 'wasm', 'sessions.txt');

vm.runInThisContext(fs.readFileSync(gluePath, 'utf8'), { filename: gluePath });
const wasmBindgen = vm.runInThisContext('wasm_bindgen');

function parse(buf) {
    const items = [];
    let pos = 0;
    while (pos < buf.length) {
        const nl = buf.indexOf(0x0a, pos);
        const header = buf.toString('utf8', pos, nl).split(' ');
        pos = nl + 1;
        if (header[0] === 'CASE') {
            items.push({ kind: 'case', name: header[1], reseed: header[2] === '1' });
        } else if (header[0] === 'END') {
            items.push({ kind: 'end' });
        } else {
            const alen = Number(header[1]);
            const olen = Number(header[2]);
            const arg = buf.toString('utf8', pos, pos + alen);
            const out = buf.toString('utf8', pos + alen, pos + alen + olen);
            pos += alen + olen + 1;
            items.push({ kind: 'rec', op: header[0], arg, out });
        }
    }
    return items;
}

async function main() {
    await wasmBindgen({ module_or_path: fs.readFileSync(wasmPath) });
    const items = parse(fs.readFileSync(dataPath));
    const RNG_NULL = '{"RNG":null,';
    let cases = 0, steps = 0, bytes = 0;
    let name = '', skip = false, rawGo = false, lastSave = '';
    for (const it of items) {
        if (it.kind === 'case') {
            name = it.name; skip = false; rawGo = !it.reseed; lastSave = ''; cases++;
            continue;
        }
        if (it.kind === 'end' || skip) continue;
        let got;
        switch (it.op) {
            case 'new': {
                const i = it.arg.indexOf(' ');
                got = globalThis.sstInit(Number(it.arg.slice(0, i)), it.arg.slice(i + 1));
                break;
            }
            case 'reset': got = globalThis.sstReset(); break;
            case 'cmd': got = globalThis.sstCommand(it.arg); break;
            case 'panic':
                // Go crashed here; the Rust build reseeds after Load and must not.
                globalThis.sstCommand(it.arg);
                skip = true;
                continue;
            case 'save': got = globalThis.sstSave(); lastSave = got; break;
            case 'load':
                got = globalThis.sstLoad(it.arg === '@last' ? lastSave : it.arg) ? 'ok' : 'err';
                break;
            default: throw new Error('unknown op ' + it.op);
        }
        let want = it.out;
        if (rawGo && it.op === 'save' && want.startsWith(RNG_NULL)) {
            want = '{"RNG":{},' + want.slice(RNG_NULL.length);
        }
        if (got !== want) {
            console.error(`✖ Mismatch in case ${name}, ${it.op} ${JSON.stringify(it.arg).slice(0, 80)}`);
            console.error('--- go ---\n' + JSON.stringify(want) + '\n--- wasm ---\n' + JSON.stringify(got));
            process.exit(1);
        }
        steps++;
        bytes += Buffer.byteLength(want);
    }
    console.log(`✔ WASM parity replay: ${cases} cases, ${steps} steps, ${bytes} bytes identical to the reference transcripts.`);
}

main().catch(err => {
    console.error('✖ Parity replay failed:', err);
    process.exit(1);
});
