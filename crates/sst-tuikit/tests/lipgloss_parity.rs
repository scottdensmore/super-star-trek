//! lipgloss rendering parity (`testdata/parity/tuikit/lipgloss.txt`): every
//! style property used by the game and bubbles, borders x padding x margin x
//! width/height x alignment x colours, under all four colour profiles and
//! both dark-background settings; Join*, Place*, Size, StyleRunes/Ranges and
//! the getters. Specs use the mini-language of tools/paritygen/tuikit/lipgloss.go.

mod common;

use common::{get, load, Checker, Record};
use sst_tuikit::lipgloss::{self, Border, Position, Style, TerminalColor};
use sst_tuikit::termenv::Profile;

fn parse_color(s: &str) -> TerminalColor {
    if s == "none" {
        return TerminalColor::NoColor;
    }
    if let Some(r) = s.strip_prefix("a:") {
        let p: Vec<&str> = r.split('|').collect();
        return lipgloss::adaptive_color(p[0], p[1]);
    }
    if let Some(r) = s.strip_prefix("ca:") {
        let (l, d) = r.split_once('/').unwrap();
        let mk = |x: &str| {
            let p: Vec<&str> = x.split('|').collect();
            lipgloss::CompleteColor {
                true_color: p[0].into(),
                ansi256: p[1].into(),
                ansi: p[2].into(),
            }
        };
        return TerminalColor::CompleteAdaptiveColor {
            light: mk(l),
            dark: mk(d),
        };
    }
    if let Some(r) = s.strip_prefix("c:") {
        let p: Vec<&str> = r.split('|').collect();
        return TerminalColor::CompleteColor(lipgloss::CompleteColor {
            true_color: p[0].into(),
            ansi256: p[1].into(),
            ansi: p[2].into(),
        });
    }
    if let Some(r) = s.strip_prefix("u:") {
        return TerminalColor::ANSIColor(r.parse().unwrap());
    }
    lipgloss::color(s)
}

fn border_by_name(n: &str) -> Border {
    match n {
        "normal" => lipgloss::normal_border(),
        "rounded" => lipgloss::rounded_border(),
        "block" => lipgloss::block_border(),
        "outer" => lipgloss::outer_half_block_border(),
        "inner" => lipgloss::inner_half_block_border(),
        "thick" => lipgloss::thick_border(),
        "double" => lipgloss::double_border(),
        "hidden" => lipgloss::hidden_border(),
        "markdown" => lipgloss::markdown_border(),
        "ascii" => lipgloss::ascii_border(),
        "custom" => Border {
            top: "=-".into(),
            bottom: "日".into(),
            left: "|:".into(),
            top_left: "<>".into(),
            bottom_left: "*".into(),
            bottom_right: "★".into(),
            ..Border::default()
        },
        _ => Border::default(),
    }
}

fn go_atoi(s: &str) -> Option<i32> {
    s.parse().ok()
}

fn pos(s: &str) -> Position {
    Position(s.parse().unwrap())
}

pub fn apply_spec(spec: &str) -> Style {
    let mut st = lipgloss::new_style();
    if spec.is_empty() {
        return st;
    }
    for op in spec.split(';') {
        let (name, arg) = op.split_once('=').unwrap_or((op, ""));
        let args: Vec<&str> = arg.split(',').collect();
        let b = arg == "1";
        let ints: Vec<i32> = args.iter().filter_map(|a| go_atoi(a)).collect();
        let colors: Vec<TerminalColor> = args.iter().map(|a| parse_color(a)).collect();
        let n = || go_atoi(arg).unwrap();
        st = match name {
            "bold" => st.bold(b),
            "italic" => st.italic(b),
            "underline" => st.underline(b),
            "strike" => st.strikethrough(b),
            "reverse" => st.reverse(b),
            "blink" => st.blink(b),
            "faint" => st.faint(b),
            "fg" => st.foreground(parse_color(arg)),
            "bg" => st.background(parse_color(arg)),
            "marginbg" => st.margin_background(parse_color(arg)),
            "width" => st.width(n()),
            "height" => st.height(n()),
            "maxw" => st.max_width(n()),
            "maxh" => st.max_height(n()),
            "tabw" => st.tab_width(n()),
            "align" => {
                let ps: Vec<Position> = args.iter().map(|a| pos(a)).collect();
                st.align(&ps)
            }
            "alignh" => st.align_horizontal(pos(arg)),
            "alignv" => st.align_vertical(pos(arg)),
            "pad" => st.padding(&ints),
            "padl" => st.padding_left(n()),
            "padr" => st.padding_right(n()),
            "padt" => st.padding_top(n()),
            "padb" => st.padding_bottom(n()),
            "margin" => st.margin(&ints),
            "marginl" => st.margin_left(n()),
            "marginr" => st.margin_right(n()),
            "margint" => st.margin_top(n()),
            "marginb" => st.margin_bottom(n()),
            "border" => {
                let sides: Vec<bool> = args[1..].iter().map(|a| *a == "1").collect();
                st.border(border_by_name(args[0]), &sides)
            }
            "borderstyle" => st.border_style(border_by_name(arg)),
            "bordertop" => st.border_top(b),
            "borderright" => st.border_right(b),
            "borderbottom" => st.border_bottom(b),
            "borderleft" => st.border_left(b),
            "borderfg" => st.border_foreground(&colors),
            "borderbg" => st.border_background(&colors),
            "btfg" => st.border_top_foreground(parse_color(arg)),
            "blfg" => st.border_left_foreground(parse_color(arg)),
            "bbbg" => st.border_bottom_background(parse_color(arg)),
            "inline" => st.inline(b),
            "ulspaces" => st.underline_spaces(b),
            "stspaces" => st.strikethrough_spaces(b),
            "colorws" => st.color_whitespace(b),
            "setstring" => st.set_string(&args),
            "transform" => st.transform(|s: &str| s.to_uppercase()),
            "unsetbold" => st.unset_bold(),
            "unsetpad" => st.unset_padding(),
            "unsetborderstyle" => st.unset_border_style(),
            "inheritfrom" => st.inherit(
                &lipgloss::new_style()
                    .bold(true)
                    .underline(true)
                    .foreground(lipgloss::color("#00ff00"))
                    .background(lipgloss::color("#000080"))
                    .padding(&[1])
                    .margin(&[2])
                    .width(7),
            ),
            other => panic!("unknown op {other}"),
        };
    }
    st
}

fn color_name(c: &TerminalColor) -> String {
    match c {
        TerminalColor::NoColor => "NoColor".into(),
        TerminalColor::Color(s) => format!("Color({s})"),
        TerminalColor::ANSIColor(u) => format!("ANSIColor({u})"),
        TerminalColor::AdaptiveColor { light, dark } => format!("Adaptive({light}|{dark})"),
        TerminalColor::CompleteColor(c) => {
            format!("Complete({}|{}|{})", c.true_color, c.ansi256, c.ansi)
        }
        TerminalColor::CompleteAdaptiveColor { .. } => "CompleteAdaptive".into(),
    }
}

fn go_float(f: f64) -> String {
    // fmt %v of float64 for the values used here.
    if f == f.trunc() {
        format!("{}", f as i64)
    } else {
        format!("{f}")
    }
}

fn go_quote(s: &str) -> String {
    // %q for plain ASCII values.
    format!("{s:?}")
}

fn getters(st: &Style) -> String {
    let (t, r, b, l) = st.get_padding();
    let (mt, mr, mb, ml) = st.get_margin();
    let (_, bt, br, bb, bl) = st.get_border();
    format!(
        "pad={t},{r},{b},{l} margin={mt},{mr},{mb},{ml} border={bt},{br},{bb},{bl} hb={} vb={} hp={} vp={} hm={} vm={} hf={} vf={} bts={} brs={} bbs={} bls={} w={} h={} mw={} mh={} tw={} a={} av={} bold={} italic={} ul={} rev={} inline={} cws={} uls={} fg={} bg={} btfg={} blfg={} bbbg={} val={}",
        st.get_horizontal_border_size(),
        st.get_vertical_border_size(),
        st.get_horizontal_padding(),
        st.get_vertical_padding(),
        st.get_horizontal_margins(),
        st.get_vertical_margins(),
        st.get_horizontal_frame_size(),
        st.get_vertical_frame_size(),
        st.get_border_top_size(),
        st.get_border_right_size(),
        st.get_border_bottom_size(),
        st.get_border_left_size(),
        st.get_width(),
        st.get_height(),
        st.get_max_width(),
        st.get_max_height(),
        st.get_tab_width(),
        go_float(st.get_align().0),
        go_float(st.get_align_vertical().0),
        st.get_bold(),
        st.get_italic(),
        st.get_underline(),
        st.get_reverse(),
        st.get_inline(),
        st.get_color_whitespace(),
        st.get_underline_spaces(),
        color_name(&st.get_foreground()),
        color_name(&st.get_background()),
        color_name(&st.get_border_top_foreground()),
        color_name(&st.get_border_left_foreground()),
        color_name(&st.get_border_bottom_background()),
        go_quote(st.value()),
    )
}

fn set_cfg(key: &str) {
    let b = key.as_bytes();
    let p = match b[1] {
        b'0' => Profile::TrueColor,
        b'1' => Profile::ANSI256,
        b'2' => Profile::ANSI,
        _ => Profile::Ascii,
    };
    lipgloss::set_color_profile(p);
    lipgloss::set_has_dark_background(b[2] == b'd');
}

fn cfg_outputs(r: &Record) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = r
        .iter()
        .filter(|(k, _)| k.len() == 3 && k.starts_with('o'))
        .map(|(k, v)| (k.clone(), v.to_string()))
        .collect();
    // Resolve "\0same:<key>" references.
    let snapshot = v.clone();
    for (_, val) in v.iter_mut() {
        if let Some(k) = val.strip_prefix("\0same:") {
            *val = snapshot.iter().find(|(kk, _)| kk == k).unwrap().1.clone();
        }
    }
    v
}

fn ws_opts(ws: &str) -> Vec<lipgloss::WhitespaceOption> {
    let mut opts = Vec::new();
    for o in ws.split(';') {
        if o.is_empty() {
            continue;
        }
        let (k, v) = o.split_once('=').unwrap();
        match k {
            "chars" => opts.push(lipgloss::with_whitespace_chars(v)),
            "fg" => opts.push(lipgloss::with_whitespace_foreground(lipgloss::color(v))),
            "bg" => opts.push(lipgloss::with_whitespace_background(lipgloss::color(v))),
            _ => panic!("ws {k}"),
        }
    }
    opts
}

fn box_block() -> String {
    lipgloss::set_color_profile(Profile::Ascii);
    lipgloss::set_has_dark_background(true);
    apply_spec("border=rounded;pad=0,1").render("box\ncontent")
}

#[test]
fn lipgloss_parity() {
    let recs = load("lipgloss.txt");
    let mut c = Checker::default();
    let box_ = box_block();
    let blocks = [
        "a",
        "bb\nbbb",
        "c\nc\nc\nc",
        box_.as_str(),
        "日本\nx",
        "",
        "\x1b[1mbold\x1b[0m\nline",
    ];
    for r in &recs {
        let op = get(r, "op");
        match op {
            "render" => {
                let spec = get(r, "spec");
                let input = get(r, "in");
                for (k, want) in cfg_outputs(r) {
                    set_cfg(&k);
                    let got = apply_spec(spec).render(input);
                    c.check(
                        || format!("render {k} spec={spec:?} in={input:?}"),
                        &got,
                        &want,
                    );
                }
            }
            "getters" => {
                lipgloss::set_color_profile(Profile::Ascii);
                let spec = get(r, "spec");
                c.check(
                    || format!("getters {spec:?}"),
                    &getters(&apply_spec(spec)),
                    get(r, "get"),
                );
            }
            "joinh" | "joinv" => {
                lipgloss::set_color_profile(Profile::Ascii);
                let p = pos(get(r, "pos"));
                let (a, b) = (get(r, "a"), get(r, "b"));
                let got = if op == "joinh" {
                    lipgloss::join_horizontal(p, &[a, b])
                } else {
                    lipgloss::join_vertical(p, &[a, b])
                };
                c.check(|| format!("{op} {r:?}"), &got, get(r, "out"));
            }
            "joinh3" => {
                let p = pos(get(r, "pos"));
                let got = lipgloss::join_horizontal(p, &[blocks[0], blocks[3], blocks[2]]);
                c.check(|| format!("{op} {r:?}"), &got, get(r, "out"));
            }
            "joinv3" => {
                let p = pos(get(r, "pos"));
                let got = lipgloss::join_vertical(p, &[blocks[1], blocks[3], blocks[4]]);
                c.check(|| format!("{op} {r:?}"), &got, get(r, "out"));
            }
            "joinh0" => c.check(
                || op.into(),
                &lipgloss::join_horizontal(lipgloss::TOP, &[]),
                get(r, "out"),
            ),
            "joinh1" => c.check(
                || op.into(),
                &lipgloss::join_horizontal(lipgloss::TOP, &["solo\nx"]),
                get(r, "out"),
            ),
            "place" => {
                let opts = ws_opts(get(r, "ws"));
                let w: i32 = get(r, "w").parse().unwrap();
                let h: i32 = get(r, "h").parse().unwrap();
                let (hp, vp) = (pos(get(r, "hp")), pos(get(r, "vp")));
                for (k, want) in cfg_outputs(r) {
                    set_cfg(&k);
                    let got = lipgloss::place(w, h, hp, vp, get(r, "in"), &opts);
                    c.check(|| format!("place {k} {r:?}"), &got, &want);
                }
            }
            "placeh" => {
                lipgloss::set_color_profile(Profile::Ascii);
                let opts = ws_opts(get(r, "ws"));
                let w: i32 = get(r, "w").parse().unwrap();
                let got = lipgloss::place_horizontal(w, pos(get(r, "hp")), get(r, "in"), &opts);
                c.check(|| format!("placeh {r:?}"), &got, get(r, "out"));
            }
            "placev" => {
                lipgloss::set_color_profile(Profile::Ascii);
                let opts = ws_opts(get(r, "ws"));
                let h: i32 = get(r, "h").parse().unwrap();
                let got = lipgloss::place_vertical(h, pos(get(r, "vp")), get(r, "in"), &opts);
                c.check(|| format!("placev {r:?}"), &got, get(r, "out"));
            }
            "size" => {
                let s = get(r, "in");
                let (w, h) = lipgloss::size(s);
                let got = format!("{} {} {w} {h}", lipgloss::width(s), lipgloss::height(s));
                c.check(|| format!("size {s:?}"), &got, get(r, "out"));
            }
            "stylerunes" | "styleranges" => {
                set_cfg(get(r, "cfg"));
                let m = lipgloss::new_style()
                    .underline(true)
                    .foreground(lipgloss::color("#ff0000"));
                let u = lipgloss::new_style().foreground(lipgloss::color("#00ff00"));
                let got = if op == "stylerunes" {
                    lipgloss::style_runes("Hello, 世界! nav", &[0, 1, 4, 7, 8, 9, 13, 14], &m, &u)
                } else {
                    lipgloss::style_ranges(
                        "\x1b[1mbold text\x1b[0m and more",
                        &[
                            lipgloss::new_range(0, 4, m.clone()),
                            lipgloss::new_range(10, 13, u.clone()),
                        ],
                    )
                };
                c.check(|| format!("{op} {r:?}"), &got, get(r, "out"));
            }
            other => panic!("unknown op {other}"),
        }
    }
    c.finish("lipgloss");
}
