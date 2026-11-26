//! bubbles/list parity (`testdata/parity/tuikit/list.txt`): the command
//! palette setup (custom delegate styles, no status bar/pagination/help,
//! queries via DefaultFilter + SetItems) and default lists driven through
//! Update (navigation, pagination, filtering, help toggle, quit), under
//! Ascii, TrueColor/dark and ANSI256/light; plus DefaultFilter ranks.

mod common;

use std::rc::Rc;
use std::time::Duration;

use common::{get, go_quote, load, Checker};
use sst_tuikit::bubbles::list::{self, DefaultItem, Item};
use sst_tuikit::lipgloss;
use sst_tuikit::tea::{
    self, BatchMsg, Cmd, KeyMsg, KeyType, Msg, QuitMsg, WindowSizeMsg, KEY_LEFT, KEY_RIGHT,
    KEY_SPACE,
};
use sst_tuikit::termenv::Profile;

#[derive(Clone)]
struct PaletteItem {
    title: String,
    desc: String,
}

impl DefaultItem for PaletteItem {
    fn title(&self) -> String {
        self.title.clone()
    }
    fn description(&self) -> String {
        self.desc.clone()
    }
}

impl Item for PaletteItem {
    fn filter_value(&self) -> String {
        format!("{} {}", self.title, self.desc)
    }
    fn as_default_item(&self) -> Option<&dyn DefaultItem> {
        Some(self)
    }
}

const CATALOG: &[(&str, &str)] = &[
    (
        "TOR",
        "Fire photon torpedo along bearing or sector (tor <course|r c>)",
    ),
    ("PHA", "Fire ship phaser bank with energy (pha <energy>)"),
    (
        "SHE",
        "Transfer energy between warp and shields (she <amount>)",
    ),
    ("TARGET", "Open Tactical Target Lock HUD"),
    (
        "NAV",
        "Maneuver: nav q <r c> (quad), nav s <r c> (sector), or nav <course warp>",
    ),
    ("DOC", "Dock with adjacent Starbase for fuel & repair"),
    ("SRSCAN", "Refresh short-range sensor tactical grid"),
    ("LRSCAN", "Scan adjacent 3x3 quadrant cluster"),
    ("STATUS", "Review ship status, condition, and stardate"),
    ("DAM", "Review subsystem device damage repair times"),
    (
        "CHART",
        "Interactive 8x8 galactic star chart and warp planner (Ctrl+G)",
    ),
    ("SAVES", "Browse, inspect, and load saved missions (Ctrl+O)"),
    (
        "SCENARIOS",
        "Browse and launch tactical challenge scenarios",
    ),
    (
        "THEME: Modern",
        "Switch theme to Starfleet Modern (Electric Cyan/Navy)",
    ),
    (
        "THEME: LCARS",
        "Switch theme to 24th Century LCARS (Gold/Purple)",
    ),
    (
        "THEME: CRT",
        "Switch theme to Phosphor Monochrome CRT (Green/Amber)",
    ),
    (
        "THEME MODE AUTO",
        "Set theme adaptation to automatic terminal background detection",
    ),
    ("THEME MODE DARK", "Force dark theme palette"),
    ("THEME MODE LIGHT", "Force light theme palette"),
    ("HELP", "Display tactical command reference summary"),
    (
        "HELP NAV",
        "Navigation reference: quadrant warping, course angles, and impulse",
    ),
    ("QUIT", "Abandon mission and exit to shell"),
];

const EXTRA: &[(&str, &str)] = &[
    (
        "Ünïcödé títle with a very long name that must be truncated",
        "desc line one\nsecond line hidden",
    ),
    ("日本語のタイトル", "説明文 ★ wide"),
    ("⚠️ Alert", "🔊 sound on"),
    ("short", ""),
    ("camelCaseItem", "under_score-sep.dot/slash"),
];

fn mk(src: &[(&str, &str)]) -> Vec<Rc<dyn Item>> {
    src.iter()
        .map(|(t, d)| {
            Rc::new(PaletteItem {
                title: t.to_string(),
                desc: d.to_string(),
            }) as Rc<dyn Item>
        })
        .collect()
}

fn list_items(set: &str) -> Vec<Rc<dyn Item>> {
    match set {
        "palette" => mk(CATALOG),
        "mixed" => {
            let mut v = mk(&CATALOG[..6]);
            v.extend(mk(EXTRA));
            v
        }
        "few" => mk(&CATALOG[..3]),
        _ => Vec::new(),
    }
}

fn title_of(it: &Rc<dyn Item>) -> String {
    it.as_any()
        .downcast_ref::<PaletteItem>()
        .unwrap()
        .title
        .clone()
}

fn new_palette_list(items: Vec<Rc<dyn Item>>, w: i32, h: i32) -> list::Model {
    let mut d = list::new_default_delegate();
    d.show_description = true;
    d.set_spacing(0);
    let mut l = list::new(items, d, w, h);
    l.title = "COMMAND PALETTE".into();
    l.set_show_title(true);
    l.set_show_status_bar(false);
    l.set_show_pagination(false);
    l.set_show_help(false);
    l.disable_quit_keybindings();

    let command_text = lipgloss::new_style().foreground(lipgloss::color("#E2E8F0"));
    let gauge_label = lipgloss::new_style().foreground(lipgloss::color("#E2E8F0"));
    let prompt = lipgloss::new_style()
        .bold(true)
        .foreground(lipgloss::color("#00E5FF"));
    let panel_title = lipgloss::new_style()
        .bold(true)
        .foreground(lipgloss::color("#00E5FF"));
    let cond_red = lipgloss::new_style()
        .bold(true)
        .foreground(lipgloss::color("#FF3344"));

    let mut d2 = list::new_default_delegate();
    d2.show_description = true;
    d2.set_spacing(0);
    d2.styles.normal_title = command_text.padding(&[0, 0, 0, 2]);
    d2.styles.normal_desc = gauge_label.clone().padding(&[0, 0, 0, 2]);
    let accent = prompt.get_foreground();
    d2.styles.selected_title = prompt
        .border(lipgloss::normal_border(), &[false, false, false, true])
        .border_foreground(std::slice::from_ref(&accent))
        .padding(&[0, 0, 0, 1]);
    d2.styles.selected_desc = gauge_label
        .border(lipgloss::normal_border(), &[false, false, false, true])
        .border_foreground(&[accent])
        .padding(&[0, 0, 0, 1]);
    d2.styles.filter_match = lipgloss::new_style().underline(true);
    l.set_delegate(d2);
    l.styles.title = panel_title.padding(&[0, 1]);
    l.styles.no_items = cond_red.padding(&[0, 0, 0, 2]);
    l.set_size(w, h);
    l
}

fn new_default_list(items: Vec<Rc<dyn Item>>, w: i32, h: i32, variant: &str) -> list::Model {
    let mut d = list::new_default_delegate();
    match variant {
        "nodesc" => d.show_description = false,
        "spacing0" => d.set_spacing(0),
        "height3" => d.set_height(3),
        _ => {}
    }
    let mut l = list::new(items, d, w, h);
    l.filter_input.cursor.blink_speed = Duration::from_millis(1);
    match variant {
        "infinite" => l.infinite_scrolling = true,
        "noquit" => l.disable_quit_keybindings(),
        "nofilter" => l.set_filtering_enabled(false),
        "names" => l.set_status_bar_item_name("command", "commands"),
        _ => {}
    }
    l
}

fn list_state(l: &list::Model) -> String {
    let titles: Vec<String> = l.visible_items().iter().map(title_of).collect();
    let sel = l
        .selected_item()
        .map(|s| title_of(&s))
        .unwrap_or_else(|| "nil".into());
    format!(
        "index={} global={} cursor={} page={}/{} per={} state={} filter={} sel={} visible={}",
        l.index(),
        l.global_index(),
        l.cursor(),
        l.paginator.page,
        l.paginator.total_pages,
        l.paginator.per_page,
        l.filter_state().string(),
        go_quote(&l.filter_value()),
        go_quote(&sel),
        go_quote(&titles.join("|"))
    )
}

fn apply_palette_query(l: &mut list::Model, items: &[Rc<dyn Item>], query: &str) {
    if query.is_empty() {
        l.title = "COMMAND PALETTE".into();
        let _ = l.set_items(items.to_vec());
        l.filter_input.reset();
        l.select(0);
        return;
    }
    l.title = format!("COMMAND PALETTE: {query}");
    l.filter_input.set_value(query);
    let targets: Vec<String> = items.iter().map(|i| i.filter_value()).collect();
    let ranks = list::default_filter(query, &targets);
    let filtered: Vec<Rc<dyn Item>> = ranks
        .iter()
        .map(|r| items[r.index as usize].clone())
        .collect();
    let _ = l.set_items(filtered);
    l.select(0);
}

fn list_key(step: &str) -> Msg {
    let (kind, arg) = step.split_once(':').unwrap_or((step, ""));
    match kind {
        "r" => tea::msg(KeyMsg::runes(arg)),
        "k" => tea::msg(KeyMsg::new(KeyType(arg.parse().unwrap()))),
        "sp" => tea::msg(KeyMsg {
            typ: KEY_SPACE,
            runes: vec![' '],
            ..Default::default()
        }),
        "ws" => tea::msg(WindowSizeMsg {
            width: 80,
            height: 24,
        }),
        _ => panic!("bad step {step}"),
    }
}

fn run_cmds(cmds: Vec<Option<Cmd>>) -> Vec<Msg> {
    let mut out = Vec::new();
    for c in cmds.into_iter().flatten() {
        let Some(m) = c.call() else { continue };
        match m.downcast::<BatchMsg>() {
            Ok(b) => out.extend(run_cmds(b.0.into_iter().map(Some).collect())),
            Err(m) => out.push(m),
        }
    }
    out
}

fn set_run(tag: &str) {
    let (p, dark) = match tag {
        "a" => (Profile::Ascii, true),
        "t" => (Profile::TrueColor, true),
        _ => (Profile::ANSI256, false),
    };
    lipgloss::set_color_profile(p);
    lipgloss::set_has_dark_background(dark);
}

fn ranks_string(r: &[list::Rank]) -> String {
    r.iter()
        .map(|x| {
            let idx: Vec<String> = x.matched_indexes.iter().map(|i| i.to_string()).collect();
            format!("{}:[{}];", x.index, idx.join(" "))
        })
        .collect()
}

#[test]
fn list_parity() {
    let recs = load("list.txt");
    let mut c = Checker::default();
    let mut all: Vec<(&str, &str)> = CATALOG.to_vec();
    all.extend_from_slice(EXTRA);
    let targets: Vec<String> = all.iter().map(|(t, d)| format!("{t} {d}")).collect();
    for r in &recs {
        match get(r, "op") {
            "filter" => {
                let q = get(r, "q");
                c.check(
                    || format!("filter {q:?}"),
                    &ranks_string(&list::default_filter(q, &targets)),
                    get(r, "out"),
                );
            }
            "unsorted" => {
                let q = get(r, "q");
                c.check(
                    || format!("unsorted {q:?}"),
                    &ranks_string(&list::unsorted_filter(q, &targets)),
                    get(r, "out"),
                );
            }
            "palette" => {
                let tag = get(r, "run");
                set_run(tag);
                let set = get(r, "set");
                let w: i32 = get(r, "w").parse().unwrap();
                let h: i32 = get(r, "h").parse().unwrap();
                let steps = get(r, "steps");
                let items = list_items(set);
                let mut l = new_palette_list(items.clone(), w, h);
                let what = |i: &str| {
                    format!("palette run={tag} set={set} {w}x{h} steps={steps:?} step {i}")
                };
                c.check(|| what("init"), &l.view(), get(r, "v_init"));
                c.check(|| what("init"), &list_state(&l), get(r, "s_init"));
                for (i, step) in steps.split('|').enumerate() {
                    let (kind, arg) = step.split_once(':').unwrap_or((step, ""));
                    match kind {
                        "down" => l.cursor_down(),
                        "up" => l.cursor_up(),
                        "pgdown" => l.next_page(),
                        "pgup" => l.prev_page(),
                        "home" => l.select(0),
                        "end" => {
                            let n = l.visible_items().len() as i32;
                            if n > 0 {
                                l.select(n - 1);
                            }
                        }
                        "sel" => {
                            let n: i32 = arg.parse().unwrap();
                            if (n as usize) < l.visible_items().len() {
                                l.select(n);
                            }
                        }
                        "q" => apply_palette_query(&mut l, &items, arg),
                        "left" | "right" => {
                            let t = if kind == "right" { KEY_RIGHT } else { KEY_LEFT };
                            let _ = l.update(&tea::msg(KeyMsg::new(t)));
                        }
                        "key" => {
                            let _ = l.update(&list_key(arg));
                        }
                        other => panic!("{other}"),
                    }
                    let p = i.to_string();
                    c.check(|| what(&p), &l.view(), get(r, &format!("v{p}")));
                    c.check(|| what(&p), &list_state(&l), get(r, &format!("s{p}")));
                }
            }
            "default" => {
                let tag = get(r, "run");
                set_run(tag);
                let set = get(r, "set");
                let variant = get(r, "variant");
                let w: i32 = get(r, "w").parse().unwrap();
                let h: i32 = get(r, "h").parse().unwrap();
                let steps = get(r, "steps");
                let mut l = new_default_list(list_items(set), w, h, variant);
                let what = |i: &str| {
                    format!("default run={tag} set={set} variant={variant} {w}x{h} steps={steps:?} step {i}")
                };
                c.check(|| what("init"), &l.view(), get(r, "v_init"));
                c.check(|| what("init"), &list_state(&l), get(r, "s_init"));
                for (i, step) in steps.split('|').enumerate() {
                    let cmd = l.update(&list_key(step));
                    let mut quit = false;
                    for m in run_cmds(vec![cmd]) {
                        if m.downcast_ref::<QuitMsg>().is_some() {
                            quit = true;
                            continue;
                        }
                        let _ = l.update(&m);
                    }
                    let p = i.to_string();
                    c.check(|| what(&p), &l.view(), get(r, &format!("v{p}")));
                    c.check(|| what(&p), &list_state(&l), get(r, &format!("s{p}")));
                    c.check(|| what(&p), &quit.to_string(), get(r, &format!("quit{p}")));
                }
            }
            other => panic!("unknown op {other}"),
        }
    }
    c.finish("list");
}
