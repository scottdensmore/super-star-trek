//! bubbletea parity: `Key.String`, `MouseEvent.String`, and terminal input
//! decoding (`testdata/parity/tuikit/tea.txt`, recorded through a real
//! `tea.Program` reading from a custom reader).

mod common;

use common::{get, go_quote, load, Checker};
use sst_tuikit::tea::input::{parse_input, InputMsg, UnknownCsiSequenceMsg, UnknownInputByteMsg};
use sst_tuikit::tea::{
    KeyMsg, KeyType, MouseAction, MouseButton, MouseMsg, KEY_CTRL_CLOSE_BRACKET, KEY_RUNES,
    KEY_SPACE,
};

fn describe(m: &InputMsg) -> String {
    match m {
        InputMsg::Key(k) => {
            let runes: String = k.runes.iter().collect();
            format!(
                "key type={} runes={} alt={} paste={} str={}",
                k.typ.0,
                go_quote(&runes),
                k.alt,
                k.paste,
                go_quote(&k.string())
            )
        }
        InputMsg::Mouse(v) => format!(
            "mouse x={} y={} shift={} alt={} ctrl={} action={} button={} type={} str={}",
            v.x,
            v.y,
            v.shift,
            v.alt,
            v.ctrl,
            v.action.0,
            v.button.0,
            v.typ.0,
            go_quote(&v.string())
        ),
        InputMsg::Focus => "focus".into(),
        InputMsg::Blur => "blur".into(),
        InputMsg::UnknownCsi(b) => format!(
            "tea.unknownCSISequenceMsg {}",
            UnknownCsiSequenceMsg(b.clone())
        ),
        InputMsg::UnknownByte(b) => format!("tea.unknownInputByteMsg {}", UnknownInputByteMsg(*b)),
    }
}

#[test]
fn tea_parity() {
    let recs = load("tea.txt");
    let mut c = Checker::default();
    for r in &recs {
        let op = get(r, "op");
        match op {
            "keystring" => {
                let t: i32 = get(r, "type").parse().unwrap();
                let k = KeyMsg {
                    typ: KeyType(t),
                    alt: get(r, "alt") == "true",
                    ..Default::default()
                };
                c.check(|| format!("{r:?}"), &k.string(), get(r, "out"));
                c.check(|| format!("{r:?}"), &KeyType(t).string(), get(r, "typestr"));
            }
            "keyrunes" => {
                let k = KeyMsg {
                    typ: KEY_RUNES,
                    runes: get(r, "runes").chars().collect(),
                    alt: get(r, "alt") == "true",
                    paste: get(r, "paste") == "true",
                };
                c.check(|| format!("{r:?}"), &k.string(), get(r, "out"));
            }
            "keyspace" => {
                let k = KeyMsg {
                    typ: KEY_SPACE,
                    runes: vec![' '],
                    ..Default::default()
                };
                c.check(|| format!("{r:?}"), &k.string(), get(r, "out"));
            }
            "mousestring" => {
                let mods: i32 = get(r, "mods").parse().unwrap();
                let m = MouseMsg {
                    x: 3,
                    y: 4,
                    button: MouseButton(get(r, "b").parse().unwrap()),
                    action: MouseAction(get(r, "a").parse().unwrap()),
                    ctrl: mods & 1 != 0,
                    alt: mods & 2 != 0,
                    shift: mods & 4 != 0,
                    ..Default::default()
                };
                c.check(|| format!("{r:?}"), &m.string(), get(r, "out"));
            }
            "decode" => {
                let input = r.bytes("in");
                let input_dbg = String::from_utf8_lossy(input).into_owned();
                let msgs = parse_input(input);
                let got: Vec<String> = msgs
                    .iter()
                    .filter(|m| !matches!(m, InputMsg::Key(k) if k.typ == KEY_CTRL_CLOSE_BRACKET && !k.alt))
                    .map(describe)
                    .collect();
                c.check(
                    || format!("decode {input_dbg:?}"),
                    &got.join("\n"),
                    get(r, "out"),
                );
            }
            other => panic!("unknown op {other}"),
        }
    }
    c.finish("tea");
}
