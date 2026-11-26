//! Parity of `x/ansi` width/strip/truncate/cut, grapheme clustering, uniseg
//! widths and `cellbuf.Wrap` against `testdata/parity/tuikit/ansi.txt`.

mod common;

use common::{get, load, Checker};
use sst_tuikit::{ansi, cellbuf, lipgloss, unicode};
use unicode_segmentation::UnicodeSegmentation;

#[test]
fn ansi_parity() {
    let recs = load("ansi.txt");
    let mut c = Checker::default();
    for r in &recs {
        let op = get(r, "op");
        let input = get(r, "in");
        let desc = || format!("{op} {:?} {:?}", input, r);
        match op {
            "width" => {
                c.check(desc, &ansi::string_width(input).to_string(), get(r, "ansi"));
                c.check(
                    desc,
                    &lipgloss::width(input).to_string(),
                    get(r, "lipgloss"),
                );
                c.check(
                    desc,
                    &unicode::uniseg_string_width(input).to_string(),
                    get(r, "uniseg"),
                );
                c.check(desc, &ansi::strip(input), get(r, "strip"));
            }
            "clusters" => {
                let cl: Vec<String> = input.graphemes(true).map(|g| g.len().to_string()).collect();
                c.check(desc, &cl.join(","), get(r, "uax29"));
                let mut ucl = Vec::new();
                let mut rest = input;
                while !rest.is_empty() {
                    let (cluster, rr, w) = unicode::uniseg_first_grapheme_cluster(rest);
                    ucl.push(format!("{}:{}", cluster.len(), w));
                    rest = rr;
                }
                c.check(desc, &ucl.join(","), get(r, "uniseg"));
            }
            "truncate" => {
                let n: i32 = get(r, "n").parse().unwrap();
                c.check(
                    desc,
                    &ansi::truncate(input, n, get(r, "tail")),
                    get(r, "out"),
                );
            }
            "truncateleft" => {
                let n: i32 = get(r, "n").parse().unwrap();
                c.check(
                    desc,
                    &ansi::truncate_left(input, n, get(r, "tail")),
                    get(r, "out"),
                );
            }
            "cut" => {
                let n: i32 = get(r, "n").parse().unwrap();
                let m: i32 = get(r, "m").parse().unwrap();
                c.check(desc, &ansi::cut(input, n, m), get(r, "out"));
            }
            "wrap" => {
                let n: i32 = get(r, "n").parse().unwrap();
                let bp = r.get("bp").unwrap_or("");
                c.check(desc, &cellbuf::wrap(input, n, bp), get(r, "out"));
            }
            other => panic!("unknown op {other}"),
        }
    }
    c.finish("ansi");
}
