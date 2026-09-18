const term = new Terminal({
    cursorBlink: true,
    fontFamily: '"Courier New", Courier, monospace',
    fontSize: 14,
    theme: {
        background: '#05070a',
        foreground: '#66fcf1',
        cursor: '#66fcf1'
    }
});

const fitAddon = new FitAddon.FitAddon();
term.loadAddon(fitAddon);

const container = document.getElementById('terminal-container');
term.open(container);
fitAddon.fit();
window.addEventListener('resize', () => fitAddon.fit());

let lineBuffer = '';
const history = [];
let historyIndex = -1;

function prompt() {
    term.write('\x1b[36mCommand?\x1b[0m ');
}

term.onData(e => {
    switch (e) {
        case '\r': // Enter
            term.write('\r\n');
            const trimmed = lineBuffer.trim();
            if (trimmed.length > 0) {
                history.push(lineBuffer);
                historyIndex = history.length;

                const tokens = trimmed.split(/\s+/);
                const cmd = tokens[0].toLowerCase();
                const slot = tokens[1] || '1';

                if (cmd === 'save') {
                    const state = typeof window.sstSave === 'function' ? window.sstSave() : null;
                    if (!state) {
                        term.write('\x1b[31m[SYSTEM] Error: unable to save game state.\x1b[0m\r\n');
                    } else {
                        try {
                            localStorage.setItem('sst_slot_' + slot, state);
                            term.write(`\x1b[32m[SYSTEM] Game state saved to browser storage (Slot: ${slot})\x1b[0m\r\n`);
                        } catch (err) {
                            term.write('\x1b[31m[SYSTEM] Error: unable to save game state.\x1b[0m\r\n');
                        }
                    }
                } else if (cmd === 'load') {
                    let saved = null;
                    try {
                        saved = localStorage.getItem('sst_slot_' + slot);
                    } catch (err) {
                        saved = null;
                    }
                    if (!saved) {
                        term.write(`\x1b[31m[SYSTEM] No saved game found in Slot: ${slot}\x1b[0m\r\n`);
                    } else {
                        const ok = typeof window.sstLoad === 'function' ? window.sstLoad(saved) : false;
                        if (ok) {
                            term.write(`\x1b[32m[SYSTEM] Game restored from browser storage (Slot: ${slot})\x1b[0m\r\n`);
                            if (typeof window.sstCommand === 'function') {
                                term.write(window.sstCommand('srs'));
                            }
                        } else {
                            term.write('\x1b[31m[SYSTEM] Error: saved game corrupted or incompatible.\x1b[0m\r\n');
                        }
                    }
                } else if (typeof window.sstCommand === 'function') {
                    const output = window.sstCommand(lineBuffer);
                    term.write(output);
                    if (output && output.includes('Session terminated')) {
                        term.write('\x1b[33m[SYSTEM] Session terminated. Click "Restart Game" or type "srs" to resume.\x1b[0m\r\n');
                    }
                }
            }
            lineBuffer = '';
            prompt();
            break;
        case '\u007F': // Backspace
            if (lineBuffer.length > 0) {
                lineBuffer = lineBuffer.slice(0, -1);
                term.write('\b \b');
            }
            break;
        case '\u0003': // Ctrl+C
            term.write('^C\r\n');
            lineBuffer = '';
            prompt();
            break;
        case '\u001b[A': // Up arrow
            if (historyIndex > 0) {
                historyIndex--;
                while (lineBuffer.length > 0) {
                    term.write('\b \b');
                    lineBuffer = lineBuffer.slice(0, -1);
                }
                lineBuffer = history[historyIndex];
                term.write(lineBuffer);
            }
            break;
        case '\u001b[B': // Down arrow
            if (historyIndex < history.length - 1) {
                historyIndex++;
                while (lineBuffer.length > 0) {
                    term.write('\b \b');
                    lineBuffer = lineBuffer.slice(0, -1);
                }
                lineBuffer = history[historyIndex];
                term.write(lineBuffer);
            } else if (historyIndex === history.length - 1) {
                historyIndex = history.length;
                while (lineBuffer.length > 0) {
                    term.write('\b \b');
                    lineBuffer = lineBuffer.slice(0, -1);
                }
            }
            break;
        default:
            if (e >= ' ' && e <= '~') {
                lineBuffer += e;
                term.write(e);
            }
            break;
    }
});

// Color palettes
document.getElementById('palette-select').addEventListener('change', (e) => {
    const val = e.target.value;
    if (val === 'green') {
        term.options.theme = { background: '#020d02', foreground: '#33ff33', cursor: '#33ff33' };
    } else if (val === 'amber') {
        term.options.theme = { background: '#0e0802', foreground: '#ffb000', cursor: '#ffb000' };
    } else {
        term.options.theme = { background: '#05070a', foreground: '#66fcf1', cursor: '#66fcf1' };
    }
});

document.getElementById('restart-btn').addEventListener('click', () => {
    if (typeof window.sstReset === 'function') {
        term.clear();
        term.write(window.sstReset());
        prompt();
    }
});

// Initialize Go WASM runtime
async function initWasm() {
    term.write('\x1b[33mInitializing Starfleet Computer Interface (WASM)...\x1b[0m\r\n');
    const go = new Go();
    try {
        const result = await WebAssembly.instantiateStreaming(fetch('sst.wasm'), go.importObject);
        go.run(result.instance);
        term.write('\x1b[32mInterface ready. Subspace radio link established.\x1b[0m\r\n\r\n');
        if (typeof window.sstInit === 'function') {
            term.write(window.sstInit());
        }
        prompt();
    } catch (err) {
        term.write('\x1b[31;1mError loading WebAssembly interface: ' + err.message + '\x1b[0m\r\n');
    }
}

initWasm();
