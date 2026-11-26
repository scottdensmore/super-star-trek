//! Root model rendering (port of `pkg/tui/view.go`).

use sst_tuikit::ansi;
use sst_tuikit::lipgloss;

use crate::model::*;
use crate::theme;

impl Model {
    /// Go `View`: the size notice below 80x24, the options or drydock overlay,
    /// otherwise the split dashboard.
    pub fn view(&self) -> String {
        if self.width < 80 || self.height < 24 {
            return self.render_size_notice();
        }
        if self.show_options {
            let overlay = self.options_modal.view();
            return lipgloss::place(
                self.width,
                self.height,
                lipgloss::CENTER,
                lipgloss::CENTER,
                &overlay,
                &[],
            );
        }
        if self.tour.as_ref().is_some_and(|t| t.borrow().in_drydock) {
            let overlay = self.drydock.view();
            return lipgloss::place(
                self.width,
                self.height,
                lipgloss::CENTER,
                lipgloss::CENTER,
                &overlay,
                &[],
            );
        }
        self.render_dashboard()
    }

    /// Go `renderSizeNotice`.
    pub fn render_size_notice(&self) -> String {
        let th = self.theme.unwrap_or_else(theme::default_theme);
        let styles = th.styles();
        let title = styles.title.render("TERMINAL WINDOW TOO SMALL");
        let dims = styles.gauge_label.render(&format!(
            "Current dimensions: {} columns x {} rows",
            self.width, self.height
        ));
        let min_req = styles
            .gauge_value
            .render("Required minimum:   80 columns x 24 rows");
        let hint = styles
            .log_text
            .render("Please resize your terminal window to continue.");
        let content = format!("{}\n\n{}\n{}\n\n{}", title, dims, min_req, hint);
        let bx = styles.panel.render(&content);
        if self.width > 0 && self.height > 0 {
            return lipgloss::place(
                self.width,
                self.height,
                lipgloss::CENTER,
                lipgloss::CENTER,
                &bx,
                &[],
            );
        }
        bx
    }

    /// Go `renderDashboard`.
    pub fn render_dashboard(&self) -> String {
        let header = self.render_header();
        let g = self.game.as_ref().map(|g| g.borrow());
        let (quad, ent_sector) = match &g {
            Some(g) => (Some(&g.current_quad), g.enterprise.sector),
            None => (None, [0, 0]),
        };
        let grid_view = self.grid.view(quad, ent_sector, self.selected_sector);
        let status_view = self.status.view(g.as_deref());
        drop(g);

        let middle = if self.width >= 115 && self.game.is_some() {
            let chart_view = self.render_mini_galactic_chart();
            lipgloss::join_horizontal(lipgloss::TOP, &[&grid_view, &status_view, &chart_view])
        } else {
            lipgloss::join_horizontal(lipgloss::TOP, &[&grid_view, &status_view])
        };

        let mut cb = self.command_bar.clone();
        if self.width > 0 {
            cb.set_width(self.width);
        }
        let command_bar_view = cb.view();

        let dashboard =
            lipgloss::join_vertical(lipgloss::LEFT, &[&header, &middle, &command_bar_view]);

        let modal_view = match self.active_modal {
            MODAL_TARGET_LOCK => self.target_lock.view(),
            MODAL_COMMAND_PALETTE => self.command_palette.view(),
            MODAL_SAVE_BROWSER => self.save_browser.view(),
            MODAL_GALACTIC_CHART => self.galactic_chart.view(),
            MODAL_DAMAGE_SCHEMATIC => self.damage_schematic.view(),
            MODAL_HALL_OF_FAME => self.hall_of_fame.view(),
            MODAL_MANUAL => self.manual.view(),
            MODAL_SCENARIO => self.scenario_modal.view(),
            _ => return dashboard,
        };
        composite_overlay(&dashboard, &modal_view, self.width, self.height)
    }

    /// Go `renderHeader`.
    pub fn render_header(&self) -> String {
        let th = self.theme.unwrap_or_else(theme::default_theme);
        let mut title = String::from("★ SUPER STAR TREK ★  USS ENTERPRISE");
        if let Some(t) = &self.tour {
            let t = t.borrow();
            let sec_idx = t.current_sector_index + 1;
            let mut total = t.sectors.len();
            if total == 0 {
                total = 4;
            }
            title += &format!("  TOUR: SEC {}/{}", sec_idx, total);
        } else {
            title += " NCC-1701";
        }
        let theme_info = format!("[Theme: {} (F2)]", th.name().to_uppercase());
        let w = self.width.max(80);
        th.format_header(&title, &theme_info, w)
    }

    /// Go `renderMiniGalacticChart`: the 8x8 chart and mission ops panel shown
    /// beside the telemetry when the terminal is at least 115 columns wide.
    pub fn render_mini_galactic_chart(&self) -> String {
        let Some(g) = &self.game else {
            return String::new();
        };
        let g = g.borrow();
        let th = self.theme.unwrap_or_else(theme::default_theme);
        let styles = th.styles();
        let mut b = String::with_capacity(512);

        let title_text = "GALACTIC STAR CHART [K-B-S]";
        let title_padding = ((43 - lipgloss::width(title_text)) / 2).max(0);
        b += &styles
            .panel_title
            .render(&(" ".repeat(title_padding as usize) + title_text));

        b.push('\n');
        let mut col_hdr = String::from("   ");
        for col in 1..=8 {
            col_hdr += &format!("  {}  ", col);
        }
        b += &styles.grid_header.render(&col_hdr);

        let mut explored = 0;
        for r in 1..=8usize {
            b.push('\n');
            b += &styles.grid_header.render(&format!(" {} ", r));
            for c in 1..=8usize {
                let is_ent = r as i32 == g.enterprise.quad[0] && c as i32 == g.enterprise.quad[1];
                let discovered = g.chart_discovered[r][c];
                if discovered {
                    explored += 1;
                }
                if is_ent {
                    let val_str = if discovered {
                        format!("{:03}", g.galaxy_chart[r][c])
                    } else {
                        "···".to_string()
                    };
                    b += &styles.enterprise.render(&format!("<{}>", val_str));
                } else if discovered {
                    let val = g.galaxy_chart[r][c];
                    let val_str = format!(" {:03} ", val);
                    if val >= 100 {
                        b += &styles.subsystem_damaged.render(&val_str);
                    } else if (val % 100) / 10 > 0 {
                        b += &styles.command_text.render(&val_str);
                    } else {
                        b += &styles.log_text.render(&val_str);
                    }
                } else if g.chart_known_bases[r][c] {
                    b += &styles.command_text.render(" >B< ");
                } else {
                    b += &styles.empty.render(" ··· ");
                }
            }
        }

        b.push('\n');
        b += &styles.grid_header.render(&"─".repeat(43));

        b.push('\n');
        let kl = styles.gauge_label.render("Klingons Left: ");
        let kv = styles
            .gauge_value
            .render(&format!("{:2}", g.remaining_klingons));
        let bl = styles.gauge_label.render("    Starbases: ");
        let bv = styles
            .gauge_value
            .render(&format!("{}", g.remaining_starbases));
        b += &(kl + &kv + &bl + &bv);

        b.push('\n');
        let el = styles.gauge_label.render("Explored: ");
        let ev = styles.gauge_value.render(&format!("{:2}/64", explored));
        let cl = styles.gauge_label.render("        Casualties: ");
        let cv = styles
            .gauge_value
            .render(&format!("{}", g.metrics.casualties));
        b += &(el + &ev + &cl + &cv);

        b.push('\n');
        b += &styles
            .gauge_label
            .render("[Ctrl+G] Interactive Map   [T] Target Lock");

        styles.panel.render(&b)
    }
}

/// Go `compositeOverlay`: centers `overlay` over `background` using
/// ANSI-aware cutting so the background stays visible around it.
pub fn composite_overlay(
    background: &str,
    overlay: &str,
    total_width: i32,
    total_height: i32,
) -> String {
    let mut bg_lines: Vec<String> = background.split('\n').map(String::from).collect();
    let overlay_lines: Vec<&str> = overlay.split('\n').collect();

    let mut overlay_width = 0;
    for l in &overlay_lines {
        let w = ansi::string_width(l);
        if w > overlay_width {
            overlay_width = w;
        }
    }
    let overlay_height = overlay_lines.len() as i32;

    let mut total_height = total_height;
    let mut total_width = total_width;
    if total_height < overlay_height {
        total_height = overlay_height;
    }
    if total_height < bg_lines.len() as i32 {
        total_height = bg_lines.len() as i32;
    }
    if total_width < overlay_width {
        total_width = overlay_width;
    }
    if total_width <= 0 {
        for l in &bg_lines {
            let w = ansi::string_width(l);
            if w > total_width {
                total_width = w;
            }
        }
    }

    while (bg_lines.len() as i32) < total_height {
        bg_lines.push(String::new());
    }

    let start_x = ((total_width - overlay_width) / 2).max(0);
    let start_y = ((total_height - overlay_height) / 2).max(0);

    let mut result = Vec::with_capacity(bg_lines.len());
    for (y, bg_line) in bg_lines.iter().enumerate() {
        let y = y as i32;
        if y < start_y || y >= start_y + overlay_height {
            result.push(bg_line.clone());
            continue;
        }
        let mut modal_line = overlay_lines[(y - start_y) as usize].to_string();
        let mlw = ansi::string_width(&modal_line);
        if mlw < overlay_width {
            modal_line += &" ".repeat((overlay_width - mlw) as usize);
        }

        let bg_width = ansi::string_width(bg_line);
        let left = if bg_width <= start_x {
            bg_line.clone() + &" ".repeat((start_x - bg_width) as usize)
        } else {
            let mut l = ansi::truncate(bg_line, start_x, "");
            let lw = ansi::string_width(&l);
            if lw < start_x {
                l += &" ".repeat((start_x - lw) as usize);
            }
            l
        };

        let right = if bg_width > start_x + overlay_width {
            ansi::truncate_left(bg_line, start_x + overlay_width, "")
        } else {
            String::new()
        };

        result.push(format!("{}\x1b[0m{}{}", left, modal_line, right));
    }
    result.join("\n")
}
