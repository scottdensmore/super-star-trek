//! Damage Control Schematic modal (port of
//! `pkg/tui/components/damageschematic/schematic.go`).

use sst_engine::gofmt::fmt_f;
use sst_engine::{DeviceID, EnterpriseState, NUM_DEVICES};
use sst_tuikit::ansi;
use sst_tuikit::lipgloss::{self, Style};
use sst_tuikit::tea::{self, Cmd, KeyMsg, Msg};

use crate::theme::{self, Styles, Theme};

#[cfg(test)]
mod parity_tests;
#[cfg(test)]
mod tests;

struct DeviceMeta {
    id: DeviceID,
    name: &'static str,
    impact: &'static str,
}

const DEVICES: [DeviceMeta; 8] = [
    DeviceMeta {
        id: DeviceID::Warp,
        name: "Warp Engines",
        impact: "Max Warp 0.2",
    },
    DeviceMeta {
        id: DeviceID::SRSensors,
        name: "Short-Range Sens",
        impact: "SRS Offline",
    },
    DeviceMeta {
        id: DeviceID::LRSensors,
        name: "Long-Range Sens",
        impact: "LRS Offline",
    },
    DeviceMeta {
        id: DeviceID::Phasers,
        name: "Phaser Controls",
        impact: "Phasers Locked",
    },
    DeviceMeta {
        id: DeviceID::PhotonTubes,
        name: "Photon Tubes",
        impact: "Tubes Locked",
    },
    DeviceMeta {
        id: DeviceID::DamageControl,
        name: "Damage Control",
        impact: "Repairs Slow",
    },
    DeviceMeta {
        id: DeviceID::Shields,
        name: "Shield System",
        impact: "Shields Locked",
    },
    DeviceMeta {
        id: DeviceID::Computer,
        name: "Library Computer",
        impact: "No Chart/Nav",
    },
];

/// Emitted when a dismissal key is pressed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CloseModalMsg;

/// The damage schematic modal (Go `damageschematic.Model`).
#[derive(Clone, Debug, Default)]
pub struct Model {
    width: i32,
    theme: Option<Theme>,
    devices: [f64; NUM_DEVICES],
    condition: String,
    is_docked: bool,
    repair_mult: f64,
}

struct SchematicStyles {
    base: Styles,
    border: Style,
    text_warn: Style,
}

fn get_schematic_styles(th: Option<Theme>) -> SchematicStyles {
    let base = th.unwrap_or_else(theme::default_theme).styles();
    let border_fg = base.panel.get_border_top_foreground();
    SchematicStyles {
        border: lipgloss::new_style().foreground(border_fg),
        text_warn: base.condition_red.clone(),
        base,
    }
}

/// `New`: a non-positive width defaults to 66; nil theme to the default.
/// (Go also stored a height that its view never used; the schematic sizes
/// itself to its rows.)
pub fn new(th: Option<Theme>, width: i32) -> Model {
    Model {
        width: if width <= 0 { 66 } else { width },
        theme: Some(th.unwrap_or_else(theme::default_theme)),
        repair_mult: 1.0,
        ..Default::default()
    }
}

impl Model {
    /// `SetTheme`: a nil theme is ignored.
    pub fn set_theme(&mut self, th: Option<Theme>) {
        if th.is_some() {
            self.theme = th;
        }
    }

    /// `SetState`: device damage, condition, docking and repair scaling
    /// (non-positive multipliers become 1.0).
    pub fn set_state(
        &mut self,
        enterprise: &EnterpriseState,
        condition: &str,
        is_docked: bool,
        repair_mult: f64,
    ) {
        self.devices = enterprise.devices;
        self.condition = condition.to_string();
        self.is_docked = is_docked;
        self.repair_mult = if repair_mult <= 0.0 { 1.0 } else { repair_mult };
    }

    /// `Update`: esc/enter/q/Q/d/D/space emit [`CloseModalMsg`].
    pub fn update(&mut self, msg: &Msg) -> Option<Cmd> {
        let key = msg.downcast_ref::<KeyMsg>()?;
        match key.string().as_str() {
            "esc" | "enter" | "q" | "Q" | "d" | "D" | " " | "space" => tea::cmd(|| CloseModalMsg),
            _ => None,
        }
    }

    fn format_pin(&self, dev: DeviceID, tag: &str, styles: &SchematicStyles) -> String {
        let dmg = self.devices[dev.as_i32() as usize];
        if dmg <= 0.0 {
            return styles.base.subsystem_normal.render(&format!("[{tag}: OK]"));
        }
        let scaled = fmt_f(dmg * self.repair_mult, 1);
        if dmg >= 2.0 {
            return styles.text_warn.render(&format!("[{tag}: {scaled}]"));
        }
        styles
            .base
            .subsystem_damaged
            .render(&format!("[{tag}: {scaled}]"))
    }

    /// `View`: the bordered 66x18 overlay.
    pub fn view(&self) -> String {
        let styles = get_schematic_styles(self.theme);
        let pin = |d: DeviceID, tag: &str| self.format_pin(d, tag, &styles);

        let srs_pin = pin(DeviceID::SRSensors, "SRS");
        let comp_pin = pin(DeviceID::Computer, "COMP");
        let lrs_pin = pin(DeviceID::LRSensors, "LRS");
        let phas_pin = pin(DeviceID::Phasers, "PHAS");
        let tub_pin = pin(DeviceID::PhotonTubes, "TUB");
        let shl_pin = pin(DeviceID::Shields, "SHL");
        let dam_pin = pin(DeviceID::DamageControl, "DAM");
        let warp_pin = pin(DeviceID::Warp, "WARP");

        let mut cond_text = self.condition.as_str();
        if self.is_docked {
            cond_text = "DOCKED";
        }
        if cond_text.is_empty() {
            cond_text = "GREEN";
        }

        let inner_w = self.width - 2;
        let b = &styles.border;

        let header_title = styles.base.panel_title.render("DAMAGE CONTROL SCHEMATIC");
        let title_w = ansi::string_width(&header_title);
        let top_fill = (inner_w - title_w - 1).max(0);
        let border_top = format!(
            "{}{}{}{}{}",
            b.render("┌"),
            b.render("─"),
            header_title,
            b.render(&"─".repeat(top_fill as usize)),
            b.render("┐")
        );

        let format_row = |content: &str| -> String {
            let w = ansi::string_width(content);
            let mut pad = inner_w - w;
            let mut content = content.to_string();
            if pad < 0 {
                content = ansi::truncate(&content, inner_w, "");
                pad = 0;
            }
            format!(
                "{}{}{}{}",
                b.render("│"),
                content,
                " ".repeat(pad as usize),
                b.render("│")
            )
        };

        let row1 = format!(" USS ENTERPRISE  NCC-1701                ALERT STATUS: {cond_text}");
        let row3 = format!("          .---{srs_pin}---.              {lrs_pin}");
        let row4 = format!("         /    {comp_pin}  \\             {phas_pin}");
        let row5 = format!("        |   (=) Saucer      |==={tub_pin}===.");
        let row6 = "         \\     Bridge      /                 |".to_string();
        let row7 = format!("          '---{shl_pin}---'                  |");
        let row8 = "                   \\                         |".to_string();
        let row9 = format!("                    \\==={dam_pin}======{warp_pin}");

        let repair_mult = if self.repair_mult <= 0.0 {
            1.0
        } else {
            self.repair_mult
        };

        let mut damaged_rows = Vec::new();
        for d in &DEVICES {
            let dmg = self.devices[d.id.as_i32() as usize];
            if dmg > 0.0 {
                let in_flight = dmg * repair_mult;
                let docked = in_flight * 0.25;
                let status = if dmg >= 2.0 { "OFFLINE" } else { "DAMAGED" };
                damaged_rows.push(format!(
                    " {:<18} {:<9} {:>5} SD  {:>5} SD  {:<13}",
                    d.name,
                    status,
                    fmt_f(in_flight, 1),
                    fmt_f(docked, 1),
                    d.impact
                ));
            }
        }

        let row11 = " SUBSYSTEM          STATUS    IN-FLIGHT  DOCKED  EFFECT";
        const ALL_OTHER: &str = " All other primary and tactical systems operational.";
        let (row12, row13, row14, row15) = if damaged_rows.is_empty() {
            (
                " All primary and auxiliary subsystems operational.".to_string(),
                ALL_OTHER.to_string(),
                String::new(),
                " Scotty- \"All systems purring like kittens, Captain!\"".to_string(),
            )
        } else {
            let n = damaged_rows.len();
            let row13 = if n > 1 {
                damaged_rows[1].clone()
            } else {
                ALL_OTHER.to_string()
            };
            let row14 = if n > 2 {
                damaged_rows[2].clone()
            } else if n == 2 {
                ALL_OTHER.to_string()
            } else {
                String::new()
            };
            let row15 = if n > 3 {
                format!(
                    " (+{} more damaged subsystems: view in status panel)",
                    n - 3
                )
            } else {
                " Dock at starbase for accelerated 4x damage repair.".to_string()
            };
            (damaged_rows[0].clone(), row13, row14, row15)
        };

        let row16 = " [ESC / ENTER / Q / D] Dismiss Damage Schematic";
        let bottom_fill = inner_w.max(0);
        let border_bottom = format!(
            "{}{}{}",
            b.render("└"),
            b.render(&"─".repeat(bottom_fill as usize)),
            b.render("┘")
        );

        let rows = [
            border_top,
            format_row(&row1),
            format_row(""),
            format_row(&row3),
            format_row(&row4),
            format_row(&row5),
            format_row(&row6),
            format_row(&row7),
            format_row(&row8),
            format_row(&row9),
            format_row(""),
            format_row(row11),
            format_row(&row12),
            format_row(&row13),
            format_row(&row14),
            format_row(&row15),
            format_row(row16),
            border_bottom,
        ];
        rows.join("\n")
    }
}
