//! Interactive galactic star chart modal (port of
//! `pkg/tui/components/galacticchart/chart.go` and `math.go`).

mod math;

pub use math::{calculate_telemetry, Telemetry};

use sst_engine::gofmt::fmt_f;
use sst_engine::Coord;
use sst_tuikit::lipgloss::{self, Style, CENTER};
use sst_tuikit::tea::{
    self, Cmd, KeyMsg, Msg, WindowSizeMsg, KEY_DOWN, KEY_ENTER, KEY_ESC, KEY_LEFT, KEY_RIGHT,
    KEY_UP,
};

use crate::theme::{self, Styles, Theme};

#[cfg(test)]
mod parity_tests;
#[cfg(test)]
mod tests;

/// Emitted when the user confirms a warp to the cursor quadrant.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WarpToQuadrantMsg {
    pub dest_quad: Coord,
    pub warp: f64,
}

/// Emitted when quick-warp is attempted with a damaged library computer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WarpBlockedMsg {
    pub reason: String,
}

/// Emitted when the chart is dismissed without warping.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CloseChartMsg;

const COMPUTER_DAMAGED_REASON: &str =
    "COMPUTER DAMAGED, USE A POCKET CALCULATOR. Manual navigation required (nav q <r> <c> [warp]).";

/// The galactic chart modal (Go `galacticchart.Model`).
#[derive(Clone, Debug, Default)]
pub struct Model {
    theme: Option<Theme>,
    cursor: Coord,
    enterprise_quad: Coord,
    galaxy_chart: [[i32; 9]; 9],
    chart_discovered: [[bool; 9]; 9],
    known_bases: [[bool; 9]; 9],
    computer_damaged: bool,
    show_help: bool,
    width: i32,
    height: i32,
}

/// `New`: non-positive sizes default to 64x18; nil theme to the default.
pub fn new(th: Option<Theme>, width: i32, height: i32) -> Model {
    Model {
        theme: Some(th.unwrap_or_else(theme::default_theme)),
        cursor: [1, 1],
        enterprise_quad: [1, 1],
        width: if width <= 0 { 64 } else { width },
        height: if height <= 0 { 18 } else { height },
        ..Default::default()
    }
}

fn clamp_coord(val: i32, min: i32, max: i32) -> i32 {
    if val < min {
        min
    } else if val > max {
        max
    } else {
        val
    }
}

fn format_delta(d: i32) -> String {
    if d > 0 {
        format!("+{d}")
    } else {
        d.to_string()
    }
}

struct ChartStyles {
    base: Styles,
    text_warn: Style,
    text_muted: Style,
}

fn get_chart_styles(th: Option<Theme>) -> ChartStyles {
    let base = th.unwrap_or_else(theme::default_theme).styles();
    ChartStyles {
        text_warn: base.subsystem_damaged.clone(),
        text_muted: base.empty.clone(),
        base,
    }
}

impl Model {
    /// `Init`: no command.
    pub fn init(&self) -> Option<Cmd> {
        None
    }

    /// `SetTheme` (nil falls back to the default theme).
    pub fn set_theme(&mut self, th: Option<Theme>) {
        self.theme = Some(th.unwrap_or_else(theme::default_theme));
    }

    /// `Theme` (nil only for a zero-value model).
    pub fn theme(&self) -> Option<Theme> {
        self.theme
    }

    /// `SetSize`.
    pub fn set_size(&mut self, w: i32, h: i32) {
        self.width = w;
        self.height = h;
    }

    /// `SetState`: Enterprise quadrant (clamped to 1..8), chart data and
    /// discovery flags; closes help and moves the cursor to the Enterprise.
    pub fn set_state(
        &mut self,
        ent_quad: Coord,
        chart: [[i32; 9]; 9],
        discovered: [[bool; 9]; 9],
        known_bases: [[bool; 9]; 9],
        computer_damaged: bool,
    ) {
        self.enterprise_quad = [
            clamp_coord(ent_quad[0], 1, 8),
            clamp_coord(ent_quad[1], 1, 8),
        ];
        self.galaxy_chart = chart;
        self.chart_discovered = discovered;
        self.known_bases = known_bases;
        self.computer_damaged = computer_damaged;
        self.show_help = false;
        self.cursor = self.enterprise_quad;
    }

    /// `Cursor`.
    pub fn cursor(&self) -> Coord {
        self.cursor
    }

    /// `ShowingHelp`.
    pub fn showing_help(&self) -> bool {
        self.show_help
    }

    /// `SetCursor` (clamped to 1..8).
    pub fn set_cursor(&mut self, c: Coord) {
        self.cursor = [clamp_coord(c[0], 1, 8), clamp_coord(c[1], 1, 8)];
    }

    /// `Width`.
    pub fn width(&self) -> i32 {
        self.width
    }

    /// `Height`.
    pub fn height(&self) -> i32 {
        self.height
    }

    /// `ComputerDamaged`.
    pub fn computer_damaged(&self) -> bool {
        self.computer_damaged
    }

    /// `HitTest`: maps modal-relative coordinates to a quadrant (`None` is
    /// Go's `(Coord{}, false)`; always `None` while help is shown).
    pub fn hit_test(&self, rel_x: i32, rel_y: i32) -> Option<Coord> {
        if self.show_help || !(4..=11).contains(&rel_y) {
            return None;
        }
        let r = rel_y - 3;
        if rel_x < 7 {
            return None;
        }
        let offset = rel_x - 7;
        let c = offset / 7 + 1;
        if !(1..=8).contains(&c) || offset % 7 >= 5 {
            return None;
        }
        Some([r, c])
    }

    /// `SetShowingHelp`.
    pub fn set_showing_help(&mut self, s: bool) {
        self.show_help = s;
    }

    fn warp_cmd(&self) -> Option<Cmd> {
        if self.computer_damaged {
            return tea::cmd(|| WarpBlockedMsg {
                reason: COMPUTER_DAMAGED_REASON.into(),
            });
        }
        let dest = self.cursor;
        let telem = calculate_telemetry(self.enterprise_quad, dest);
        tea::cmd(move || WarpToQuadrantMsg {
            dest_quad: dest,
            warp: telem.recommended_warp,
        })
    }

    /// `Update`: cursor movement, Enter warp, Esc close, `?` help; window
    /// size messages are ignored.
    pub fn update(&mut self, msg: &Msg) -> Option<Cmd> {
        if msg.downcast_ref::<WindowSizeMsg>().is_some() {
            return None;
        }
        let key = msg.downcast_ref::<KeyMsg>()?;
        let s = key.string();

        if self.show_help {
            if key.typ == KEY_ESC || s == "esc" || s == "?" || s == "q" || s == "Q" {
                self.show_help = false;
                return None;
            }
            if key.typ == KEY_ENTER || s == "enter" {
                self.show_help = false;
                return self.warp_cmd();
            }
            // Any other key closes help and is then handled normally.
            self.show_help = false;
        }

        if s == "?" {
            self.show_help = true;
        } else if key.typ == KEY_UP || s == "k" || s == "K" || s == "up" {
            if self.cursor[0] > 1 {
                self.cursor[0] -= 1;
            }
        } else if key.typ == KEY_DOWN || s == "j" || s == "J" || s == "down" {
            if self.cursor[0] < 8 {
                self.cursor[0] += 1;
            }
        } else if key.typ == KEY_LEFT || s == "h" || s == "H" || s == "left" {
            if self.cursor[1] > 1 {
                self.cursor[1] -= 1;
            }
        } else if key.typ == KEY_RIGHT || s == "l" || s == "L" || s == "right" {
            if self.cursor[1] < 8 {
                self.cursor[1] += 1;
            }
        } else if key.typ == KEY_ENTER || s == "enter" {
            return self.warp_cmd();
        } else if key.typ == KEY_ESC || s == "esc" {
            return tea::cmd(|| CloseChartMsg);
        }
        None
    }

    fn render_footer(&self, styles: &ChartStyles) -> (String, String) {
        let s = &styles.base;
        if self.computer_damaged {
            let target = s.gauge_label.render("Target: ")
                + &s.prompt
                    .render(&format!("Quad [{}, {}]", self.cursor[0], self.cursor[1]));
            let distance = styles.text_warn.render("Distance: [CALC OFFLINE]");
            return (
                target + &s.gauge_label.render("  •  ") + &distance,
                styles
                    .text_warn
                    .render("Course: [CALC OFFLINE] • Warp: [CALC OFFLINE]"),
            );
        }
        let telem = calculate_telemetry(self.enterprise_quad, self.cursor);
        let target = s.gauge_label.render("Target: ")
            + &s.prompt
                .render(&format!("Quad [{}, {}]", telem.to[0], telem.to[1]));
        if telem.is_current {
            (
                target + &s.gauge_label.render("  •  Current Position"),
                s.gauge_label.render("Course: --  •  Warp: --"),
            )
        } else {
            (
                target
                    + &s.gauge_label.render(&format!(
                        "  •  Dist: {} quads (ΔR: {}, ΔC: {})",
                        fmt_f(telem.distance, 1),
                        format_delta(telem.delta_r),
                        format_delta(telem.delta_c)
                    )),
                s.gauge_label.render(&format!(
                    "Course: {} rad ({})  •  Warp: {}",
                    fmt_f(telem.course, 2),
                    telem.direction,
                    fmt_f(telem.recommended_warp, 1)
                )),
            )
        }
    }

    /// `View`: the 64x18 chart dialog (or its help guide).
    pub fn view(&self) -> String {
        let styles = get_chart_styles(self.theme);
        let s = &styles.base;

        let target_width = if self.width <= 0 { 64 } else { self.width };
        let target_height = if self.height <= 0 { 18 } else { self.height };

        let panel_style = s.panel.clone().border(s.border.clone(), &[true]);
        let mut border_h = panel_style.get_horizontal_border_size();
        if border_h == 0 {
            border_h = 2;
        }
        let padding_h = panel_style.get_horizontal_padding();
        let mut border_v = panel_style.get_vertical_border_size();
        if border_v == 0 {
            border_v = 2;
        }
        let inner_width = (target_width - border_h - padding_h).max(10);
        let width_no_borders = target_width - border_h;
        let height_no_borders = target_height - border_v;

        if self.show_help {
            return render_help_view(
                &styles,
                panel_style,
                inner_width,
                width_no_borders,
                height_no_borders,
            );
        }

        let title = s.panel_title.render("GALACTIC STAR CHART");
        let line_title = lipgloss::new_style()
            .width(inner_width)
            .align(&[CENTER])
            .render(&title);

        let mut col_header = String::from("     ");
        for col in 1..=8 {
            if col < 8 {
                col_header.push_str(&format!("  {col}    "));
            } else {
                col_header.push_str(&format!("  {col}  "));
            }
        }
        let line_col_header = s.grid_header.render(&col_header);

        let mut lines = vec![line_title, String::new(), line_col_header];
        for row in 1..=8usize {
            let mut rb = s.grid_header.render(&format!("  {row}  "));
            for col in 1..=8usize {
                let val = if self.chart_discovered[row][col] {
                    format!("{:03}", self.galaxy_chart[row][col])
                } else if self.known_bases[row][col] {
                    ".1.".to_string()
                } else {
                    "···".to_string()
                };
                let is_ent =
                    row as i32 == self.enterprise_quad[0] && col as i32 == self.enterprise_quad[1];
                let is_cur = row as i32 == self.cursor[0] && col as i32 == self.cursor[1];
                let cell = if is_cur {
                    let bracket = format!("[{val}]");
                    if is_ent {
                        s.enterprise.render(&bracket)
                    } else {
                        s.command_text.render(&bracket)
                    }
                } else if is_ent {
                    s.enterprise.render(&format!("<{val}>"))
                } else {
                    s.log_text.render(&format!(" {val} "))
                };
                rb.push_str(&cell);
                if col < 8 {
                    rb.push_str("  ");
                }
            }
            lines.push(rb);
        }

        lines.push(s.grid_header.render(&"─".repeat(inner_width as usize)));
        let (t1, t2) = self.render_footer(&styles);
        lines.push(t1);
        lines.push(t2);
        lines.push(
            styles
                .text_muted
                .render("Legend: ··· Unexplored • .1. Base • KBS (Klingon/Base/Star)"),
        );
        lines.push(if self.computer_damaged {
            styles
                .text_muted
                .render("[Enter] Disabled (Comp Offline)  [?] Help  [Esc] Close")
        } else {
            s.log_text
                .render("[Enter] Warp  [?] Help  [Arrows/HJKL] Move  [Esc] Close")
        });

        panel_style
            .width(width_no_borders)
            .height(height_no_borders)
            .render(&lines.join("\n"))
    }
}

/// `renderHelpView`: the in-modal help guide.
fn render_help_view(
    styles: &ChartStyles,
    panel_style: Style,
    inner_width: i32,
    width_no_borders: i32,
    height_no_borders: i32,
) -> String {
    let s = &styles.base;
    let title = s.panel_title.render("GALACTIC STAR CHART GUIDE");
    let line_title = lipgloss::new_style()
        .width(inner_width)
        .align(&[CENTER])
        .render(&title);
    let line_divider = s.grid_header.render(&"─".repeat(inner_width as usize));
    let ct = |t: &str| s.command_text.render(t);
    let lt = |t: &str| s.log_text.render(t);

    let help_lines = [
        line_title,
        String::new(),
        lt("The 8x8 chart maps the 64 quadrants of the galaxy:"),
        String::new(),
        ct("  ··· ") + &lt(" Unexplored quadrant (not yet scanned by sensors)"),
        ct("  .1. ") + &lt(" Starbase detected via Starfleet Command report"),
        ct("  KBS ") + &lt(" 3-digit quadrant sensor scan:"),
        lt("        • K (hundreds) = Klingon battlecruisers"),
        lt("        • B (tens)     = Federation starbases"),
        lt("        • S (units)    = Stars"),
        String::new(),
        s.enterprise.render("  <···>") + &lt(" Enterprise current position"),
        ct("  [···]") + &lt(" Targeted destination reticle"),
        line_divider,
        s.gauge_label
            .render("Telemetry displays course, distance, and recommended warp."),
        styles
            .text_muted
            .render("[?] / [Esc] Return to chart  •  [Enter] Warp to target"),
    ];
    panel_style
        .width(width_no_borders)
        .height(height_no_borders)
        .render(&help_lines.join("\n"))
}
