//! The DEC ANSI state-machine transition table (`x/ansi/parser`,
//! `transition_table.go`, `const.go`, `seq.go`).

use std::sync::OnceLock;

pub type Action = u8;
pub type State = u8;

pub const NONE_ACTION: Action = 0;
pub const CLEAR_ACTION: Action = 1;
pub const COLLECT_ACTION: Action = 2;
pub const PREFIX_ACTION: Action = 3;
pub const DISPATCH_ACTION: Action = 4;
pub const EXECUTE_ACTION: Action = 5;
pub const START_ACTION: Action = 6;
pub const PUT_ACTION: Action = 7;
pub const PARAM_ACTION: Action = 8;
pub const PRINT_ACTION: Action = 9;
pub const IGNORE_ACTION: Action = NONE_ACTION;

pub const GROUND_STATE: State = 0;
pub const CSI_ENTRY_STATE: State = 1;
pub const CSI_INTERMEDIATE_STATE: State = 2;
pub const CSI_PARAM_STATE: State = 3;
pub const DCS_ENTRY_STATE: State = 4;
pub const DCS_INTERMEDIATE_STATE: State = 5;
pub const DCS_PARAM_STATE: State = 6;
pub const DCS_STRING_STATE: State = 7;
pub const ESCAPE_STATE: State = 8;
pub const ESCAPE_INTERMEDIATE_STATE: State = 9;
pub const OSC_STRING_STATE: State = 10;
pub const SOS_STRING_STATE: State = 11;
pub const PM_STRING_STATE: State = 12;
pub const APC_STRING_STATE: State = 13;
pub const UTF8_STATE: State = 14;

const TRANSITION_ACTION_SHIFT: u8 = 4;
const TRANSITION_STATE_MASK: u8 = 15;
const INDEX_STATE_SHIFT: usize = 8;
const DEFAULT_TABLE_SIZE: usize = 4096;

/// Parameter flags/masks (`seq.go`). Go `int` is 64-bit.
pub const PREFIX_SHIFT: i64 = 8;
pub const INTERMED_SHIFT: i64 = 16;
pub const FINAL_MASK: i64 = 0xff;
pub const HAS_MORE_FLAG: i64 = i32::MIN as i64;
pub const PARAM_MASK: i64 = !HAS_MORE_FLAG;
pub const MISSING_PARAM: i64 = PARAM_MASK;
pub const MISSING_COMMAND: i64 = MISSING_PARAM;
pub const MAX_PARAMS_SIZE: usize = 32;

pub struct TransitionTable(Vec<u8>);

impl TransitionTable {
    fn new() -> Self {
        TransitionTable(vec![0; DEFAULT_TABLE_SIZE])
    }
    fn set_default(&mut self, action: Action, state: State) {
        for v in self.0.iter_mut() {
            *v = (action << TRANSITION_ACTION_SHIFT) | state;
        }
    }
    fn add_one(&mut self, code: u8, state: State, action: Action, next: State) {
        let idx = ((state as usize) << INDEX_STATE_SHIFT) | code as usize;
        self.0[idx] = (action << TRANSITION_ACTION_SHIFT) | next;
    }
    fn add_many(&mut self, codes: &[u8], state: State, action: Action, next: State) {
        for &c in codes {
            self.add_one(c, state, action, next);
        }
    }
    fn add_range(&mut self, start: u8, end: u8, state: State, action: Action, next: State) {
        for i in start as usize..=end as usize {
            self.add_one(i as u8, state, action, next);
        }
    }

    /// Returns the next state and the action for `code` in `state`.
    #[inline]
    pub fn transition(&self, state: State, code: u8) -> (State, Action) {
        let index = ((state as usize) << INDEX_STATE_SHIFT) | code as usize;
        let value = self.0[index];
        (
            value & TRANSITION_STATE_MASK,
            value >> TRANSITION_ACTION_SHIFT,
        )
    }
}

/// The shared table (`parser.Table`).
pub fn table() -> &'static TransitionTable {
    static T: OnceLock<TransitionTable> = OnceLock::new();
    T.get_or_init(generate_transition_table)
}

fn generate_transition_table() -> TransitionTable {
    let mut t = TransitionTable::new();
    t.set_default(NONE_ACTION, GROUND_STATE);

    for state in GROUND_STATE..=UTF8_STATE {
        t.add_many(
            &[0x18, 0x1a, 0x99, 0x9a],
            state,
            EXECUTE_ACTION,
            GROUND_STATE,
        );
        t.add_range(0x80, 0x8F, state, EXECUTE_ACTION, GROUND_STATE);
        t.add_range(0x90, 0x97, state, EXECUTE_ACTION, GROUND_STATE);
        t.add_one(0x9C, state, EXECUTE_ACTION, GROUND_STATE);
        t.add_one(0x1B, state, CLEAR_ACTION, ESCAPE_STATE);
        t.add_one(0x98, state, START_ACTION, SOS_STRING_STATE);
        t.add_one(0x9E, state, START_ACTION, PM_STRING_STATE);
        t.add_one(0x9F, state, START_ACTION, APC_STRING_STATE);
        t.add_one(0x9B, state, CLEAR_ACTION, CSI_ENTRY_STATE);
        t.add_one(0x90, state, CLEAR_ACTION, DCS_ENTRY_STATE);
        t.add_one(0x9D, state, START_ACTION, OSC_STRING_STATE);
        t.add_range(0xC2, 0xDF, state, COLLECT_ACTION, UTF8_STATE);
        t.add_range(0xE0, 0xEF, state, COLLECT_ACTION, UTF8_STATE);
        t.add_range(0xF0, 0xF4, state, COLLECT_ACTION, UTF8_STATE);
    }

    t.add_range(0x00, 0x17, GROUND_STATE, EXECUTE_ACTION, GROUND_STATE);
    t.add_one(0x19, GROUND_STATE, EXECUTE_ACTION, GROUND_STATE);
    t.add_range(0x1C, 0x1F, GROUND_STATE, EXECUTE_ACTION, GROUND_STATE);
    t.add_range(0x20, 0x7E, GROUND_STATE, PRINT_ACTION, GROUND_STATE);
    t.add_one(0x7F, GROUND_STATE, EXECUTE_ACTION, GROUND_STATE);

    let s = ESCAPE_INTERMEDIATE_STATE;
    t.add_range(0x00, 0x17, s, EXECUTE_ACTION, s);
    t.add_one(0x19, s, EXECUTE_ACTION, s);
    t.add_range(0x1C, 0x1F, s, EXECUTE_ACTION, s);
    t.add_range(0x20, 0x2F, s, COLLECT_ACTION, s);
    t.add_one(0x7F, s, IGNORE_ACTION, s);
    t.add_range(0x30, 0x7E, s, DISPATCH_ACTION, GROUND_STATE);

    let s = ESCAPE_STATE;
    t.add_range(0x00, 0x17, s, EXECUTE_ACTION, s);
    t.add_one(0x19, s, EXECUTE_ACTION, s);
    t.add_range(0x1C, 0x1F, s, EXECUTE_ACTION, s);
    t.add_one(0x7F, s, IGNORE_ACTION, s);
    t.add_range(0x30, 0x4F, s, DISPATCH_ACTION, GROUND_STATE);
    t.add_range(0x51, 0x57, s, DISPATCH_ACTION, GROUND_STATE);
    t.add_one(0x59, s, DISPATCH_ACTION, GROUND_STATE);
    t.add_one(0x5A, s, DISPATCH_ACTION, GROUND_STATE);
    t.add_one(0x5C, s, DISPATCH_ACTION, GROUND_STATE);
    t.add_range(0x60, 0x7E, s, DISPATCH_ACTION, GROUND_STATE);
    t.add_range(0x20, 0x2F, s, COLLECT_ACTION, ESCAPE_INTERMEDIATE_STATE);
    t.add_one(b'X', s, START_ACTION, SOS_STRING_STATE);
    t.add_one(b'^', s, START_ACTION, PM_STRING_STATE);
    t.add_one(b'_', s, START_ACTION, APC_STRING_STATE);
    t.add_one(b'P', s, CLEAR_ACTION, DCS_ENTRY_STATE);
    t.add_one(b'[', s, CLEAR_ACTION, CSI_ENTRY_STATE);
    t.add_one(b']', s, START_ACTION, OSC_STRING_STATE);

    for state in SOS_STRING_STATE..=APC_STRING_STATE {
        t.add_range(0x00, 0x17, state, PUT_ACTION, state);
        t.add_one(0x19, state, PUT_ACTION, state);
        t.add_range(0x1C, 0x1F, state, PUT_ACTION, state);
        t.add_range(0x20, 0x7F, state, PUT_ACTION, state);
        t.add_one(0x1B, state, DISPATCH_ACTION, ESCAPE_STATE);
        t.add_one(0x9C, state, DISPATCH_ACTION, GROUND_STATE);
        t.add_many(&[0x18, 0x1A], state, IGNORE_ACTION, GROUND_STATE);
    }

    let s = DCS_ENTRY_STATE;
    t.add_range(0x00, 0x07, s, IGNORE_ACTION, s);
    t.add_range(0x0E, 0x17, s, IGNORE_ACTION, s);
    t.add_one(0x19, s, IGNORE_ACTION, s);
    t.add_range(0x1C, 0x1F, s, IGNORE_ACTION, s);
    t.add_one(0x7F, s, IGNORE_ACTION, s);
    t.add_range(0x20, 0x2F, s, COLLECT_ACTION, DCS_INTERMEDIATE_STATE);
    t.add_range(0x30, 0x3B, s, PARAM_ACTION, DCS_PARAM_STATE);
    t.add_range(0x3C, 0x3F, s, PREFIX_ACTION, DCS_PARAM_STATE);
    t.add_range(0x08, 0x0D, s, PUT_ACTION, DCS_STRING_STATE);
    t.add_one(0x1B, s, PUT_ACTION, DCS_STRING_STATE);
    t.add_range(0x40, 0x7E, s, START_ACTION, DCS_STRING_STATE);

    let s = DCS_INTERMEDIATE_STATE;
    t.add_range(0x00, 0x17, s, IGNORE_ACTION, s);
    t.add_one(0x19, s, IGNORE_ACTION, s);
    t.add_range(0x1C, 0x1F, s, IGNORE_ACTION, s);
    t.add_range(0x20, 0x2F, s, COLLECT_ACTION, s);
    t.add_one(0x7F, s, IGNORE_ACTION, s);
    t.add_range(0x30, 0x3F, s, START_ACTION, DCS_STRING_STATE);
    t.add_range(0x40, 0x7E, s, START_ACTION, DCS_STRING_STATE);

    let s = DCS_PARAM_STATE;
    t.add_range(0x00, 0x17, s, IGNORE_ACTION, s);
    t.add_one(0x19, s, IGNORE_ACTION, s);
    t.add_range(0x1C, 0x1F, s, IGNORE_ACTION, s);
    t.add_range(0x30, 0x3B, s, PARAM_ACTION, s);
    t.add_one(0x7F, s, IGNORE_ACTION, s);
    t.add_range(0x3C, 0x3F, s, IGNORE_ACTION, s);
    t.add_range(0x20, 0x2F, s, COLLECT_ACTION, DCS_INTERMEDIATE_STATE);
    t.add_range(0x40, 0x7E, s, START_ACTION, DCS_STRING_STATE);

    let s = DCS_STRING_STATE;
    t.add_range(0x00, 0x17, s, PUT_ACTION, s);
    t.add_one(0x19, s, PUT_ACTION, s);
    t.add_range(0x1C, 0x1F, s, PUT_ACTION, s);
    t.add_range(0x20, 0x7E, s, PUT_ACTION, s);
    t.add_one(0x7F, s, PUT_ACTION, s);
    t.add_range(0x80, 0xFF, s, PUT_ACTION, s);
    t.add_one(0x1B, s, DISPATCH_ACTION, ESCAPE_STATE);
    t.add_one(0x9C, s, DISPATCH_ACTION, GROUND_STATE);
    t.add_many(&[0x18, 0x1A], s, IGNORE_ACTION, GROUND_STATE);

    let s = CSI_PARAM_STATE;
    t.add_range(0x00, 0x17, s, EXECUTE_ACTION, s);
    t.add_one(0x19, s, EXECUTE_ACTION, s);
    t.add_range(0x1C, 0x1F, s, EXECUTE_ACTION, s);
    t.add_range(0x30, 0x3B, s, PARAM_ACTION, s);
    t.add_one(0x7F, s, IGNORE_ACTION, s);
    t.add_range(0x3C, 0x3F, s, IGNORE_ACTION, s);
    t.add_range(0x40, 0x7E, s, DISPATCH_ACTION, GROUND_STATE);
    t.add_range(0x20, 0x2F, s, COLLECT_ACTION, CSI_INTERMEDIATE_STATE);

    let s = CSI_INTERMEDIATE_STATE;
    t.add_range(0x00, 0x17, s, EXECUTE_ACTION, s);
    t.add_one(0x19, s, EXECUTE_ACTION, s);
    t.add_range(0x1C, 0x1F, s, EXECUTE_ACTION, s);
    t.add_range(0x20, 0x2F, s, COLLECT_ACTION, s);
    t.add_one(0x7F, s, IGNORE_ACTION, s);
    t.add_range(0x40, 0x7E, s, DISPATCH_ACTION, GROUND_STATE);
    t.add_range(0x30, 0x3F, s, IGNORE_ACTION, GROUND_STATE);

    let s = CSI_ENTRY_STATE;
    t.add_range(0x00, 0x17, s, EXECUTE_ACTION, s);
    t.add_one(0x19, s, EXECUTE_ACTION, s);
    t.add_range(0x1C, 0x1F, s, EXECUTE_ACTION, s);
    t.add_one(0x7F, s, IGNORE_ACTION, s);
    t.add_range(0x40, 0x7E, s, DISPATCH_ACTION, GROUND_STATE);
    t.add_range(0x20, 0x2F, s, COLLECT_ACTION, CSI_INTERMEDIATE_STATE);
    t.add_range(0x30, 0x3B, s, PARAM_ACTION, CSI_PARAM_STATE);
    t.add_range(0x3C, 0x3F, s, PREFIX_ACTION, CSI_PARAM_STATE);

    let s = OSC_STRING_STATE;
    t.add_range(0x00, 0x06, s, IGNORE_ACTION, s);
    t.add_range(0x08, 0x17, s, IGNORE_ACTION, s);
    t.add_one(0x19, s, IGNORE_ACTION, s);
    t.add_range(0x1C, 0x1F, s, IGNORE_ACTION, s);
    t.add_range(0x20, 0xFF, s, PUT_ACTION, s);
    t.add_one(0x1B, s, DISPATCH_ACTION, ESCAPE_STATE);
    t.add_one(0x07, s, DISPATCH_ACTION, GROUND_STATE);
    t.add_one(0x9C, s, DISPATCH_ACTION, GROUND_STATE);
    t.add_many(&[0x18, 0x1A], s, IGNORE_ACTION, GROUND_STATE);

    t
}
